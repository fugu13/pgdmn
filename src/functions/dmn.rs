use pgrx::prelude::*;

use crate::cache::get_or_build_evaluator;
use crate::convert::{feel_to_json, json_to_context, tuple_to_context};
use crate::types::dmn_model::DmnModel;

/// Parse XML into a DmnModel (convenience wrapper).
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_load(xml: &str) -> DmnModel {
    DmnModel::from_xml(xml).unwrap_or_else(|e| pgrx::error!("{}", e))
}

/// Evaluate a named invocable from a DMN model.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_eval(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::JsonB>, "NULL"),
) -> pgrx::JsonB {
    let evaluator = get_or_build_evaluator(&model).unwrap_or_else(|e| pgrx::error!("{}", e));

    let ctx = match input {
        Some(pgrx::JsonB(json)) => json_to_context(&json),
        None => dsntk_feel::context::FeelContext::new(),
    };

    let result = evaluator.evaluate_invocable(&model.namespace, &model.name, invocable, &ctx);

    pgrx::JsonB(feel_to_json(&result))
}

/// Evaluate a named invocable from a DMN model using a composite-type record as input.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_record_eval(
    model: DmnModel,
    invocable: &str,
    input: default!(Option<pgrx::composite_type!("record")>, "NULL"),
) -> pgrx::JsonB {
    let evaluator = get_or_build_evaluator(&model).unwrap_or_else(|e| pgrx::error!("{}", e));

    let ctx = match input {
        Some(ref tuple) => tuple_to_context(tuple),
        None => dsntk_feel::context::FeelContext::new(),
    };

    let result = evaluator.evaluate_invocable(&model.namespace, &model.name, invocable, &ctx);

    pgrx::JsonB(feel_to_json(&result))
}
