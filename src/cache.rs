use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::sync::Arc;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{Evaluator, FeelScope, FeelType, Name};
use dsntk_model_evaluator::ModelEvaluator;
use lru::LruCache;

use crate::types::dmn_model::DmnModel;

type RapidBuildHasher = std::hash::BuildHasherDefault<rapidhash::RapidInlineHasher>;

/// Second seed for the 128-bit content hash (the first pass uses
/// `rapidhash::RAPID_SEED`). Any constant distinct from `RAPID_SEED` works;
/// this is the 64-bit golden-ratio constant.
const SECOND_SEED: u64 = 0x9e37_79b9_7f4a_7c15;

/// 128-bit content hash: two independently seeded 64-bit rapidhash passes.
///
/// Collision reasoning: rapidhash is a well-mixed 64-bit hash, and two passes
/// with independent seeds behave as independent hash functions, so two
/// distinct inputs collide in all 128 bits with probability ~2^-128 —
/// negligible for any number of distinct models or context shapes a backend
/// will ever see. This is why cache probes keyed on these digests skip byte
/// comparison of the hashed content.
pub const fn content_hash128(bytes: &[u8]) -> [u64; 2] {
    [
        rapidhash::rapidhash_seeded(bytes, rapidhash::RAPID_SEED),
        rapidhash::rapidhash_seeded(bytes, SECOND_SEED),
    ]
}

/// DMN evaluator cache key: 128-bit XML content hash plus the XML length.
type DmnEvaluatorKey = ([u64; 2], usize);

/// Inner FEEL cache level: full expression text -> prepared evaluator.
type ExpressionEvaluators = HashMap<String, Rc<Evaluator>>;

/// Upper bound on cached DMN model evaluators per backend, enforced by true
/// LRU eviction (see `EVALUATOR_CACHE`). A `ModelEvaluator` wraps a compiled
/// whole model — every decision table and FEEL AST it contains — so it is far
/// more memory-expensive per entry than a prepared FEEL evaluator, and this
/// bound is deliberately much smaller than `FEEL_CACHE_MAX_ENTRIES`. A
/// database-embedded decision engine plausibly keeps a modest, fairly stable
/// set of distinct model versions active per connection (low tens, e.g.
/// versions in rotation during a deploy); 64 gives headroom above that
/// without leaving the cache large enough to be a meaningful memory sink on
/// its own. This is the mitigation for a resource-exhaustion finding: nothing
/// revokes the default `PUBLIC` EXECUTE grant on `dmn_load`, so any
/// connected role can otherwise grow this cache without bound by looping
/// `dmn_load` over a stream of distinct model XML.
const EVALUATOR_CACHE_MAX_ENTRIES: usize = 64;

/// `EVALUATOR_CACHE_MAX_ENTRIES` as a `NonZeroUsize` for `LruCache::with_hasher`.
/// A `match` rather than `.unwrap()`/`.expect()` keeps this free of panicking
/// calls in non-test code; the `None` arm is unreachable since the constant
/// above is a nonzero literal.
const EVALUATOR_CACHE_CAP: NonZeroUsize = match NonZeroUsize::new(EVALUATOR_CACHE_MAX_ENTRIES) {
    Some(cap) => cap,
    None => NonZeroUsize::MIN,
};

/// Upper bound on cached prepared FEEL evaluators per backend. Reaching it
/// clears the whole cache: realistic workloads evaluate a small, stable set of
/// (expression, context shape) pairs, so LRU bookkeeping buys nothing, and the
/// worst case—a workload cycling through more than this many distinct pairs —
/// merely degrades to the previous parse-per-call behavior plus a map probe.
const FEEL_CACHE_MAX_ENTRIES: usize = 1024;

