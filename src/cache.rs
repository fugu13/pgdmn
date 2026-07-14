use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{Evaluator, FeelScope, FeelType, Name};
use dsntk_model_evaluator::ModelEvaluator;

use crate::types::dmn_model::DmnModel;

type RapidBuildHasher = std::hash::BuildHasherDefault<rapidhash::RapidInlineHasher>;

/// Second seed for the 128-bit content hash (the first pass uses
/// `rapidhash::RAPID_SEED`). Any constant distinct from `RAPID_SEED` works;
/// this is the 64-bit golden-ratio constant.
const SECOND_SEED: u64 = 0x9e37_79b9_7f4a_7c15;

/// 128-bit content hash: two independently seeded 64-bit rapidhash passes.
///
/// Collision reasoning: rapidhash is a well-mixed 64-bit hash, and two passes
/// with independent seeds behave as independent hash functions, so two
/// distinct inputs collide in all 128 bits with probability ~2^-128 —
/// negligible for any number of distinct models or context shapes a backend
/// will ever see. This is why cache probes keyed on these digests skip byte
/// comparison of the hashed content.
pub const fn content_hash128(bytes: &[u8]) -> [u64; 2] {
    [
        rapidhash::rapidhash_seeded(bytes, rapidhash::RAPID_SEED),
        rapidhash::rapidhash_seeded(bytes, SECOND_SEED),
    ]
}

/// DMN evaluator cache key: 128-bit XML content hash plus the XML length.
type DmnEvaluatorKey = ([u64; 2], usize);

/// Inner FEEL cache level: full expression text -> prepared evaluator.
type ExpressionEvaluators = HashMap<String, Rc<Evaluator>>;

/// Upper bound on cached prepared FEEL evaluators per backend. Reaching it
/// clears the whole cache: realistic workloads evaluate a small, stable set of
/// (expression, context shape) pairs, so LRU bookkeeping buys nothing, and the
/// worst case — a workload cycling through more than this many distinct pairs —
/// merely degrades to the previous parse-per-call behavior plus a map probe.
const FEEL_CACHE_MAX_ENTRIES: usize = 1024;

thread_local! {
    static EVALUATOR_CACHE: RefCell<HashMap<DmnEvaluatorKey, Arc<ModelEvaluator>, RapidBuildHasher>> =
        RefCell::new(HashMap::with_hasher(RapidBuildHasher::default()));

    /// Prepared FEEL evaluators keyed by context-shape digest, then by the full
    /// expression text. Two levels so that cache hits allocate nothing: the
    /// inner lookup borrows the incoming `&str`.
    static FEEL_EVALUATOR_CACHE: RefCell<HashMap<[u64; 2], ExpressionEvaluators, RapidBuildHasher>> =
        RefCell::new(HashMap::with_hasher(RapidBuildHasher::default()));
}

/// Get or create a ModelEvaluator for the given model.
///
/// Keyed by the model's 128-bit XML content hash (computed once when the
/// `DmnModel` is created) plus the XML length, so per-row cache probes never
/// rehash or memcmp the XML itself. A false hit would require two different
/// models of equal length whose XML collides in all 128 hash bits (see
/// `content_hash128`), so no equality check on the XML is performed.
pub fn get_or_build_evaluator(model: &DmnModel) -> Result<Arc<ModelEvaluator>, String> {
    let key = (model.xml_hash, model.xml.len());

    // Check cache first
    let cached = EVALUATOR_CACHE.with_borrow(|cache| cache.get(&key).cloned());
    if let Some(evaluator) = cached {
        return Ok(evaluator);
    }

    // Parse and build
    let definitions =
        dsntk_model::parse(&model.xml).map_err(|e| format!("failed to parse DMN XML: {e}"))?;
    let evaluator = ModelEvaluator::new(&[definitions])
        .map_err(|e| format!("failed to build model evaluator: {e}"))?;

    // Cache it
    EVALUATOR_CACHE.with_borrow_mut(|cache| {
        cache.insert(key, Arc::clone(&evaluator));
    });

    Ok(evaluator)
}

