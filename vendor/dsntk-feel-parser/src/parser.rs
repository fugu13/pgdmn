//! Implementation of the `LALR` parser for `FEEL` grammar.

use crate::AstNode;
use crate::errors::*;
use crate::lalr::*;
use crate::lexer::*;
use crate::scope::ParsingScope;
use dsntk_common::Result;
use dsntk_feel::{FeelType, IntervalType, Name};

enum Action {
  Accept,
  NewState,
  Default,
  Shift,
  Reduce,
  Error,
  Error1,
}

// PGDMN: H22 — maximum nesting depth accepted for a parsed FEEL expression's AST. This
// parser is LALR/table-driven (see the shift-reduce loop below), so parsing itself never
// recurses on the native stack regardless of input nesting; the risk lives entirely in the
// *AstNode tree* it builds, which is a Box-linked recursive structure that every downstream
// consumer (Debug/Clone/derived Drop, dsntk-feel-evaluator's evaluator builder and every
// invocation of the evaluator it builds, this crate's own ClosureBuilder, and pgdmn's
// src/guard.rs) walks with ordinary native recursion. 500 was chosen with large margin
// above realistic FEEL/DMN nesting (real expressions run to a few dozen levels at most) and
// large margin below the measured native-stack-overflow floor (~7,000-10,000 host-native,
// macOS/aarch64, across parse+drop and parse+evaluate on several nesting constructs -- see
// vendor/PATCHES.md). Left-recursive constructs (chained `+`, `.`, comparisons, ...) build a
// left-nested AST just as deep as explicit nesting despite the shift-reduce parse stack
// itself staying flat, so they are capped identically to explicit nesting -- see
// `pop_depth`/`push_depth`/`widen_depth` below.
const MAX_FEEL_NESTING_DEPTH: u32 = 500;

macro_rules! trace {
  ($s:tt, $fmt:expr, $($arg:tt)*) => {
    if $s.yy_trace { println!($fmt, $($arg)*); }
  };
  ($s:tt, $fmt:expr) => {
    if $s.yy_trace { println!($fmt); }
  };
}

macro_rules! trace_action {
  ($s:tt, $msg:literal) => {
    if $s.yy_trace {
      println!("  action: [\u{001b}[32m{}\u{001b}[0m]", $msg);
      println!("  state_stack={:?}", $s.yy_state_stack);
      println!("  value_stack={:?}", $s.yy_value_stack);
      println!("  node_stack={:?}", $s.yy_node_stack);
    }
  };
}

/// Parser.
pub struct Parser<'parser> {
  /// Parsing scope.
  scope: &'parser ParsingScope,
  /// Parsed input.
  input: &'parser str,
  /// Flag indicating whether the tracing messages should be printed to standard output.
  yy_trace: bool,
  /// The FEEL [Lexer] used by this FEEL [Parser] as an input token stream.
  yy_lexer: Lexer<'parser>,
  /// The lookahead token type returned by lexer.
  yy_char: i16,
  /// The lookahead semantic value associated with token type returned by lexer.
  yy_value: TokenValue,
  /// Current token.
  yy_token: i16,
  /// Current state.
  yy_state: usize,
  /// This is an all-purpose variable, it may represent a state number or a rule number.
  yy_n: i16,
  /// The number of symbols on the RHS of the reduced rule, keep to zero when no symbol should be popped.
  yy_len: i16,
  /// State stack.
  yy_state_stack: Vec<usize>,
  /// Semantic value stack.
  yy_value_stack: Vec<TokenValue>,
  /// AST node stack.
  yy_node_stack: Vec<AstNode>,
  /// PGDMN: H22 — nesting depth of each `AstNode` value on `yy_node_stack`, kept in
  /// lock-step (same length, same push/pop order) with it. See MAX_FEEL_NESTING_DEPTH.
  yy_depth_stack: Vec<u32>,
}

impl<'parser> Parser<'parser> {
  /// Creates a new parser.
  pub fn new(scope: &'parser ParsingScope, start_token_type: TokenType, input: &'parser str, trace: bool) -> Self {
    let lexer = Lexer::new(scope, start_token_type, input);
    Self {
      scope,
      input,
      yy_trace: trace,
      yy_lexer: lexer,
      yy_char: TokenType::YyEmpty as i16,
      yy_value: TokenValue::YyEmpty,
      yy_token: TokenType::YyEmpty as i16,
      yy_state: 0,
      yy_n: 0,
      yy_len: 0,
      yy_state_stack: vec![0],
      yy_value_stack: vec![TokenValue::YyEmpty],
      yy_node_stack: vec![],
      yy_depth_stack: vec![], // PGDMN: H22
    }
  }

