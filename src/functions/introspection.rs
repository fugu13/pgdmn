use dsntk_model::{DmnElement, NamedElement};
use pgrx::prelude::*;

use crate::types::dmn_model::DmnModel;

/// List invocable elements from a DMN model.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_invocables(
    model: DmnModel,
) -> TableIterator<'static, (name!(name, String), name!(kind, String))> {
    let definitions = dsntk_model::parse(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("failed to parse DMN XML: {}", e));

    let mut rows = Vec::new();

    for d in definitions.decisions() {
        rows.push((d.name().to_string(), "decision".to_string()));
    }
    for bkm in definitions.business_knowledge_models() {
        rows.push((bkm.name().to_string(), "business_knowledge_model".to_string()));
    }
    for ds in definitions.decision_services() {
        rows.push((ds.name().to_string(), "decision_service".to_string()));
    }

    TableIterator::new(rows)
}

/// Return model metadata as JSONB.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_info(model: DmnModel) -> pgrx::JsonB {
    let definitions = dsntk_model::parse(&model.xml)
        .unwrap_or_else(|e| pgrx::error!("failed to parse DMN XML: {}", e));

    let info = serde_json::json!({
        "namespace": definitions.namespace(),
        "name": definitions.name(),
        "decisions": definitions.decisions().len(),
        "business_knowledge_models": definitions.business_knowledge_models().len(),
        "decision_services": definitions.decision_services().len(),
        "invocables": model.invocable_names,
    });

    pgrx::JsonB(info)
}

/// Extract raw XML source from a DmnModel.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_xml(model: DmnModel) -> String {
    model.xml
}

/// Get the model name.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_name(model: DmnModel) -> String {
    model.name
}

/// Get the model namespace.
#[pg_extern(immutable, parallel_safe)]
pub fn dmn_namespace(model: DmnModel) -> String {
    model.namespace
}
