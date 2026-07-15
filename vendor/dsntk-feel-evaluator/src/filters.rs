//! # FEEL filter implementation

use dsntk_feel::context::FeelContext;
use dsntk_feel::values::{Value, Values};
use dsntk_feel::{Evaluator, FeelScope, Name, value_null};
use std::sync::LazyLock;

// PGDMN: H20 — the `item` binding name is fixed; avoid re-allocating it on every filter call.
static NAME_ITEM: LazyLock<Name> = LazyLock::new(|| "item".into());

pub struct FilterExpressionEvaluator {}

impl Default for FilterExpressionEvaluator {
  /// Implements [Default] trait for [FilterExpressionEvaluator].
  fn default() -> Self {
    Self::new()
  }
}

impl FilterExpressionEvaluator {
  pub fn new() -> Self {
    Self {}
  }

  pub fn evaluate(&self, scope: &FeelScope, value: Value, evaluator: &Evaluator) -> Value {
    self.evaluate_with_hint(scope, value, evaluator, true)
  }

  /// Like [FilterExpressionEvaluator::evaluate], with a build-time hint whether the filter
  /// expression may evaluate to a number (numeric index). When `may_yield_number` is `false`,
  /// the extra probe evaluation that only detects numeric indexing is skipped.
  // PGDMN: H20 — added hint variant; `evaluate` keeps the previous behavior.
  pub fn evaluate_with_hint(&self, scope: &FeelScope, value: Value, evaluator: &Evaluator, may_yield_number: bool) -> Value {
    let name_item: &Name = &NAME_ITEM;
    match value {
      Value::List(values) => {
        let mut filtered_values = vec![];
        // PGDMN: H20 — reuse one scratch context across elements instead of building a fresh one per element
        let mut special_context = FeelContext::default();
        for value in &values {
          let (added_local_context, has_item_entry) = if let Value::Context(local_context) = value {
            scope.push(local_context.clone());
            if local_context.contains_entry(name_item) { (true, true) } else { (true, false) }
          } else {
            (false, false)
          };
          if !has_item_entry {
            special_context.set_entry(name_item, value.clone());
            scope.push(special_context.clone());
          }
          let rhv = evaluator(scope);
          match rhv {
            Value::Boolean(true) => filtered_values.push(value.clone()),
            Value::Boolean(false) | Value::Number(_) | Value::Null(_) => {}
            _ => return value_null!("only number or boolean indexes are allowed in filters"),
          }
          if !has_item_entry {
            scope.pop();
          }
          if added_local_context {
            scope.pop();
          }
        }
        // PGDMN: H20 — when the filter expression cannot evaluate to a number,
        // skip the probe evaluation used only to detect numeric indexing.
        if !may_yield_number {
          return Value::List(filtered_values);
        }
        let rhv = evaluator(scope);
        match rhv {
          Value::Number(index) => {
            if index.is_integer() {
              let list_size = values.len();
              if list_size > 0 {
                if !index.is_negative() {
                  let n = {
                    if let Ok(u_index) = usize::try_from(index) {
                      u_index
                    } else {
                      return value_null!("index is out of range 1..2⁶⁴: {}", index.to_string());
                    }
                  };
                  if n > 0 && n <= list_size {
                    // unwrap below is safe, index `n` is checked above, `values` variable is immutable
                    values.get(n - 1).unwrap().to_owned()
                  } else {
                    value_null!("index in filter is out of range [1..{}], actual index is {}", list_size, n)
                  }
                } else {
                  let n = {
                    if let Ok(u_index) = usize::try_from(index.abs()) {
                      u_index
                    } else {
                      return value_null!("index is out of range 1..2⁶⁴: {}", index.to_string());
                    }
                  };
                  if n > 0 && n <= list_size {
                    // unwrap below is safe, index `n` is checked above, `values` variable is immutable
                    values.get(list_size - n).unwrap().to_owned()
                  } else {
                    value_null!("index in filter is out of range [-{}..-1], actual index is -{}", list_size, n)
                  }
                }
              } else {
                // return null when the list is empty, no matter what value the index has
                value_null!()
              }
            } else {
              value_null!("index in filter must be an integer value, actual value is {}", index)
            }
          }
          _ => Value::List(filtered_values),
        }
      }
      v @ Value::Number(_)
      | v @ Value::Boolean(_)
      | v @ Value::String(_)
      | v @ Value::Date(_)
      | v @ Value::DateTime(_)
      | v @ Value::Time(_)
      | v @ Value::DaysAndTimeDuration(_)
      | v @ Value::YearsAndMonthsDuration(_)
      | v @ Value::Context(_) => match evaluator(scope) {
        Value::Boolean(flag) => {
          if flag {
            Value::List(vec![v])
          } else {
            Value::List(Values::default())
          }
        }
        Value::Number(num) => {
          if num.is_one() || (-num).is_one() {
            v
          } else {
            value_null!("for singletons, only filter index with value 1 or -1 is accepted")
          }
        }
        _ => value_null!("only number or boolean indexes are allowed in filters"),
      },
      other => value_null!("unexpected value type in filter: {}", other.type_of()),
    }
  }
}
