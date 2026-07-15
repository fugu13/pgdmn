//! JSON <-> FEEL value conversions that do not touch PostgreSQL.
//!
//! Shared verbatim with the host-side profiling harness (via `#[path]`
//! inclusion), so benchmark numbers always describe the shipping conversion
//! code. Everything pgrx-dependent lives in `convert.rs`, including the
//! SQL-error wrapper around [`try_feel_to_json`].

use dsntk_feel::FeelNumber;
use dsntk_feel::Name;
use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;

/// Convert a serde_json::Value to a dsntk FEEL Value.
pub fn json_to_feel(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null(None),
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            // Integer fast path: serde_json already holds i64/u64 for integer
            // JSON numbers, and FeelNumber::from(i64/u64) is one exact FFI
            // conversion — no String, CString, or per-character decimal parse.
            if let Some(i) = n.as_i64() {
                Value::Number(FeelNumber::from(i))
            } else if let Some(u) = n.as_u64() {
                Value::Number(FeelNumber::from(u))
            } else {
                // Non-integer: keep the decimal-string path. The shortest
                // decimal rendering -> decimal128 preserves intended decimal
                // values (e.g. 0.1); do not route floats through f64 binary.
                let s = n.to_string();
                match s.parse::<FeelNumber>() {
                    Ok(num) => Value::Number(num),
                    Err(_) => Value::Null(Some(format!("cannot convert number: {s}"))),
                }
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let values = arr.iter().map(json_to_feel).collect();
            Value::List(values)
        }
        serde_json::Value::Object(map) => {
            let mut ctx = FeelContext::new();
            for (key, val) in map {
                ctx.insert(Name::from(key.as_str()), json_to_feel(val));
            }
            Value::Context(ctx)
        }
    }
}

/// Whether a FEEL number is finite. dsntk arithmetic can overflow decimal128
/// to ±Inf/NaN, whose Display panics inside dsntk (BUG-004) — callers must
/// check before stringifying. NaN compares false to everything, so it fails
/// the range check too.
pub fn feel_number_is_finite(n: &FeelNumber) -> bool {
    let infinite = FeelNumber::infinite();
    -infinite < *n && *n < infinite
}

/// Convert a dsntk FEEL Value to serde_json::Value.
///
/// Fails when the value contains a non-finite number (BUG-004: stringifying
/// ±Inf/NaN panics inside dsntk); the pgrx boundary in `convert.rs` turns
/// that into a SQL error.
pub fn try_feel_to_json(value: &Value) -> Result<serde_json::Value, String> {
    Ok(match value {
        Value::Null(_) => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => feel_number_to_json(n)?,
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => {
            let arr: Vec<serde_json::Value> =
                items.iter().map(try_feel_to_json).collect::<Result<_, _>>()?;
            serde_json::Value::Array(arr)
        }
        Value::Context(ctx) => {
            let mut map = serde_json::Map::new();
            for (name, val) in ctx.iter() {
                map.insert(String::from(name), try_feel_to_json(val)?);
            }
            serde_json::Value::Object(map)
        }
        Value::Date(d) => serde_json::Value::String(d.to_string()),
        Value::Time(t) => serde_json::Value::String(t.to_string()),
        Value::DateTime(dt) => serde_json::Value::String(dt.to_string()),
        Value::DaysAndTimeDuration(d) => serde_json::Value::String(d.to_string()),
        Value::YearsAndMonthsDuration(d) => serde_json::Value::String(d.to_string()),
        _ => serde_json::Value::String(value.to_string()),
    })
}

/// Convert a FEEL number to a JSON number.
///
/// Non-finite numbers fail (see [`try_feel_to_json`]). Finite integers take a
/// single exact FFI conversion; the `is_integer` guard is required because
/// `i64::try_from(&FeelNumber)` truncates fractional values instead of
/// failing. Everything else renders as a decimal string: f64 when parseable
/// (non-integers, and integers wider than i64 — the latter lossy, see
/// CONVERT-001), otherwise a JSON string. The rendered form of those values
/// never parses as i64, so no i64 attempt is made on the string path.
fn feel_number_to_json(n: &FeelNumber) -> Result<serde_json::Value, String> {
    if !feel_number_is_finite(n) {
        return Err("FEEL number result is not finite and cannot be represented".to_string());
    }
    if n.is_integer() {
        if let Ok(i) = i64::try_from(n) {
            return Ok(serde_json::Value::Number(serde_json::Number::from(i)));
        }
    }
    let s = n.to_string();
    Ok(if let Ok(f) = s.parse::<f64>() {
        serde_json::json!(f)
    } else {
        serde_json::Value::String(s)
    })
}

/// Convert a JSON value (expected object) to a FeelContext.
pub fn json_to_context(json: &serde_json::Value) -> FeelContext {
    if let Value::Context(ctx) = json_to_feel(json) {
        ctx
    } else {
        FeelContext::new()
    }
}