thread_local! {
    /// DMN model evaluators keyed by XML content hash. Bounded with true LRU
    /// eviction (unlike `FEEL_EVALUATOR_CACHE`'s clear-wholesale strategy):
    /// each entry is expensive enough, and cache pressure here is intended to
    /// stay rare enough, that evicting only the single least-recently-used
    /// entry is worth the bookkeeping — clearing the whole cache on overflow
    /// would also discard a model that is still hot.
    static EVALUATOR_CACHE: RefCell<LruCache<DmnEvaluatorKey, Arc<ModelEvaluator>, RapidBuildHasher>> =
        RefCell::new(LruCache::with_hasher(EVALUATOR_CACHE_CAP, RapidBuildHasher::default()));

    /// Prepared FEEL evaluators keyed by context-shape digest, then by the full
    /// expression text. Two levels so that cache hits allocate nothing: the
    /// inner lookup borrows the incoming `&str`.
    static FEEL_EVALUATOR_CACHE: RefCell<HashMap<[u64; 2], ExpressionEvaluators, RapidBuildHasher>> =
        RefCell::new(HashMap::with_hasher(RapidBuildHasher::default()));
}

// Counts evaluator builds (cache misses) in this backend. Test-only
// observability so cache tests assert deterministically instead of
// comparing wall-clock timings (TEST-003).
#[cfg(any(test, feature = "pg_test"))]
thread_local! {
    static EVALUATOR_BUILDS: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// Number of DMN evaluator builds (cache misses) in this backend so far.
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_extern]
fn dmn_evaluator_builds() -> i64 {
    EVALUATOR_BUILDS.with(std::cell::Cell::get)
}

/// Get or create a ModelEvaluator for the given model.
///
/// Keyed by the model's 128-bit XML content hash (computed once when the
/// `DmnModel` is created) plus the XML length, so per-row cache probes never
/// rehash or memcmp the XML itself. A false hit would require two different
/// models of equal length whose XML collides in all 128 hash bits (see
/// `content_hash128`), so no equality check on the XML is performed. Bounded
/// at `EVALUATOR_CACHE_MAX_ENTRIES` with LRU eviction, so an unbounded stream
/// of distinct models (e.g. `dmn_load` looped over generated XML by any role
/// with the default `PUBLIC` EXECUTE grant) cannot grow this cache without
/// bound.
pub fn get_or_build_evaluator(model: &DmnModel) -> Result<Arc<ModelEvaluator>, String> {
    let key = (model.xml_hash, model.xml.len());

    // Check cache first. `LruCache::get` takes `&mut self` because a hit also
    // marks the entry most-recently-used, so this borrows mutably even though
    // it's only a read.
    let cached = EVALUATOR_CACHE.with_borrow_mut(|cache| cache.get(&key).cloned());
    if let Some(evaluator) = cached {
        return Ok(evaluator);
    }

    // Parse and build. The guard re-runs here (not just at input parsing) so
    // dmn_model values stored before the external-function guard existed are
    // still rejected at evaluation time.
    #[cfg(any(test, feature = "pg_test"))]
    EVALUATOR_BUILDS.with(|count| count.set(count.get() + 1));
    let definitions =
        dsntk_model::parse(&model.xml).map_err(|e| format!("failed to parse DMN XML: {e}"))?;
    crate::guard::reject_external_definitions(&definitions)?;
    let evaluator = ModelEvaluator::new(&[definitions])
        .map_err(|e| format!("failed to build model evaluator: {e}"))?;

    // Cache it. `put` evicts the least-recently-used entry itself if the
    // cache is already at `EVALUATOR_CACHE_MAX_ENTRIES`.
    EVALUATOR_CACHE.with_borrow_mut(|cache| {
        cache.put(key, Arc::clone(&evaluator));
    });

    Ok(evaluator)
}

/// Get the prepared evaluator for a FEEL expression under the given context,
/// together with the scope to evaluate it in.
///
/// Owning the whole digest-then-probe sequence here keeps the cache-key
/// invariant unexpressible-wrong for callers: the shape that keys the cache is
/// always derived from the very context pushed onto the returned scope.
pub fn prepared_feel_evaluator(
    expression: &str,
    ctx: FeelContext,
) -> Result<(Rc<Evaluator>, FeelScope), String> {
    let shape = context_shape_digest(&ctx);
    let scope = FeelScope::default();
    scope.push(ctx);
    let evaluator = get_or_prepare_feel_evaluator(expression, shape, &scope)?;
    Ok((evaluator, scope))
}

