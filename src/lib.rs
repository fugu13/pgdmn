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
    /// Same model, same rows, same answers — only the query differs. Gated
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
        // *output* row again — which silently undoes the deduplication.
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
        // parallelism away — so materialise into a real table instead, which the
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
        // Dara earns 120000 and would sail through on the numbers — but the
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
        // the table is FIRST, so it wins — this is the row that makes the hit
        // policy visible on the website.
        assert_eq!(
            eligibility(17, 95000, false),
            serde_json::json!("Denied: underage")
        );
    }

    /// Evaluate a pricing decision exactly the way the website's SQL does —
    /// unwrap the JSONB, cast to numeric, round to pennies — and return what
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
        // Same orders, same query, same invocable name — a different model row.
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
