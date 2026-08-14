::pgrx::pg_module_magic!();

mod cache;
mod convert;
mod convert_core;
#[cfg(test)]
mod convert_props;
mod functions;
mod guard;
mod types;

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_feel_eval_simple_addition() {
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('1 + 2')").expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(3));
    }

    #[pg_test]
    fn test_feel_eval_with_context() {
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('x * 2', '{\"x\": 21}'::jsonb)")
            .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(42));
    }

    #[pg_test]
    fn test_feel_eval_list() {
        let result =
            Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('for i in [1,2,3] return i * i')")
                .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!([1, 4, 9]));
    }

    #[pg_test]
    fn test_feel_eval_numeric() {
        let result = Spi::get_one::<pgrx::AnyNumeric>(
            "SELECT feel_eval_numeric('x * 2', '{\"x\": 21}'::jsonb)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().to_string(), "42");
    }

    #[pg_test]
    fn test_feel_eval_bool_true() {
        let result = Spi::get_one::<bool>("SELECT feel_eval_bool('5 > 3')").expect("SPI failed");
        assert!(result.unwrap());
    }

    #[pg_test]
    fn test_feel_eval_bool_with_context() {
        let result =
            Spi::get_one::<bool>("SELECT feel_eval_bool('age >= 18', '{\"age\": 25}'::jsonb)")
                .expect("SPI failed");
        assert!(result.unwrap());
    }

    #[pg_test]
    fn test_feel_eval_text() {
        let result =
            Spi::get_one::<String>("SELECT feel_eval_text('\"hello\" + \" \" + \"world\"')")
                .expect("SPI failed");
        assert_eq!(result.unwrap(), "hello world");
    }

    #[pg_test]
    fn test_feel_eval_date() {
        let result =
            Spi::get_one::<pgrx::datum::Date>("SELECT feel_eval_date('date(\"2024-03-15\")')")
                .expect("SPI failed");
        let d = result.unwrap();
        assert_eq!(d.to_string(), "2024-03-15");
    }

    #[pg_test]
    fn test_feel_eval_timestamp() {
        let result = Spi::get_one::<pgrx::datum::Timestamp>(
            "SELECT feel_eval_timestamp('date and time(\"2024-03-15T10:30:00\")')",
        )
        .expect("SPI failed");
        let ts = result.unwrap();
        assert_eq!(ts.to_string(), "2024-03-15 10:30:00");
    }

    #[pg_test]
    fn test_feel_eval_interval_days() {
        let result = Spi::get_one::<bool>(
            "SELECT feel_eval_interval('duration(\"P2D\")') = interval '2 days'",
        )
        .expect("SPI failed");
        assert!(result.unwrap());
    }

    #[pg_test]
    fn test_feel_eval_interval_months() {
        let result = Spi::get_one::<bool>(
            "SELECT feel_eval_interval('duration(\"P3M\")') = interval '3 months'",
        )
        .expect("SPI failed");
        assert!(result.unwrap());
    }

    #[pg_test]
    fn test_feel_eval_null_context() {
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('1 + 1')").expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(2));
    }

    #[pg_test]
    #[should_panic(expected = "nested too deeply")]
    fn test_feel_eval_depth_cap_rejects_deep_nesting() {
        // A FEEL expression nested well past the vendored parser's depth cap (H22, see
        // vendor/PATCHES.md) must raise a SQL error, not crash the backend. 600 levels of
        // unary minus is comfortably over the 500-level cap and orders of magnitude under
        // the measured native-stack-overflow floor, so this exercises the cap itself, not
        // the crash it exists to prevent.
        let expression = format!("{}1", "-".repeat(600));
        let query = format!("SELECT feel_eval('{expression}')");
        Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
    }

    // --- FEEL evaluator cache correctness ---
    // The prepared-evaluator cache key must capture the context *shape*, because
    // FEEL name tokenization depends on which names are in scope: the same
    // expression text parses to different ASTs under different key sets. Each
    // test below would produce a wrong ANSWER (not an error) if a cached
    // evaluator were reused across shapes.

    fn feel_eval_jsonb(expression: &str, context: &str) -> serde_json::Value {
        let query = format!("SELECT feel_eval('{expression}', '{context}'::jsonb)");
        Spi::get_one::<pgrx::JsonB>(&query)
            .expect("SPI failed")
            .expect("feel_eval returned NULL")
            .0
    }

    #[pg_test]
    fn test_feel_eval_shape_hyphen_name_vs_subtraction() {
        // Prime the cache with the subtraction shape first.
        assert_eq!(
            feel_eval_jsonb("a - b", r#"{"a": 100, "b": 1}"#),
            serde_json::json!(99)
        );
        // Under a shape whose key set contains "a-b", the lexer tokenizes
        // 'a - b' as that single name: the answer is 42. A shape-blind cache
        // would reuse the subtraction AST and return 99.
        assert_eq!(
            feel_eval_jsonb("a - b", r#"{"a-b": 42, "a": 100, "b": 1}"#),
            serde_json::json!(42)
        );
        // And back: the original shape still yields the subtraction result.
        assert_eq!(
            feel_eval_jsonb("a - b", r#"{"a": 100, "b": 1}"#),
            serde_json::json!(99)
        );
    }

    #[pg_test]
    fn test_feel_eval_shape_nested_vs_flat_key() {
        // Nested shape: 'order.total' is a path expression.
        assert_eq!(
            feel_eval_jsonb("order.total * 2", r#"{"order": {"total": 21}}"#),
            serde_json::json!(42)
        );
        // Flat shape: "order.total" is a single key, resolved as one name.
        // Wrong reuse of the path AST would return null instead of 10.
        assert_eq!(
            feel_eval_jsonb("order.total * 2", r#"{"order.total": 5}"#),
            serde_json::json!(10)
        );
        // The nested shape still parses as a path afterwards.
        assert_eq!(
            feel_eval_jsonb("order.total * 2", r#"{"order": {"total": 21}}"#),
            serde_json::json!(42)
        );
    }

    #[pg_test]
    fn test_feel_eval_shape_multiword_name() {
        // 'monthly salary' is a single multi-word FEEL name when in scope.
        assert_eq!(
            feel_eval_jsonb("monthly salary * 12", r#"{"monthly salary": 5000}"#),
            serde_json::json!(60000)
        );
        // A wider shape containing the same multi-word key parses the same way.
        assert_eq!(
            feel_eval_jsonb(
                "monthly salary * 12",
                r#"{"monthly salary": 7000, "bonus": 1}"#
            ),
            serde_json::json!(84000)
        );
    }

    #[pg_test]
    #[should_panic(expected = "FEEL parse error")]
    fn test_feel_eval_shape_multiword_split_is_error() {
        // Prime the cache with the multi-word shape (must succeed).
        assert_eq!(
            feel_eval_jsonb("monthly salary * 12", r#"{"monthly salary": 5000}"#),
            serde_json::json!(60000)
        );
        // With the multi-word key absent, the same text is a syntax error. A
        // shape-blind cache would evaluate the cached AST and return null
        // instead of raising.
        feel_eval_jsonb("monthly salary * 12", r#"{"monthly": 3, "salary": 7}"#);
    }

    #[pg_test]
    fn test_feel_eval_values_change_shape_constant() {
        // Same expression and shape, varying leaf values: one cache entry must
        // serve all rows and produce varying, correct results.
        assert_eq!(
            feel_eval_jsonb("x * y", r#"{"x": 2, "y": 3}"#),
            serde_json::json!(6)
        );
        assert_eq!(
            feel_eval_jsonb("x * y", r#"{"x": 5, "y": 7}"#),
            serde_json::json!(35)
        );
        assert_eq!(
            feel_eval_jsonb("x * y", r#"{"x": 6, "y": 0.5}"#),
            serde_json::json!(3)
        );
    }

    #[pg_test]
    fn test_feel_eval_shape_list_of_contexts_vs_scalars() {
        // A list of contexts contributes its element key structure to the scope
        // ('items . price' becomes a known name path).
        assert_eq!(
            feel_eval_jsonb(
                "sum(items.price)",
                r#"{"items": [{"price": 2}, {"price": 3}]}"#
            ),
            serde_json::json!(5)
        );
        // A list of scalars has no 'price' projection: the result is null.
        assert_eq!(
            feel_eval_jsonb("sum(items.price)", r#"{"items": [10, 20]}"#),
            serde_json::Value::Null
        );
        // The list-of-contexts shape still works afterwards.
        assert_eq!(
            feel_eval_jsonb(
                "sum(items.price)",
                r#"{"items": [{"price": 2}, {"price": 3}]}"#
            ),
            serde_json::json!(5)
        );
    }

    // --- Number conversion fast paths ---

    #[pg_test]
    fn test_feel_eval_large_integer_exact() {
        // 2^53 + 1 is not representable in f64; the integer paths must stay
        // exact in both directions (JSON -> FEEL and FEEL -> JSON).
        assert_eq!(
            feel_eval_jsonb("x + 1", r#"{"x": 9007199254740992}"#),
            serde_json::json!(9_007_199_254_740_993_i64)
        );
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('9007199254740993')")
            .expect("SPI failed");
        assert_eq!(
            result.unwrap().0,
            serde_json::json!(9_007_199_254_740_993_i64)
        );
    }

    #[pg_test]
    fn test_feel_eval_non_integer_float_output() {
        let result =
            Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('1.5 + 1')").expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(2.5));
    }

    #[pg_test]
    fn test_feel_eval_decimal_addition_is_exact() {
        // The pricing article contrasts FEEL's exact decimal arithmetic with
        // binary float: 0.10 + 0.20 is exactly 0.3, never the
        // 0.30000000000000004 an f64 would give. Postgres numeric equality would
        // reject a binary-tainted value, so this pins the claim the page makes.
        let equals = Spi::get_one::<bool>("SELECT feel_eval_numeric('0.10 + 0.20') = 0.3")
            .expect("SPI failed")
            .expect("null");
        assert!(
            equals,
            "0.10 + 0.20 should be exactly 0.3, not a binary-float tail"
        );

        // It also renders cleanly, with no binary-float tail (scale-tolerant).
        let text = Spi::get_one::<String>("SELECT feel_eval_numeric('0.10 + 0.20')::text")
            .expect("SPI failed")
            .expect("null");
        assert_eq!(
            text.trim_end_matches('0').trim_end_matches('.'),
            "0.3",
            "0.10 + 0.20 rendered with an unexpected tail: {text}"
        );

        // FEEL's own equality agrees—what a decision table would rely on.
        let feel_equal = Spi::get_one::<bool>("SELECT feel_eval_bool('0.10 + 0.20 = 0.3')")
            .expect("SPI failed")
            .expect("null");
        assert!(feel_equal, "FEEL should treat 0.10 + 0.20 = 0.3 as true");
    }

    #[pg_test]
    fn test_feel_eval_integer_beyond_i64_as_float() {
        // Integers wider than i64 fall back to the f64 output path.
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('9223372036854775807 + 1')")
            .expect("SPI failed");
        let f = result.unwrap().0.as_f64().expect("expected f64 result");
        assert!((f - 2f64.powi(63)).abs() < 1.0, "unexpected value: {f}");
    }

    // dsntk 0.3 behavior pins: these lock in SQL-visible upstream behavior that
    // changed in the 0.2 -> 0.3 bump, so the next dsntk upgrade cannot shift it
    // silently. If one of these fails after a dependency bump, treat it as a
    // behavior change to document, not a pgdmn regression.

    #[pg_test]
    fn test_feel_unary_equal_not_equal_tests() {
        // 0.3 behavior pin: unary '=' / '!=' tests evaluate (0.2 returned
        // null for these forms). Decision-table entries using them now match.
        let eq = Spi::get_one::<bool>("SELECT feel_eval_bool('5 in (= 5)')").expect("SPI failed");
        assert_eq!(eq, Some(true));
        let ne = Spi::get_one::<bool>("SELECT feel_eval_bool('5 in (!= 5)')").expect("SPI failed");
        assert_eq!(ne, Some(false));
        let ne_list =
            Spi::get_one::<bool>("SELECT feel_eval_bool('5 in (!= 3, != 4)')").expect("SPI failed");
        assert_eq!(ne_list, Some(true));
    }

    #[pg_test]
    fn test_parser_scope_derivation_contract() {
        // Pins the parser's scope-shape derivation (ParsingContext flattening)
        // that the FEEL evaluator cache key mirrors: a multi-word key inside a
        // nested context is only resolvable as a path if the parser flattens
        // nested keys as "parent . child". If an upstream dsntk change alters
        // that derivation, this fails loudly instead of silently skewing the
        // cache key (see cache.rs::context_shape_digest).
        let result = Spi::get_one::<pgrx::JsonB>(
            r#"SELECT feel_eval('order.line total * 2', '{"order": {"line total": 5}}'::jsonb)"#,
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(10));
    }

    #[pg_test]
    fn test_feel_in_operator_accepts_unary_test_list() {
        // 0.2 evaluated this to null('eval_in_list')
        let result =
            Spi::get_one::<bool>("SELECT feel_eval_bool('5 in (=4, =5)')").expect("SPI failed");
        assert_eq!(result, Some(true));
    }

    #[pg_test]
    fn test_feel_temporal_keyword_as_variable_name() {
        // 0.2 lexed 'duration' as a temporal function name and failed to parse
        let result = Spi::get_one::<bool>(
            r#"SELECT feel_eval_bool('duration > 30', '{"duration": 45}'::jsonb)"#,
        )
        .expect("SPI failed");
        assert_eq!(result, Some(true));
    }

    #[pg_test]
    fn test_feel_multiword_property_end_included() {
        // 0.2 could not parse multi-word built-in properties after a dot
        let result = Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('[1..10].end included')")
            .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(true));
    }

    #[pg_test]
    fn test_feel_unbounded_range_renders_without_null() {
        // 0.2 rendered this as "[1..null)"
        let result = Spi::get_one::<pgrx::JsonB>(r#"SELECT feel_eval('range("[1..)")')"#)
            .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("[1..)"));
    }

    #[pg_test(error = "FEEL parse error: <LexerError> expected hex digit but encountered 'G'")]
    fn test_feel_invalid_unicode_escape_is_parse_error() {
        // 0.2 silently produced a garbage code point for malformed \u escapes
        Spi::get_one::<pgrx::JsonB>(r#"SELECT feel_eval('"\uGGGG"')"#).expect("SPI failed");
    }

    // External (Java/PMML) function rejection: dsntk 0.3 would otherwise make a
    // blocking HTTP call from inside the backend (see src/guard.rs).

    #[pg_test(error = "FEEL 'external' function definitions are not supported")]
    fn test_feel_external_function_rejected() {
        Spi::get_one::<pgrx::JsonB>(
            r#"SELECT feel_eval('function(x) external { java: { class: "java.lang.Math", method signature: "cos(double)" } }')"#,
        )
        .expect("SPI failed");
    }

    #[pg_test(error = "FEEL 'external' function definitions are not supported")]
    fn test_feel_nested_external_function_rejected() {
        Spi::get_one::<pgrx::JsonB>(
            r#"SELECT feel_eval('{ f: function(x) external { java: { class: "java.lang.Math", method signature: "cos(double)" } } }')"#,
        )
        .expect("SPI failed");
    }

    #[pg_test]
    fn test_feel_eval_numrange_bounded() {
        let result =
            Spi::get_one::<String>(r#"SELECT feel_eval_numrange('range("[18..65)")')::text"#)
                .expect("SPI failed");
        assert_eq!(result.as_deref(), Some("[18,65)"));
    }

    #[pg_test]
    fn test_feel_eval_numrange_unbounded_end() {
        let result = Spi::get_one::<String>(r#"SELECT feel_eval_numrange('range("[1..)")')::text"#)
            .expect("SPI failed");
        assert_eq!(result.as_deref(), Some("[1,)"));
    }

    #[pg_test]
    fn test_feel_eval_numrange_from_context() {
        let result = Spi::get_one::<String>(
            r#"SELECT feel_eval_numrange('[low..high)', '{"low": 18, "high": 65}'::jsonb)::text"#,
        )
        .expect("SPI failed");
        assert_eq!(result.as_deref(), Some("[18,65)"));

        let contained = Spi::get_one::<bool>(
            r#"SELECT 42::numeric <@ feel_eval_numrange('[low..high)', '{"low": 18, "high": 65}'::jsonb)"#,
        )
        .expect("SPI failed");
        assert_eq!(contained, Some(true));
    }

    #[pg_test(error = "expected FEEL range, got: 5")]
    fn test_feel_eval_numrange_rejects_non_range() {
        Spi::get_one::<String>("SELECT feel_eval_numrange('5')::text").expect("SPI failed");
    }

    #[pg_test(error = "FEEL number result is not finite and cannot be converted to NUMERIC")]
    fn test_feel_eval_numeric_rejects_decimal_overflow() {
        // decimal128 overflow rounds to +Inf, whose Display panics inside
        // dsntk—the boundary must reject before stringifying (BUG-004)
        Spi::get_one::<pgrx::AnyNumeric>(
            "SELECT feel_eval_numeric(repeat('9', 3100) || ' * ' || repeat('9', 3100))",
        )
        .expect("SPI failed");
    }

    #[pg_test(error = "FEEL number result is not finite and cannot be represented")]
    fn test_feel_eval_rejects_decimal_overflow() {
        // same overflow through the JSONB path (BUG-004)
        Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_eval(repeat('9', 3100) || ' * ' || repeat('9', 3100))",
        )
        .expect("SPI failed");
    }

    #[pg_test(
        error = "business knowledge model 'CosFn' uses an external Java function, which pgdmn does not support"
    )]
    fn test_dmn_load_rejects_java_bkm() {
        const JAVA_BKM_DMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="java_bkm_model"
             name="JavaBkmModel"
             namespace="https://example.org/javabkm">
    <businessKnowledgeModel id="CosFn" name="CosFn">
        <variable name="CosFn"/>
        <encapsulatedLogic kind="Java">
            <formalParameter typeRef="number" name="x"/>
            <context>
                <contextEntry>
                    <variable name="class"/>
                    <literalExpression><text>"java.lang.Math"</text></literalExpression>
                </contextEntry>
                <contextEntry>
                    <variable name="method signature"/>
                    <literalExpression><text>"cos(double)"</text></literalExpression>
                </contextEntry>
            </context>
        </encapsulatedLogic>
    </businessKnowledgeModel>
</definitions>"#;
        Spi::get_one::<String>(&format!("SELECT dmn_name(dmn_load('{JAVA_BKM_DMN}'))"))
            .expect("SPI failed");
    }

    #[pg_test(
        error = "decision 'UsesJava' uses an external Java function, which pgdmn does not support"
    )]
    fn test_dmn_load_rejects_java_function_boxed_in_decision() {
        const JAVA_BOXED_DMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="java_boxed_model"
             name="JavaBoxedModel"
             namespace="https://example.org/javaboxed">
    <decision id="UsesJava" name="UsesJava">
        <variable name="UsesJava"/>
        <context>
            <contextEntry>
                <variable name="cos"/>
                <functionDefinition kind="Java">
                    <formalParameter typeRef="number" name="x"/>
                    <context>
                        <contextEntry>
                            <variable name="class"/>
                            <literalExpression><text>"java.lang.Math"</text></literalExpression>
                        </contextEntry>
                        <contextEntry>
                            <variable name="method signature"/>
                            <literalExpression><text>"cos(double)"</text></literalExpression>
                        </contextEntry>
                    </context>
                </functionDefinition>
            </contextEntry>
            <contextEntry>
                <literalExpression><text>cos(0)</text></literalExpression>
            </contextEntry>
        </context>
    </decision>
</definitions>"#;
        Spi::get_one::<String>(&format!("SELECT dmn_name(dmn_load('{JAVA_BOXED_DMN}'))"))
            .expect("SPI failed");
    }

    // DMN model tests using a simple inline model
    const SIMPLE_DMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             xmlns:dmndi="https://www.omg.org/spec/DMN/20191111/DMNDI/"
             xmlns:dc="http://www.omg.org/spec/DMN/20180521/DC/"
             id="simple_decisions"
             name="SimpleDecisions"
             namespace="https://example.org/simple">
    <decision id="Greeting" name="Greeting">
        <variable name="Greeting" typeRef="string"/>
        <literalExpression>
            <text>"Hello, World!"</text>
        </literalExpression>
    </decision>
</definitions>"#;

    // A decision table model: loan eligibility based on age and income
    const DECISION_TABLE_DMN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="loan_eligibility"
             name="LoanEligibility"
             namespace="https://example.org/loan">
    <inputData id="Age" name="Age">
        <variable name="Age" typeRef="number"/>
    </inputData>
    <inputData id="Income" name="Income">
        <variable name="Income" typeRef="number"/>
    </inputData>
    <decision id="Eligibility" name="Eligibility">
        <variable name="Eligibility" typeRef="string"/>
        <informationRequirement>
            <requiredInput href="#Age"/>
        </informationRequirement>
        <informationRequirement>
            <requiredInput href="#Income"/>
        </informationRequirement>
        <decisionTable hitPolicy="FIRST">
            <input>
                <inputExpression typeRef="number"><text>Age</text></inputExpression>
            </input>
            <input>
                <inputExpression typeRef="number"><text>Income</text></inputExpression>
            </input>
            <output name="Eligibility" typeRef="string"/>
            <rule>
                <inputEntry><text>&lt; 18</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"Denied: underage"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>&gt;= 50000</text></inputEntry>
                <outputEntry><text>"Approved"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>&lt; 50000</text></inputEntry>
                <outputEntry><text>"Denied: low income"</text></outputEntry>
            </rule>
        </decisionTable>
    </decision>
</definitions>"##;

    // A model with multiple dependent decisions
    const MULTI_DECISION_DMN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="multi_decision"
             name="MultiDecision"
             namespace="https://example.org/multi">
    <inputData id="BasePrice" name="Base Price">
        <variable name="Base Price" typeRef="number"/>
    </inputData>
    <inputData id="TaxRate" name="Tax Rate">
        <variable name="Tax Rate" typeRef="number"/>
    </inputData>
    <decision id="TaxAmount" name="Tax Amount">
        <variable name="Tax Amount" typeRef="number"/>
        <informationRequirement>
            <requiredInput href="#BasePrice"/>
        </informationRequirement>
        <informationRequirement>
            <requiredInput href="#TaxRate"/>
        </informationRequirement>
        <literalExpression>
            <text>Base Price * Tax Rate</text>
        </literalExpression>
    </decision>
    <decision id="TotalPrice" name="Total Price">
        <variable name="Total Price" typeRef="number"/>
        <informationRequirement>
            <requiredInput href="#BasePrice"/>
        </informationRequirement>
        <informationRequirement>
            <requiredDecision href="#TaxAmount"/>
        </informationRequirement>
        <literalExpression>
            <text>Base Price + Tax Amount</text>
        </literalExpression>
    </decision>
</definitions>"##;

    #[pg_test]
    fn test_dmn_load_and_name() {
        let query = format!(
            "SELECT dmn_name(dmn_load('{}'))",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<String>(&query).expect("SPI failed");
        assert_eq!(result.unwrap(), "SimpleDecisions");
    }

    #[pg_test]
    fn test_dmn_namespace() {
        let query = format!(
            "SELECT dmn_namespace(dmn_load('{}'))",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<String>(&query).expect("SPI failed");
        assert_eq!(result.unwrap(), "https://example.org/simple");
    }

    #[pg_test]
    fn test_dmn_xml_roundtrip() {
        let query = format!(
            "SELECT dmn_xml(dmn_load('{}'))",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<String>(&query).expect("SPI failed");
        assert!(result.unwrap().contains("<definitions"));
    }

    #[pg_test]
    fn test_dmn_eval_greeting() {
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Hello, World!"));
    }

    #[pg_test]
    fn test_dmn_info() {
        let query = format!(
            "SELECT dmn_info(dmn_load('{}'))",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        let info = result.unwrap().0;
        assert_eq!(info["name"], "SimpleDecisions");
        assert_eq!(info["decisions"], 1);
    }

    // --- Failure case tests ---

    #[pg_test]
    #[should_panic(expected = "failed to parse DMN XML")]
    fn test_dmn_load_invalid_xml() {
        Spi::run("SELECT dmn_load('not valid xml at all')").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "failed to parse DMN XML")]
    fn test_dmn_load_empty_string() {
        Spi::run("SELECT dmn_load('')").unwrap();
    }

    #[pg_test]
    #[should_panic(expected = "failed to parse DMN XML")]
    fn test_dmn_load_non_dmn_xml() {
        Spi::run("SELECT dmn_load('<root><child/></root>')").unwrap();
    }

    /// `dmn_load` parses caller-supplied XML, so it must not resolve external
    /// entities: a `SYSTEM` entity pointing at a file is the classic XXE vector,
    /// and this runs inside the database. Write a secret to a temp file, ask a
    /// model to embed it via an external entity, and assert the secret does not
    /// come back—whether because the parser rejects the DOCTYPE or because it
    /// leaves the entity unresolved.
    #[pg_test]
    fn test_dmn_load_does_not_resolve_external_entities() {
        let dir = std::env::temp_dir();
        let secret_path = dir.join("pgdmn_xxe_probe.txt");
        let secret = "TOP-SECRET-DO-NOT-LEAK";
        std::fs::write(&secret_path, secret).expect("could not write probe file");

        let payload = format!(
            r#"<?xml version="1.0"?>
<!DOCTYPE definitions [<!ENTITY xxe SYSTEM "file://{}">]>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="xxe" name="&xxe;" namespace="https://example.org/xxe">
  <decision id="D" name="D"><literalExpression><text>1</text></literalExpression></decision>
</definitions>"#,
            secret_path.display()
        );

        // dmn_load may succeed or fail; either is fine. What must never happen is
        // the file's contents appearing in the parsed model.
        let escaped = payload.replace('\'', "''");
        let leaked = std::panic::catch_unwind(|| {
            Spi::get_one::<String>(&format!("SELECT dmn_name(dmn_load('{escaped}'))"))
                .ok()
                .flatten()
                .unwrap_or_default()
        })
        .unwrap_or_default();

        let _ = std::fs::remove_file(&secret_path);
        assert!(
            !leaked.contains(secret),
            "XXE: external entity resolved, model name leaked the file: {leaked:?}"
        );
    }

    #[pg_test]
    fn test_dmn_eval_nonexistent_invocable() {
        // Evaluating a non-existent invocable should return null, not crash
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'DoesNotExist')",
            SIMPLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::Value::Null);
    }

    #[pg_test]
    fn test_dmn_eval_no_input_when_required() {
        // Decision table expects inputs; omitting them should produce null result
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility')",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::Value::Null);
    }

    // --- Decision table tests ---

    #[pg_test]
    fn test_dmn_decision_table_approved() {
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility', '{{\"Age\": 30, \"Income\": 75000}}'::jsonb)",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Approved"));
    }

    #[pg_test]
    fn test_dmn_decision_table_denied_underage() {
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility', '{{\"Age\": 16, \"Income\": 100000}}'::jsonb)",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Denied: underage"));
    }

    #[pg_test]
    fn test_dmn_decision_table_denied_low_income() {
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility', '{{\"Age\": 25, \"Income\": 30000}}'::jsonb)",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Denied: low income"));
    }

    #[pg_test]
    fn test_dmn_decision_table_boundary_age() {
        // Exactly 18 with sufficient income should be approved
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility', '{{\"Age\": 18, \"Income\": 50000}}'::jsonb)",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Approved"));
    }

    #[pg_test]
    fn test_dmn_decision_table_boundary_income() {
        // 18+ with income just below threshold
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Eligibility', '{{\"Age\": 25, \"Income\": 49999}}'::jsonb)",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Denied: low income"));
    }

    // A decision table whose input entries use the special `?` variable,
    // which the DMN spec binds to the tested input value.
    const QMARK_TABLE_DMN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="qmark_table"
             name="QmarkTable"
             namespace="https://example.org/qmark">
    <inputData id="Age" name="Age">
        <variable name="Age" typeRef="number"/>
    </inputData>
    <decision id="Adult" name="Adult">
        <variable name="Adult" typeRef="string"/>
        <informationRequirement>
            <requiredInput href="#Age"/>
        </informationRequirement>
        <decisionTable hitPolicy="FIRST">
            <input>
                <inputExpression typeRef="number"><text>Age</text></inputExpression>
            </input>
            <output name="Adult" typeRef="string"/>
            <rule>
                <inputEntry><text>? >= 18</text></inputEntry>
                <outputEntry><text>"adult"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"minor"</text></outputEntry>
            </rule>
        </decisionTable>
    </decision>
</definitions>"##;

    #[pg_test]
    fn test_dmn_decision_table_question_mark_binding() {
        // `?` in an input entry must see the column's input value (DMN spec);
        // guards the input-expression hoisting in the vendored decision-table
        // evaluator (H10) against binding regressions.
        let escaped = QMARK_TABLE_DMN.replace('\'', "''");
        for (age, expected) in [(20, "adult"), (16, "minor"), (18, "adult")] {
            let query = format!(
                "SELECT dmn_eval(dmn_load('{escaped}'), 'Adult', '{{\"Age\": {age}}}'::jsonb)"
            );
            let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
            assert_eq!(
                result.unwrap().0,
                serde_json::json!(expected),
                "Age {age} misclassified"
            );
        }
    }

    // --- Multi-decision (dependent decisions) tests ---

    #[pg_test]
    fn test_dmn_multi_decision_tax_amount() {
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Tax Amount', '{{\"Base Price\": 100, \"Tax Rate\": 0.1}}'::jsonb)",
            MULTI_DECISION_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(10));
    }

    #[pg_test]
    fn test_dmn_multi_decision_total_price() {
        // Total Price depends on Tax Amount (chained decision)
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Total Price', '{{\"Base Price\": 100, \"Tax Rate\": 0.2}}'::jsonb)",
            MULTI_DECISION_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(120));
    }

    // --- Introspection tests for complex models ---

    #[pg_test]
    fn test_dmn_info_decision_table_model() {
        let query = format!(
            "SELECT dmn_info(dmn_load('{}'))",
            DECISION_TABLE_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        let info = result.unwrap().0;
        assert_eq!(info["name"], "LoanEligibility");
        assert_eq!(info["namespace"], "https://example.org/loan");
        assert_eq!(info["decisions"], 1);
    }

    #[pg_test]
    fn test_dmn_info_multi_decision_model() {
        let query = format!(
            "SELECT dmn_info(dmn_load('{}'))",
            MULTI_DECISION_DMN.replace('\'', "''")
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        let info = result.unwrap().0;
        assert_eq!(info["name"], "MultiDecision");
        assert_eq!(info["decisions"], 2);
        let invocables = info["invocables"].as_array().unwrap();
        assert!(invocables.contains(&serde_json::json!("Tax Amount")));
        assert!(invocables.contains(&serde_json::json!("Total Price")));
    }

    #[pg_test]
    fn test_dmn_invocables_multi_decision() {
        let query = format!(
            "SELECT name, kind FROM dmn_invocables(dmn_load('{}')) ORDER BY name",
            MULTI_DECISION_DMN.replace('\'', "''")
        );
        let result = Spi::connect(|client| {
            let rows: Vec<(String, String)> = client
                .select(&query, None, &[])
                .unwrap()
                .map(|row| {
                    (
                        row.get_by_name::<String, _>("name").unwrap().unwrap(),
                        row.get_by_name::<String, _>("kind").unwrap().unwrap(),
                    )
                })
                .collect();
            rows
        });
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "Tax Amount");
        assert_eq!(result[0].1, "decision");
        assert_eq!(result[1].0, "Total Price");
        assert_eq!(result[1].1, "decision");
    }

    #[pg_test]
    fn test_cache_speeds_up_repeated_eval() {
        let escaped = SIMPLE_DMN.replace('\'', "''");
        let query = format!("SELECT dmn_eval(dmn_load('{escaped}'), 'Greeting')");

        // Cold call: first evaluation parses XML and builds evaluator
        let cold_start = std::time::Instant::now();
        let cold_result = Spi::get_one::<pgrx::JsonB>(&query)
            .expect("SPI failed")
            .expect("dmn_eval returned NULL on cold run");
        let cold_duration = cold_start.elapsed();
        assert_eq!(cold_result.0, serde_json::json!("Hello, World!"));

        // Warm calls: subsequent evaluations use cached evaluator
        let iterations = 100;
        let warm_start = std::time::Instant::now();
        for _ in 0..iterations {
            Spi::get_one::<pgrx::JsonB>(&query)
                .expect("SPI failed")
                .expect("dmn_eval returned NULL on warm run");
        }
        let warm_duration = warm_start.elapsed();
        let warm_avg = warm_duration / iterations;

        // Cached evaluation should be at least 2x faster than cold
        assert!(
            warm_avg < cold_duration / 2,
            "Cache did not provide expected speedup: cold={cold_duration:?}, warm_avg={warm_avg:?}"
        );
    }

    // DMN model that concatenates first_name and last_name with a space
    const CONCAT_DMN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="concat_decision"
             name="ConcatDecision"
             namespace="https://example.org/concat">
    <decision id="FullName" name="FullName">
        <variable name="FullName" typeRef="string"/>
        <informationRequirement>
            <requiredInput href="#input_first_name"/>
        </informationRequirement>
        <informationRequirement>
            <requiredInput href="#input_last_name"/>
        </informationRequirement>
        <literalExpression>
            <text>first_name + " " + last_name</text>
        </literalExpression>
    </decision>
    <inputData id="input_first_name" name="first_name">
        <variable name="first_name" typeRef="string"/>
    </inputData>
    <inputData id="input_last_name" name="last_name">
        <variable name="last_name" typeRef="string"/>
    </inputData>
</definitions>"##;

    // Complex DMN: risk assessment with a decision table (6 rules), a derived
    // risk score literal expression, and a chained final decision. Uses 5 inputs.
    const RISK_DMN: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="risk_assessment"
             name="RiskAssessment"
             namespace="https://example.org/risk">
    <inputData id="input_age" name="age">
        <variable name="age" typeRef="number"/>
    </inputData>
    <inputData id="input_income" name="income">
        <variable name="income" typeRef="number"/>
    </inputData>
    <inputData id="input_credit_score" name="credit_score">
        <variable name="credit_score" typeRef="number"/>
    </inputData>
    <inputData id="input_employment" name="employment_status">
        <variable name="employment_status" typeRef="string"/>
    </inputData>
    <inputData id="input_years" name="years_employed">
        <variable name="years_employed" typeRef="number"/>
    </inputData>
    <decision id="RiskCategory" name="RiskCategory">
        <variable name="RiskCategory" typeRef="string"/>
        <informationRequirement><requiredInput href="#input_age"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_income"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_credit_score"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_employment"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_years"/></informationRequirement>
        <decisionTable hitPolicy="FIRST">
            <input><inputExpression typeRef="number"><text>age</text></inputExpression></input>
            <input><inputExpression typeRef="number"><text>credit_score</text></inputExpression></input>
            <input><inputExpression typeRef="number"><text>income</text></inputExpression></input>
            <input><inputExpression typeRef="string"><text>employment_status</text></inputExpression></input>
            <input><inputExpression typeRef="number"><text>years_employed</text></inputExpression></input>
            <output name="RiskCategory" typeRef="string"/>
            <rule>
                <inputEntry><text>&lt; 18</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"ineligible"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>&gt;= 750</text></inputEntry>
                <inputEntry><text>&gt;= 80000</text></inputEntry>
                <inputEntry><text>"employed"</text></inputEntry>
                <inputEntry><text>&gt;= 3</text></inputEntry>
                <outputEntry><text>"low"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>[650..750)</text></inputEntry>
                <inputEntry><text>&gt;= 50000</text></inputEntry>
                <inputEntry><text>"employed","self-employed"</text></inputEntry>
                <inputEntry><text>&gt;= 1</text></inputEntry>
                <outputEntry><text>"medium"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>[650..750)</text></inputEntry>
                <inputEntry><text>&lt; 50000</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"high"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>&lt; 650</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"very high"</text></outputEntry>
            </rule>
            <rule>
                <inputEntry><text>&gt;= 18</text></inputEntry>
                <inputEntry><text>&gt;= 750</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <inputEntry><text>-</text></inputEntry>
                <outputEntry><text>"medium"</text></outputEntry>
            </rule>
        </decisionTable>
    </decision>
    <decision id="RiskScore" name="RiskScore">
        <variable name="RiskScore" typeRef="number"/>
        <informationRequirement><requiredDecision href="#RiskCategory"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_credit_score"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_income"/></informationRequirement>
        <informationRequirement><requiredInput href="#input_years"/></informationRequirement>
        <literalExpression>
            <text>if RiskCategory = "ineligible" then 0
            else if RiskCategory = "low" then credit_score * 0.5 + income / 1000 + years_employed * 10
            else if RiskCategory = "medium" then credit_score * 0.3 + income / 2000 + years_employed * 5
            else if RiskCategory = "high" then credit_score * 0.1 + income / 5000
            else credit_score * 0.05</text>
        </literalExpression>
    </decision>
</definitions>"##;

    /// Warm (LIMIT 1) then time a per-row benchmark query. Warmup keeps
    /// parse/cache-miss cost out of the timed run.
    fn timed_query(label: &str, sql: &str) -> std::time::Duration {
        Spi::run(&format!("{sql} LIMIT 1"))
            .unwrap_or_else(|e| panic!("{label} warmup failed: {e}"));
        let start = std::time::Instant::now();
        Spi::run(sql).unwrap_or_else(|e| panic!("{label} query failed: {e}"));
        start.elapsed()
    }

    #[pg_test]
    // Benchmark: long inline scenario setup; float division only for human-readable reporting
    #[expect(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn bench_dmn_eval_vs_pg_concat() {
        if std::env::var("PGDMN_BENCH").ok().as_deref() != Some("1") {
            return;
        }

        let escaped_concat = CONCAT_DMN.replace('\'', "''");
        let escaped_risk = RISK_DMN.replace('\'', "''");

        // Create a table with skewed duplication: 10,000 unique rows,
        // then 100 of those get extra copies (1000, 500, 250, 125, and 50 each)
        Spi::run(
            "CREATE TABLE bench_names (\
             first_name TEXT NOT NULL, last_name TEXT NOT NULL, \
             age INT NOT NULL, income NUMERIC NOT NULL, credit_score INT NOT NULL, \
             employment_status TEXT NOT NULL, years_employed INT NOT NULL)",
        )
        .expect("CREATE TABLE failed");

        // Generate 10,000 unique rows (100 x 100) with varied extra columns
        Spi::run(
            "INSERT INTO bench_names (first_name, last_name, age, income, credit_score, employment_status, years_employed)
             SELECT 'First_' || lpad(f.n::text, 3, '0'),
                    'Last_' || lpad(l.n::text, 3, '0'),
                    18 + ((f.n * 7 + l.n * 3) % 50),
                    25000 + ((f.n * 13 + l.n * 17) % 100) * 1000,
                    500 + ((f.n * 11 + l.n * 7) % 350),
                    (ARRAY['employed','self-employed','unemployed','retired'])[1 + ((f.n + l.n) % 4)],
                    ((f.n * 3 + l.n * 5) % 20)
             FROM generate_series(1, 100) AS f(n)
             CROSS JOIN generate_series(1, 100) AS l(n)",
        )
        .expect("base INSERT failed");

        // Add skewed duplicates for the first 100 rows (first_name=First_001):
        // row 1: 1000 copies, row 2: 500, row 3: 250, row 4: 125, rows 5-100: 50 each
        for (last_n, copies) in [(1, 1000), (2, 500), (3, 250), (4, 125)] {
            Spi::run(&format!(
                "INSERT INTO bench_names (first_name, last_name, age, income, credit_score, employment_status, years_employed)
                 SELECT b.first_name, b.last_name, b.age, b.income, b.credit_score, b.employment_status, b.years_employed
                 FROM bench_names b, generate_series(1, {copies})
                 WHERE b.first_name = 'First_001' AND b.last_name = 'Last_{last_n:03}'"
            ))
            .expect("skewed INSERT failed");
        }
        Spi::run(
            "INSERT INTO bench_names (first_name, last_name, age, income, credit_score, employment_status, years_employed)
             SELECT b.first_name, b.last_name, b.age, b.income, b.credit_score, b.employment_status, b.years_employed
             FROM bench_names b, generate_series(5, 100) AS s(n), generate_series(1, 50)
             WHERE b.first_name = 'First_001'
               AND b.last_name = 'Last_' || lpad(s.n::text, 3, '0')",
        )
        .expect("bulk skewed INSERT failed");

        let row_count = Spi::get_one::<i64>("SELECT count(*) FROM bench_names")
            .expect("SPI failed")
            .unwrap();
        let distinct_count =
            Spi::get_one::<i64>("SELECT count(DISTINCT (first_name, last_name)) FROM bench_names")
                .expect("SPI failed")
                .unwrap();

        // Create composite types for record-based evaluation
        Spi::run("CREATE TYPE concat_input AS (first_name text, last_name text)")
            .expect("CREATE TYPE concat_input failed");
        Spi::run("CREATE TYPE risk_input AS (age int, income numeric, credit_score int, employment_status text, years_employed int)")
            .expect("CREATE TYPE risk_input failed");

        // Warm all query paths before timing
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped_concat}'), 'FullName', \
             jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN concat warmup failed");
        Spi::run(
            "SELECT (j->>'first_name') || ' ' || (j->>'last_name') \
             FROM (SELECT jsonb_build_object('first_name', first_name, 'last_name', last_name) AS j \
                   FROM bench_names LIMIT 1) t"
        )
        .expect("PG jsonb concat warmup failed");
        Spi::run("SELECT first_name || ' ' || last_name FROM bench_names LIMIT 1")
            .expect("PG plain concat warmup failed");
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped_risk}'), 'RiskScore', \
             jsonb_build_object('age', age, 'income', income, 'credit_score', credit_score, \
             'employment_status', employment_status, 'years_employed', years_employed)) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN risk warmup failed");
        Spi::run(
            "SELECT CASE \
               WHEN (j->>'age')::int < 18 THEN 0 \
               WHEN (j->>'credit_score')::int >= 750 AND (j->>'income')::numeric >= 80000 \
                    AND j->>'employment_status' = 'employed' AND (j->>'years_employed')::int >= 3 \
                 THEN (j->>'credit_score')::int * 0.5 + (j->>'income')::numeric / 1000 + (j->>'years_employed')::int * 10 \
               WHEN (j->>'credit_score')::int >= 650 AND (j->>'credit_score')::int < 750 \
                    AND (j->>'income')::numeric >= 50000 \
                    AND j->>'employment_status' IN ('employed','self-employed') AND (j->>'years_employed')::int >= 1 \
                 THEN (j->>'credit_score')::int * 0.3 + (j->>'income')::numeric / 2000 + (j->>'years_employed')::int * 5 \
               WHEN (j->>'credit_score')::int >= 650 AND (j->>'credit_score')::int < 750 \
                    AND (j->>'income')::numeric < 50000 \
                 THEN (j->>'credit_score')::int * 0.1 + (j->>'income')::numeric / 5000 \
               WHEN (j->>'credit_score')::int < 650 THEN (j->>'credit_score')::int * 0.05 \
               ELSE (j->>'credit_score')::int * 0.3 + (j->>'income')::numeric / 2000 + (j->>'years_employed')::int * 5 \
             END \
             FROM (SELECT jsonb_build_object('age', age, 'income', income, 'credit_score', credit_score, \
                          'employment_status', employment_status, 'years_employed', years_employed) AS j \
                   FROM bench_names LIMIT 1) t"
        )
        .expect("PG jsonb risk warmup failed");
        Spi::run(
            "SELECT CASE \
               WHEN age < 18 THEN 0 \
               WHEN credit_score >= 750 AND income >= 80000 \
                    AND employment_status = 'employed' AND years_employed >= 3 \
                 THEN credit_score * 0.5 + income / 1000 + years_employed * 10 \
               WHEN credit_score >= 650 AND credit_score < 750 \
                    AND income >= 50000 \
                    AND employment_status IN ('employed','self-employed') AND years_employed >= 1 \
                 THEN credit_score * 0.3 + income / 2000 + years_employed * 5 \
               WHEN credit_score >= 650 AND credit_score < 750 \
                    AND income < 50000 \
                 THEN credit_score * 0.1 + income / 5000 \
               WHEN credit_score < 650 THEN credit_score * 0.05 \
               ELSE credit_score * 0.3 + income / 2000 + years_employed * 5 \
             END FROM bench_names LIMIT 1",
        )
        .expect("PG plain risk warmup failed");
        Spi::run(&format!(
            "SELECT dmn_record_eval(dmn_load('{escaped_concat}'), 'FullName', \
             ROW(first_name, last_name)::concat_input) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN record concat warmup failed");
        Spi::run(&format!(
            "SELECT dmn_record_eval(dmn_load('{escaped_risk}'), 'RiskScore', \
             ROW(age, income, credit_score, employment_status, years_employed)::risk_input) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN record risk warmup failed");

        // --- Benchmark 1: simple concat ---
        let dmn_concat_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped_concat}'), 'FullName', \
             jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
             FROM bench_names"
        ))
        .expect("DMN concat query failed");
        let dmn_concat_dur = dmn_concat_start.elapsed();

        let pg_jsonb_concat_start = std::time::Instant::now();
        Spi::run(
            "SELECT (j->>'first_name') || ' ' || (j->>'last_name') \
             FROM (SELECT jsonb_build_object('first_name', first_name, 'last_name', last_name) AS j \
                   FROM bench_names) t",
        )
        .expect("PG jsonb concat query failed");
        let pg_jsonb_concat_dur = pg_jsonb_concat_start.elapsed();

        let pg_plain_concat_start = std::time::Instant::now();
        Spi::run("SELECT first_name || ' ' || last_name FROM bench_names")
            .expect("PG plain concat query failed");
        let pg_plain_concat_dur = pg_plain_concat_start.elapsed();

        let dmn_record_concat_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_record_eval(dmn_load('{escaped_concat}'), 'FullName', \
             ROW(first_name, last_name)::concat_input) \
             FROM bench_names"
        ))
        .expect("DMN record concat query failed");
        let dmn_record_concat_dur = dmn_record_concat_start.elapsed();

        // --- Benchmark 2: complex risk score ---
        let dmn_risk_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped_risk}'), 'RiskScore', \
             jsonb_build_object('age', age, 'income', income, 'credit_score', credit_score, \
             'employment_status', employment_status, 'years_employed', years_employed)) \
             FROM bench_names"
        ))
        .expect("DMN risk query failed");
        let dmn_risk_dur = dmn_risk_start.elapsed();

        let pg_jsonb_risk_start = std::time::Instant::now();
        Spi::run(
            "SELECT CASE \
               WHEN (j->>'age')::int < 18 THEN 0 \
               WHEN (j->>'credit_score')::int >= 750 AND (j->>'income')::numeric >= 80000 \
                    AND j->>'employment_status' = 'employed' AND (j->>'years_employed')::int >= 3 \
                 THEN (j->>'credit_score')::int * 0.5 + (j->>'income')::numeric / 1000 + (j->>'years_employed')::int * 10 \
               WHEN (j->>'credit_score')::int >= 650 AND (j->>'credit_score')::int < 750 \
                    AND (j->>'income')::numeric >= 50000 \
                    AND j->>'employment_status' IN ('employed','self-employed') AND (j->>'years_employed')::int >= 1 \
                 THEN (j->>'credit_score')::int * 0.3 + (j->>'income')::numeric / 2000 + (j->>'years_employed')::int * 5 \
               WHEN (j->>'credit_score')::int >= 650 AND (j->>'credit_score')::int < 750 \
                    AND (j->>'income')::numeric < 50000 \
                 THEN (j->>'credit_score')::int * 0.1 + (j->>'income')::numeric / 5000 \
               WHEN (j->>'credit_score')::int < 650 THEN (j->>'credit_score')::int * 0.05 \
               ELSE (j->>'credit_score')::int * 0.3 + (j->>'income')::numeric / 2000 + (j->>'years_employed')::int * 5 \
             END \
             FROM (SELECT jsonb_build_object('age', age, 'income', income, 'credit_score', credit_score, \
                          'employment_status', employment_status, 'years_employed', years_employed) AS j \
                   FROM bench_names) t"
        )
        .expect("PG jsonb risk query failed");
        let pg_jsonb_risk_dur = pg_jsonb_risk_start.elapsed();

        let pg_plain_risk_start = std::time::Instant::now();
        Spi::run(
            "SELECT CASE \
               WHEN age < 18 THEN 0 \
               WHEN credit_score >= 750 AND income >= 80000 \
                    AND employment_status = 'employed' AND years_employed >= 3 \
                 THEN credit_score * 0.5 + income / 1000 + years_employed * 10 \
               WHEN credit_score >= 650 AND credit_score < 750 \
                    AND income >= 50000 \
                    AND employment_status IN ('employed','self-employed') AND years_employed >= 1 \
                 THEN credit_score * 0.3 + income / 2000 + years_employed * 5 \
               WHEN credit_score >= 650 AND credit_score < 750 \
                    AND income < 50000 \
                 THEN credit_score * 0.1 + income / 5000 \
               WHEN credit_score < 650 THEN credit_score * 0.05 \
               ELSE credit_score * 0.3 + income / 2000 + years_employed * 5 \
             END FROM bench_names",
        )
        .expect("PG plain risk query failed");
        let pg_plain_risk_dur = pg_plain_risk_start.elapsed();

        let dmn_record_risk_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_record_eval(dmn_load('{escaped_risk}'), 'RiskScore', \
             ROW(age, income, credit_score, employment_status, years_employed)::risk_input) \
             FROM bench_names"
        ))
        .expect("DMN record risk query failed");
        let dmn_record_risk_dur = dmn_record_risk_start.elapsed();

        // --- Benchmark 3: FEEL expression evaluation per row ---
        let feel_concat_dur = timed_query(
            "FEEL concat",
            "SELECT feel_eval('first_name + \" \" + last_name', \
             jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
             FROM bench_names",
        );
        let feel_numeric_dur = timed_query(
            "FEEL numeric",
            "SELECT feel_eval_numeric(\
             'credit_score * 0.3 + income / 2000 + years_employed * 5', \
             jsonb_build_object('income', income, 'credit_score', credit_score, \
             'years_employed', years_employed)) FROM bench_names",
        );

        // --- Benchmark 4: large model (85KB lending DMN) per-row overhead ---
        let escaped_lending = EXAMPLE_LENDING.replace('\'', "''");
        let dmn_lending_dur = timed_query(
            "DMN lending",
            &format!(
                "SELECT dmn_eval(dmn_load('{escaped_lending}'), 'Strategy', jsonb_build_object(\
                 'ApplicantData', jsonb_build_object('Age', 51, 'MaritalStatus', 'M', \
                 'EmploymentStatus', 'EMPLOYED', 'ExistingCustomer', false, \
                 'Monthly', jsonb_build_object('Income', income / 10, 'Expenses', 3000, 'Repayments', 0)), \
                 'RequestedProduct', jsonb_build_object('ProductType', 'STANDARD LOAN', \
                 'Amount', income, 'Rate', 0.06, 'Term', 36), \
                 'BureauData', jsonb_build_object('CreditScore', credit_score, 'Bankrupt', false))) \
                 FROM bench_names"
            ),
        );

        let rc = row_count as f64;

        // Report results
        let report = format!(
            "Benchmark: {row_count} rows, {distinct_count} distinct input combos\n\
             \n\
             --- Simple (concat) ---\n\
             DMN jsonb:       {:.1} us/row ({:?})\n\
             DMN record:      {:.1} us/row ({:?})\n\
             PG via jsonb:    {:.1} us/row ({:?})\n\
             PG plain SQL:    {:.1} us/row ({:?})\n\
             record/jsonb:    {:.2}x | DMN jsonb/plain: {:.1}x | DMN record/plain: {:.1}x\n\
             \n\
             --- Complex (risk score) ---\n\
             DMN jsonb:       {:.1} us/row ({:?})\n\
             DMN record:      {:.1} us/row ({:?})\n\
             PG via jsonb:    {:.1} us/row ({:?})\n\
             PG plain SQL:    {:.1} us/row ({:?})\n\
             record/jsonb:    {:.2}x | DMN jsonb/plain: {:.1}x | DMN record/plain: {:.1}x\n\
             \n\
             --- FEEL per-row (evaluator cache path) ---\n\
             feel_eval concat:    {:.1} us/row ({:?})\n\
             feel_eval_numeric:   {:.1} us/row ({:?})\n\
             \n\
             --- Large model, 85KB lending DMN (per-row model overhead) ---\n\
             DMN lending jsonb:   {:.1} us/row ({:?})\n\
             \n\
             Complex/Simple DMN: {:.1}x",
            dmn_concat_dur.as_micros() as f64 / rc,
            dmn_concat_dur,
            dmn_record_concat_dur.as_micros() as f64 / rc,
            dmn_record_concat_dur,
            pg_jsonb_concat_dur.as_micros() as f64 / rc,
            pg_jsonb_concat_dur,
            pg_plain_concat_dur.as_micros() as f64 / rc,
            pg_plain_concat_dur,
            dmn_record_concat_dur.as_secs_f64() / dmn_concat_dur.as_secs_f64(),
            dmn_concat_dur.as_secs_f64() / pg_plain_concat_dur.as_secs_f64(),
            dmn_record_concat_dur.as_secs_f64() / pg_plain_concat_dur.as_secs_f64(),
            dmn_risk_dur.as_micros() as f64 / rc,
            dmn_risk_dur,
            dmn_record_risk_dur.as_micros() as f64 / rc,
            dmn_record_risk_dur,
            pg_jsonb_risk_dur.as_micros() as f64 / rc,
            pg_jsonb_risk_dur,
            pg_plain_risk_dur.as_micros() as f64 / rc,
            pg_plain_risk_dur,
            dmn_record_risk_dur.as_secs_f64() / dmn_risk_dur.as_secs_f64(),
            dmn_risk_dur.as_secs_f64() / pg_plain_risk_dur.as_secs_f64(),
            dmn_record_risk_dur.as_secs_f64() / pg_plain_risk_dur.as_secs_f64(),
            feel_concat_dur.as_micros() as f64 / rc,
            feel_concat_dur,
            feel_numeric_dur.as_micros() as f64 / rc,
            feel_numeric_dur,
            dmn_lending_dur.as_micros() as f64 / rc,
            dmn_lending_dur,
            dmn_risk_dur.as_secs_f64() / dmn_concat_dur.as_secs_f64(),
        );
        if let Err(e) = std::fs::write("/pgdmn/benchmark_results.txt", &report) {
            pgrx::warning!("Failed to write benchmark_results.txt: {}", e);
        }
        pgrx::warning!("{}", report);

        // Sanity check: concat produces matching results
        let mismatches = Spi::get_one::<i64>(&format!(
            "SELECT count(*) FROM bench_names \
             WHERE (dmn_eval(dmn_load('{escaped_concat}'), 'FullName', \
                    jsonb_build_object('first_name', first_name, 'last_name', last_name))) #>> '{{}}' \
                   <> (jsonb_build_object('first_name', first_name, 'last_name', last_name)->>'first_name') \
                      || ' ' || \
                      (jsonb_build_object('first_name', first_name, 'last_name', last_name)->>'last_name')"
        ))
        .expect("SPI failed")
        .unwrap();
        assert_eq!(
            mismatches, 0,
            "DMN and PG concat produced different results"
        );
    }

    /// Does the *shape* of the SQL change what a DMN evaluation costs?
    ///
    /// Same model, same rows, same answers—only the query differs. Gated
    /// behind `PGDMN_BENCH_SHAPES=1` (`make bench-shapes`) because it is a
    /// measurement, not an assertion.
    #[pg_test]
    #[expect(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn bench_query_shapes() {
        if std::env::var("PGDMN_BENCH_SHAPES").ok().as_deref() != Some("1") {
            return;
        }

        let concat = CONCAT_DMN.replace('\'', "''");
        let risk = RISK_DMN.replace('\'', "''");

        Spi::run(
            "CREATE TABLE shapes (\
             first_name TEXT NOT NULL, last_name TEXT NOT NULL, \
             age INT NOT NULL, income NUMERIC NOT NULL, credit_score INT NOT NULL, \
             employment_status TEXT NOT NULL, years_employed INT NOT NULL)",
        )
        .expect("CREATE TABLE failed");
        Spi::run(
            "INSERT INTO shapes (first_name, last_name, age, income, credit_score, employment_status, years_employed)
             SELECT 'First_' || lpad(f.n::text, 3, '0'),
                    'Last_' || lpad(l.n::text, 3, '0'),
                    18 + ((f.n * 7 + l.n * 3) % 50),
                    25000 + ((f.n * 13 + l.n * 17) % 100) * 1000,
                    500 + ((f.n * 11 + l.n * 7) % 350),
                    (ARRAY['employed','self-employed','unemployed','retired'])[1 + ((f.n + l.n) % 4)],
                    ((f.n * 3 + l.n * 5) % 20)
             FROM generate_series(1, 100) AS f(n)
             CROSS JOIN generate_series(1, 100) AS l(n)",
        )
        .expect("INSERT failed");
        // The same skew the other benchmark uses: some inputs repeat a lot.
        Spi::run(
            "INSERT INTO shapes
             SELECT b.* FROM shapes b, generate_series(5, 100) AS s(n), generate_series(1, 50)
             WHERE b.first_name = 'First_001'
               AND b.last_name = 'Last_' || lpad(s.n::text, 3, '0')",
        )
        .expect("skew INSERT failed");

        let rows = Spi::get_one::<i64>("SELECT count(*) FROM shapes")
            .expect("SPI failed")
            .unwrap();
        let distinct_names = Spi::get_one::<i64>(
            "SELECT count(*) FROM (SELECT DISTINCT first_name, last_name FROM shapes) d",
        )
        .expect("SPI failed")
        .unwrap();
        let distinct_risk = Spi::get_one::<i64>(
            "SELECT count(*) FROM (SELECT DISTINCT age, income, credit_score, \
             employment_status, years_employed FROM shapes) d",
        )
        .expect("SPI failed")
        .unwrap();

        let time = |label: &str, sql: &str| -> std::time::Duration {
            // Warm, then measure.
            Spi::run(sql).unwrap_or_else(|e| panic!("{label} failed: {e}"));
            let start = std::time::Instant::now();
            Spi::run(sql).unwrap_or_else(|e| panic!("{label} failed: {e}"));
            start.elapsed()
        };

        // (A) The shape the docs show: evaluate once per row.
        let naive_concat = time(
            "naive concat",
            &format!(
                "SELECT dmn_eval(dmn_load('{concat}'), 'FullName', \
                 jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
                 FROM shapes"
            ),
        );

        // (B) Evaluate once per DISTINCT input, then join the answers back on.
        let dedup_concat = time(
            "dedup concat",
            &format!(
                "WITH answers AS (\
                   SELECT first_name, last_name, \
                          dmn_eval(dmn_load('{concat}'), 'FullName', \
                          jsonb_build_object('first_name', first_name, 'last_name', last_name)) AS r \
                   FROM (SELECT DISTINCT first_name, last_name FROM shapes) d) \
                 SELECT a.r FROM shapes s \
                 JOIN answers a USING (first_name, last_name)"
            ),
        );

        // (B2) The same dedup, but MATERIALIZED. Without this the planner inlines
        // the CTE and pulls `dmn_eval` up above the join, evaluating it once per
        // *output* row again—which silently undoes the deduplication.
        let dedup_mat_concat = time(
            "dedup materialized concat",
            &format!(
                "WITH answers AS MATERIALIZED (\
                   SELECT first_name, last_name, \
                          dmn_eval(dmn_load('{concat}'), 'FullName', \
                          jsonb_build_object('first_name', first_name, 'last_name', last_name)) AS r \
                   FROM (SELECT DISTINCT first_name, last_name FROM shapes) d) \
                 SELECT a.r FROM shapes s \
                 JOIN answers a USING (first_name, last_name)"
            ),
        );

        // (C) The naive shape, but with parallelism actually permitted.
        Spi::run("SET LOCAL max_parallel_workers_per_gather = 4").expect("SET failed");
        Spi::run("SET LOCAL parallel_setup_cost = 0").expect("SET failed");
        Spi::run("SET LOCAL parallel_tuple_cost = 0").expect("SET failed");
        Spi::run("SET LOCAL min_parallel_table_scan_size = 0").expect("SET failed");
        let parallel_concat = time(
            "parallel concat",
            &format!(
                "SELECT dmn_eval(dmn_load('{concat}'), 'FullName', \
                 jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
                 FROM shapes"
            ),
        );
        let parallel_dedup_concat = time(
            "parallel dedup concat",
            &format!(
                "WITH answers AS MATERIALIZED (\
                   SELECT first_name, last_name, \
                          dmn_eval(dmn_load('{concat}'), 'FullName', \
                          jsonb_build_object('first_name', first_name, 'last_name', last_name)) AS r \
                   FROM (SELECT DISTINCT first_name, last_name FROM shapes) d) \
                 SELECT a.r FROM shapes s \
                 JOIN answers a USING (first_name, last_name)"
            ),
        );
        // (E) Both at once. A MATERIALIZED CTE is scanned serially, which throws
        // parallelism away—so materialise into a real table instead, which the
        // planner *can* fill in parallel, then join that.
        let both_start = std::time::Instant::now();
        Spi::run(&format!(
            "CREATE UNLOGGED TABLE answers AS \
             SELECT first_name, last_name, \
                    dmn_eval(dmn_load('{concat}'), 'FullName', \
                    jsonb_build_object('first_name', first_name, 'last_name', last_name)) AS r \
             FROM (SELECT DISTINCT first_name, last_name FROM shapes) d"
        ))
        .expect("CTAS failed");
        Spi::run("SELECT a.r FROM shapes s JOIN answers a USING (first_name, last_name)")
            .expect("join failed");
        let both_concat = both_start.elapsed();
        Spi::run("DROP TABLE answers").expect("DROP failed");

        Spi::run("RESET max_parallel_workers_per_gather").expect("RESET failed");
        Spi::run("RESET parallel_setup_cost").expect("RESET failed");
        Spi::run("RESET parallel_tuple_cost").expect("RESET failed");
        Spi::run("RESET min_parallel_table_scan_size").expect("RESET failed");

        // (D) Is dmn_load('<literal>') folded once, or re-parsed per row? Put the
        // model in a table and force it to be a column reference to find out.
        Spi::run("CREATE TABLE shape_models (name text PRIMARY KEY, model dmnmodel NOT NULL)")
            .expect("CREATE TABLE failed");
        Spi::run(&format!(
            "INSERT INTO shape_models VALUES ('concat', dmn_load('{concat}'))"
        ))
        .expect("INSERT model failed");
        let from_column_concat = time(
            "model-from-column concat",
            "SELECT dmn_eval(m.model, 'FullName', \
             jsonb_build_object('first_name', s.first_name, 'last_name', s.last_name)) \
             FROM shapes s CROSS JOIN shape_models m WHERE m.name = 'concat'",
        );

        // The complex model, naive vs deduplicated.
        let naive_risk = time(
            "naive risk",
            &format!(
                "SELECT dmn_eval(dmn_load('{risk}'), 'RiskScore', \
                 jsonb_build_object('age', age, 'income', income, 'credit_score', credit_score, \
                 'employment_status', employment_status, 'years_employed', years_employed)) \
                 FROM shapes"
            ),
        );
        let dedup_risk = time(
            "dedup risk",
            &format!(
                "WITH answers AS MATERIALIZED (\
                   SELECT age, income, credit_score, employment_status, years_employed, \
                          dmn_eval(dmn_load('{risk}'), 'RiskScore', \
                          jsonb_build_object('age', age, 'income', income, \
                          'credit_score', credit_score, 'employment_status', employment_status, \
                          'years_employed', years_employed)) AS r \
                   FROM (SELECT DISTINCT age, income, credit_score, employment_status, \
                         years_employed FROM shapes) d) \
                 SELECT a.r FROM shapes s \
                 JOIN answers a USING (age, income, credit_score, employment_status, years_employed)"
            ),
        );

        let rc = rows as f64;
        let per_row = |d: std::time::Duration| d.as_micros() as f64 / rc;
        let speedup = |base: std::time::Duration, other: std::time::Duration| {
            base.as_secs_f64() / other.as_secs_f64()
        };

        let report = format!(
            "Query-shape benchmark: {rows} rows\n\
             distinct name combos: {distinct_names} | distinct risk combos: {distinct_risk}\n\
             \n\
             --- Simple (concat) ---\n\
             naive (per row):        {:.1} us/row ({:?})\n\
             dedup, CTE inlined:     {:.1} us/row ({:?})  {:.2}x\n\
             dedup, MATERIALIZED:    {:.1} us/row ({:?})  {:.2}x\n\
             parallel (4 workers):   {:.1} us/row ({:?})  {:.2}x\n\
             parallel + dedup (CTE): {:.1} us/row ({:?})  {:.2}x\n\
             parallel + dedup (tbl): {:.1} us/row ({:?})  {:.2}x\n\
             model from a column:    {:.1} us/row ({:?})  {:.2}x\n\
             \n\
             --- Complex (risk score) ---\n\
             naive (per row):        {:.1} us/row ({:?})\n\
             dedup, MATERIALIZED:    {:.1} us/row ({:?})  {:.2}x\n",
            per_row(naive_concat),
            naive_concat,
            per_row(dedup_concat),
            dedup_concat,
            speedup(naive_concat, dedup_concat),
            per_row(dedup_mat_concat),
            dedup_mat_concat,
            speedup(naive_concat, dedup_mat_concat),
            per_row(parallel_concat),
            parallel_concat,
            speedup(naive_concat, parallel_concat),
            per_row(parallel_dedup_concat),
            parallel_dedup_concat,
            speedup(naive_concat, parallel_dedup_concat),
            per_row(both_concat),
            both_concat,
            speedup(naive_concat, both_concat),
            per_row(from_column_concat),
            from_column_concat,
            speedup(naive_concat, from_column_concat),
            per_row(naive_risk),
            naive_risk,
            per_row(dedup_risk),
            dedup_risk,
            speedup(naive_risk, dedup_risk),
        );
        if let Err(e) = std::fs::write("/pgdmn/benchmark_shapes.txt", &report) {
            pgrx::warning!("Failed to write benchmark_shapes.txt: {}", e);
        }
        pgrx::warning!("{}", report);
    }

    // A model whose decisions each return a different FEEL type, so every typed
    // dmn_eval_* variant has something to unwrap.
    const TYPED_DMN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="typed" name="Typed" namespace="https://example.org/typed">
    <decision id="Verdict" name="Verdict">
        <variable name="Verdict" typeRef="string"/>
        <literalExpression><text>"Approved"</text></literalExpression>
    </decision>
    <decision id="Total" name="Total">
        <variable name="Total" typeRef="number"/>
        <literalExpression><text>100 + 10</text></literalExpression>
    </decision>
    <decision id="Eligible" name="Eligible">
        <variable name="Eligible" typeRef="boolean"/>
        <literalExpression><text>5 > 3</text></literalExpression>
    </decision>
    <decision id="Due" name="Due">
        <variable name="Due" typeRef="date"/>
        <literalExpression><text>date("2024-03-15")</text></literalExpression>
    </decision>
    <decision id="Stamp" name="Stamp">
        <variable name="Stamp" typeRef="dateTime"/>
        <literalExpression><text>date and time("2024-03-15T10:30:00")</text></literalExpression>
    </decision>
    <decision id="Term" name="Term">
        <variable name="Term" typeRef="yearMonthDuration"/>
        <literalExpression><text>duration("P2Y3M")</text></literalExpression>
    </decision>
</definitions>"#;

    #[pg_test]
    fn test_dmn_eval_text() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<String>(&format!(
            "SELECT dmn_eval_text(dmn_load('{escaped}'), 'Verdict')"
        ))
        .expect("SPI failed");
        // No `#>> '{}'`: a string decision comes back as text, unquoted.
        assert_eq!(result.unwrap(), "Approved");
    }

    #[pg_test]
    fn test_dmn_eval_numeric() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<pgrx::AnyNumeric>(&format!(
            "SELECT dmn_eval_numeric(dmn_load('{escaped}'), 'Total')"
        ))
        .expect("SPI failed");
        assert_eq!(result.unwrap().to_string(), "110");
    }

    #[pg_test]
    fn test_dmn_eval_numeric_is_arithmetic_without_a_cast() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<pgrx::AnyNumeric>(&format!(
            "SELECT round(dmn_eval_numeric(dmn_load('{escaped}'), 'Total') * 1.5, 2)"
        ))
        .expect("SPI failed");
        assert_eq!(result.unwrap().to_string(), "165.00");
    }

    #[pg_test]
    fn test_dmn_eval_bool() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<bool>(&format!(
            "SELECT dmn_eval_bool(dmn_load('{escaped}'), 'Eligible')"
        ))
        .expect("SPI failed");
        assert!(result.unwrap());
    }

    #[pg_test]
    fn test_dmn_eval_date() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<pgrx::datum::Date>(&format!(
            "SELECT dmn_eval_date(dmn_load('{escaped}'), 'Due')"
        ))
        .expect("SPI failed")
        .unwrap();
        assert_eq!((result.year(), result.month(), result.day()), (2024, 3, 15));
    }

    #[pg_test]
    fn test_dmn_eval_timestamp() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<pgrx::datum::Timestamp>(&format!(
            "SELECT dmn_eval_timestamp(dmn_load('{escaped}'), 'Stamp')"
        ))
        .expect("SPI failed")
        .unwrap();
        assert_eq!(
            (result.year(), result.month(), result.day(), result.hour()),
            (2024, 3, 15, 10)
        );
    }

    #[pg_test]
    fn test_dmn_eval_interval() {
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = Spi::get_one::<pgrx::datum::Interval>(&format!(
            "SELECT dmn_eval_interval(dmn_load('{escaped}'), 'Term')"
        ))
        .expect("SPI failed")
        .unwrap();
        assert_eq!(result.months(), 27); // P2Y3M
    }

    #[pg_test]
    fn test_dmn_eval_typed_rejects_the_wrong_type() {
        // Asking a string decision for a number is a mistake worth hearing about,
        // and the error names what came back instead.
        let escaped = TYPED_DMN.replace('\'', "''");
        let result = std::panic::catch_unwind(|| {
            Spi::get_one::<pgrx::AnyNumeric>(&format!(
                "SELECT dmn_eval_numeric(dmn_load('{escaped}'), 'Verdict')"
            ))
        });
        assert!(result.is_err(), "expected an error, got a number");
    }

    #[pg_test]
    fn test_dmn_eval_text_on_the_website_loan_model() {
        // The shape the website's SQL now uses.
        let escaped = LOAN_DMN.replace('\'', "''");
        let result = Spi::get_one::<String>(&format!(
            "SELECT dmn_eval_text(dmn_load('{escaped}'), 'Eligibility', \
             '{{\"Age\": 34, \"Income\": 82000, \"Bankrupt\": false}}'::jsonb)"
        ))
        .expect("SPI failed");
        assert_eq!(result.unwrap(), "Approved");
    }

    // --- Record-based evaluation tests ---

    #[pg_test]
    fn test_feel_record_eval_basic() {
        Spi::run("CREATE TYPE feel_rec_basic AS (x int, y int)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('x + y', ROW(3, 4)::feel_rec_basic)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(7));
    }

    #[pg_test]
    fn test_feel_record_eval_text() {
        Spi::run("CREATE TYPE feel_rec_text AS (greeting text)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            r#"SELECT feel_record_eval('greeting + " world"', ROW('hello')::feel_rec_text)"#,
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("hello world"));
    }

    #[pg_test]
    fn test_feel_record_eval_numeric() {
        Spi::run("CREATE TYPE feel_rec_num AS (val numeric)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('val * 3', ROW(1234567890.123456789::numeric)::feel_rec_num)",
        )
        .expect("SPI failed");
        let v = result.unwrap().0;
        let s = v.to_string();
        assert!(
            s.starts_with("3703703670.3703"),
            "unexpected numeric result: {s}"
        );
    }

    #[pg_test]
    fn test_dmn_record_eval_decision_table() {
        let escaped = DECISION_TABLE_DMN.replace('\'', "''");
        Spi::run("CREATE TYPE loan_input AS (\"Age\" int, \"Income\" numeric)")
            .expect("CREATE TYPE failed");
        let query = format!(
            "SELECT dmn_record_eval(dmn_load('{escaped}'), 'Eligibility', \
             ROW(30, 75000::numeric)::loan_input)"
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Approved"));
    }

    #[pg_test]
    fn test_dmn_record_eval_null_input() {
        let escaped = SIMPLE_DMN.replace('\'', "''");
        let query = format!("SELECT dmn_record_eval(dmn_load('{escaped}'), 'Greeting', NULL)");
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Hello, World!"));
    }

    #[pg_test]
    fn test_dmn_record_eval_multi_decision() {
        let escaped = MULTI_DECISION_DMN.replace('\'', "''");
        Spi::run("CREATE TYPE multi_input AS (\"Base Price\" numeric, \"Tax Rate\" numeric)")
            .expect("CREATE TYPE failed");
        let query = format!(
            "SELECT dmn_record_eval(dmn_load('{escaped}'), 'Total Price', \
             ROW(100::numeric, 0.2::numeric)::multi_input)"
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(120));
    }

    #[pg_test]
    fn test_feel_record_eval_date() {
        Spi::run("CREATE TYPE feel_rec_date AS (d date)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('d', ROW('2024-03-15'::date)::feel_rec_date)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("2024-03-15"));
    }

    #[pg_test]
    fn test_feel_record_eval_timestamp() {
        Spi::run("CREATE TYPE feel_rec_ts AS (ts timestamp)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('ts', ROW('2024-03-15 10:30:00'::timestamp)::feel_rec_ts)",
        )
        .expect("SPI failed");
        let s = result.unwrap().0.as_str().unwrap().to_string();
        assert!(
            s.starts_with("2024-03-15T10:30:00"),
            "unexpected timestamp: {s}"
        );
    }

    #[pg_test]
    fn test_feel_record_eval_interval_days() {
        Spi::run("CREATE TYPE feel_rec_iv_days AS (iv interval)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('iv', ROW('2 days 3 hours'::interval)::feel_rec_iv_days)",
        )
        .expect("SPI failed");
        let s = result.unwrap().0.as_str().unwrap().to_string();
        assert!(s.contains("P2DT3H"), "unexpected day-time duration: {s}");
    }

    #[pg_test]
    fn test_feel_record_eval_interval_months() {
        Spi::run("CREATE TYPE feel_rec_iv_months AS (iv interval)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('iv', ROW('3 months'::interval)::feel_rec_iv_months)",
        )
        .expect("SPI failed");
        let s = result.unwrap().0.as_str().unwrap().to_string();
        assert!(s.contains("P3M"), "unexpected year-month duration: {s}");
    }

    #[pg_test]
    fn test_feel_record_eval_interval_negative() {
        Spi::run("CREATE TYPE feel_rec_iv_neg AS (iv interval)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_record_eval('iv', ROW('-1.5 seconds'::interval)::feel_rec_iv_neg)",
        )
        .expect("SPI failed");
        let s = result.unwrap().0.as_str().unwrap().to_string();
        assert!(s.contains('P'), "unexpected negative duration: {s}");
    }

    #[pg_test]
    #[should_panic(expected = "FEEL does not support mixed intervals")]
    fn test_feel_record_eval_interval_mixed_error() {
        Spi::run("CREATE TYPE feel_rec_iv_mixed AS (iv interval)").expect("CREATE TYPE failed");
        Spi::run(
            "SELECT feel_record_eval('iv', ROW('1 month 2 days'::interval)::feel_rec_iv_mixed)",
        )
        .unwrap();
    }

    // --- Example file tests ---
    // Tests exercising the DMN example files in examples/

    const EXAMPLE_SIMPLE_APPROVAL: &str = include_str!("../examples/simple-approval.dmn");
    const EXAMPLE_MULTI_OUTPUT: &str = include_str!("../examples/multi-output-approval.dmn");
    const EXAMPLE_LOAN_PAYMENT: &str = include_str!("../examples/loan-payment.dmn");
    const EXAMPLE_MONTHLY_PAYMENT_BKM: &str = include_str!("../examples/monthly-payment-bkm.dmn");
    const EXAMPLE_VACATION_DAYS: &str = include_str!("../examples/vacation-days.dmn");
    const EXAMPLE_LENDING: &str = include_str!("../examples/lending.dmn");
    const EXAMPLE_LOAN_COMPARISON: &str = include_str!("../examples/loan-comparison.dmn");

    fn eval_example(xml: &str, invocable: &str, input_json: &str) -> serde_json::Value {
        let escaped_xml = xml.replace('\'', "''");
        let escaped_json = input_json.replace('\'', "''");
        let query = format!(
            "SELECT dmn_eval(dmn_load('{escaped_xml}'), '{invocable}', '{escaped_json}'::jsonb)"
        );
        Spi::get_one::<pgrx::JsonB>(&query)
            .expect("SPI failed")
            .expect("dmn_eval returned NULL")
            .0
    }

    // -- Simple Approval: UNIQUE hit policy, outputs "Approved" or "Declined" --

    #[pg_test]
    fn test_example_simple_approval_approved() {
        let result = eval_example(
            EXAMPLE_SIMPLE_APPROVAL,
            "Approval Status",
            r#"{"Age": 25, "RiskCategory": "Medium", "isAffordable": true}"#,
        );
        assert_eq!(result, serde_json::json!("Approved"));
    }

    #[pg_test]
    fn test_example_simple_approval_declined_underage() {
        let result = eval_example(
            EXAMPLE_SIMPLE_APPROVAL,
            "Approval Status",
            r#"{"Age": 17, "RiskCategory": "Low", "isAffordable": true}"#,
        );
        assert_eq!(result, serde_json::json!("Declined"));
    }

    #[pg_test]
    fn test_example_simple_approval_declined_high_risk() {
        let result = eval_example(
            EXAMPLE_SIMPLE_APPROVAL,
            "Approval Status",
            r#"{"Age": 30, "RiskCategory": "High", "isAffordable": true}"#,
        );
        assert_eq!(result, serde_json::json!("Declined"));
    }

    #[pg_test]
    fn test_example_simple_approval_declined_not_affordable() {
        let result = eval_example(
            EXAMPLE_SIMPLE_APPROVAL,
            "Approval Status",
            r#"{"Age": 30, "RiskCategory": "Low", "isAffordable": false}"#,
        );
        assert_eq!(result, serde_json::json!("Declined"));
    }

    // -- Multi-Output Approval: outputs (Status, Rate) pairs --

    #[pg_test]
    fn test_example_multi_output_approved_best() {
        let result = eval_example(
            EXAMPLE_MULTI_OUTPUT,
            "Approval",
            r#"{"Age": 25, "RiskCategory": "Low", "isAffordable": true}"#,
        );
        assert_eq!(result["Status"], serde_json::json!("Approved"));
        assert_eq!(result["Rate"], serde_json::json!("Best"));
    }

    #[pg_test]
    fn test_example_multi_output_approved_standard() {
        let result = eval_example(
            EXAMPLE_MULTI_OUTPUT,
            "Approval",
            r#"{"Age": 25, "RiskCategory": "Medium", "isAffordable": true}"#,
        );
        assert_eq!(result["Status"], serde_json::json!("Approved"));
        assert_eq!(result["Rate"], serde_json::json!("Standard"));
    }

    #[pg_test]
    fn test_example_multi_output_declined() {
        let result = eval_example(
            EXAMPLE_MULTI_OUTPUT,
            "Approval",
            r#"{"Age": 25, "RiskCategory": "High", "isAffordable": true}"#,
        );
        assert_eq!(result["Status"], serde_json::json!("Declined"));
        assert_eq!(result["Rate"], serde_json::json!("Standard"));
    }

    // -- Loan Payment: FEEL literal expression computing amortization --

    #[pg_test]
    fn test_example_loan_payment() {
        let result = eval_example(
            EXAMPLE_LOAN_PAYMENT,
            "payment",
            r#"{"loan": {"principal": 600000, "rate": 0.0375, "termMonths": 360}}"#,
        );
        let payment = result.as_f64().expect("expected numeric result");
        assert!(
            payment > 2000.0 && payment < 3500.0,
            "unexpected payment: {payment}"
        );
    }

    // -- Monthly Payment with BKM: reusable PMT function + fee --

    #[pg_test]
    fn test_example_monthly_payment_bkm() {
        let result = eval_example(
            EXAMPLE_MONTHLY_PAYMENT_BKM,
            "MonthlyPayment",
            r#"{"Loan": {"amount": 600000, "rate": 0.0375, "term": 360}, "fee": 100}"#,
        );
        let payment = result.as_f64().expect("expected numeric result");
        // Should be PMT + 100 fee, so > 100
        assert!(
            payment > 2000.0 && payment < 3600.0,
            "unexpected payment: {payment}"
        );
    }

    // -- Vacation Days: COLLECT/MAX hit policy, sub-decisions --

    #[pg_test]
    fn test_example_vacation_days_young() {
        // Age 16 (< 18) gets 5 extra days from case 1
        let result = eval_example(
            EXAMPLE_VACATION_DAYS,
            "Total Vacation Days",
            r#"{"Age": 16, "Years of Service": 1}"#,
        );
        assert_eq!(result, serde_json::json!(27));
    }

    #[pg_test]
    fn test_example_vacation_days_midcareer() {
        // Age 25, 5 years: no extra days
        let result = eval_example(
            EXAMPLE_VACATION_DAYS,
            "Total Vacation Days",
            r#"{"Age": 25, "Years of Service": 5}"#,
        );
        assert_eq!(result, serde_json::json!(22));
    }

    #[pg_test]
    fn test_example_vacation_days_senior() {
        // Age 44, 30 years: max extra days from multiple rules
        let result = eval_example(
            EXAMPLE_VACATION_DAYS,
            "Total Vacation Days",
            r#"{"Age": 44, "Years of Service": 30}"#,
        );
        assert_eq!(result, serde_json::json!(30));
    }

    // -- Lending: complex DRG with Strategy and Routing decisions --

    // Strategy: DECLINE (existing customer with risk score < 80 → DECLINE risk → INELIGIBLE)
    #[pg_test]
    fn test_example_lending_strategy_decline() {
        // Age 25, Single, Unemployed, ExistingCustomer=true
        // AppRiskScore = 35 + 25 + 15 = 75 → existing customer < 80 → DECLINE risk
        let result = eval_example(
            EXAMPLE_LENDING,
            "Strategy",
            r#"{
                "ApplicantData": {
                    "Age": 25, "MaritalStatus": "S", "EmploymentStatus": "UNEMPLOYED",
                    "ExistingCustomer": true,
                    "Monthly": {"Income": 2000, "Expenses": 1000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 10000, "Rate": 0.10, "Term": 36
                },
                "BureauData": {"CreditScore": 600, "Bankrupt": false}
            }"#,
        );
        assert_eq!(result, serde_json::json!("DECLINE"));
    }

    // Strategy: BUREAU (eligible but needs bureau call due to HIGH pre-bureau risk)
    #[pg_test]
    fn test_example_lending_strategy_bureau() {
        // Age 20, Single, Student → AppRiskScore = 32+25+18 = 75 → pre-bureau HIGH → FULL
        let result = eval_example(
            EXAMPLE_LENDING,
            "Strategy",
            r#"{
                "ApplicantData": {
                    "Age": 20, "MaritalStatus": "S", "EmploymentStatus": "STUDENT",
                    "ExistingCustomer": false,
                    "Monthly": {"Income": 5000, "Expenses": 1000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 50000, "Rate": 0.10, "Term": 36
                },
                "BureauData": {"CreditScore": 600, "Bankrupt": false}
            }"#,
        );
        assert_eq!(result, serde_json::json!("BUREAU"));
    }

    // Strategy: THROUGH (eligible, very low risk, no bureau call)
    #[pg_test]
    fn test_example_lending_strategy_through() {
        // Age 51, Married, Employed → AppRiskScore = 48+45+45 = 138 → VERY LOW → NONE
        let result = eval_example(
            EXAMPLE_LENDING,
            "Strategy",
            r#"{
                "ApplicantData": {
                    "Age": 51, "MaritalStatus": "M", "EmploymentStatus": "EMPLOYED",
                    "ExistingCustomer": false,
                    "Monthly": {"Income": 10000, "Expenses": 3000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 100000, "Rate": 0.06, "Term": 36
                },
                "BureauData": {"CreditScore": 700, "Bankrupt": false}
            }"#,
        );
        assert_eq!(result, serde_json::json!("THROUGH"));
    }

    // Routing: DECLINE (bankrupt applicant)
    #[pg_test]
    fn test_example_lending_routing_decline() {
        let result = eval_example(
            EXAMPLE_LENDING,
            "Routing",
            r#"{
                "ApplicantData": {
                    "Age": 51, "MaritalStatus": "M", "EmploymentStatus": "EMPLOYED",
                    "ExistingCustomer": false,
                    "Monthly": {"Income": 10000, "Expenses": 3000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 100000, "Rate": 0.06, "Term": 36
                },
                "BureauData": {"CreditScore": 700, "Bankrupt": true}
            }"#,
        );
        assert_eq!(result, serde_json::json!("DECLINE"));
    }

    // Routing: REFER (high post-bureau risk)
    #[pg_test]
    fn test_example_lending_routing_refer() {
        // Low credit score + low app risk score → post-bureau HIGH → REFER
        let result = eval_example(
            EXAMPLE_LENDING,
            "Routing",
            r#"{
                "ApplicantData": {
                    "Age": 20, "MaritalStatus": "S", "EmploymentStatus": "STUDENT",
                    "ExistingCustomer": false,
                    "Monthly": {"Income": 5000, "Expenses": 1000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 10000, "Rate": 0.10, "Term": 36
                },
                "BureauData": {"CreditScore": 500, "Bankrupt": false}
            }"#,
        );
        assert_eq!(result, serde_json::json!("REFER"));
    }

    // Routing: ACCEPT (good applicant)
    #[pg_test]
    fn test_example_lending_routing_accept() {
        let result = eval_example(
            EXAMPLE_LENDING,
            "Routing",
            r#"{
                "ApplicantData": {
                    "Age": 51, "MaritalStatus": "M", "EmploymentStatus": "EMPLOYED",
                    "ExistingCustomer": false,
                    "Monthly": {"Income": 10000, "Expenses": 3000, "Repayments": 0}
                },
                "RequestedProduct": {
                    "ProductType": "STANDARD LOAN", "Amount": 100000, "Rate": 0.06, "Term": 36
                },
                "BureauData": {"CreditScore": 700, "Bankrupt": false}
            }"#,
        );
        assert_eq!(result, serde_json::json!("ACCEPT"));
    }

    // -- Loan Comparison: iteration, sorting, embedded data, BKMs --

    #[pg_test]
    fn test_example_loan_comparison() {
        let result = eval_example(
            EXAMPLE_LOAN_COMPARISON,
            "RankedProducts",
            r#"{"RequestedAmt": 330000}"#,
        );
        // Should contain metricsTable with 10 lender entries
        let metrics = result["metricsTable"]
            .as_array()
            .expect("metricsTable should be an array");
        assert_eq!(
            metrics.len(),
            10,
            "expected 10 loan products in metricsTable"
        );

        // Each ranking should also have 10 entries
        for key in [
            "rankByRate",
            "rankByDownPmt",
            "rankByMonthlyPmt",
            "rankByEquityPct",
        ] {
            let ranked = result[key]
                .as_array()
                .unwrap_or_else(|| panic!("{key} should be an array"));
            assert_eq!(ranked.len(), 10, "{key} should have 10 entries");
        }

        // First in rankByRate should have the lowest rate
        let best_rate = result["rankByRate"][0]["rate"]
            .as_f64()
            .expect("rate should be a number");
        let worst_rate = result["rankByRate"][9]["rate"]
            .as_f64()
            .expect("rate should be a number");
        assert!(best_rate <= worst_rate, "rankByRate should be ascending");
    }

    #[pg_test]
    fn test_cache_different_models_independent() {
        let model_a = SIMPLE_DMN;
        let model_b = SIMPLE_DMN
            .replace("SimpleDecisions", "OtherModel")
            .replace("https://example.org/simple", "https://example.org/other")
            .replace("Hello", "Goodbye");

        let escaped_a = model_a.replace('\'', "''");
        let escaped_b = model_b.replace('\'', "''");

        let query_a = format!("SELECT dmn_eval(dmn_load('{escaped_a}'), 'Greeting')");
        let query_b = format!("SELECT dmn_eval(dmn_load('{escaped_b}'), 'Greeting')");

        // Deterministic cache observability (TEST-003): count evaluator
        // builds instead of comparing wall-clock timings.
        let builds = || {
            Spi::get_one::<i64>("SELECT dmn_evaluator_builds()")
                .expect("SPI failed")
                .expect("builds count returned NULL")
        };
        let builds_start = builds();

        // Load model A (cold—one build)
        let result_a = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL");
        assert_eq!(builds(), builds_start + 1, "model A should build once");

        // Load model B (cold—different XML, not cached)
        let result_b = Spi::get_one::<pgrx::JsonB>(&query_b)
            .expect("SPI failed")
            .expect("model B returned NULL");
        assert_eq!(builds(), builds_start + 2, "model B should build once");

        // Models produce different output—a false cache hit would fail here
        assert_eq!(result_a.0, serde_json::json!("Hello, World!"));
        assert_eq!(result_b.0, serde_json::json!("Goodbye, World!"));
        assert_ne!(
            result_a.0, result_b.0,
            "Models A and B returned the same result; cache keying may not distinguish them"
        );

        // Model A again (warm—served from cache, no new build)
        let result_a_warm = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL on warm run");
        assert_eq!(builds(), builds_start + 2, "warm model A must not rebuild");

        // Cached result matches original
        assert_eq!(result_a.0, result_a_warm.0);
    }

    #[pg_test]
    fn test_cache_same_name_different_xml() {
        // Two models with identical namespace AND name but different decision
        // logic: the evaluator cache is keyed by XML content hash, so anything
        // keyed on less than full content would collide here.
        let model_b = SIMPLE_DMN.replace("Hello, World!", "Goodbye, World!");
        let escaped_a = SIMPLE_DMN.replace('\'', "''");
        let escaped_b = model_b.replace('\'', "''");

        let query_a = format!("SELECT dmn_eval(dmn_load('{escaped_a}'), 'Greeting')");
        let query_b = format!("SELECT dmn_eval(dmn_load('{escaped_b}'), 'Greeting')");

        let result_a = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL");
        assert_eq!(result_a.0, serde_json::json!("Hello, World!"));

        let result_b = Spi::get_one::<pgrx::JsonB>(&query_b)
            .expect("SPI failed")
            .expect("model B returned NULL");
        assert_eq!(result_b.0, serde_json::json!("Goodbye, World!"));

        // Model A again: the cache must still hold A's evaluator, not B's.
        let result_a_again = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL on second run");
        assert_eq!(result_a_again.0, serde_json::json!("Hello, World!"));
    }

    // The models published on the website, evaluated against the exact rows of
    // the CSV datasets published alongside them. The site tells visitors what
    // these produce; these tests are what make that claim true, and what will
    // fail loudly if a model or an engine upgrade ever changes the answer.
    const LOAN_DMN: &str = include_str!("../website/public/examples/loan-eligibility.dmn");
    const PRICING_DMN: &str = include_str!("../website/public/examples/order-pricing.dmn");
    const PROMO_DMN: &str = include_str!("../website/public/examples/order-pricing-promo.dmn");
    const ROUTING_DMN: &str = include_str!("../website/public/examples/ticket-routing.dmn");
    const COMPLIANCE_DMN: &str = include_str!("../website/public/examples/compliance.dmn");

    fn eligibility(age: i32, income: i32, bankrupt: bool) -> serde_json::Value {
        eval_example(
            LOAN_DMN,
            "Eligibility",
            &format!(r#"{{"Age": {age}, "Income": {income}, "Bankrupt": {bankrupt}}}"#),
        )
    }

    #[pg_test]
    fn test_example_loan_applicants() {
        // applicants.csv, row by row.
        assert_eq!(eligibility(34, 82000, false), serde_json::json!("Approved")); // Ada
        assert_eq!(
            eligibility(17, 0, false),
            serde_json::json!("Denied: underage")
        ); // Bo
        assert_eq!(
            eligibility(29, 41000, false),
            serde_json::json!("Denied: low income")
        ); // Chen
        assert_eq!(eligibility(64, 68000, false), serde_json::json!("Approved")); // Gus
    }

    #[pg_test]
    fn test_example_loan_boundaries() {
        // Income of exactly 50000 is approved; a pound short is not.
        assert_eq!(eligibility(22, 50000, false), serde_json::json!("Approved")); // Eli
        assert_eq!(
            eligibility(19, 49999, false),
            serde_json::json!("Denied: low income")
        ); // Fay
    }

    #[pg_test]
    fn test_example_loan_boolean_input() {
        // Dara earns 120000 and would sail through on the numbers—but the
        // boolean says otherwise, and its rule is listed above the income rules.
        assert_eq!(
            eligibility(45, 120_000, true),
            serde_json::json!("Denied: prior bankruptcy")
        );
        // The same applicant without the bankruptcy is approved.
        assert_eq!(
            eligibility(45, 120_000, false),
            serde_json::json!("Approved")
        );
    }

    #[pg_test]
    fn test_example_loan_first_hit_policy_wins() {
        // Hana is 17 with a large income. The underage rule is listed first, and
        // the table is FIRST, so it wins—this is the row that makes the hit
        // policy visible on the website.
        assert_eq!(
            eligibility(17, 95000, false),
            serde_json::json!("Denied: underage")
        );
    }

    /// Evaluate a pricing decision exactly the way the website's SQL does —
    /// unwrap the JSONB, cast to numeric, round to pennies—and return what
    /// psql would print.
    ///
    /// Asserting on the rounded numeric rather than the raw FEEL number is
    /// deliberate: whether a whole result serialises as `10` or `10.0` is an
    /// engine detail, while `10.00` is what the page promises a reader.
    fn priced(model: &str, base: &str, rate: &str, invocable: &str) -> String {
        let escaped_xml = model.replace('\'', "''");
        let query = format!(
            "SELECT round((dmn_eval(dmn_load('{escaped_xml}'), '{invocable}', \
             '{{\"Base Price\": {base}, \"Tax Rate\": {rate}}}'::jsonb) #>> '{{}}')::numeric, 2)::text"
        );
        Spi::get_one::<String>(&query)
            .expect("SPI failed")
            .expect("dmn_eval returned NULL")
    }

    #[pg_test]
    fn test_example_order_pricing_chains_decisions() {
        // Total Price depends on Tax Amount, which pgdmn resolves without the
        // caller asking for it: we only ever request the decision we want.
        assert_eq!(priced(PRICING_DMN, "100.00", "0.10", "Tax Amount"), "10.00");
        assert_eq!(
            priced(PRICING_DMN, "100.00", "0.10", "Total Price"),
            "110.00"
        );
    }

    #[pg_test]
    fn test_example_orders_match_the_published_table() {
        // Every row of orders.csv under the standard model, and every figure
        // printed in the results table on the website.
        let rows = [
            ("100.00", "0.10", "10.00", "110.00"),      // Northwind Traders
            ("2499.99", "0.0825", "206.25", "2706.24"), // Globex
            ("45.50", "0.20", "9.10", "54.60"),         // Initech
            ("1000.00", "0.00", "0.00", "1000.00"),     // Umbrella Corp
            ("19.99", "0.075", "1.50", "21.49"),        // Acme Supply
        ];

        for (base, rate, tax, total) in rows {
            assert_eq!(
                priced(PRICING_DMN, base, rate, "Tax Amount"),
                tax,
                "tax for {base} @ {rate}"
            );
            assert_eq!(
                priced(PRICING_DMN, base, rate, "Total Price"),
                total,
                "total for {base} @ {rate}"
            );
        }
    }

    #[pg_test]
    fn test_example_promo_model_prices_the_same_orders_differently() {
        // The promotional model takes 10% off first, then taxes the net price.
        // Same orders, same query, same invocable name—a different model row.
        let rows = [
            ("100.00", "0.10", "99.00"),      // Northwind Traders
            ("2499.99", "0.0825", "2435.62"), // Globex
            ("45.50", "0.20", "49.14"),       // Initech
            ("1000.00", "0.00", "900.00"),    // Umbrella Corp
            ("19.99", "0.075", "19.34"),      // Acme Supply
        ];

        for (base, rate, total) in rows {
            assert_eq!(
                priced(PROMO_DMN, base, rate, "Total Price"),
                total,
                "promo total for {base} @ {rate}"
            );
        }

        // And it exposes the same invocable names, which is what lets one query
        // serve both models.
        assert_eq!(priced(PROMO_DMN, "100.00", "0.10", "Net Price"), "90.00");
        assert_eq!(priced(PROMO_DMN, "100.00", "0.10", "Tax Amount"), "9.00");
    }

    #[pg_test]
    fn test_example_pricing_policies_takes_effect_switch() {
        // The pricing example stores two dated versions of one `retail` policy in
        // a `pricing_policies` table and lets the query pick whichever is in
        // effect on a given day—the latest whose takes_effect has arrived.
        // Through June that is the standard model; from 1 July it is the
        // promotional one, with nothing updated in between. This is the switch
        // the article and examples page turn on, so verify it against the real
        // engine rather than trusting the prose.
        let standard = PRICING_DMN.replace('\'', "''");
        let promo = PROMO_DMN.replace('\'', "''");

        Spi::run(&format!(
            "CREATE TEMP TABLE pricing_policies (
               name         text NOT NULL,
               takes_effect date NOT NULL,
               model        dmnmodel NOT NULL,
               PRIMARY KEY (name, takes_effect));
             INSERT INTO pricing_policies VALUES
               ('retail', DATE '2026-01-01', dmn_load('{standard}')),
               ('retail', DATE '2026-07-01', dmn_load('{promo}'));"
        ))
        .expect("failed to set up pricing_policies");

        // Single order (100.00 @ 0.10) under the version in effect on the day —
        // the article's "Set up" query and its result.
        let price_as_of = |as_of: &str| -> String {
            Spi::get_one::<pgrx::AnyNumeric>(&format!(
                "SELECT round(dmn_eval_numeric(model, 'Total Price', \
                   '{{\"Base Price\": 100.00, \"Tax Rate\": 0.10}}'::jsonb), 2) \
                 FROM pricing_policies \
                 WHERE name = 'retail' AND takes_effect <= DATE '{as_of}' \
                 ORDER BY takes_effect DESC LIMIT 1"
            ))
            .expect("SPI failed")
            .expect("no policy in effect")
            .to_string()
        };
        assert_eq!(price_as_of("2026-06-30"), "110.00", "standard through June");
        assert_eq!(price_as_of("2026-07-01"), "99.00", "promo from 1 July");

        // The whole book (orders.csv) under the in-effect version: the standard
        // book through June, the promotional book from July—the same totals
        // the article's cost query and the examples page's table print.
        let book_as_of = |as_of: &str| -> String {
            Spi::get_one::<pgrx::AnyNumeric>(&format!(
                "WITH orders(base_price, tax_rate) AS (VALUES \
                   (100.00, 0.10), (2499.99, 0.0825), (45.50, 0.20), \
                   (1000.00, 0.00), (19.99, 0.075)) \
                 SELECT round(sum(dmn_eval_numeric(pol.model, 'Total Price', \
                   jsonb_build_object('Base Price', base_price, 'Tax Rate', tax_rate))), 2) \
                 FROM orders \
                 CROSS JOIN LATERAL ( \
                   SELECT model FROM pricing_policies \
                   WHERE name = 'retail' AND takes_effect <= DATE '{as_of}' \
                   ORDER BY takes_effect DESC LIMIT 1) pol"
            ))
            .expect("SPI failed")
            .expect("no policy in effect")
            .to_string()
        };
        assert_eq!(
            book_as_of("2026-06-30"),
            "3892.33",
            "standard book through June"
        );
        assert_eq!(
            book_as_of("2026-07-01"),
            "3503.10",
            "promo book from 1 July"
        );
    }

    /// The aggregate figures printed in the "Going further" sections of the
    /// articles—approval rate, book values, promotion cost—computed by the
    /// real engine over the published datasets, exactly as the articles' SQL
    /// does. Hand-computed aggregates are the easy place for a wrong number to
    /// slip onto the site; this is what keeps them honest.
    #[pg_test]
    fn test_article_aggregate_figures() {
        let loan = LOAN_DMN.replace('\'', "''");
        let standard = PRICING_DMN.replace('\'', "''");
        let promo = PROMO_DMN.replace('\'', "''");

        // Loan approval rate over applicants.csv: 3 approved of 8 -> 37.5%.
        let approval = Spi::get_one::<pgrx::AnyNumeric>(&format!(
            "WITH applicants(age, income, bankrupt) AS (VALUES \
               (34, 82000, false), (17, 0, false), (29, 41000, false), \
               (45, 120000, true), (22, 50000, false), (19, 49999, false), \
               (64, 68000, false), (17, 95000, false)) \
             SELECT round(100.0 * count(*) FILTER (WHERE dmn_eval_text(dmn_load('{loan}'), \
               'Eligibility', jsonb_build_object('Age', age, 'Income', income, \
               'Bankrupt', bankrupt)) = 'Approved') / count(*), 1) \
             FROM applicants"
        ))
        .expect("SPI failed")
        .expect("null");
        assert_eq!(approval.to_string(), "37.5");

        // Pricing book values over orders.csv, summed unrounded then rounded —
        // the shape the order-pricing article's query uses.
        let orders = "orders(base_price, tax_rate) AS (VALUES \
             (100.00, 0.10), (2499.99, 0.0825), (45.50, 0.20), \
             (1000.00, 0.00), (19.99, 0.075))";
        let book = |model: &str| -> String {
            Spi::get_one::<pgrx::AnyNumeric>(&format!(
                "WITH {orders} \
                 SELECT round(sum(dmn_eval_numeric(dmn_load('{model}'), 'Total Price', \
                   jsonb_build_object('Base Price', base_price, 'Tax Rate', tax_rate))), 2) \
                 FROM orders"
            ))
            .expect("SPI failed")
            .expect("null")
            .to_string()
        };
        assert_eq!(book(&standard), "3892.33", "standard book value");
        assert_eq!(book(&promo), "3503.10", "promo book value");

        let given_away = Spi::get_one::<pgrx::AnyNumeric>(&format!(
            "WITH {orders} \
             SELECT round(sum(dmn_eval_numeric(dmn_load('{standard}'), 'Total Price', \
                   jsonb_build_object('Base Price', base_price, 'Tax Rate', tax_rate)) \
                 - dmn_eval_numeric(dmn_load('{promo}'), 'Total Price', \
                   jsonb_build_object('Base Price', base_price, 'Tax Rate', tax_rate))), 2) \
             FROM orders"
        ))
        .expect("SPI failed")
        .expect("null");
        assert_eq!(given_away.to_string(), "389.23", "promotion cost");
    }

    fn queue(priority: &str, tier: &str) -> serde_json::Value {
        eval_example(
            ROUTING_DMN,
            "Queue",
            &format!(r#"{{"Priority": "{priority}", "Customer Tier": "{tier}"}}"#),
        )
    }

    #[pg_test]
    fn test_example_ticket_routing() {
        // tickets.csv, row by row.
        assert_eq!(queue("critical", "startup"), serde_json::json!("pager"));
        assert_eq!(queue("high", "enterprise"), serde_json::json!("pager"));
        assert_eq!(queue("low", "startup"), serde_json::json!("tier-1"));
        assert_eq!(queue("high", "startup"), serde_json::json!("tier-2"));
        assert_eq!(queue("normal", "enterprise"), serde_json::json!("tier-2"));
        assert_eq!(queue("low", "free"), serde_json::json!("tier-1"));
        assert_eq!(queue("critical", "enterprise"), serde_json::json!("pager"));
        assert_eq!(queue("normal", "free"), serde_json::json!("tier-1"));
    }

    fn handling(region: &str, data_class: &str) -> serde_json::Value {
        eval_example(
            COMPLIANCE_DMN,
            "Handling",
            &format!(r#"{{"Region": "{region}", "Data Class": "{data_class}"}}"#),
        )
    }

    #[pg_test]
    fn test_example_compliance_handling() {
        // customers.csv, row by row. "special" is caught first regardless of
        // region, so Umbrella's EU residency never gets a say.
        assert_eq!(
            handling("EU", "personal"),
            serde_json::json!("store in EU, retain 24 months")
        );
        assert_eq!(
            handling("US", "personal"),
            serde_json::json!("retain 24 months")
        );
        assert_eq!(
            handling("US", "special"),
            serde_json::json!("encrypt, restrict access, retain 6 months")
        );
        assert_eq!(
            handling("EU", "special"),
            serde_json::json!("encrypt, restrict access, retain 6 months")
        );
        assert_eq!(
            handling("UK", "public"),
            serde_json::json!("standard handling")
        );
        assert_eq!(
            handling("EU", "public"),
            serde_json::json!("standard handling")
        );
        assert_eq!(
            handling("UK", "personal"),
            serde_json::json!("retain 24 months")
        );
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // No setup needed
    }

    pub const fn postgresql_conf_options() -> Vec<&'static str> {
        Vec::new()
    }
}
