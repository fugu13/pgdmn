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

        // Create a table with a mix of repeated and unique values
        Spi::run(
            "CREATE TABLE bench_names (first_name TEXT NOT NULL, last_name TEXT NOT NULL)",
        )
        .expect("CREATE TABLE failed");

        // Insert 1000 rows: 10 distinct first names x 10 distinct last names = 100 combos,
        // each repeated 10 times, giving a realistic mix of cache hits
        Spi::run(
            "INSERT INTO bench_names (first_name, last_name)
             SELECT first_names.n, last_names.n
             FROM unnest(ARRAY['Alice','Bob','Carol','Dave','Eve',
                               'Frank','Grace','Heidi','Ivan','Judy']) AS first_names(n)
             CROSS JOIN unnest(ARRAY['Smith','Jones','Brown','Davis','Miller',
                                     'Wilson','Moore','Taylor','Anderson','Thomas']) AS last_names(n)
             CROSS JOIN generate_series(1, 10)",
        )
        .expect("INSERT failed");

        let row_count = Spi::get_one::<i64>("SELECT count(*) FROM bench_names")
            .expect("SPI failed")
            .unwrap();
        assert_eq!(row_count, 1000);

        // Warm the cache with one call
        Spi::run(&format!(
            "SELECT dmn_eval(dmn_load('{escaped}'), 'FullName', \
             jsonb_build_object('first_name', 'warmup', 'last_name', 'warmup'))"
        ))
        .expect("warmup failed");

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
            "Benchmark: 1000 rows, 100 distinct input combos\n\
             DMN eval:  {:.1} us/row ({:?} total)\n\
             PG concat: {:.1} us/row ({:?} total)\n\
             Ratio:     {:.1}x",
            dmn_duration.as_micros() as f64 / row_count as f64,
            dmn_duration,
            pg_duration.as_micros() as f64 / row_count as f64,
            pg_duration,
            ratio,
        );
        // Write to mounted volume so results are visible on the host
        std::fs::write("/pgdmn/benchmark_results.txt", &report).ok();
        pgrx::warning!("{}", report);

        // Sanity check: both approaches produce the same results
        let mismatches = Spi::get_one::<i64>(&format!(
            "SELECT count(*) FROM bench_names \
             WHERE (dmn_eval(dmn_load('{escaped}'), 'FullName', \
                    jsonb_build_object('first_name', first_name, 'last_name', last_name)))->>0 \
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
