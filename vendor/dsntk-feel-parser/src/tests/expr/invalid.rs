//! Testing parsing of invalid statements.

use super::super::*;
use dsntk_common::DsntkError;
use std::str::from_utf8;

/// Utility function to shorten repeatable code in tests.
fn te(input: &str, source: &str, message: &str) {
  assert_eq!(Err(DsntkError::new(source, message)), Parser::new(&scope!(), StartExpression, input, false).parse())
}

#[test]
fn _0001() {
  // Caret character is not valid at the beginning of any FEEL statement.
  te(r#"^123"#, "ParserError", r#"syntax error: ^123"#);
}

#[test]
fn _0002() {
  // Vertical space is not allowed inside FEEL string.
  let input = from_utf8(&[34, 49, 50, 10, 51, 34]).unwrap(); // "12\n3"
  te(input, "ParserError", "syntax error: \"12\n3\"");
}

#[test]
fn _0003() {
  // Unexpected end of input before the FEEL string is closed with quotation mark `"`.
  te(r#""123"#, "ParserError", "syntax error: \"123");
}

#[test]
fn _0004() {
  // Unexpected end of input when parsing hex digits.
  te(r#""a\u4B"#, "LexerError", "unexpected end of file");
}

#[test]
fn _0005() {
  // After decimal point must always be a digit.
  te(r#"1. + 2"#, "ParserError", "syntax error: 1. + 2");
}

#[test]
fn _0006() {
  // After decimal point must be a digit.
  te(r#"1.^ + 2"#, "ParserError", "syntax error: 1.^ + 2");
}

#[test]
fn _0007() {
  // End of input after decimal point.
  te(r#"1."#, "ParserError", "syntax error: 1.");
}

#[test]
fn _0008() {
  // Keyword `function` interpreted as a name. This is ok, when such name is in scope.
  let scope = scope!();
  accept(
    &scope,
    StartExpression,
    "function ",
    r#"
       Name
       └─ `function`
    "#,
    false,
  );
}

#[test]
fn _0009() {
  // Invalid UTF-16 surrogate.
  te(r#""\uD800\uE000""#, "LexerError", "surrogate value is out of allowed range 0xD800..0xDFFF : E000");
}

#[test]
fn _0010() {
  // Invalid UTF value.
  te(r#""\U110000""#, "LexerError", "value is out of allowed Unicode range 0x0000..0x10FFFF : 110000");
}

#[test]
fn _0011() {
  // There is no `+=` operator in FEEL, but this expression parses properly (will return null on execution).
  let input = "1 += 2";
  let expected = r#"
       Add
       ├─ Numeric
       │  └─ `1`
       └─ UnaryEq
          └─ Numeric
             └─ `2`
    "#;
  accept(&scope!(), StartExpression, input, expected, false);
}

// PGDMN: H22 — the depth cap (500) guards against native stack overflow in every recursive
// consumer of the AST built here (Debug/Clone/Drop, dsntk-feel-evaluator, ClosureBuilder,
// pgdmn's own guard.rs). Chained unary minus adds exactly one level of nesting per `-`, so it
// gives an exact, cheap way to sit right at (or one past) the cap; see vendor/PATCHES.md.

#[test]
fn _0012() {
  // Exactly at the cap: parses normally, does not error.
  let input = format!("{}1", "-".repeat(499));
  let result = Parser::new(&scope!(), StartExpression, &input, false).parse();
  assert!(result.is_ok(), "expected a successful parse at the depth cap, got: {result:?}");
}

#[test]
fn _0013() {
  // One level past the cap: a clean parse error, not a panic or stack overflow.
  let input = format!("{}1", "-".repeat(500));
  te(&input, "ParserError", "expression nested too deeply, exceeds maximum allowed depth of 500");
}