/// Get or build the prepared evaluator for a FEEL expression.
///
/// `shape` must be `context_shape_digest` of the context pushed onto `scope`.
/// The shape is part of the cache key because FEEL name tokenization depends
/// on the names in scope (multi-word names like `monthly salary`): the same
/// expression text can parse to different ASTs under different context shapes.
/// The full expression string is kept verbatim in the key, so a wrong hit
/// additionally requires two distinct shapes colliding in all 128 digest bits
/// (see `content_hash128`) under the very same expression.
fn get_or_prepare_feel_evaluator(
    expression: &str,
    shape: [u64; 2],
    scope: &FeelScope,
) -> Result<Rc<Evaluator>, String> {
    let cached = FEEL_EVALUATOR_CACHE.with_borrow(|cache| {
        cache
            .get(&shape)
            .and_then(|by_expr| by_expr.get(expression))
            .cloned()
    });
    if let Some(evaluator) = cached {
        return Ok(evaluator);
    }

    let node = dsntk_feel_parser::parse_expression(scope, expression, false)
        .map_err(|e| format!("FEEL parse error: {e}"))?;
    // Guard once per parse: every cached evaluator was screened for external
    // (Java/PMML) function definitions when its expression first entered the
    // cache, so cache hits need no re-check.
    crate::guard::reject_external_functions(&node)?;
    let evaluator = Rc::new(dsntk_feel_evaluator::prepare(&node));

    FEEL_EVALUATOR_CACHE.with_borrow_mut(|cache| {
        let entries: usize = cache.values().map(ExpressionEvaluators::len).sum();
        if entries >= FEEL_CACHE_MAX_ENTRIES {
            cache.clear();
        }
        cache
            .entry(shape)
            .or_default()
            .insert(expression.to_owned(), Rc::clone(&evaluator));
    });

    Ok(evaluator)
}

/// 128-bit digest of a context's *shape*: the recursive key structure the FEEL
/// parser derives from the scope (`ParsingContext::from(&FeelContext)` in
/// vendor/dsntk-feel-parser/src/context.rs). Only information that influences
/// parsing is included: entry names, and whether each entry is a nested
/// context (recursed), a list whose unified element type is a context
/// (recursed via `Value::type_of`, exactly as the parser does), or a plain
/// variable. Leaf values never affect the digest, so rows differing only in
/// data share one cache entry; any change to the key structure changes it.
fn context_shape_digest(ctx: &FeelContext) -> [u64; 2] {
    // Thread-local scratch buffer: the digest runs on every feel_eval row
    // (cache hits included), so it must not allocate per call.
    thread_local! {
        static SHAPE_SCRATCH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    }
    SHAPE_SCRATCH.with_borrow_mut(|bytes| {
        bytes.clear();
        write_context_shape(ctx, bytes);
        content_hash128(bytes)
    })
}

/// Serialize a context's shape canonically: `{` entries `}` with each entry as
/// a length-prefixed name followed by its shape. Length prefixes make the
/// encoding injective (a name containing braces or `v` cannot forge structure)
/// and BTreeMap iteration order makes it deterministic.
fn write_context_shape(ctx: &FeelContext, out: &mut Vec<u8>) {
    out.push(b'{');
    for (name, value) in ctx.iter() {
        write_name(name, out);
        match value {
            Value::Context(sub_ctx) => write_context_shape(sub_ctx, out),
            list @ Value::List(items) => {
                // type_of() materializes a FeelType tree for every element —
                // data-proportional allocation. Only a list whose FIRST element
                // is a context can unify to a context type (anything else
                // unifies to a non-context and renders as a variable), so all
                // other lists short-circuit to 'v' with the same digest.
                if matches!(items.first(), Some(Value::Context(_))) {
                    write_type_shape(&list.type_of(), out);
                } else {
                    out.push(b'v');
                }
            }
            Value::FeelType(feel_type) => write_type_shape(feel_type, out),
            _ => out.push(b'v'),
        }
    }
    out.push(b'}');
}

