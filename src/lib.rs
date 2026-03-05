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
        let distinct_count = Spi::get_one::<i64>(
            "SELECT count(DISTINCT (first_name, last_name)) FROM bench_names",
        )
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
             END FROM bench_names LIMIT 1"
        )
        .expect("PG plain risk warmup failed");
        Spi::run(&format!(
            "SELECT dmn_eval_record(dmn_load('{escaped_concat}'), 'FullName', \
             ROW(first_name, last_name)::concat_input) \
             FROM bench_names LIMIT 1"
        ))
        .expect("DMN record concat warmup failed");
        Spi::run(&format!(
            "SELECT dmn_eval_record(dmn_load('{escaped_risk}'), 'RiskScore', \
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
            "SELECT dmn_eval_record(dmn_load('{escaped_concat}'), 'FullName', \
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
             END FROM bench_names"
        )
        .expect("PG plain risk query failed");
        let pg_plain_risk_dur = pg_plain_risk_start.elapsed();

        let dmn_record_risk_start = std::time::Instant::now();
        Spi::run(&format!(
            "SELECT dmn_eval_record(dmn_load('{escaped_risk}'), 'RiskScore', \
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
            dmn_concat_dur.as_micros() as f64 / rc, dmn_concat_dur,
            dmn_record_concat_dur.as_micros() as f64 / rc, dmn_record_concat_dur,
            pg_jsonb_concat_dur.as_micros() as f64 / rc, pg_jsonb_concat_dur,
            pg_plain_concat_dur.as_micros() as f64 / rc, pg_plain_concat_dur,
            dmn_record_concat_dur.as_secs_f64() / dmn_concat_dur.as_secs_f64(),
            dmn_concat_dur.as_secs_f64() / pg_plain_concat_dur.as_secs_f64(),
            dmn_record_concat_dur.as_secs_f64() / pg_plain_concat_dur.as_secs_f64(),
            dmn_risk_dur.as_micros() as f64 / rc, dmn_risk_dur,
            dmn_record_risk_dur.as_micros() as f64 / rc, dmn_record_risk_dur,
            pg_jsonb_risk_dur.as_micros() as f64 / rc, pg_jsonb_risk_dur,
            pg_plain_risk_dur.as_micros() as f64 / rc, pg_plain_risk_dur,
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
        assert_eq!(mismatches, 0, "DMN and PG concat produced different results");
    }

    // --- Record-based evaluation tests ---

    #[pg_test]
    fn test_feel_eval_record_basic() {
        Spi::run("CREATE TYPE feel_rec_basic AS (x int, y int)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_eval_record('x + y', ROW(3, 4)::feel_rec_basic)",
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(7));
    }

    #[pg_test]
    fn test_feel_eval_record_text() {
        Spi::run("CREATE TYPE feel_rec_text AS (greeting text)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            r#"SELECT feel_eval_record('greeting + " world"', ROW('hello')::feel_rec_text)"#,
        )
        .expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("hello world"));
    }

    #[pg_test]
    fn test_feel_eval_record_numeric() {
        Spi::run("CREATE TYPE feel_rec_num AS (val numeric)").expect("CREATE TYPE failed");
        let result = Spi::get_one::<pgrx::JsonB>(
            "SELECT feel_eval_record('val * 3', ROW(1234567890.123456789::numeric)::feel_rec_num)",
        )
        .expect("SPI failed");
        let v = result.unwrap().0;
        let s = v.to_string();
        assert!(s.starts_with("3703703670.3703"), "unexpected numeric result: {s}");
    }

    #[pg_test]
    fn test_dmn_eval_record_decision_table() {
        let escaped = DECISION_TABLE_DMN.replace('\'', "''");
        Spi::run("CREATE TYPE loan_input AS (\"Age\" int, \"Income\" numeric)")
            .expect("CREATE TYPE failed");
        let query = format!(
            "SELECT dmn_eval_record(dmn_load('{escaped}'), 'Eligibility', \
             ROW(30, 75000::numeric)::loan_input)"
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Approved"));
    }

    #[pg_test]
    fn test_dmn_eval_record_null_input() {
        let escaped = SIMPLE_DMN.replace('\'', "''");
        let query = format!(
            "SELECT dmn_eval_record(dmn_load('{escaped}'), 'Greeting', NULL)"
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!("Hello, World!"));
    }

    #[pg_test]
    fn test_dmn_eval_record_multi_decision() {
        let escaped = MULTI_DECISION_DMN.replace('\'', "''");
        Spi::run("CREATE TYPE multi_input AS (\"Base Price\" numeric, \"Tax Rate\" numeric)")
            .expect("CREATE TYPE failed");
        let query = format!(
            "SELECT dmn_eval_record(dmn_load('{escaped}'), 'Total Price', \
             ROW(100::numeric, 0.2::numeric)::multi_input)"
        );
        let result = Spi::get_one::<pgrx::JsonB>(&query).expect("SPI failed");
        assert_eq!(result.unwrap().0, serde_json::json!(120));
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
