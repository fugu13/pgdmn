use pgrx::prelude::*;

use dsntk_feel::FeelScope;
use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel_evaluator::evaluate;
use dsntk_feel_parser::parse_expression;

use crate::convert::{
    feel_to_bool, feel_to_date, feel_to_interval, feel_to_json, feel_to_numeric, feel_to_text,
    feel_to_timestamp, json_to_context, tuple_to_context,
};

/// Evaluate a FEEL expression with a pre-built FeelContext.
fn eval_feel_ctx(expression: &str, ctx: FeelContext) -> Value {
    let scope = FeelScope::default();
    scope.push(ctx);
    let node = parse_expression(&scope, expression, false)
        .unwrap_or_else(|e| pgrx::error!("FEEL parse error: {}", e));
    evaluate(&scope, &node)
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
    feel_to_numeric(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a FEEL expression expecting a BOOL result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_bool(expression: &str, context: default!(Option<pgrx::JsonB>, "NULL")) -> bool {
    let result = eval_feel(expression, context);
    feel_to_bool(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a FEEL expression expecting a TEXT result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_text(expression: &str, context: default!(Option<pgrx::JsonB>, "NULL")) -> String {
    let result = eval_feel(expression, context);
    feel_to_text(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a FEEL expression expecting a DATE result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_date(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Date {
    let result = eval_feel(expression, context);
    feel_to_date(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a FEEL expression expecting a TIMESTAMP result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_timestamp(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Timestamp {
    let result = eval_feel(expression, context);
    feel_to_timestamp(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a FEEL expression expecting an INTERVAL result.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_interval(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Interval {
    let result = eval_feel(expression, context);
    feel_to_interval(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}
