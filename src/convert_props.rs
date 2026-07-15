//! Property-based tests for the JSON <-> FEEL conversion layer (TEST-001).
//!
//! These are plain Rust tests (no PostgreSQL involved) exercising
//! `convert::json_to_feel` / `convert::feel_to_json`; they run inside the
//! same `cargo pgrx test` invocation as the `#[pg_test]` suite.
//!
//! Round-trip semantics being pinned down:
//! - JSON -> FEEL -> JSON preserves structure and *numeric value*, not
//!   lexical number form: an integral float (`2.0`) legitimately comes back
//!   as the JSON integer `2` because FEEL numbers are decimals with no
//!   int/float distinction.
//! - Integers within i64 round-trip exactly.
//! - Integers above i64::MAX currently lose precision through the f64
//!   fallback in `feel_to_json` (tracked as CONVERT-001 in TODO.md), so the
//!   generators stay within i64.

use proptest::prelude::*;

use crate::convert_core::{json_to_feel, try_feel_to_json};

/// Generators produce only finite numbers, so conversion cannot fail here.
fn feel_to_json(value: &dsntk_feel::values::Value) -> serde_json::Value {
    try_feel_to_json(value).expect("non-finite number in generated data")
}

/// Structural equality with numeric-value comparison for numbers.
fn json_value_eq(left: &serde_json::Value, right: &serde_json::Value) -> bool {
    use serde_json::Value as J;
    match (left, right) {
        (J::Number(ln), J::Number(rn)) => {
            match (ln.as_i64(), rn.as_i64()) {
                (Some(li), Some(ri)) => li == ri,
                // Mixed or float representation: compare as f64 (both sides
                // came from the same decimal, so this is exact in practice).
                _ => match (ln.as_f64(), rn.as_f64()) {
                    (Some(lf), Some(rf)) => lf == rf,
                    _ => false,
                },
            }
        }
        (J::Array(la), J::Array(ra)) => {
            la.len() == ra.len() && la.iter().zip(ra).all(|(lv, rv)| json_value_eq(lv, rv))
        }
        (J::Object(lo), J::Object(ro)) => {
            lo.len() == ro.len()
                && lo
                    .iter()
                    .all(|(key, lv)| ro.get(key).is_some_and(|rv| json_value_eq(lv, rv)))
        }
        _ => left == right,
    }
}

/// Keys exercise FEEL-relevant shapes: multi-word names, unicode, symbols.
fn key_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-z]{1,8}",
        "[a-z]{1,6} [a-z]{1,6}",
        "[a-z]{1,4} [a-z]{1,4} [a-z]{1,4}",
        Just("monthly salary".to_string()),
        Just("crédit score".to_string()),
    ]
}

/// Finite f64s (every branch below generates finite values only, which
/// serde_json's Number representation requires).
fn float_strategy() -> impl Strategy<Value = f64> {
    prop_oneof![
        -1.0e12..1.0e12f64,
        proptest::num::f64::NORMAL,
        Just(0.0),
        Just(-0.5),
        Just(0.1),
    ]
}

fn json_strategy() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|i| serde_json::json!(i)),
        float_strategy().prop_map(|f| serde_json::json!(f)),
        "[ -~]{0,24}".prop_map(serde_json::Value::String),
    ];
    leaf.prop_recursive(3, 48, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(serde_json::Value::Array),
            prop::collection::btree_map(key_strategy(), inner, 0..6)
                .prop_map(|m| { serde_json::Value::Object(m.into_iter().collect()) }),
        ]
    })
}

proptest! {
    /// JSON -> FEEL -> JSON preserves structure and numeric value.
    #[test]
    fn json_feel_json_roundtrip(json in json_strategy()) {
        let feel = json_to_feel(&json);
        let back = feel_to_json(&feel);
        prop_assert!(
            json_value_eq(&json, &back),
            "round trip diverged:\n  in:  {json}\n  out: {back}"
        );
    }

    /// Integers within i64 round-trip exactly (stricter than value equality).
    #[test]
    fn integer_roundtrip_exact(i in any::<i64>()) {
        let json = serde_json::json!(i);
        let back = feel_to_json(&json_to_feel(&json));
        prop_assert_eq!(back, serde_json::json!(i));
    }

    /// Object key insertion order does not affect conversion results.
    /// (Keys come from a BTreeMap so they are unique; duplicate keys would
    /// make insertion order observable via last-wins before conversion.)
    #[test]
    fn object_key_order_irrelevant(
        entries in prop::collection::btree_map(key_strategy(), any::<i64>(), 1..8)
    ) {
        let forward: serde_json::Map<String, serde_json::Value> = entries
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(v)))
            .collect();
        let reversed: serde_json::Map<String, serde_json::Value> = entries
            .iter()
            .rev()
            .map(|(k, v)| (k.clone(), serde_json::json!(v)))
            .collect();
        let a = feel_to_json(&json_to_feel(&serde_json::Value::Object(forward)));
        let b = feel_to_json(&json_to_feel(&serde_json::Value::Object(reversed)));
        prop_assert_eq!(a, b);
    }
}
