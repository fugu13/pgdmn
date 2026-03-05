# Bypassing JSONB for PG-to-DMN Data Passing

## Problem Statement

The current data path for `dmn_eval` and `feel_eval` requires callers to pack input data into JSONB:

    SELECT dmn_eval(model, 'Eligibility',
        jsonb_build_object('Age', age, 'Income', income))
    FROM applicants;

This has three serialization hops per row:

1. PG native types -> JSONB binary (via `jsonb_build_object`)
2. JSONB binary -> `serde_json::Value` (pgrx deserializes the JSONB datum)
3. `serde_json::Value` -> `FeelContext` with `Value::Number`, `Value::String`, etc.

Each hop involves allocation, copying, and type-tag overhead. For bulk evaluation (thousands of rows), this dominates runtime.

## Candidate Approaches

Status key: NOT STARTED | INVESTIGATING | VIABLE | NOT VIABLE | NEEDS PROTOTYPE

| ID | Approach | Status | Plausibility |
|----|----------|--------|-------------|
| A  | Composite type (RECORD) via PgHeapTuple | IMPLEMENTED | High |
| B  | Direct Datum-to-FEEL conversion (keep JSONB, skip serde_json) | NOT STARTED | High |
| C  | SPI batch table function | NOT STARTED | Medium |
| D  | Individual typed parameters with codegen | NOT STARTED | Medium |
| E  | HSTORE input | NOT STARTED | Low |
| F  | Variadic text pairs | NOT STARTED | Low |
| G  | Dynamic SQL wrapper generation per DMN model | NOT STARTED | Medium |

---

## Investigation A: Composite Type (RECORD) via PgHeapTuple

**Status: IMPLEMENTED**

Implemented as `dmn_eval_record` and `feel_eval_record` functions that accept `composite_type!("record")`. Conversion logic is in `convert.rs` via `pg_datum_to_feel` and `tuple_to_context`. Supported PG types: bool, int2/4/8, float4/8, numeric, text, varchar, date, timestamp, interval. Mixed intervals (both months and days/micros) error. Callers must cast via a named type for meaningful field names.

### Concept

Accept a PostgreSQL composite type or anonymous `RECORD` as the input parameter instead of JSONB. pgrx exposes `PgHeapTuple` which can read individual fields by name or index, with full type information from `TupleDesc`. Each field's PG datum can be converted directly to a FEEL `Value` without any JSON intermediary.

Caller usage would look like:

    SELECT dmn_eval_record(model, 'Eligibility',
        ROW(age, income)::applicant_input)
    FROM applicants;

Or with an anonymous record:

    SELECT dmn_eval_record(model, 'Eligibility',
        ROW(age, income))
    FROM applicants;

### How PgHeapTuple Works

pgrx's `PgHeapTuple<'a, AllocatedByPostgres>` wraps a PG `HeapTuple` pointer and its `TupleDesc`. Key APIs:

- `attributes()` returns an iterator of `(NonZeroUsize, &FormData_pg_attribute)` giving field index, name, type OID, and nullability
- `get_by_name::<T>(name)` extracts a field value as Rust type `T` (requires `T: FromDatum`)
- `get_by_index::<T>(attno)` same but by ordinal position
- `len()` gives number of attributes

The `composite_type!("TypeName")` macro in a `#[pg_extern]` signature tells pgrx to accept a named composite type and present it as `PgHeapTuple`.

### Implementation Sketch

