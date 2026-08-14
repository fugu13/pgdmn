use super::super::*;
use dsntk_feel::scope;

// PGDMN: H23 — `now()` is no longer a recognized built-in function name
// (removed from `dsntk_feel::bif::Bif` so it can never resolve as a FEEL
// function, keeping pgdmn's `IMMUTABLE` SQL functions honest about the wall
// clock). Both call conventions now hit the same "unresolved function" null
// every unknown FEEL function name produces — see `build_name` and
// `build_function_invocation_with_{positional,named}_parameters` in
// dsntk-feel-evaluator/src/builders.rs.

#[test]
fn _0001() {
  te_null(
    false,
    &scope!(),
    "now()",
    r#"expected built-in function name or function definition, actual is null(context has no value for key 'now')"#,
  );
}

#[test]
fn _0002() {
  te_null(
    false,
    &scope!(),
    r#"now(when: 1)"#,
    r#"expected built-in function name or function definition, actual is null(context has no value for key 'now')"#,
  );
}
