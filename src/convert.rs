use dsntk_feel::context::FeelContext;
use dsntk_feel::values::Value;
use dsntk_feel::{FeelNumber, Name};

/// Convert a serde_json::Value to a dsntk FEEL Value.
pub fn json_to_feel(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null(None),
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            match s.parse::<FeelNumber>() {
                Ok(num) => Value::Number(num),
                Err(_) => Value::Null(Some(format!("cannot convert number: {}", s))),
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

/// Convert a dsntk FEEL Value to serde_json::Value.
pub fn feel_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null(_) => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => {
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
                map.insert(name.to_string(), feel_to_json(val));
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