```rust
use pgrx::prelude::*;
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::pg_sys::Oid;
use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelNumber, Name};

/// Convert a PG datum (by type OID) to a FEEL Value.
fn pg_attrib_to_feel(
    tuple: &PgHeapTuple<'_, pgrx::AllocatedByPostgres>,
    attno: std::num::NonZeroUsize,
    atttypid: Oid,
) -> Value {
    // pg_sys type OIDs are constants like BOOLOID, INT4OID, etc.
    match atttypid {
        pg_sys::BOOLOID => match tuple.get_by_index::<bool>(attno) {
            Ok(Some(v)) => Value::Boolean(v),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT2OID => match tuple.get_by_index::<i16>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(v as i64)),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT4OID => match tuple.get_by_index::<i32>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(v as i64)),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT8OID => match tuple.get_by_index::<i64>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(v)),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::FLOAT4OID => match tuple.get_by_index::<f32>(attno) {
            Ok(Some(v)) => {
                let s = v.to_string();
                s.parse::<FeelNumber>()
                    .map(Value::Number)
                    .unwrap_or_else(|_| Value::Null(Some(format!("bad float4: {v}"))))
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::FLOAT8OID => match tuple.get_by_index::<f64>(attno) {
            Ok(Some(v)) => {
                let s = v.to_string();
                s.parse::<FeelNumber>()
                    .map(Value::Number)
                    .unwrap_or_else(|_| Value::Null(Some(format!("bad float8: {v}"))))
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::NUMERICOID => match tuple.get_by_index::<pgrx::AnyNumeric>(attno) {
            Ok(Some(v)) => {
                let s = v.to_string();
                s.parse::<FeelNumber>()
                    .map(Value::Number)
                    .unwrap_or_else(|_| Value::Null(Some(format!("bad numeric: {s}"))))
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::TEXTOID | pg_sys::VARCHAROID => {
            match tuple.get_by_index::<String>(attno) {
                Ok(Some(v)) => Value::String(v),
                Ok(None) => Value::Null(None),
                Err(e) => Value::Null(Some(format!("{e}"))),
            }
        }
        pg_sys::DATEOID => match tuple.get_by_index::<pgrx::datum::Date>(attno) {
            // Would need to convert pgrx::Date -> FeelDate
            // FeelDate construction requires year/month/day
            Ok(Some(_d)) => {
                // d.year(), d.month(), d.day() -> FeelDate::new(y, m, d)
                todo!("Date conversion")
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::TIMESTAMPOID => match tuple.get_by_index::<pgrx::datum::Timestamp>(attno) {
            Ok(Some(_ts)) => {
                todo!("Timestamp conversion")
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        _ => Value::Null(Some(format!("unsupported PG type OID: {}", atttypid.as_u32()))),
    }
}

/// Convert a PgHeapTuple to a FeelContext by iterating its attributes.
fn tuple_to_context(
    tuple: &PgHeapTuple<'_, pgrx::AllocatedByPostgres>,
) -> FeelContext {
    let mut ctx = FeelContext::new();
    for (attno, attr) in tuple.attributes() {
        let name_cstr = unsafe {
            std::ffi::CStr::from_ptr(attr.attname.data.as_ptr())
        };
        let name_str = name_cstr.to_str().unwrap_or("?");
        let feel_name = Name::from(name_str);
        let value = pg_attrib_to_feel(tuple, attno, Oid::from(attr.atttypid));
        ctx.set_entry(&feel_name, value);
    }
    ctx
}

/// Evaluate a DMN invocable using a composite-type record as input.
#[pg_extern(immutable, parallel_safe)]
fn dmn_eval_record(
    model: DmnModel,
    invocable: &str,
    input: Option<pgrx::composite_type!("record")>,
) -> pgrx::JsonB {
    let evaluator = get_or_build_evaluator(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("{}", e));
    let ctx = match input {
        Some(tuple) => tuple_to_context(&tuple),
        None => FeelContext::new(),
    };
    let result = evaluator.evaluate_invocable(
        &model.namespace, &model.name, invocable, &ctx
    );
    pgrx::JsonB(feel_to_json(&result))
}
```

### Advantages

- Eliminates all three serialization hops: PG datums are read directly and converted to FEEL values in one step
- Preserves full PG type information (no loss of numeric precision through JSON floats)
- Natural SQL syntax: `ROW(col1, col2)` or a named composite type
- The field-name-to-FEEL-name mapping is automatic from the composite type's attribute names

### Limitations