/// Get or build the prepared evaluator for a FEEL expression.
///
/// `shape` must be `context_shape_digest` of the context pushed onto `scope`.
/// The shape is part of the cache key because FEEL name tokenization depends
/// on the names in scope (multi-word names like `monthly salary`): the same
/// expression text can parse to different ASTs under different context shapes.
/// The full expression string is kept verbatim in the key, so a wrong hit
/// additionally requires two distinct shapes colliding in all 128 digest bits
/// (see `content_hash128`) under the very same expression.
pub fn get_or_prepare_feel_evaluator(
    expression: &str,
    shape: [u64; 2],
    scope: &FeelScope,
) -> Result<Rc<Evaluator>, String> {
    let cached = FEEL_EVALUATOR_CACHE.with_borrow(|cache| {
        cache
            .get(&shape)
            .and_then(|by_expr| by_expr.get(expression))
            .cloned()
    });
    if let Some(evaluator) = cached {
        return Ok(evaluator);
    }

    let node = dsntk_feel_parser::parse_expression(scope, expression, false)
        .map_err(|e| format!("FEEL parse error: {e}"))?;
    let evaluator = Rc::new(dsntk_feel_evaluator::prepare(&node));

    FEEL_EVALUATOR_CACHE.with_borrow_mut(|cache| {
        let entries: usize = cache.values().map(ExpressionEvaluators::len).sum();
        if entries >= FEEL_CACHE_MAX_ENTRIES {
            cache.clear();
        }
        cache
            .entry(shape)
            .or_default()
            .insert(expression.to_owned(), Rc::clone(&evaluator));
    });

    Ok(evaluator)
}

/// 128-bit digest of a context's *shape*: the recursive key structure the FEEL
/// parser derives from the scope (`ParsingContext::from(&FeelContext)` in
/// vendor/dsntk-feel-parser/src/context.rs). Only information that influences
/// parsing is included: entry names, and whether each entry is a nested
/// context (recursed), a list whose unified element type is a context
/// (recursed via `Value::type_of`, exactly as the parser does), or a plain
/// variable. Leaf values never affect the digest, so rows differing only in
/// data share one cache entry; any change to the key structure changes it.
pub fn context_shape_digest(ctx: &FeelContext) -> [u64; 2] {
    let mut bytes = Vec::with_capacity(128);
    write_context_shape(ctx, &mut bytes);
    content_hash128(&bytes)
}

/// Serialize a context's shape canonically: `{` entries `}` with each entry as
/// a length-prefixed name followed by its shape. Length prefixes make the
/// encoding injective (a name containing braces or `v` cannot forge structure)
/// and BTreeMap iteration order makes it deterministic.
fn write_context_shape(ctx: &FeelContext, out: &mut Vec<u8>) {
    out.push(b'{');
    for (name, value) in ctx.iter() {
        write_name(name, out);
        match value {
            Value::Context(sub_ctx) => write_context_shape(sub_ctx, out),
            list @ Value::List(_) => write_type_shape(&list.type_of(), out),
            Value::FeelType(feel_type) => write_type_shape(feel_type, out),
            _ => out.push(b'v'),
        }
    }
    out.push(b'}');
}

/// Mirror of `ParsingEntry::from(&FeelType)`: context types contribute their
/// recursive key structure, a list of contexts contributes the element
/// context's structure, everything else is an opaque variable.
fn write_type_shape(feel_type: &FeelType, out: &mut Vec<u8>) {
    match feel_type {
        FeelType::Context(entries) => {
            out.push(b'{');
            for (name, entry_type) in entries {
                write_name(name, out);
                write_type_shape(entry_type, out);
            }
            out.push(b'}');
        }
        FeelType::List(items) => {
            if matches!(items.as_ref(), FeelType::Context(_)) {
                write_type_shape(items, out);
            } else {
                out.push(b'v');
            }
        }
        _ => out.push(b'v'),
    }
}