  /// Parses the input.
  pub fn parse(&mut self) -> Result<AstNode> {
    let mut action = Action::NewState;
    loop {
      match action {
        Action::NewState => {
          trace!(self, "\nNEW-STATE: {}", self.yy_state);
          // if the final state then accept
          if self.yy_state == YY_FINAL {
            action = Action::Accept;
            continue;
          }
          // first try to decide what to do without reference to lookahead token
          self.yy_n = YY_PACT[self.yy_state];
          if self.yy_n == YY_PACT_N_INF {
            // process the default action
            action = Action::Default;
            continue;
          }
          // not known, so get a lookahead token, only if we don't have one already
          if self.yy_char == TokenType::YyEmpty as i16 {
            let (token_type, opt_token_value) = self.yy_lexer.next_token()?;
            self.yy_char = token_type as i16;
            self.yy_token = SymbolKind::YyEmpty as i16;
            self.yy_value = opt_token_value;
            trace!(self, "  lexer: yy_char={}", self.yy_char);
            trace!(self, "  lexer: yy_value={:?}", self.yy_value);
          }
          trace!(self, "  yy_char={}", self.yy_char);
          if self.yy_char <= TokenType::YyEof as i16 {
            self.yy_char = TokenType::YyEof as i16;
            self.yy_token = SymbolKind::YyEof as i16;
            trace!(self, "  Now at end of input.");
          } else if self.yy_char == TokenType::YyError as i16 {
            self.yy_char = TokenType::YyUndef as i16;
            self.yy_token = SymbolKind::YyUndef as i16;
            action = Action::Error1;
            continue;
          } else {
            self.yy_token = YY_TRANSLATE[self.yy_char as usize] as i16;
          }
          trace!(self, "  yy_token={}", self.yy_token);
          trace!(self, "  state_stack={:?}", self.yy_state_stack);
          trace!(self, "  value_stack={:?}", self.yy_value_stack);
          trace!(self, "  node_stack={:?}", self.yy_node_stack);
          //
          let yy_token_code = self.yy_token;
          self.yy_n += yy_token_code;
          if self.yy_n < 0 || YY_LAST < self.yy_n || YY_CHECK[self.yy_n as usize] != yy_token_code {
            action = Action::Default;
            continue;
          }
          self.yy_n = YY_TABLE[self.yy_n as usize];
          if self.yy_n <= 0 {
            if self.yy_n == YY_TABLE_N_INF {
              action = Action::Error;
            } else {
              self.yy_n = -self.yy_n;
              action = Action::Reduce;
            }
          } else {
            action = Action::Shift;
          }
        }
        Action::Default => {
          trace!(self, "\nDEFAULT");
          self.yy_n = YY_DEF_ACT[self.yy_state] as i16;
          if self.yy_n == 0 {
            action = Action::Error;
          } else {
            trace!(self, "  reduce_using_rule = {}", self.yy_n);
            action = Action::Reduce;
          }
        }
        Action::Shift => {
          trace!(self, "\nSHIFT");
          self.yy_state = self.yy_n as usize;
          self.yy_state_stack.push(self.yy_state);
          self.yy_value_stack.push(self.yy_value.clone());
          trace!(self, "  state_stack={:?}", self.yy_state_stack);
          trace!(self, "  value_stack={:?}", self.yy_value_stack);
          trace!(self, "  node_stack={:?}", self.yy_node_stack);
          self.yy_char = TokenType::YyEmpty as i16;
          self.yy_value = TokenValue::YyEmpty;
          action = Action::NewState;
        }
        Action::Reduce => {
          trace!(self, "\nREDUCE");
          // get the length of the right-hand side
          self.yy_len = YY_R2[self.yy_n as usize] as i16;
          trace!(self, "  reduce count = {}", self.yy_len);
          // yy_n is the number of a rule to reduce with
          trace!(self, "  --------------------------------------------");
          trace!(self, "  reducing_using_rule = {}", self.yy_n);
          reduce(self, self.yy_n)?;
          trace!(self, "  --------------------------------------------");
          // pop the state stack and semantic value stack
          for _ in 0..self.yy_len {
            self.yy_state_stack.pop();
            self.yy_value_stack.pop();
          }
          // keep yy_len = 0
          self.yy_len = 0;
          let yy_lhs = (YY_R1[self.yy_n as usize] as usize) - YY_N_TOKENS;
          let top_state = self.yy_state_stack[self.yy_state_stack.len() - 1] as i16;
          let yy_i = YY_P_GOTO[yy_lhs] + top_state;
          // calculate the new state number
          self.yy_state = if (0..=YY_LAST).contains(&yy_i) && YY_CHECK[yy_i as usize] == top_state {
            YY_TABLE[yy_i as usize] as usize
          } else {
            YY_DEF_GOTO[yy_lhs] as usize
          };
          trace!(self, "  new_state = {}", self.yy_state);
          // push the new state on the stack
          self.yy_state_stack.push(self.yy_state);
          self.yy_value_stack.push(TokenValue::YyState);
          trace!(self, "  state_stack={:?}", self.yy_state_stack);
          trace!(self, "  value_stack={:?}", self.yy_value_stack);
          trace!(self, "  node_stack={:?}", self.yy_node_stack);
          action = Action::NewState;
        }
        Action::Error => {
          trace!(self, "\nERROR");
          self.yy_token = SymbolKind::YyError as i16;
          return Err(err_syntax_error(self.input));
        }
        Action::Error1 => {
          trace!(self, "\nERROR 1");
          return Err(err_syntax_error(self.input));
        }
        Action::Accept => {
          trace!(self, "\n**********");
          trace!(self, "* ACCEPT *");
          trace!(self, "**********\n");
          self.yy_token = SymbolKind::YyAccept as i16;
          let node = self.yy_node_stack.pop().unwrap();
          debug_assert!(self.yy_node_stack.is_empty());
          if self.yy_trace {
            println!("    AST:{}", node.trace());
          }
          return Ok(node);
        }
      }
    }
  }

