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
        let _ = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        let cold_duration = cold_start.elapsed();

        // Warm calls: subsequent evaluations use cached evaluator
        let iterations = 100;
        let warm_start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        }
        let warm_duration = warm_start.elapsed();
        let warm_avg = warm_duration / iterations;

        // Cached evaluation should be at least 2x faster than cold
        assert!(
            warm_avg < cold_duration / 2,
            "Cache did not provide expected speedup: cold={cold_duration:?}, warm_avg={warm_avg:?}"
        );
    }

    #[pg_test]
    fn test_cache_different_models_independent() {
        let model_a = SIMPLE_DMN;
        let model_b = SIMPLE_DMN.replace("SimpleDecisions", "OtherModel")
            .replace("https://example.org/simple", "https://example.org/other");

        let escaped_a = model_a.replace('\'', "''");
        let escaped_b = model_b.replace('\'', "''");

        // Load model A (cold)
        let cold_a_start = std::time::Instant::now();
        let _ = Spi::get_one::<pgrx::JsonB>(&format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')", escaped_a
        )).expect("SPI failed");
        let cold_a = cold_a_start.elapsed();

        // Load model B (cold — different XML, not cached)
        let cold_b_start = std::time::Instant::now();
        let _ = Spi::get_one::<pgrx::JsonB>(&format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')", escaped_b
        )).expect("SPI failed");
        let cold_b = cold_b_start.elapsed();

        // Model A again (warm — should be cached)
        let warm_a_start = std::time::Instant::now();
        let _ = Spi::get_one::<pgrx::JsonB>(&format!(
            "SELECT dmn_eval(dmn_load('{}'), 'Greeting')", escaped_a
        )).expect("SPI failed");
        let warm_a = warm_a_start.elapsed();

        // Warm A should be significantly faster than cold A
        assert!(
            warm_a < cold_a / 2,
            "Model A was not faster on second call: cold={cold_a:?}, warm={warm_a:?}"
        );

        // Cold B should be comparable to cold A (both require parsing),
        // not dramatically faster (which would indicate false cache hit)
        // We just check B also took non-trivial time relative to warm A
        assert!(
            cold_b > warm_a,
            "Model B first call was suspiciously fast — may be a false cache hit: cold_b={cold_b:?}, warm_a={warm_a:?}"
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
