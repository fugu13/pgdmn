use pgrx::prelude::*;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelNumber, IntervalType};
use pgrx::datum::{Range, RangeBound};

use crate::cache::prepared_feel_evaluator;
use crate::convert::{
    feel_to_bool, feel_to_date, feel_to_interval, feel_to_json, feel_to_numeric, feel_to_text,
    feel_to_timestamp, json_to_context, tuple_to_context,
};

/// Evaluate a FEEL expression with a pre-built FeelContext.
///
/// Parsing and evaluator construction are amortized across calls: the prepared
/// evaluator is cached per (expression, context shape) — see cache.rs — so a
/// cache hit costs one shape digest, one map probe, and the evaluation itself.
fn eval_feel_ctx(expression: &str, ctx: FeelContext) -> Value {
    // The external-function AST guard runs inside the cache at parse time
    // (cache.rs::get_or_prepare_feel_evaluator): a cached evaluator was
    // guarded when its expression was first parsed.
    let (evaluator, scope) =
        prepared_feel_evaluator(expression, ctx).unwrap_or_else(|e| pgrx::error!("{}", e));
    evaluator(&scope)
}

/// Raise the SQL error for a FEEL null result in a typed variant. Used by the
/// range variant, which does not go through the `Result`-returning converters.
fn feel_null_error(msg: Option<String>) -> ! {
    pgrx::error!(
        "FEEL expression returned null{}",
        msg.map(|m| format!(": {m}")).unwrap_or_default()
    )
}

/// Convert a FEEL number to a PG NUMERIC (string round trip is the only
/// lossless bridge between the two decimal representations).
fn feel_number_to_numeric(n: &FeelNumber) -> pgrx::AnyNumeric {
    if crate::convert::feel_number_is_finite(n) {
        n.to_string()
            .parse::<pgrx::AnyNumeric>()
            .unwrap_or_else(|e| pgrx::error!("cannot convert FEEL number to NUMERIC: {}", e))
    } else {
        pgrx::error!("FEEL number result is not finite and cannot be converted to NUMERIC")
    }
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

/// Convert one FEEL range endpoint to a NUMRANGE bound.
fn numrange_bound(endpoint: &Value, interval_type: IntervalType) -> RangeBound<pgrx::AnyNumeric> {
    if interval_type.undefined() {
        return RangeBound::Infinite;
    }
    match endpoint {
        Value::Number(n) => {
            let numeric = feel_number_to_numeric(n);
            if interval_type.closed() {
                RangeBound::Inclusive(numeric)
            } else {
                RangeBound::Exclusive(numeric)
            }
        }
        other => pgrx::error!("expected FEEL number as range endpoint, got: {}", other),
    }
}

/// Evaluate a FEEL expression expecting a range of numbers, returned as NUMRANGE.
#[pg_extern(immutable, parallel_safe)]
pub fn feel_eval_numrange(
    expression: &str,
    context: default!(Option<pgrx::JsonB>, "NULL"),
) -> Range<pgrx::AnyNumeric> {
    let result = eval_feel(expression, context);
    match result {
        Value::Range(start, start_type, end, end_type) => Range::new(
            numrange_bound(&start, start_type),
            numrange_bound(&end, end_type),
        ),
        Value::Null(msg) => feel_null_error(msg),
        other => pgrx::error!("expected FEEL range, got: {}", other),
    }
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
