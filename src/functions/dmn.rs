use pgrx::prelude::*;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;

use crate::cache::get_or_build_evaluator;
use crate::convert::{
    feel_to_bool, feel_to_date, feel_to_interval, feel_to_json, feel_to_numeric, feel_to_text,
    feel_to_timestamp, json_to_context, tuple_to_context,
};
use crate::types::dmn_model::DmnModel;

/// Parse XML into a DmnModel (convenience wrapper).
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_load(xml: &str) -> DmnModel {
    DmnModel::from_xml(xml).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a named invocable, returning the raw FEEL value.
fn eval_invocable(model: &DmnModel, invocable: &str, ctx: &FeelContext) -> Value {
    let evaluator = get_or_build_evaluator(model).unwrap_or_else(|e| pgrx::error!("{}", e));
    evaluator.evaluate_invocable(&model.namespace, &model.name, invocable, ctx)
}

/// The input context from JSONB, or an empty one when no input is given.
fn json_input(input: Option<pgrx::JsonB>) -> FeelContext {
    match input {
        Some(pgrx::JsonB(json)) => json_to_context(&json),
        None => FeelContext::new(),
    }
}

/// Evaluate a named invocable from a DMN model.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::JsonB {
    let result = eval_invocable(&model, invocable, &json_input(input));
    pgrx::JsonB(feel_to_json(&result))
}

/// Evaluate a named invocable from a DMN model using a composite-type record as input.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_record_eval(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::composite_type!("record")>, "NULL"),
) -> pgrx::JsonB {
    let ctx = match input {
        Some(ref tuple) => tuple_to_context(tuple),
        None => FeelContext::new(),
    };
    let result = eval_invocable(&model, invocable, &ctx);
    pgrx::JsonB(feel_to_json(&result))
}

/// Evaluate a decision expecting a TEXT result.
///
/// The typed variants exist so the caller does not have to unwrap JSONB by hand.
/// `dmn_eval(...) #>> '{}'` says nothing about what the decision returns;
/// `dmn_eval_text(...)` says it must be a string, and errors if it is not.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_text(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> String {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_text(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a decision expecting a NUMERIC result.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_numeric(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::AnyNumeric {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_numeric(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a decision expecting a BOOL result.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_bool(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> bool {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_bool(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a decision expecting a DATE result.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_date(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Date {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_date(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a decision expecting a TIMESTAMP result.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_timestamp(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Timestamp {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_timestamp(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a decision expecting an INTERVAL result.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval_interval(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::datum::Interval {
    let result = eval_invocable(&model, invocable, &json_input(input));
    feel_to_interval(&result).unwrap_or_else(|e| pgrx::error!("{}", e))
}