  // PGDMN: H22 — depth-cap bookkeeping used by every reduce action below that touches
  // `yy_node_stack`; see MAX_FEEL_NESTING_DEPTH for the rationale and vendor/PATCHES.md.

  /// Pops the nesting depth paired with the `AstNode` value just popped off
  /// `yy_node_stack`, keeping `yy_depth_stack` in lock-step with it.
  fn pop_depth(&mut self) -> u32 {
    self.yy_depth_stack.pop().unwrap_or(0)
  }

  /// Pushes `depth` paired with the `AstNode` value just pushed onto `yy_node_stack`,
  /// rejecting the parse before an AST deep enough to overflow the native stack in a later
  /// recursive consumer (Debug/Clone/derived Drop, the evaluator, guard.rs, ClosureBuilder)
  /// is ever built.
  fn push_depth(&mut self, depth: u32) -> Result<()> {
    if depth > MAX_FEEL_NESTING_DEPTH {
      return Err(err_expression_too_deeply_nested(MAX_FEEL_NESTING_DEPTH));
    }
    self.yy_depth_stack.push(depth);
    Ok(())
  }

  /// Folds one more element into an already-tracked *flat* accumulator (context entries,
  /// list/parameter items, ...): the accumulator's depth is the deeper of what it already
  /// covered and one more than the new element, never the sum across elements -- otherwise a
  /// wide-but-shallow expression (many flat entries) would be rejected as if it were deeply
  /// nested, since these accumulators are rebuilt one element at a time.
  fn widen_depth(&mut self, container_depth: u32, item_depth: u32) -> Result<()> {
    self.push_depth(container_depth.max(item_depth + 1))
  }
}