- **Named composite type required for named fields.** Anonymous `ROW()` in PG gives fields names like `f1`, `f2`, etc., not the meaningful names DMN expects. Callers would need to `CREATE TYPE` first or use a named record type. This could be mitigated by a helper function `dmn_create_input_type(model, invocable)` that creates the composite type automatically.
- **Type OID dispatch is manual.** Need a match arm for every PG type we want to support. The list above covers the common ones but would need to grow for `interval`, `time`, `timestamptz`, `jsonb` (nested), arrays, etc.
- **pgrx composite_type! requires a type name at compile time.** The string `"record"` may or may not work for anonymous records. May need to use raw `pg_sys::HeapTupleData` access instead, which means more unsafe code.

### Verdict

This is the most promising approach for eliminating serialization overhead. The main practical issue is the requirement for a named composite type matching the DMN model's input schema. A companion function that creates the type from DMN metadata would make this ergonomic.

---

## Investigation B: Direct Datum-to-FEEL Conversion (Skip serde_json)

**Status: VIABLE**

### Concept

Keep the JSONB input signature but skip the `serde_json::Value` intermediate. Instead of letting pgrx deserialize JSONB into `serde_json::Value`, access the raw JSONB binary (a `varlena`/`Jsonb` struct in PG) and parse it directly into FEEL values.

PostgreSQL's JSONB is stored in a binary format with type tags. The structure is:
- A header with count of key-value pairs
- A sorted array of key entries (offset + length + type)
- Contiguous value data

### How JSONB Binary Format Works

PG's JSONB internal format (`JsonbContainer`) stores:
- `header`: flags + count of entries
- For objects: sorted key-offset pairs, then key strings, then values
- Each value has a 4-byte header encoding its type (null, string, number, bool true, bool false, container)
- Numbers are stored as PG `Numeric` in the binary data

### Implementation Sketch

There are two sub-approaches:

**Sub-approach B1: Use pgrx's JsonB but iterate with pg_sys JSONB functions**

```rust
use pgrx::pg_sys;

/// Walk a PG JSONB value and produce FEEL values directly,
/// without materializing serde_json::Value.
unsafe fn jsonb_to_feel(jb: *mut pg_sys::Jsonb) -> Value {
    let container = &(*jb).root;
    // Use PG's JsonbIterator API to walk the structure
    let mut it: *mut pg_sys::JsonbIterator = std::ptr::null_mut();
    let mut val = pg_sys::JsonbValue::default();
    it = pg_sys::JsonbIteratorInit(container);

    // The iterator yields tokens: WJB_BEGIN_OBJECT, WJB_KEY, WJB_VALUE,
    // WJB_END_OBJECT, WJB_BEGIN_ARRAY, WJB_ELEM, WJB_END_ARRAY
    // We can recursively build FEEL values from these tokens.
    jsonb_iter_to_feel(&mut it, &mut val)
}

unsafe fn jsonb_iter_to_feel(
    it: &mut *mut pg_sys::JsonbIterator,
    val: &mut pg_sys::JsonbValue,
) -> Value {
    let token = pg_sys::JsonbIteratorNext(it, val, false);
    match token as u32 {
        pg_sys::JsonbIteratorToken_WJB_BEGIN_OBJECT => {
            let mut ctx = FeelContext::new();
            loop {
                let t = pg_sys::JsonbIteratorNext(it, val, false);
                if t as u32 == pg_sys::JsonbIteratorToken_WJB_END_OBJECT {
                    break;
                }
                // t == WJB_KEY, val contains the key string
                let key = extract_jsonb_string(val);
                // Next token is the value
                let feel_val = jsonb_iter_to_feel(it, val);
                ctx.set_entry(&Name::from(key.as_str()), feel_val);
            }
            Value::Context(ctx)
        }
        pg_sys::JsonbIteratorToken_WJB_BEGIN_ARRAY => {
            let mut items = vec![];
            loop {
                let t = pg_sys::JsonbIteratorNext(it, val, false);
                if t as u32 == pg_sys::JsonbIteratorToken_WJB_END_ARRAY {
                    break;
                }
                items.push(jsonb_scalar_to_feel(val));
            }
            Value::List(items)
        }
        _ => jsonb_scalar_to_feel(val),
    }
}

unsafe fn jsonb_scalar_to_feel(val: &pg_sys::JsonbValue) -> Value {
    match val.type_ as u32 {
        pg_sys::jbvType_jbvNull => Value::Null(None),
        pg_sys::jbvType_jbvBool => Value::Boolean(val.val.boolean),
        pg_sys::jbvType_jbvNumeric => {
            // val.val.numeric is a PG Numeric pointer
            // Convert to string via PG's numeric_out, then parse to FeelNumber
            let numeric_str = numeric_to_string(val.val.numeric);
            numeric_str.parse::<FeelNumber>()
                .map(Value::Number)
                .unwrap_or_else(|_| Value::Null(Some(format!("bad numeric"))))
        }
        pg_sys::jbvType_jbvString => {
            let s = std::str::from_utf8(
                std::slice::from_raw_parts(val.val.string.val as *const u8, val.val.string.len as usize)
            ).unwrap_or("").to_string();
            Value::String(s)
        }
        _ => Value::Null(Some("unknown jsonb type".into())),
    }
}
```