/// Append a name as a 4-byte little-endian length prefix plus its UTF-8 bytes,
/// written through the `Display` impl to avoid a `String` allocation per key.
fn write_name(name: &Name, out: &mut Vec<u8>) {
    let start = out.len();
    out.extend_from_slice(&[0u8; 4]);
    let _ = write!(out, "{name}"); // io::Write into a Vec<u8> is infallible
    // Names are bounded far below u32::MAX by PostgreSQL's 1 GB value limit.
    let len = u32::try_from(out.len() - start - 4).unwrap_or(u32::MAX);
    out[start..start + 4].copy_from_slice(&len.to_le_bytes());
}

// Pure-Rust unit tests for the shape digest; they run under `cargo pgrx test`
// (the crate itself only compiles where pg_config is available).
#[cfg(test)]
mod tests {
    use super::*;
    use dsntk_feel::FeelNumber;

    fn num(i: i64) -> Value {
        Value::Number(FeelNumber::from(i))
    }

    fn ctx_of(entries: Vec<(&str, Value)>) -> FeelContext {
        let mut ctx = FeelContext::new();
        for (key, value) in entries {
            ctx.set_entry(&Name::from(key), value);
        }
        ctx
    }

    #[test]
    fn same_shape_different_values_share_digest() {
        let a = ctx_of(vec![("x", num(1)), ("y", num(2))]);
        let b = ctx_of(vec![("x", num(100)), ("y", Value::String("s".to_string()))]);
        // Scalars of any kind are opaque variables to the parser.
        assert_eq!(context_shape_digest(&a), context_shape_digest(&b));
    }

    #[test]
    fn different_key_sets_differ() {
        let a = ctx_of(vec![("a", num(1)), ("b", num(2))]);
        let b = ctx_of(vec![("a", num(1)), ("b", num(2)), ("a-b", num(3))]);
        assert_ne!(context_shape_digest(&a), context_shape_digest(&b));
    }

    #[test]
    fn nested_context_vs_flat_dotted_key_differ() {
        let nested = ctx_of(vec![(
            "order",
            Value::Context(ctx_of(vec![("total", num(21))])),
        )]);
        let flat = ctx_of(vec![("order.total", num(5))]);
        assert_ne!(context_shape_digest(&nested), context_shape_digest(&flat));
    }

    #[test]
    fn value_flipping_between_scalar_and_context_differs() {
        let scalar = ctx_of(vec![("x", num(1))]);
        let context = ctx_of(vec![("x", Value::Context(ctx_of(vec![("y", num(1))])))]);
        assert_ne!(
            context_shape_digest(&scalar),
            context_shape_digest(&context)
        );
    }

    #[test]
    fn list_of_contexts_vs_list_of_scalars_differ() {
        let of_contexts = ctx_of(vec![(
            "items",
            Value::List(vec![
                Value::Context(ctx_of(vec![("price", num(2))])),
                Value::Context(ctx_of(vec![("price", num(3))])),
            ]),
        )]);
        let of_scalars = ctx_of(vec![("items", Value::List(vec![num(10), num(20)]))]);
        assert_ne!(
            context_shape_digest(&of_contexts),
            context_shape_digest(&of_scalars)
        );
    }

    #[test]
    fn empty_list_matches_scalar_list() {
        // The parser treats both as opaque variables (List(Null) vs
        // List(Number) both collapse to Variable), so the digests must match —
        // this locks in the exact mirror of ParsingEntry::from(&FeelType).
        let empty = ctx_of(vec![("items", Value::List(vec![]))]);
        let scalars = ctx_of(vec![("items", Value::List(vec![num(1)]))]);
        assert_eq!(context_shape_digest(&empty), context_shape_digest(&scalars));
    }

    #[test]
    fn name_bytes_cannot_forge_structure() {
        // A single key whose text mimics the serialized form of two entries
        // must not collide with an actual two-entry context.
        let forged = ctx_of(vec![("a\u{1}v\u{0}\u{0}\u{0}b", num(1))]);
        let real = ctx_of(vec![("a", num(1)), ("b", num(2))]);
        assert_ne!(context_shape_digest(&forged), context_shape_digest(&real));
    }
}
