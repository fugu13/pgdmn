::pgrx::pg_module_magic!();

mod cache;
mod convert;
mod functions;
mod types;

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_feel_eval_simple_addition() {
        let result =
            Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('1 + 2')").expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(3));
    }

    #[pg_test]
    fn test_feel_eval_with_context() {
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_eval('x * 2', '{\"x\": 21}'::jsonb)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(42));
    }

    #[pg_test]
    fn test_feel_eval_list() {
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_eval('for i in [1,2,3] return i * i')",
        )
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
        assert_eq!(result.unwrap(), true);
    }

    #[pg_test]
    fn test_feel_eval_bool_with_context() {
        let result = Spi::get_one::<bool>(
            "SELECT feel_eval_bool('age >= 18', '{\"age\": 25}'::jsonb)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap(), true);
    }

    #[pg_test]
    fn test_feel_eval_text() {
        let result = Spi::get_one::<String>(
            "SELECT feel_eval_text('\"hello\" + \" \" + \"world\"')",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap(), "hello world");
    }

    #[pg_test]
    fn test_feel_eval_date() {
        let result = Spi::get_one::<pgrx::datum::Date>(
            "SELECT feel_eval_date('date(\"2024-03-15\")')",
        )
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
        assert_eq!(result.unwrap(), true);
    }

    #[pg_test]
    fn test_feel_eval_interval_months() {
        let result = Spi::get_one::<bool>(
            "SELECT feel_eval_interval('duration(\"P3M\")') = interval '3 months'",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap(), true);
    }

    #[pg_test]
    fn test_feel_eval_null_context() {
        let result =
            Spi::get_one::<pgrx::JsonB>("SELECT feel_eval('1 + 1')").expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(2));
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
        let query = format!("SELECT dmn_name(dmn_load('{}'))", SIMPLE_DMN.replace('\'', "''"));
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
                        row.get_by_name::<String, _>("name")
                            .unwrap()
                            .unwrap(),
                        row.get_by_name::<String, _>("kind")
                            .unwrap()
                            .unwrap(),
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
        let query = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')",
            escaped
        );

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

    #[pg_test]
    fn bench_dmn_eval_vs_pg_concat() {
        let escaped = CONCAT_DMN.replace('\'', "''");

        // Create a table with skewed duplication: 10,000 unique rows,
        // then 100 of those get extra copies (1000, 500, 250, 125, and 50 each)
        Spi::run(
            "CREATE TABLE bench_names (first_name TEXT NOT NULL, last_name TEXT NOT NULL)",
        )
        .expect("CREATE TABLE failed");

        // Generate 10,000 unique name pairs (100 x 100)
        Spi::run(
            "INSERT INTO bench_names (first_name, last_name)
             SELECT 'First_' || lpad(f.n::text, 3, '0'),
                    'Last_' || lpad(l.n::text, 3, '0')
             FROM generate_series(1, 100) AS f(n)
             CROSS JOIN generate_series(1, 100) AS l(n)",
        )
        .expect("base INSERT failed");

        // Add skewed duplicates for the first 100 rows (first_name=First_001):
        // row 1: 1000 copies, row 2: 500, row 3: 250, row 4: 125, rows 5-100: 50 each
        for (last_n, copies) in [(1, 1000), (2, 500), (3, 250), (4, 125)] {
            Spi::run(&format!(
                "INSERT INTO bench_names (first_name, last_name)
                 SELECT 'First_001', 'Last_{last_n:03}'
                 FROM generate_series(1, {copies})"
            ))
            .expect("skewed INSERT failed");
        }
        Spi::run(
            "INSERT INTO bench_names (first_name, last_name)
             SELECT 'First_001', 'Last_' || lpad(n::text, 3, '0')
             FROM generate_series(5, 100) AS s(n)
             CROSS JOIN generate_series(1, 50)",
        )
        .expect("bulk skewed INSERT failed");

        let row_count = Spi::get_one::<i64>("SELECT count(*) FROM bench_names")
            .expect("SPI failed")
            .unwrap();
        let distinct_count = Spi::get_one::<i64>(
            "SELECT count(DISTINCT (first_name, last_name)) FROM bench_names",
        )
        .expect("SPI failed")
        .unwrap();

        // Warm both query paths before timing
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped}'), 'FullName', \
             jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN warmup failed");
        Spi::run("SELECT first_name || ' ' || last_name FROM bench_names LIMIT 1")
            .expect("PG warmup failed");

        // Benchmark: DMN evaluation over the table
        let dmn_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped}'), 'FullName', \
             jsonb_build_object('first_name', first_name, 'last_name', last_name)) \
             FROM bench_names"
        ))
        .expect("DMN eval query failed");
        let dmn_duration = dmn_start.elapsed();

        // Benchmark: equivalent PostgreSQL concat expression
        let pg_start = std::time::Instant::now();
        Spi::run(
            "SELECT first_name || ' ' || last_name FROM bench_names",
        )
        .expect("PG concat query failed");
        let pg_duration = pg_start.elapsed();

        let ratio = dmn_duration.as_secs_f64() / pg_duration.as_secs_f64();

        // Report results
        let report = format!(
            "Benchmark: {row_count} rows, {distinct_count} distinct input combos\n\
             DMN eval:  {:.1} us/row ({:?} total)\n\
             PG concat: {:.1} us/row ({:?} total)\n\
             Ratio:     {:.1}x",
            dmn_duration.as_micros() as f64 / row_count as f64,
            dmn_duration,
            pg_duration.as_micros() as f64 / row_count as f64,
            pg_duration,
            ratio,
        );
        if let Err(e) = std::fs::write("/pgdmn/benchmark_results.txt", &report) {
            pgrx::warning!("Failed to write benchmark_results.txt: {}", e);
        }
        pgrx::warning!("{}", report);

        // Sanity check: both approaches produce the same results
        let mismatches = Spi::get_one::<i64>(&format!(
            "SELECT count(*) FROM bench_names \
             WHERE (dmn_eval(dmn_load('{escaped}'), 'FullName', \
                    jsonb_build_object('first_name', first_name, 'last_name', last_name))) #>> '{{}}' \
                   <> first_name || ' ' || last_name"
        ))
        .expect("SPI failed")
        .unwrap();
        assert_eq!(mismatches, 0, "DMN and PG concat produced different results");
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

        let query_a = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')", escaped_a
        );
        let query_b = format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')", escaped_b
        );

        // Load model A (cold)
        let cold_a_start = std::time::Instant::now();
        let result_a = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL");
        let cold_a = cold_a_start.elapsed();

        // Load model B (cold — different XML, not cached)
        let result_b = Spi::get_one::<pgrx::JsonB>(&query_b)
            .expect("SPI failed")
            .expect("model B returned NULL");

        // Models produce different output — a false cache hit would fail here
        assert_eq!(result_a.0, serde_json::json!("Hello, World!"));
        assert_eq!(result_b.0, serde_json::json!("Goodbye, World!"));
        assert_ne!(
            result_a.0, result_b.0,
            "Models A and B returned the same result; cache keying may not distinguish them"
        );

        // Model A again (warm — should be cached)
        let warm_a_start = std::time::Instant::now();
        let result_a_warm = Spi::get_one::<pgrx::JsonB>(&query_a)
            .expect("SPI failed")
            .expect("model A returned NULL on warm run");
        let warm_a = warm_a_start.elapsed();

        // Cached result matches original
        assert_eq!(result_a.0, result_a_warm.0);

        // Warm A should be significantly faster than cold A
        assert!(
            warm_a < cold_a / 2,
            "Model A was not faster on second call: cold={cold_a:?}, warm={warm_a:?}"
        );
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // No setup needed
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