**Sub-approach B2: Use a lighter JSON parser (simd-json or serde_json from bytes)**

Less invasive: instead of parsing JSONB via PG's C API, get the JSONB bytes via pgrx, but use a zero-copy JSON parser that produces FEEL values directly without a `serde_json::Value` tree.

This would require writing a custom `serde::Deserializer` for FEEL values, or using `serde_json::StreamDeserializer`.

### Advantages

- No API change: callers still pass JSONB
- Eliminates the `serde_json::Value` intermediate allocation (hop 2)
- Sub-approach B1 also eliminates hop 2's parsing entirely since PG already has the JSONB in binary form
- Numbers stay as PG Numeric through the whole path (no f64 precision loss)

### Limitations

- Still requires hop 1 (`jsonb_build_object`): PG must still construct the JSONB binary from native columns
- Sub-approach B1 requires significant unsafe code and deep knowledge of PG's JSONB internal API (which is technically internal and could change)
- The JSONB C API types (`JsonbIterator`, `JsonbValue`, etc.) may not be fully exposed in pgrx's `pg_sys` bindings
- Moderate complexity for moderate gain: saves one allocation layer but not the fundamental serialization

### Verdict

Viable as an incremental optimization that doesn't change the public API. The main win is eliminating the `serde_json::Value` intermediate tree. Best combined with approach A (offer both JSONB and RECORD inputs). The unsafe PG JSONB API approach (B1) is riskier but faster; the custom deserializer approach (B2) is safer but still involves some parsing.

---

## Investigation C: SPI Batch Table Function

**Status: VIABLE (with caveats)**

### Concept

