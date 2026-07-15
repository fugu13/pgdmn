use std::num::NonZeroUsize;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelNumber, Name};
use dsntk_feel_temporal::{
    FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelYearsAndMonthsDuration,
};
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::pg_sys;
/// Convert a serde_json::Value to a dsntk FEEL Value.
pub fn json_to_feel(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null(None),
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            match s.parse::<FeelNumber>() {
                Ok(num) => Value::Number(num),
                Err(_) => Value::Null(Some(format!("cannot convert number: {s}"))),
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
                ctx.set_entry(&Name::from(key.as_str()), json_to_feel(val));
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
pub fn feel_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null(_) => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => {
            if !feel_number_is_finite(n) {
                pgrx::error!("FEEL number result is not finite and cannot be represented");
            }
            let s = n.to_string();
            if let Ok(i) = s.parse::<i64>() {
                serde_json::Value::Number(serde_json::Number::from(i))
            } else if let Ok(f) = s.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::Value::String(s)
            }
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(items) => {
            let arr: Vec<serde_json::Value> = items.iter().map(feel_to_json).collect();
            serde_json::Value::Array(arr)
        }
        Value::Context(ctx) => {
            let mut map = serde_json::Map::new();
            for (name, val) in ctx.iter() {
                map.insert(String::from(name), feel_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        Value::Date(d) => serde_json::Value::String(d.to_string()),
        Value::Time(t) => serde_json::Value::String(t.to_string()),
        Value::DateTime(dt) => serde_json::Value::String(dt.to_string()),
        Value::DaysAndTimeDuration(d) => serde_json::Value::String(d.to_string()),
        Value::YearsAndMonthsDuration(d) => serde_json::Value::String(d.to_string()),
        _ => serde_json::Value::String(value.to_string()),
    }
}

/// Convert a JSON value (expected object) to a FeelContext.
pub fn json_to_context(json: &serde_json::Value) -> FeelContext {
    if let serde_json::Value::Object(map) = json {
        let mut ctx = FeelContext::new();
        for (key, val) in map {
            ctx.set_entry(&Name::from(key.as_str()), json_to_feel(val));
        }
        ctx
    } else {
        FeelContext::new()
    }
}

/// Convert a single PG datum from a PgHeapTuple to a FEEL Value, dispatching on the type OID.
#[expect(clippy::too_many_lines)] // OID dispatch match; one arm per supported PG type
fn pg_datum_to_feel<A: pgrx::WhoAllocated>(
    tuple: &PgHeapTuple<'_, A>,
    attno: NonZeroUsize,
    type_oid: pg_sys::Oid,
) -> Value {
    match type_oid {
        pg_sys::BOOLOID => match tuple.get_by_index::<bool>(attno) {
            Ok(Some(v)) => Value::Boolean(v),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT2OID => match tuple.get_by_index::<i16>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(i64::from(v))),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT4OID => match tuple.get_by_index::<i32>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(i64::from(v))),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INT8OID => match tuple.get_by_index::<i64>(attno) {
            Ok(Some(v)) => Value::Number(FeelNumber::from(v)),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::FLOAT4OID => match tuple.get_by_index::<f32>(attno) {
            Ok(Some(v)) => v.to_string().parse::<FeelNumber>().map_or_else(
                |_| Value::Null(Some(format!("bad float4: {v}"))),
                Value::Number,
            ),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::FLOAT8OID => match tuple.get_by_index::<f64>(attno) {
            Ok(Some(v)) => v.to_string().parse::<FeelNumber>().map_or_else(
                |_| Value::Null(Some(format!("bad float8: {v}"))),
                Value::Number,
            ),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::NUMERICOID => match tuple.get_by_index::<pgrx::AnyNumeric>(attno) {
            Ok(Some(v)) => {
                let s = v.to_string();
                s.parse::<FeelNumber>().map_or_else(
                    |_| Value::Null(Some(format!("bad numeric: {s}"))),
                    Value::Number,
                )
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::TEXTOID | pg_sys::VARCHAROID => match tuple.get_by_index::<String>(attno) {
            Ok(Some(v)) => Value::String(v),
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::DATEOID => match tuple.get_by_index::<pgrx::datum::Date>(attno) {
            Ok(Some(d)) => {
                let y = d.year();
                let m = u32::from(d.month());
                let day = u32::from(d.day());
                match FeelDate::new(y, m, day) {
                    Some(fd) => Value::Date(fd),
                    None => Value::Null(Some(format!("invalid date: {y}-{m}-{day}"))),
                }
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::TIMESTAMPOID => match tuple.get_by_index::<pgrx::datum::Timestamp>(attno) {
            Ok(Some(ts)) => {
                let y = ts.year();
                let m = u32::from(ts.month());
                let d = u32::from(ts.day());
                let h = ts.hour();
                let min = ts.minute();
                // Whole seconds always fit in u8; the fraction is carried via microseconds()
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let sec = ts.second() as u8;
                let nano = u64::from(ts.microseconds()) * 1000;
                match FeelDateTime::local(y, m, d, h, min, sec, nano) {
                    Some(fdt) => Value::DateTime(fdt),
                    None => Value::Null(Some(format!("invalid timestamp: {ts}"))),
                }
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        pg_sys::INTERVALOID => match tuple.get_by_index::<pgrx::datum::Interval>(attno) {
            Ok(Some(iv)) => {
                let months = iv.months();
                let days = iv.days();
                let micros = iv.micros();
                let has_months = months != 0;
                let has_days_or_time = days != 0 || micros != 0;
                if has_months && has_days_or_time {
                    pgrx::error!(
                        "FEEL does not support mixed intervals (months={months}, days={days}, micros={micros}); \
                         split into year-month and day-time parts"
                    );
                }
                if has_months {
                    Value::YearsAndMonthsDuration(FeelYearsAndMonthsDuration::from_m(i64::from(
                        months,
                    )))
                } else {
                    let secs_from_micros = micros.div_euclid(1_000_000);
                    let micros_remainder = micros.rem_euclid(1_000_000);
                    let total_secs = i64::from(days) * 86400 + secs_from_micros;
                    let nanos = micros_remainder * 1000;
                    Value::DaysAndTimeDuration(FeelDaysAndTimeDuration::from_sn(total_secs, nanos))
                }
            }
            Ok(None) => Value::Null(None),
            Err(e) => Value::Null(Some(format!("{e}"))),
        },
        _ => Value::Null(Some(format!(
            "unsupported PG type OID: {}",
            type_oid.to_u32()
        ))),
    }
}

/// Convert a PgHeapTuple to a FeelContext by iterating its attributes.
pub fn tuple_to_context<A: pgrx::WhoAllocated>(tuple: &PgHeapTuple<'_, A>) -> FeelContext {
    let mut ctx = FeelContext::new();
    for (attno, attr) in tuple.attributes() {
        let name_str = attr.name();
        let feel_name = Name::from(name_str);
        let value = pg_datum_to_feel(tuple, attno, attr.atttypid);
        ctx.set_entry(&feel_name, value);
    }
    ctx
}