impl ReduceActions for Parser<'_> {
  fn action_addition(&mut self) -> Result<()> {
    trace_action!(self, "addition");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Add(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_after_dot(&mut self) -> Result<()> {
    trace_action!(self, "after dot");
    self.yy_lexer.set_after_dot();
    Ok(())
  }

  fn action_between(&mut self) -> Result<()> {
    trace_action!(self, "between");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let mhs = self.yy_node_stack.pop().unwrap();
    let mhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Between(Box::new(lhs), Box::new(mhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(mhs_depth).max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_between_begin(&mut self) -> Result<()> {
    trace_action!(self, "between_begin");
    self.yy_lexer.set_between();
    Ok(())
  }

  fn action_built_in_type_name(&mut self) -> Result<()> {
    trace_action!(self, "built_in_type_name");
    if let TokenValue::BuiltInTypeName(name) = &self.yy_value_stack[self.yy_value_stack.len() - 1] {
      self.yy_node_stack.push(AstNode::FeelType(name.into()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_comparison_eq(&mut self) -> Result<()> {
    trace_action!(self, "comparison_equal");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Eq(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_ge(&mut self) -> Result<()> {
    trace_action!(self, "comparison_greater_or_equal");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Ge(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_gt(&mut self) -> Result<()> {
    trace_action!(self, "comparison_greater_than");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Gt(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_in(&mut self) -> Result<()> {
    trace_action!(self, "comparison_in");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::In(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_le(&mut self) -> Result<()> {
    trace_action!(self, "comparison_less_or_equal");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Le(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_lt(&mut self) -> Result<()> {
    trace_action!(self, "comparison_less_than");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Lt(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_ne(&mut self) -> Result<()> {
    trace_action!(self, "comparison_not_equal");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Nq(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_eq(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_eq");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryEq(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_ge(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_ge");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryGe(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_gt(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_gt");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryGt(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_le(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_le");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryLe(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_lt(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_lt");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryLt(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_comparison_unary_ne(&mut self) -> Result<()> {
    trace_action!(self, "comparison_unary_ne");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::UnaryNe(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_conjunction(&mut self) -> Result<()> {
    trace_action!(self, "conjunction");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::And(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_context_begin(&mut self) -> Result<()> {
    trace_action!(self, "context_begin");
    self.yy_lexer.push_to_scope();
    Ok(())
  }

  fn action_context_end(&mut self) -> Result<()> {
    trace_action!(self, "context_end");
    self.yy_lexer.pop_from_scope();
    Ok(())
  }

  fn action_context_entry(&mut self) -> Result<()> {
    trace_action!(self, "context_entry");
    let value = self.yy_node_stack.pop().unwrap();
    let value_depth = self.pop_depth(); // PGDMN: H22
    let key = self.yy_node_stack.pop().unwrap();
    let key_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::ContextEntryKey(name) = &key {
      self.yy_lexer.add_parsed_name(name.clone());
    }
    self.yy_node_stack.push(AstNode::ContextEntry(Box::new(key), Box::new(value)));
    self.push_depth(1 + key_depth.max(value_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_context_entry_tail(&mut self) -> Result<()> {
    trace_action!(self, "context_entry_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::Context(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::Context(items));
      // PGDMN: H22 — flat accumulator: widen, don't compound (see `widen_depth`).
      self.widen_depth(node_depth, item_depth)?;
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::Context(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_context_type_entry(&mut self) -> Result<()> {
    trace_action!(self, "context_type_entry");
    let type_node = self.yy_node_stack.pop().unwrap();
    let type_node_depth = self.pop_depth(); // PGDMN: H22
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize] {
      let lhs = Box::new(AstNode::ContextTypeEntryKey(name.clone()));
      let rhs = Box::new(type_node);
      self.yy_node_stack.push(AstNode::ContextTypeEntry(lhs, rhs));
      self.push_depth(1 + type_node_depth)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_context_type_entry_tail(&mut self) -> Result<()> {
    trace_action!(self, "context_type_entry_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::ContextType(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::ContextType(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::ContextType(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_disjunction(&mut self) -> Result<()> {
    trace_action!(self, "disjunction");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Or(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_division(&mut self) -> Result<()> {
    trace_action!(self, "division");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Div(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_empty_context(&mut self) -> Result<()> {
    trace_action!(self, "empty context");
    self.yy_node_stack.push(AstNode::Context(vec![]));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_every(&mut self) -> Result<()> {
    trace_action!(self, "every");
    // pop temporary context from the top of the scope
    self.yy_lexer.pop_from_scope();
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let satisfies = Box::new(AstNode::Satisfies(Box::new(rhs)));
    self.yy_node_stack.push(AstNode::Every(Box::new(lhs), satisfies));
    // PGDMN: H22 — rhs is wrapped twice (Satisfies, then Every), lhs only once.
    self.push_depth(1 + lhs_depth.max(1 + rhs_depth))?;
    Ok(())
  }

  fn action_every_begin(&mut self) -> Result<()> {
    trace_action!(self, "every_begin");
    // push temporary context on the top of the scope,
    // this context will be used to store
    // local variable names of quantified expressions
    self.yy_lexer.push_to_scope();
    Ok(())
  }

  fn action_exponentiation(&mut self) -> Result<()> {
    trace_action!(self, "exponentiation");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Exp(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_expression_list_tail(&mut self) -> Result<()> {
    trace_action!(self, "expression_list_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::ExpressionList(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::ExpressionList(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::ExpressionList(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_filter(&mut self) -> Result<()> {
    trace_action!(self, "filter");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Filter(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  /// Reduces `for` expression.
  /// At the top of `yy_node_stack` there is a node representing an expression to be evaluated,
  /// followed by the node representing iteration contexts.
  fn action_for(&mut self) -> Result<()> {
    trace_action!(self, "for");
    // pop temporary context from the top of the scope
    self.yy_lexer.pop_from_scope();
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let evaluated_expression = AstNode::EvaluatedExpression(Box::new(rhs));
    self.yy_node_stack.push(AstNode::For(Box::new(lhs), Box::new(evaluated_expression)));
    // PGDMN: H22 — rhs is wrapped twice (EvaluatedExpression, then For), lhs only once.
    self.push_depth(1 + lhs_depth.max(1 + rhs_depth))?;
    Ok(())
  }

  fn action_for_begin(&mut self) -> Result<()> {
    trace_action!(self, "for_begin");
    // push temporary context on the top of the scope,
    // this context will be used to store
    // local variable names in iteration contexts
    self.yy_lexer.push_to_scope();
    // add name `partial` to the temporary context present on top of the scope,
    // this is the name of the implicit variable containing results of all previous iterations
    self.yy_lexer.add_name_to_scope(&Name::from("partial"));
    Ok(())
  }

  fn action_formal_parameter_with_type(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameter_with_type");
    // the type of the parameter is on top of node stack
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    // the name of the parameter is in value stack
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize] {
      // push the new formal parameter on top of node stack
      let parameter_name = Box::new(AstNode::ParameterName(name.clone()));
      let parameter_type = Box::new(rhs);
      self.yy_node_stack.push(AstNode::FormalParameter(parameter_name, parameter_type));
      // set the name of the parameter to local context on the top of the scope stack
      // this name will be properly interpreted as a name while parsing the function body
      self.scope.set_entry_name(name.to_owned());
      // PGDMN: H22 — parameter_name is a fresh leaf (depth 1); moved after the last use of
      // the `name` borrow above, since `push_depth` takes `&mut self`.
      self.push_depth(1 + rhs_depth.max(1))?;
    }
    Ok(())
  }

  fn action_formal_parameter_without_type(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameter_without_type");
    // the name of the parameter is in value stack
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize] {
      // push the new formal parameter on top of node stack
      let parameter_name = Box::new(AstNode::ParameterName(name.clone()));
      let parameter_type = Box::new(AstNode::FeelType(FeelType::Any));
      self.yy_node_stack.push(AstNode::FormalParameter(parameter_name, parameter_type));
      // set the name of the parameter to local context on the top of the scope stack
      // this name will be properly interpreted as a name while parsing the function body
      self.scope.set_entry_name(name.to_owned());
      // PGDMN: H22 — both children are fresh leaves (depth 1 each); moved after the last use
      // of the `name` borrow above, since `push_depth` takes `&mut self`.
      self.push_depth(2)?;
    }
    Ok(())
  }

  fn action_formal_parameters_begin(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameters_begin");
    // when the list of formal parameters begins, push an empty local context onto the scope stack
    self.scope.push_default();
    Ok(())
  }

  fn action_formal_parameters_empty(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameters_empty");
    // push the empty list of formal parameters onto the node stack
    self.yy_node_stack.push(AstNode::FormalParameters(vec![]));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_formal_parameters_first(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameters_first");
    // the first parameter is on the top of the node stack
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::FormalParameters(vec![lhs]));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_formal_parameters_tail(&mut self) -> Result<()> {
    trace_action!(self, "function_formal_parameters_tail");
    // the next parameter is on the top of the node stack
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    // the collection of formal parameters is now on top of the node stack
    let container_depth = self.pop_depth(); // PGDMN: H22
    if let Some(AstNode::FormalParameters(mut items)) = self.yy_node_stack.pop() {
      items.push(rhs);
      self.yy_node_stack.push(AstNode::FormalParameters(items));
      self.widen_depth(container_depth, rhs_depth)?; // PGDMN: H22
    }
    Ok(())
  }

  /// Reduces the definition of the function body. This function body is **not** `external`.
  /// The content of the function body is the [AstNode] on the top of the `yy_node-stack`.
  /// After reducing the function body, the top context from scope is popped,
  /// because this context is temporary and contains the function parameter names
  /// to be properly interpreted while parsing the function's body.
  fn action_function_body(&mut self) -> Result<()> {
    trace_action!(self, "function_body");
    let body_depth = self.pop_depth(); // PGDMN: H22
    if let Some(function_body_node) = self.yy_node_stack.pop() {
      self.yy_node_stack.push(AstNode::FunctionBody(Box::new(function_body_node), false));
      self.push_depth(1 + body_depth)?; // PGDMN: H22
    }
    // pop temporary context from the top of scope stack
    self.scope.pop();
    Ok(())
  }

  /// Reduces the definition of the function body. This function body **is** `external`.
  /// The content of the function body is the [AstNode] on the top of the `yy_node-stack`.
  /// After reducing the function body, the top context from scope is popped,
  /// because this context is temporary and contains the function parameter names
  /// to be properly interpreted while parsing the function's body.
  fn action_function_body_external(&mut self) -> Result<()> {
    trace_action!(self, "function_body_external");
    let body_depth = self.pop_depth(); // PGDMN: H22
    if let Some(function_body_node) = self.yy_node_stack.pop() {
      self.yy_node_stack.push(AstNode::FunctionBody(Box::new(function_body_node), true));
      self.push_depth(1 + body_depth)?; // PGDMN: H22
    }
    // pop temporary context from the top of scope stack
    self.scope.pop();
    Ok(())
  }

  /// Reduces the function definition.
  /// The top element on the `node stack` is the AstNode defining the function body.
  /// The AstNode just below the function's body is the list of function's formal parameters.
  fn action_function_definition(&mut self) -> Result<()> {
    trace_action!(self, "function_definition");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::FunctionDefinition(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_function_invocation(&mut self) -> Result<()> {
    trace_action!(self, "function_invocation");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::FunctionInvocation(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_function_invocation_no_parameters(&mut self) -> Result<()> {
    trace_action!(self, "function_invocation_no_parameters");
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    if let Some(lhs) = self.yy_node_stack.pop() {
      let rhs = AstNode::PositionalParameters(vec![]);
      self.yy_node_stack.push(AstNode::FunctionInvocation(Box::new(lhs), Box::new(rhs)));
      self.push_depth(1 + lhs_depth.max(1))?; // PGDMN: H22 — rhs is a fresh empty leaf.
    }
    Ok(())
  }

  fn action_function_type(&mut self) -> Result<()> {
    trace_action!(self, "function_type");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::FunctionType(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_function_type_parameters_empty(&mut self) -> Result<()> {
    trace_action!(self, "function_type_parameters_empty");
    self.yy_node_stack.push(AstNode::ParameterTypes(vec![]));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_function_type_parameters_tail(&mut self) -> Result<()> {
    trace_action!(self, "function_type_parameters_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::ParameterTypes(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::ParameterTypes(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::ParameterTypes(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  /// Reduces `if` expression.
  fn action_if(&mut self) -> Result<()> {
    trace_action!(self, "if");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let mid = self.yy_node_stack.pop().unwrap();
    let mid_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::If(Box::new(lhs), Box::new(mid), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(mid_depth).max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_instance_of(&mut self) -> Result<()> {
    trace_action!(self, "instance_of");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let checked_value = Box::new(lhs);
    let expected_type = Box::new(rhs);
    self.yy_node_stack.push(AstNode::InstanceOf(checked_value, expected_type));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_interval(&mut self) -> Result<()> {
    trace_action!(self, "interval");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Range(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_interval_end(&mut self) -> Result<()> {
    trace_action!(self, "interval_end");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - 1], TokenValue::RightBracket) {
      IntervalType::Closed
    } else {
      IntervalType::Opened
    };
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::IntervalEnd(Box::new(lhs), interval_type));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_interval_start(&mut self) -> Result<()> {
    trace_action!(self, "interval_start");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize], TokenValue::LeftBracket) {
      IntervalType::Closed
    } else {
      IntervalType::Opened
    };
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::IntervalStart(Box::new(lhs), interval_type));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  /// Reduces iteration context containing a variable name and a range of values to iterate over.
  /// Nodes are located on `yy_node_stack` in the following order (looking from top):
  /// - range end,
  /// - range start,
  /// - variable name.
  fn action_iteration_context_value_range(&mut self) -> Result<()> {
    trace_action!(self, "iteration_context_value_range");
    let rhs = self.yy_node_stack.pop().unwrap(); // range end
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let mid = self.yy_node_stack.pop().unwrap(); // range start
    let mid_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap(); // variable name
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let node = AstNode::IterationContextInterval(Box::new(lhs), Box::new(mid), Box::new(rhs));
    self.yy_node_stack.push(node);
    self.push_depth(1 + lhs_depth.max(mid_depth).max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  /// Reduces iteration context containing a variable name and a single value to iterate over.
  /// Nodes are located on `yy_node_stack` in the following order (looking from top):
  /// - single value,
  /// - variable name.
  fn action_iteration_context_value_single(&mut self) -> Result<()> {
    trace_action!(self, "iteration_context_value_single");
    let rhs = self.yy_node_stack.pop().unwrap(); // single value
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap(); // variable name
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let node = AstNode::IterationContextSingle(Box::new(lhs), Box::new(rhs));
    self.yy_node_stack.push(node);
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  /// Reduces the variable name of iteration context.
  /// Variable name is located on the top of the `yy_value_stack`.
  /// This name is pushed onto `yy_node_stack`.
  fn action_iteration_context_variable_name(&mut self) -> Result<()> {
    trace_action!(self, "iteration_context_variable_name");
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - 1] {
      self.yy_node_stack.push(AstNode::Name(name.clone()));
      // add this variable name to the temporary context present on top of the scope
      self.yy_lexer.add_name_to_scope(name);
      // PGDMN: H22 — moved after the last use of the `name` borrow above, since
      // `push_depth` takes `&mut self`.
      self.push_depth(1)?;
    }
    Ok(())
  }

  fn action_iteration_context_variable_name_begin(&mut self) -> Result<()> {
    trace_action!(self, "iteration_context_variable_name_begin");
    self.yy_lexer.set_till_in();
    Ok(())
  }

  /// Reduces the iteration context.
  fn action_iteration_contexts_tail(&mut self) -> Result<()> {
    trace_action!(self, "iteration_contexts_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::IterationContexts(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::IterationContexts(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::IterationContexts(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_key_name(&mut self) -> Result<()> {
    trace_action!(self, "key_name");
    if let Some(TokenValue::Name(name)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::ContextEntryKey(name.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_key_string(&mut self) -> Result<()> {
    trace_action!(self, "key_string");
    if let Some(TokenValue::String(value)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::ContextEntryKey(Name::from(value.clone())));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_list(&mut self) -> Result<()> {
    trace_action!(self, "list");
    let depth = self.pop_depth(); // PGDMN: H22 — relabeling, depth passes through unchanged.
    if let Some(AstNode::CommaList(items)) = self.yy_node_stack.pop() {
      self.yy_node_stack.push(AstNode::List(items));
      self.push_depth(depth)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_list_empty(&mut self) -> Result<()> {
    trace_action!(self, "list_empty");
    self.yy_node_stack.push(AstNode::CommaList(vec![]));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_list_tail(&mut self) -> Result<()> {
    trace_action!(self, "list_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::CommaList(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::CommaList(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::CommaList(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_list_type(&mut self) -> Result<()> {
    trace_action!(self, "list_type");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::ListType(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_literal_at(&mut self) -> Result<()> {
    trace_action!(self, "literal_at");
    if let Some(TokenValue::String(value)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::At(value.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_literal_boolean(&mut self) -> Result<()> {
    trace_action!(self, "literal_boolean");
    if let Some(TokenValue::Boolean(value)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::Boolean(*value));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_literal_date_time(&mut self) -> Result<()> {
    trace_action!(self, "literal_date_time");
    if let TokenValue::NameDateTime(name) = &self.yy_value_stack[self.yy_value_stack.len() - 2] {
      self.yy_node_stack.push(AstNode::Name(name.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_literal_null(&mut self) -> Result<()> {
    trace_action!(self, "literal_null");
    if let Some(TokenValue::Null) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::Null);
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_literal_numeric(&mut self) -> Result<()> {
    trace_action!(self, "numeric_literal");
    if let Some(TokenValue::Numeric(before, after, sign, exponent)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::Numeric(before.clone(), after.clone(), *sign, exponent.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_literal_string(&mut self) -> Result<()> {
    trace_action!(self, "string_literal");
    if let Some(TokenValue::String(value)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::String(value.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_multiplication(&mut self) -> Result<()> {
    trace_action!(self, "multiplication");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Mul(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_name(&mut self) -> Result<()> {
    trace_action!(self, "name");
    if let Some(TokenValue::Name(value)) = self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::Name(value.clone()));
      self.push_depth(1)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_named_parameter(&mut self) -> Result<()> {
    trace_action!(self, "named_parameter");
    trace!(self, "{:?}", self.yy_value_stack);
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - 3] {
      let rhs = self.yy_node_stack.pop().unwrap();
      let parameter_name = Box::new(AstNode::ParameterName(name.clone()));
      let parameter_value = Box::new(rhs);
      self.yy_node_stack.push(AstNode::NamedParameter(parameter_name, parameter_value));
      // PGDMN: H22 — parameter_name is a fresh leaf (depth 1); `pop_depth` moved after the
      // last use of the `name` borrow above, since it takes `&mut self`. `rhs`'s depth was
      // already popped before `rhs` itself was moved into `parameter_value`, so we read it
      // via a second pop -- there's only ever one node's worth of depth left to pop here.
      let rhs_depth = self.pop_depth();
      self.push_depth(1 + rhs_depth.max(1))?;
    }
    Ok(())
  }

  fn action_named_parameters_tail(&mut self) -> Result<()> {
    trace_action!(self, "named_parameters_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::NamedParameters(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::NamedParameters(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::NamedParameters(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_negation(&mut self) -> Result<()> {
    trace_action!(self, "negation");
    let depth = self.pop_depth(); // PGDMN: H22
    if let Some(node) = self.yy_node_stack.pop() {
      self.yy_node_stack.push(AstNode::Neg(Box::new(node)));
      self.push_depth(1 + depth)?; // PGDMN: H22
    }
    Ok(())
  }

  fn action_path(&mut self) -> Result<()> {
    trace_action!(self, "path");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    if let Some(TokenValue::Name(name)) = &self.yy_value_stack.last() {
      let rhs = AstNode::Name(name.clone());
      self.yy_node_stack.push(AstNode::Path(Box::new(lhs), Box::new(rhs)));
      // PGDMN: H22 — rhs is a fresh Name leaf (depth 1).
      self.push_depth(1 + lhs_depth.max(1))?;
    }
    Ok(())
  }

  fn action_positional_parameters_tail(&mut self) -> Result<()> {
    trace_action!(self, "positional_parameters_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::PositionalParameters(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::PositionalParameters(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::PositionalParameters(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_qualified_name(&mut self) -> Result<()> {
    trace_action!(self, "action_qualified_name");
    if let Some(TokenValue::Name(name)) = &self.yy_value_stack.last() {
      self.yy_node_stack.push(AstNode::QualifiedName(vec![AstNode::QualifiedNameSegment(name.clone())]));
      self.push_depth(2)?; // PGDMN: H22 — fresh QualifiedName wrapping one fresh segment leaf.
    }
    Ok(())
  }

  fn action_qualified_name_tail(&mut self) -> Result<()> {
    trace_action!(self, "action_qualified_name_tail");
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - 4] {
      // The parent name is taken from the value stack, after adding actions to grammar rules,
      // the index must be adjusted, now it is `-4` after adding `{/* after_dot */}`.
      if let Some(AstNode::QualifiedName(mut parts)) = self.yy_node_stack.pop() {
        parts.insert(0, AstNode::QualifiedNameSegment(name.clone()));
        self.yy_node_stack.push(AstNode::QualifiedName(parts));
        // PGDMN: H22 — moved after the last use of the `name` borrow above, since
        // `pop_depth`/`widen_depth` take `&mut self`. Flat accumulator of fresh (depth-1)
        // segment leaves; widen, don't compound.
        let container_depth = self.pop_depth();
        self.widen_depth(container_depth, 1)?;
      }
    }
    Ok(())
  }

  fn action_quantified_expression(&mut self) -> Result<()> {
    trace_action!(self, "quantified_expression");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let node = AstNode::QuantifiedContext(Box::new(lhs), Box::new(rhs));
    self.yy_node_stack.push(node);
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  /// Reduces the variable name of quantified expression.
  /// Variable name is located on the top of the `yy_value_stack`.
  /// This name is pushed onto `yy_node_stack`.
  fn action_quantified_expression_variable_name(&mut self) -> Result<()> {
    trace_action!(self, "quantified_expression_variable_name");
    if let TokenValue::Name(name) = &self.yy_value_stack[self.yy_value_stack.len() - 1] {
      self.yy_node_stack.push(AstNode::Name(name.clone()));
      // add this variable name to the temporary context present on top of the scope
      self.yy_lexer.add_name_to_scope(name);
      // PGDMN: H22 — moved after the last use of the `name` borrow above, since
      // `push_depth` takes `&mut self`.
      self.push_depth(1)?;
    }
    Ok(())
  }

  fn action_quantified_expression_variable_name_begin(&mut self) -> Result<()> {
    trace_action!(self, "quantified_expression_variable_name_begin");
    self.yy_lexer.set_till_in();
    Ok(())
  }

  fn action_quantified_expressions_tail(&mut self) -> Result<()> {
    trace_action!(self, "quantified_expressions_tail");
    let node = self.yy_node_stack.pop().unwrap();
    let node_depth = self.pop_depth(); // PGDMN: H22
    if let AstNode::QuantifiedContexts(mut items) = node {
      let item = self.yy_node_stack.pop().unwrap();
      let item_depth = self.pop_depth(); // PGDMN: H22
      items.insert(0, item);
      self.yy_node_stack.push(AstNode::QuantifiedContexts(items));
      self.widen_depth(node_depth, item_depth)?; // PGDMN: H22
      return Ok(());
    }
    self.yy_node_stack.push(AstNode::QuantifiedContexts(vec![node]));
    self.push_depth(1 + node_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_range_literal(&mut self) -> Result<()> {
    trace_action!(self, "range_literal");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Range(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_range_literal_empty_end(&mut self) -> Result<()> {
    trace_action!(self, "range_literal_empty_end");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - 1], TokenValue::RightBracket) {
      IntervalType::ClosedUndef
    } else {
      IntervalType::OpenedUndef
    };
    self.yy_node_stack.push(AstNode::IntervalEnd(AstNode::Null.into(), interval_type));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_range_literal_empty_start(&mut self) -> Result<()> {
    trace_action!(self, "range_literal_empty_start");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize], TokenValue::LeftBracket) {
      IntervalType::ClosedUndef
    } else {
      IntervalType::OpenedUndef
    };
    self.yy_node_stack.push(AstNode::IntervalStart(AstNode::Null.into(), interval_type));
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_range_literal_end(&mut self) -> Result<()> {
    trace_action!(self, "range_literal_end");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - 1], TokenValue::RightBracket) {
      IntervalType::Closed
    } else {
      IntervalType::Opened
    };
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::IntervalEnd(Box::new(lhs), interval_type));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_range_literal_start(&mut self) -> Result<()> {
    trace_action!(self, "range_literal_start");
    let interval_type = if matches!(&self.yy_value_stack[self.yy_value_stack.len() - self.yy_len as usize], TokenValue::LeftBracket) {
      IntervalType::Closed
    } else {
      IntervalType::Opened
    };
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::IntervalStart(Box::new(lhs), interval_type));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_range_type(&mut self) -> Result<()> {
    trace_action!(self, "range_type");
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::RangeType(Box::new(lhs)));
    self.push_depth(1 + lhs_depth)?; // PGDMN: H22
    Ok(())
  }

  fn action_some(&mut self) -> Result<()> {
    trace_action!(self, "some");
    // pop temporary context from the top of the scope
    self.yy_lexer.pop_from_scope();
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    let satisfies = Box::new(AstNode::Satisfies(Box::new(rhs)));
    self.yy_node_stack.push(AstNode::Some(Box::new(lhs), satisfies));
    // PGDMN: H22 — rhs is wrapped twice (Satisfies, then Some), lhs only once.
    self.push_depth(1 + lhs_depth.max(1 + rhs_depth))?;
    Ok(())
  }

  fn action_some_begin(&mut self) -> Result<()> {
    trace_action!(self, "some_begin");
    // push temporary context on the top of the scope,
    // this context will be used to store
    // local variable names of quantified expressions
    self.yy_lexer.push_to_scope();
    Ok(())
  }

  fn action_subtraction(&mut self) -> Result<()> {
    trace_action!(self, "subtraction");
    let rhs = self.yy_node_stack.pop().unwrap();
    let rhs_depth = self.pop_depth(); // PGDMN: H22
    let lhs = self.yy_node_stack.pop().unwrap();
    let lhs_depth = self.pop_depth(); // PGDMN: H22
    self.yy_node_stack.push(AstNode::Sub(Box::new(lhs), Box::new(rhs)));
    self.push_depth(1 + lhs_depth.max(rhs_depth))?; // PGDMN: H22
    Ok(())
  }

  fn action_type_name(&mut self) -> Result<()> {
    trace_action!(self, "type_name");
    self.yy_lexer.set_type_name();
    Ok(())
  }

  fn action_unary_tests_begin(&mut self) -> Result<()> {
    trace_action!(self, "unary_tests_begin");
    self.yy_lexer.set_unary_tests();
    Ok(())
  }

  fn action_unary_tests_irrelevant(&mut self) -> Result<()> {
    trace_action!(self, "unary_tests_irrelevant");
    self.yy_node_stack.push(AstNode::Irrelevant);
    self.push_depth(1)?; // PGDMN: H22
    Ok(())
  }

  fn action_unary_tests_negated(&mut self) -> Result<()> {
    trace_action!(self, "unary_tests_negated");
    let depth = self.pop_depth(); // PGDMN: H22 — relabeling, depth passes through unchanged.
    if let Some(AstNode::ExpressionList(items)) = self.yy_node_stack.pop() {
      self.yy_node_stack.push(AstNode::NegatedList(items));
      self.push_depth(depth)?; // PGDMN: H22
    }
    Ok(())
  }
}