Instead of calling `dmn_eval` per-row (which PG's executor does by evaluating the function for each tuple), provide a set-returning function that accepts a SQL query string, executes it via SPI, and processes all rows internally. This amortizes per-call overhead and can batch-convert tuples.

    SELECT * FROM dmn_eval_query(
        model,
        'Eligibility',
        'SELECT age AS "Age", income AS "Income" FROM applicants'
    );

### Implementation Sketch

```rust
#[pg_extern(immutable)]
fn dmn_eval_query(
    model: DmnModel,
    invocable: &str,
    query: &str,
) -> TableIterator<'static, (name!(input_row, pgrx::JsonB), name!(result, pgrx::JsonB))> {
    let evaluator = get_or_build_evaluator(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("{}", e));

    let results = Spi::connect(|client| {
        let rows = client.select(query, None, &[]).unwrap();
        let mut output = Vec::new();

        for row in rows {
            // Build FeelContext from the SPI tuple
            // SPI gives us access to TupleDesc + HeapTuple for each row
            // We can read typed columns directly
            let mut ctx = FeelContext::new();

            // SPI rows have get_by_name which returns typed values
            // But we need to know column names and types dynamically...
            // SPI TupleTable gives us column metadata

            let result = evaluator.evaluate_invocable(
                &model.namespace, &model.name, invocable, &ctx
            );
            output.push((
                pgrx::JsonB(serde_json::Value::Null), // placeholder for input
                pgrx::JsonB(feel_to_json(&result)),
            ));
        }
        output
    });

    TableIterator::new(results)
}
```

### Advantages

- Batch processing: PG executes the inner query once, we iterate results in Rust
- Can access SPI tuples with typed column reads (avoiding JSONB entirely)
- Column names from the SELECT aliases map directly to DMN input names
- Could potentially reuse the FeelContext allocation across rows

### Limitations

- **SQL injection risk**: accepting a query string is inherently dangerous. Would need parameterized queries or a restricted API.
- **SPI overhead**: SPI itself has overhead (snapshot management, memory context switching). For large result sets, SPI buffers all rows in memory.
- **Awkward API**: passing SQL as a string is unusual for PG functions. Users expect to write `SELECT f(col) FROM table`, not to pass the query as an argument.
- **Cannot be used in WHERE clauses or joins**: unlike a scalar function, this returns a set and changes query composition patterns.
- **pgrx SPI row access**: pgrx's SPI result rows (`SpiHeapTupleData`) support `get_by_name::<T>()` but you need to know the Rust type at compile time. Dynamic type dispatch from column metadata would require the same OID-matching logic as approach A.

### Verdict

Viable but ergonomically awkward. The query-string input pattern is unusual and raises injection concerns. The SPI approach works best as an internal optimization strategy (used behind the scenes) rather than a user-facing API. A better variant would be a function that accepts a cursor name rather than a raw query string.

---

## Investigation D: Individual Typed Parameters with Codegen

**Status: VIABLE (limited scope)**

### Concept

Instead of passing all inputs as a single JSONB object, accept them as individual function parameters with their native PG types. Since DMN models have known input schemas, generate specialized functions per model.

    -- Generated function specific to the LoanEligibility model:
    SELECT dmn_loan_eligibility(age, income) FROM applicants;

    -- Or a generic approach with up to N named params:
    SELECT dmn_eval_2(model, 'Eligibility',
        'Age', age::text,
        'Income', income::text
    ) FROM applicants;

### Implementation Sketch (Generic)

```rust
/// Evaluate with up to 3 named text parameters.
/// Each parameter is a (name, value) pair passed as text.
#[pg_extern(immutable, parallel_safe)]
fn dmn_eval_params(
    model: DmnModel,
    invocable: &str,
    key1: &str, val1: &str,
    key2: default!(Option<&str>, "NULL"), val2: default!(Option<&str>, "NULL"),
    key3: default!(Option<&str>, "NULL"), val3: default!(Option<&str>, "NULL"),
) -> pgrx::JsonB {
    let evaluator = get_or_build_evaluator(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("{}", e));

    let mut ctx = FeelContext::new();
    // All values are text; FEEL's type coercion handles conversion
    ctx.set_entry(&Name::from(key1), Value::String(val1.to_string()));
    if let (Some(k), Some(v)) = (key2, val2) {
        ctx.set_entry(&Name::from(k), Value::String(v.to_string()));
    }
    if let (Some(k), Some(v)) = (key3, val3) {
        ctx.set_entry(&Name::from(k), Value::String(v.to_string()));
    }

    let result = evaluator.evaluate_invocable(
        &model.namespace, &model.name, invocable, &ctx
    );
    pgrx::JsonB(feel_to_json(&result))
}
```

### Implementation Sketch (Generated per model)

```rust
/// A function that introspects a DMN model and creates a PG function
/// with the correct signature via dynamic SQL.
#[pg_extern]
fn dmn_create_function(model: DmnModel, invocable: &str) -> String {
    // Read the DMN model's input data requirements
    // Generate CREATE FUNCTION SQL with typed parameters
    // The body calls dmn_eval_record or dmn_eval internally
    let sql = format!(
        "CREATE OR REPLACE FUNCTION dmn_{invocable}(age integer, income numeric)
         RETURNS jsonb AS $$
           SELECT dmn_eval('{model_xml}'::dmn_model, '{invocable}',
                  jsonb_build_object('age', age, 'income', income))
         $$ LANGUAGE sql IMMUTABLE PARALLEL SAFE"
    );
    sql
}
```

### Advantages

- Zero serialization for scalar types when using individual typed params
- Generated functions give the best possible ergonomics: `SELECT loan_eligible(age, income)`
- PG's query planner can reason about individual parameter types

### Limitations

- **Fixed arity**: PG doesn't support truly variadic heterogeneous arguments. Must define overloads for different parameter counts (dmn_eval_1, dmn_eval_2, ..., dmn_eval_N) or use a code generator.
- **Text coercion loses types**: the generic version passes everything as text, relying on FEEL to parse numbers from strings. This works but is slower than direct numeric conversion.
- **Generated functions are fragile**: if the DMN model changes, generated functions become stale. Needs a management layer.
- **Namespace pollution**: one generated function per (model, invocable) pair.

### Verdict

The generated-function approach is the most ergonomic end state but requires a management layer. The generic text-pair approach works as a quick win for simple models with few inputs but doesn't truly eliminate serialization overhead - it just shifts from JSON to text parsing.

---

## Investigation E: HSTORE Input

**Status: NOT VIABLE**

### Concept

Use PostgreSQL's `hstore` extension as a lighter-weight key-value format instead of JSONB. HSTORE stores flat key-value pairs as text, with a simpler binary format than JSONB.

    SELECT dmn_eval_hstore(model, 'Eligibility',
        hstore(ARRAY['Age', 'Income'], ARRAY[age::text, income::text]))
    FROM applicants;

### Analysis

HSTORE's binary format is simpler than JSONB:
- All keys and values are text (no type tags)
- Flat structure (no nesting)
- Stored as a sorted array of (key, value) text pairs

However:
- **All values are text**, requiring FEEL to parse numbers, dates, etc. from strings. This is the same problem as approach D's generic variant.
- **Requires hstore extension** to be installed (`CREATE EXTENSION hstore`), adding a dependency.
- **No nesting**: FEEL contexts can be nested (context within context). HSTORE cannot represent this.
- **pgrx does not have built-in hstore support**. Would need to access hstore's C functions via `pg_sys` or parse the text representation manually.
- **The construction overhead is similar**: `hstore(ARRAY[...], ARRAY[...::text])` still requires type casting and array construction.

### Verdict

Not viable. HSTORE trades JSONB's type richness for a format that's only marginally simpler to parse, while requiring all values to be text. The added dependency on the hstore extension and lack of nesting support make this strictly worse than approaches A or B.

---

## Investigation F: Variadic Text Pairs

**Status: NOT VIABLE**

### Concept

Use PG's `VARIADIC` to accept alternating key-value pairs as a text array:

    SELECT dmn_eval_variadic(model, 'Eligibility',
        VARIADIC ARRAY['Age', '30', 'Income', '75000'])
    FROM applicants;

pgrx supports `VariadicArray<'a, T>` where T implements `FromDatum`.

### Implementation Sketch

```rust
#[pg_extern(immutable, parallel_safe)]
fn dmn_eval_variadic(
    model: DmnModel,
    invocable: &str,
    args: pgrx::VariadicArray<&str>,
) -> pgrx::JsonB {
    let evaluator = get_or_build_evaluator(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("{}", e));

    let mut ctx = FeelContext::new();
    let items: Vec<Option<&str>> = args.iter().collect();
    for chunk in items.chunks(2) {
        if let [Some(key), Some(val)] = chunk {
            // All values are strings; FEEL must parse them
            ctx.set_entry(&Name::from(*key), Value::String(val.to_string()));
        }
    }

    let result = evaluator.evaluate_invocable(
        &model.namespace, &model.name, invocable, &ctx
    );
    pgrx::JsonB(feel_to_json(&result))
}
```

### Limitations

- **All values must be the same PG type** (text), because `VARIADIC` accepts an array of one type. No way to mix integers, numerics, and text.
- **Requires explicit `::text` casts** at the call site for non-text columns.
- **FEEL must parse every value from text**, which is slower than direct numeric conversion.
- **No compile-time validation** that keys match the DMN model's inputs.
- **Awkward syntax** with the ARRAY literal or VARIADIC keyword.

### Verdict

Not viable. Same fundamental limitation as HSTORE: all values become text. The syntax is more awkward than JSONB with no meaningful performance benefit.

---

## Investigation G: Dynamic SQL Wrapper Generation per DMN Model

**Status: VIABLE**

### Concept

Provide a PG function that inspects a DMN model's input requirements and generates a typed SQL wrapper function. This is a higher-level version of approach D that automates the codegen.

    -- One-time setup:
    SELECT dmn_generate_function(
        dmn_load('<xml>...'), 'Eligibility', 'evaluate_eligibility'
    );

    -- Then use the generated function directly:
    SELECT evaluate_eligibility(age, income) FROM applicants;

### Implementation Sketch

```rust
#[pg_extern]
fn dmn_generate_function(
    model: DmnModel,
    invocable: &str,
    function_name: &str,
) -> String {
    // 1. Parse the DMN model to find input data requirements
    let defs = dsntk_model::parse(&model.xml).unwrap();
    // 2. Find the named invocable and its required inputs
    //    Each inputData has a name and typeRef (number, string, boolean, date)
    // 3. Map DMN typeRefs to PG types:
    //    number -> numeric, string -> text, boolean -> boolean,
    //    date -> date, date and time -> timestamp
    // 4. Generate CREATE FUNCTION SQL

    // Example output for the LoanEligibility model:
    let sql = format!(r#"
        CREATE OR REPLACE FUNCTION {function_name}(
            "Age" numeric,
            "Income" numeric
        ) RETURNS jsonb
        LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
            SELECT dmn_eval(
                dmn_load('{xml}'),
                '{invocable}',
                jsonb_build_object('Age', "Age", 'Income', "Income")
            )
        $$
    "#, xml = model.xml.replace('\'', "''"));

    // Execute the SQL to create the function
    Spi::run(&sql).unwrap();

    format!("Created function {function_name}")
}
```

### Better Variant: Use Approach A Internally

If approach A (composite type) is implemented, the generated wrapper could bypass JSONB entirely:

```sql
CREATE OR REPLACE FUNCTION evaluate_eligibility(
    "Age" numeric,
    "Income" numeric
) RETURNS jsonb
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT dmn_eval_record(
        dmn_load('...'),
        'Eligibility',
        ROW("Age", "Income")::eligibility_input
    )
$$;
```

But this still requires a named composite type. The generation function could create both the type and the wrapper function.

### Advantages

- Best possible ergonomics for end users: typed parameters, natural function calls
- Generated at model registration time, not at query time
- Can be combined with any underlying data-passing mechanism (JSONB, RECORD, etc.)
- PG's function resolution handles overloading naturally

### Limitations

- **Stale functions**: if the DMN model changes, the generated function must be regenerated.
- **Schema management**: generated functions and types accumulate and need lifecycle management.
- **DMN type mapping is imperfect**: DMN's type system (number, string, boolean, date, time, duration, context, list) doesn't map 1:1 to PG types. Nested contexts and lists need JSONB fallback.
- **Requires SPI for DDL execution**, which may not be allowed in all contexts (e.g., parallel workers).

### Verdict

Viable as a convenience layer built on top of approaches A or B. Does not independently solve the serialization problem - it automates the creation of typed wrappers that use one of the other approaches internally.

---

## Recommendations

### Near-term (smallest change, biggest win)

Implement **approach A** (composite type via PgHeapTuple) as `dmn_eval_record`. This eliminates all three serialization hops and provides the foundation for other optimizations. The main deliverable is:

1. A `pg_datum_to_feel(tuple, attno, type_oid) -> Value` conversion function
2. A `tuple_to_context(tuple) -> FeelContext` iterator
3. A `dmn_eval_record(model, invocable, record)` pg_extern function
4. Same pattern for `feel_eval_record(expression, record)`

### Medium-term (incremental optimization)

Implement **approach B** (direct JSONB-to-FEEL) as an optimization for the existing JSONB path. This benefits callers who already have JSONB data or prefer the JSONB API.

### Long-term (ergonomics)

Implement **approach G** (generated wrappers) to give end users the most natural SQL experience. This builds on approach A.
