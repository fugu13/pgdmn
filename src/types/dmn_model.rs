use dsntk_model::{DmnElement, NamedElement};
use pgrx::prelude::*;
use pgrx::StringInfo;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;

/// A custom PostgreSQL type storing a DMN model.
///
/// Input: raw DMN XML text (e.g. `'<definitions ...>'::dmn_model`)
/// Output: displays as `namespace::model_name`
/// Storage: XML string plus cached metadata via serde
#[derive(PostgresType, Serialize, Deserialize)]
#[inoutfuncs]
pub struct DmnModel {
    pub xml: String,
    pub namespace: String,
    pub name: String,
    pub invocable_names: Vec<String>,
}

impl InOutFuncs for DmnModel {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        let xml = input.to_str().expect("invalid UTF-8 in DMN XML input");
        Self::from_xml(xml).unwrap_or_else(|e| {
            pgrx::error!("invalid DMN XML: {}", e);
        })
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&format!("{}::{}", self.namespace, self.name));
    }
}

impl DmnModel {
    /// Parse XML and extract metadata.
    pub fn from_xml(xml: &str) -> Result<Self, String> {
        let definitions =
            dsntk_model::parse(xml).map_err(|e| format!("failed to parse DMN XML: {}", e))?;

        let namespace = definitions.namespace().to_string();
        let name = definitions.name().to_string();

        let mut invocable_names = Vec::new();
        for d in definitions.decisions() {
            invocable_names.push(d.name().to_string());
        }
        for bkm in definitions.business_knowledge_models() {
            invocable_names.push(bkm.name().to_string());
        }
        for ds in definitions.decision_services() {
            invocable_names.push(ds.name().to_string());
        }

        Ok(DmnModel {
            xml: xml.to_string(),
            namespace,
            name,
            invocable_names,
        })
    }
}
