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
