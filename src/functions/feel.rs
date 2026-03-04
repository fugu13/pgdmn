use pgrx::prelude::*;

use dsntk_feel::values::Value;
use dsntk_feel::FeelScope;
use dsntk_feel_evaluator::evaluate;
use dsntk_feel_parser::parse_expression;

use crate::convert::{feel_to_json, json_to_context};

/// Evaluate a FEEL expression and return the result.
fn eval_feel(expression: &str, context: Option<pgrx::JsonB>) -> Value {
    let scope = FeelScope::default();
    if let Some(pgrx::JsonB(json)) = &context {
        let ctx = json_to_context(json);
        scope.push(ctx);
    }

    let node = parse_expression(&scope, expression, false)
        .unwrap_or_else(|e| pgrx::error!("FEEL parse error: {}", e));

    evaluate(&scope, &node)
}

/// General-purpose FEEL evaluator returning JSONB.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::JsonB {
    let result = eval_feel(expression, context);
    pgrx::JsonB(feel_to_json(&result))
}

/// Evaluate a FEEL expression expecting a NUMERIC result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_numeric(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::AnyNumeric {
    let result = eval_feel(expression, context);
    match result {
        Value::Number(n) => {
            let s = n.to_string();
            s.parse::<pgrx::AnyNumeric>()
                .unwrap_or_else(|e| pgrx::error!("cannot convert FEEL number to NUMERIC: {}", e))
        }
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL number, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a BOOL result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_bool(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> bool {
    let result = eval_feel(expression, context);
    match result {
        Value::Boolean(b) => b,
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL boolean, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a TEXT result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_text(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> String {
    let result = eval_feel(expression, context);
    match result {
        Value::String(s) => s,
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL string, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a DATE result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_date(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Date {
    let result = eval_feel(expression, context);
    match result {
        Value::Date(d) => {
            let (y, m, day) = d.as_tuple();
            pgrx::datum::Date::new(y as i32, m as u8, day as u8)
                .unwrap_or_else(|e| pgrx::error!("cannot convert FEEL date to PG DATE: {:?}", e))
        }
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL date, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a TIMESTAMP result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_timestamp(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Timestamp {
    let result = eval_feel(expression, context);
    match result {
        Value::DateTime(dt) => {
            let y = dt.year();
            let m = dt.month();
            let d = dt.day();
            let h = dt.hour();
            let min = dt.minute();
            let sec = dt.second();
            pgrx::datum::Timestamp::new(y as i32, m as u8, d as u8, h, min, sec as f64)
                .unwrap_or_else(|e| {
                    pgrx::error!("cannot convert FEEL datetime to PG TIMESTAMP: {:?}", e)
                })
        }
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL date-time, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting an INTERVAL result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_interval(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Interval {
    let result = eval_feel(expression, context);
    match result {
        Value::DaysAndTimeDuration(d) => {
            let total_secs = d.as_seconds();
            let total_usecs = (total_secs as i64) * 1_000_000;
            pgrx::datum::Interval::new(0, 0, total_usecs)
                .unwrap_or_else(|e| {
                    pgrx::error!("cannot convert FEEL duration to PG INTERVAL: {:?}", e)
                })
        }
        Value::YearsAndMonthsDuration(d) => {
            let months = d.as_months() as i32;
            pgrx::datum::Interval::new(months, 0, 0)
                .unwrap_or_else(|e| {
                    pgrx::error!("cannot convert FEEL duration to PG INTERVAL: {:?}", e)
                })
        }
        Value::Null(msg) => pgrx::error!(
            "FEEL expression returned null{}",
            msg.map(|m| format!(": {}", m)).unwrap_or_default()
        ),
        other => pgrx::error!("expected FEEL duration, got: {}", other),
    }
}
