use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use dsntk_model_evaluator::ModelEvaluator;

thread_local! {
    static EVALUATOR_CACHE: RefCell<HashMap<u64, Arc<ModelEvaluator>>> = RefCell::new(HashMap::new());
}

fn hash_xml(xml: &str) -> u64 {
    let mut hasher = rapidhash::RapidInlineHasher::default();
    xml.hash(&mut hasher);
    hasher.finish()
}

/// Get or create a ModelEvaluator for the given XML content.
pub fn get_or_build_evaluator(
    xml: &str,
) -> Result<Arc<ModelEvaluator>, String> {
    let key = hash_xml(xml);

    // Check cache first
    let cached = EVALUATOR_CACHE.with(|cache| cache.borrow().get(&key).cloned());
    if let Some(evaluator) = cached {
        return Ok(evaluator);
    }

    // Parse and build
    let definitions =
        dsntk_model::parse(xml).map_err(|e| format!("failed to parse DMN XML: {}", e))?;
    let evaluator = ModelEvaluator::new(&[definitions])
        .map_err(|e| format!("failed to build model evaluator: {}", e))?;

    // Cache it
    EVALUATOR_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, Arc::clone(&evaluator));
    });

    Ok(evaluator)
}