/// Mirror of `ParsingEntry::from(&FeelType)`: context types contribute their
/// recursive key structure, a list of contexts contributes the element
/// context's structure, everything else is an opaque variable.
fn write_type_shape(feel_type: &FeelType, out: &mut Vec<u8>) {
    match feel_type {
        FeelType::Context(entries) => {
            out.push(b'{');
            for (name, entry_type) in entries {
                write_name(name, out);
                write_type_shape(entry_type, out);
            }
            out.push(b'}');
        }
        FeelType::List(items) => {
            if matches!(items.as_ref(), FeelType::Context(_)) {
                write_type_shape(items, out);
            } else {
                out.push(b'v');
            }
        }
        _ => out.push(b'v'),
    }
}

/// Append a name as a 4-byte little-endian length prefix plus its UTF-8 bytes,
/// written through the `Display` impl to avoid a `String` allocation per key.
fn write_name(name: &Name, out: &mut Vec<u8>) {
    let start = out.len();
    out.extend_from_slice(&[0u8; 4]);
    let _ = write!(out, "{name}"); // io::Write into a Vec<u8> is infallible
    // Names are bounded far below u32::MAX by PostgreSQL's 1 GB value limit.
    let len = u32::try_from(out.len() - start - 4).unwrap_or(u32::MAX);
    out[start..start + 4].copy_from_slice(&len.to_le_bytes());
}

// Pure-Rust unit tests for the shape digest; they run under `cargo pgrx test`
// (the crate itself only compiles where pg_config is available).
#[cfg(test)]
mod tests {
    use super::*;
    use dsntk_feel::FeelNumber;

    fn num(i: i64) -> Value {
        Value::Number(FeelNumber::from(i))
    }

    fn ctx_of(entries: Vec<(&str, Value)>) -> FeelContext {
        let mut ctx = FeelContext::new();
        for (key, value) in entries {
            ctx.set_entry(&Name::from(key), value);
        }
        ctx
    }

    #[test]
    fn same_shape_different_values_share_digest() {
        let a = ctx_of(vec![("x", num(1)), ("y", num(2))]);
        let b = ctx_of(vec![("x", num(100)), ("y", Value::String("s".to_string()))]);
        // Scalars of any kind are opaque variables to the parser.
        assert_eq!(context_shape_digest(&a), context_shape_digest(&b));
    }

    #[test]
    fn different_key_sets_differ() {
        let a = ctx_of(vec![("a", num(1)), ("b", num(2))]);
        let b = ctx_of(vec![("a", num(1)), ("b", num(2)), ("a-b", num(3))]);
        assert_ne!(context_shape_digest(&a), context_shape_digest(&b));
    }

    #[test]
    fn nested_context_vs_flat_dotted_key_differ() {
        let nested = ctx_of(vec![(
            "order",
            Value::Context(ctx_of(vec![("total", num(21))])),
        )]);
        let flat = ctx_of(vec![("order.total", num(5))]);
        assert_ne!(context_shape_digest(&nested), context_shape_digest(&flat));
    }

    #[test]
    fn value_flipping_between_scalar_and_context_differs() {
        let scalar = ctx_of(vec![("x", num(1))]);
        let context = ctx_of(vec![("x", Value::Context(ctx_of(vec![("y", num(1))])))]);
        assert_ne!(
            context_shape_digest(&scalar),
            context_shape_digest(&context)
        );
    }

    #[test]
    fn list_of_contexts_vs_list_of_scalars_differ() {
        let of_contexts = ctx_of(vec![(
            "items",
            Value::List(vec![
                Value::Context(ctx_of(vec![("price", num(2))])),
                Value::Context(ctx_of(vec![("price", num(3))])),
            ]),
        )]);
        let of_scalars = ctx_of(vec![("items", Value::List(vec![num(10), num(20)]))]);
        assert_ne!(
            context_shape_digest(&of_contexts),
            context_shape_digest(&of_scalars)
        );
    }

    #[test]
    fn empty_list_matches_scalar_list() {
        // The parser treats both as opaque variables (List(Null) vs
        // List(Number) both collapse to Variable), so the digests must match —
        // this locks in the exact mirror of ParsingEntry::from(&FeelType).
        let empty = ctx_of(vec![("items", Value::List(vec![]))]);
        let scalars = ctx_of(vec![("items", Value::List(vec![num(1)]))]);
        assert_eq!(context_shape_digest(&empty), context_shape_digest(&scalars));
    }

