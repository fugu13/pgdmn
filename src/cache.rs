use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use dsntk_model_evaluator::ModelEvaluator;

type RapidBuildHasher = std::hash::BuildHasherDefault<rapidhash::RapidInlineHasher>;

thread_local! {
    static EVALUATOR_CACHE: RefCell<HashMap<String, Arc<ModelEvaluator>, RapidBuildHasher>> =
        RefCell::new(HashMap::with_hasher(RapidBuildHasher::default()));
}

/// Get or create a ModelEvaluator for the given XML content.
pub fn get_or_build_evaluator(
    xml: &str,
) -> Result<Arc<ModelEvaluator>, String> {
    // Check cache first
    let cached = EVALUATOR_CACHE.with(|cache| cache.borrow().get(xml).cloned());
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
        cache.borrow_mut().insert(xml.to_owned(), Arc::clone(&evaluator));
    });

    Ok(evaluator)
}
