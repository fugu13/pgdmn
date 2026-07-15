use std::num::NonZeroUsize;

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelNumber, Name};
use dsntk_feel_temporal::{
    FeelDate, FeelDateTime, FeelDaysAndTimeDuration, FeelYearsAndMonthsDuration,
};
use pgrx::heap_tuple::PgHeapTuple;
use pgrx::pg_sys;

// The pgrx-free JSON <-> FEEL conversions live in convert_core.rs (shared
// with the profiling harness); re-exported here so callers keep one import
// path for the whole conversion layer. feel_to_json is the pgrx boundary
// around the fallible core conversion: a non-finite number (BUG-004) becomes
// a SQL error.
pub use crate::convert_core::{feel_number_is_finite, json_to_context};

/// Convert a dsntk FEEL Value to serde_json::Value, raising a SQL error when
/// the value contains a non-finite number (BUG-004).
pub fn feel_to_json(value: &Value) -> serde_json::Value {
    crate::convert_core::try_feel_to_json(value).unwrap_or_else(|e| pgrx::error!("{}", e))
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
        let feel_name = Name::from(attr.name());
        let value = pg_datum_to_feel(tuple, attno, attr.atttypid);
        ctx.insert(feel_name, value);
    }
    ctx
}