    #[test]
    fn mixed_list_matches_scalar_list() {
        // A list starting with a context but containing a scalar unifies to a
        // non-context type in the parser, i.e. an opaque variable—the same
        // as a scalar list. Locks in the first-element short-circuit.
        let mixed = ctx_of(vec![(
            "items",
            Value::List(vec![
                Value::Context(ctx_of(vec![("price", num(2))])),
                num(3),
            ]),
        )]);
        let scalars = ctx_of(vec![("items", Value::List(vec![num(1)]))]);
        assert_eq!(context_shape_digest(&mixed), context_shape_digest(&scalars));
    }

    #[test]
    fn name_bytes_cannot_forge_structure() {
        // A single key whose text mimics the serialized form of two entries
        // must not collide with an actual two-entry context.
        let forged = ctx_of(vec![("a\u{1}v\u{0}\u{0}\u{0}b", num(1))]);
        let real = ctx_of(vec![("a", num(1)), ("b", num(2))]);
        assert_ne!(context_shape_digest(&forged), context_shape_digest(&real));
    }

    /// Minimal, distinct-by-`id` DMN XML (adapted from the `SIMPLE_DMN`
    /// fixture in `src/lib.rs`): a single decision with a literal expression,
    /// no decision table needed to get a byte-distinct XML per `id`, hence a
    /// distinct content hash and cache key.
    fn fixture_model(id: usize) -> DmnModel {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="https://www.omg.org/spec/DMN/20191111/MODEL/"
             id="fixture_{id}"
             name="Fixture{id}"
             namespace="https://example.org/fixture/{id}">
    <decision id="Greeting" name="Greeting">
        <variable name="Greeting" typeRef="string"/>
        <literalExpression>
            <text>"Hello, {id}!"</text>
        </literalExpression>
    </decision>
</definitions>"#
        );
        DmnModel::from_xml(&xml).expect("fixture DMN XML must parse")
    }

    fn evaluator_builds() -> i64 {
        EVALUATOR_BUILDS.with(std::cell::Cell::get)
    }

    #[test]
    fn evaluator_cache_evicts_least_recently_used() {
        let first = fixture_model(0);
        let second = fixture_model(1);

        // Fill the cache to exactly its cap: ids 0..CAP, oldest (id 0) to
        // newest (id CAP-1).
        for id in 0..EVALUATOR_CACHE_MAX_ENTRIES {
            get_or_build_evaluator(&fixture_model(id)).expect("build must succeed");
        }

        // Touch id 0 again: a cache hit (no rebuild) that also promotes it to
        // most-recently-used, so id 1 — not id 0 — becomes the LRU victim.
        let builds_before_touch = evaluator_builds();
        get_or_build_evaluator(&first).expect("cached fetch must succeed");
        assert_eq!(
            evaluator_builds(),
            builds_before_touch,
            "re-fetching a cached model must not rebuild it"
        );

        // Insert one more distinct model, pushing the cache over its cap by
        // one and forcing an eviction.
        get_or_build_evaluator(&fixture_model(EVALUATOR_CACHE_MAX_ENTRIES))
            .expect("build must succeed");

        // id 0 was just touched (most-recently-used) and must still be cached.
        let builds_before_first_refetch = evaluator_builds();
        get_or_build_evaluator(&first).expect("cached fetch must succeed");
        assert_eq!(
            evaluator_builds(),
            builds_before_first_refetch,
            "the most-recently-used entry must survive eviction"
        );

        // id 1 was the actual least-recently-used entry and must have been
        // evicted, so re-fetching it rebuilds.
        let builds_before_second_refetch = evaluator_builds();
        get_or_build_evaluator(&second).expect("rebuild must succeed");
        assert_eq!(
            evaluator_builds(),
            builds_before_second_refetch + 1,
            "the least-recently-used entry must have been evicted and rebuilt"
        );
    }
}
