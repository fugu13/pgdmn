use pgrx::prelude::*;

use dsntk_feel::FeelScope;
use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel_evaluator::evaluate;
use dsntk_feel_parser::parse_expression;

use crate::convert::{feel_to_json, json_to_context, tuple_to_context};

/// Evaluate a FEEL expression with a pre-built FeelContext.
fn eval_feel_ctx(expression: &str, ctx: FeelContext) -> Value {
    let scope = FeelScope::default();
    scope.push(ctx);
    let node = parse_expression(&scope, expression, false)
        .unwrap_or_else(|e| pgrx::error!("FEEL parse error: {}", e));
    evaluate(&scope, &node)
}

/// Raise the SQL error for a FEEL null result in a typed variant.
fn feel_null_error(msg: Option<String>) -> ! {
    pgrx::error!(
        "FEEL expression returned null{}",
        msg.map(|m| format!(": {m}")).unwrap_or_default()
    )
}

/// Evaluate a FEEL expression and return the result.
fn eval_feel(expression: &str, context: Option<pgrx::JsonB>) -> Value {
    let ctx = match &context {
        Some(pgrx::JsonB(json)) => json_to_context(json),
        None => FeelContext::new(),
    };
    eval_feel_ctx(expression, ctx)
}

/// General-purpose FEEL evaluator returning JSONB.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval(expression: &str, context: default!(Option<pgrx::JsonB>, "NULL")) -> pgrx::JsonB {
    let result = eval_feel(expression, context);
    pgrx::JsonB(feel_to_json(&result))
}

/// Evaluate a FEEL expression with a composite-type record as context.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_record_eval(
    expression: &str,
    context: default!(Option<pgrx::composite_type!("record")>, "NULL"),
) -> pgrx::JsonB {
    let ctx = match context {
        Some(ref tuple) => tuple_to_context(tuple),
        None => FeelContext::new(),
    };
    let result = eval_feel_ctx(expression, ctx);
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
        Value::Null(msg) => feel_null_error(msg),
        other => pgrx::error!("expected FEEL number, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a BOOL result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_bool(expression: &str, context: default!(Option<pgrx::JsonB>, "NULL")) -> bool {
    let result = eval_feel(expression, context);
    match result {
        Value::Boolean(b) => b,
        Value::Null(msg) => feel_null_error(msg),
        other => pgrx::error!("expected FEEL boolean, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a TEXT result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_text(expression: &str, context: default!(Option<pgrx::JsonB>, "NULL")) -> String {
    let result = eval_feel(expression, context);
    match result {
        Value::String(s) => s,
        Value::Null(msg) => feel_null_error(msg),
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
            let m = u8::try_from(m)
                .unwrap_or_else(|_| pgrx::error!("FEEL date month out of range: {m}"));
            let day = u8::try_from(day)
                .unwrap_or_else(|_| pgrx::error!("FEEL date day out of range: {day}"));
            pgrx::datum::Date::new(y, m, day)
                .unwrap_or_else(|e| pgrx::error!("cannot convert FEEL date to PG DATE: {:?}", e))
        }
        Value::Null(msg) => feel_null_error(msg),
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
            let (month, day) = (dt.month(), dt.day());
            let m = u8::try_from(month)
                .unwrap_or_else(|_| pgrx::error!("FEEL date-time month out of range: {month}"));
            let d = u8::try_from(day)
                .unwrap_or_else(|_| pgrx::error!("FEEL date-time day out of range: {day}"));
            let h = dt.hour();
            let min = dt.minute();
            let sec = dt.second();
            pgrx::datum::Timestamp::new(y, m, d, h, min, f64::from(sec)).unwrap_or_else(|e| {
                pgrx::error!("cannot convert FEEL datetime to PG TIMESTAMP: {:?}", e)
            })
        }
        Value::Null(msg) => feel_null_error(msg),
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
            let secs = d.as_seconds();
            let micros = (secs as i64) * 1_000_000;
            pgrx::datum::Interval::new(0, 0, micros).unwrap_or_else(|e| {
                pgrx::error!("cannot convert FEEL duration to PG INTERVAL: {:?}", e)
            })
        }
        Value::YearsAndMonthsDuration(d) => {
            let month_count = d.as_months();
            let months = i32::try_from(month_count).unwrap_or_else(|_| {
                pgrx::error!("FEEL duration months out of INTERVAL range: {month_count}")
            });
            pgrx::datum::Interval::new(months, 0, 0).unwrap_or_else(|e| {
                pgrx::error!("cannot convert FEEL duration to PG INTERVAL: {:?}", e)
            })
        }
        Value::Null(msg) => feel_null_error(msg),
        other => pgrx::error!("expected FEEL duration, got: {}", other),
    }
}
