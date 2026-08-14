//! Rejection of DMN/FEEL external (Java/PMML) function definitions.
//!
//! dsntk 0.3 evaluates external functions by POSTing to a local RPC server
//! (127.0.0.1:22023) with a blocking HTTP client that has no timeout. Inside a
//! PostgreSQL backend that would falsify the `immutable, parallel_safe`
//! declarations on every pgdmn function, so external definitions are rejected
//! before anything is evaluated.
//!
//! Coverage: the FEEL AST walk is complete for the `feel_*` functions. For DMN
//! models the walk covers business knowledge model logic and every boxed
//! expression reachable from a decision, including nested function
//! definitions. What it cannot see is FEEL *text*—an external definition
//! written inline in a literal expression or decision-table entry is parsed
//! and compiled inside dsntk at evaluator-build time. That residual gap closes
//! with the upstream feature gate tracked as DEPS-001 in TODO.md.

use dsntk_feel_parser::AstNode;
use dsntk_model::{
    Definitions, ExpressionInstance, FunctionDefinition, FunctionKind, List, NamedElement,
};

/// Reject a parsed FEEL expression that defines an external function.
pub fn reject_external_functions(node: &AstNode) -> Result<(), String> {
    if contains_external(node) {
        Err("FEEL 'external' function definitions are not supported".to_string())
    } else {
        Ok(())
    }
}

/// Reject DMN definitions that declare external (Java/PMML) functions, in
/// business knowledge model logic or boxed anywhere in a decision's logic.
pub fn reject_external_definitions(definitions: &Definitions) -> Result<(), String> {
    for bkm in definitions.business_knowledge_models() {
        if let Some(logic) = bkm.encapsulated_logic() {
            check_function_definition(logic, "business knowledge model", bkm.name())?;
        }
    }
    for decision in definitions.decisions() {
        if let Some(logic) = decision.decision_logic() {
            check_expression(logic, "decision", decision.name())?;
        }
    }
    Ok(())
}

/// Reject an external function definition; walk a FEEL-kind body for nested
/// definitions.
fn check_function_definition(
    function: &FunctionDefinition,
    owner_kind: &str,
    owner_name: &str,
) -> Result<(), String> {
    let kind = match function.kind() {
        FunctionKind::Feel => {
            return match function.body() {
                Some(body) => check_expression(body, owner_kind, owner_name),
                None => Ok(()),
            };
        }
        FunctionKind::Java => "Java",
        FunctionKind::Pmml => "PMML",
    };
    Err(format!(
        "{owner_kind} '{owner_name}' uses an external {kind} function, which pgdmn does not support"
    ))
}

/// Recursively search a boxed expression for external function definitions.
///
/// Deliberately exhaustive (no wildcard arm), like `contains_external` below:
/// a new `ExpressionInstance` variant in a future dsntk release must fail
/// compilation here so its children get audited. `LiteralExpression` and
/// `DecisionTable` carry only FEEL text, which dsntk compiles internally —
/// nothing to walk (see the module docs).
fn check_expression(
    expression: &ExpressionInstance,
    owner_kind: &str,
    owner_name: &str,
) -> Result<(), String> {
    match expression {
        ExpressionInstance::Conditional(conditional) => {
            check_expression(conditional.if_expression().value(), owner_kind, owner_name)?;
            check_expression(
                conditional.then_expression().value(),
                owner_kind,
                owner_name,
            )?;
            check_expression(
                conditional.else_expression().value(),
                owner_kind,
                owner_name,
            )
        }
        ExpressionInstance::Context(context) => {
            for entry in context.context_entries() {
                check_expression(&entry.value, owner_kind, owner_name)?;
            }
            Ok(())
        }
        ExpressionInstance::DecisionTable(_) | ExpressionInstance::LiteralExpression(_) => Ok(()),
        ExpressionInstance::Every(every) => {
            check_expression(every.in_expression().value(), owner_kind, owner_name)?;
            check_expression(every.satisfies_expression().value(), owner_kind, owner_name)
        }
        ExpressionInstance::Filter(filter) => {
            check_expression(filter.in_expression().value(), owner_kind, owner_name)?;
            check_expression(filter.match_expression().value(), owner_kind, owner_name)
        }
        ExpressionInstance::For(for_expression) => {
            check_expression(
                for_expression.in_expression().value(),
                owner_kind,
                owner_name,
            )?;
            check_expression(
                for_expression.return_expression().value(),
                owner_kind,
                owner_name,
            )
        }
        ExpressionInstance::FunctionDefinition(function) => {
            check_function_definition(function, owner_kind, owner_name)
        }
        ExpressionInstance::Invocation(invocation) => {
            check_expression(invocation.called_function(), owner_kind, owner_name)?;
            for binding in invocation.bindings() {
                if let Some(formula) = binding.binding_formula() {
                    check_expression(formula, owner_kind, owner_name)?;
                }
            }
            Ok(())
        }
        ExpressionInstance::List(list) => check_list(list, owner_kind, owner_name),
        ExpressionInstance::Relation(relation) => {
            for row in relation.rows() {
                check_list(row, owner_kind, owner_name)?;
            }
            Ok(())
        }
        ExpressionInstance::Some(some_expression) => {
            check_expression(
                some_expression.in_expression().value(),
                owner_kind,
                owner_name,
            )?;
            check_expression(
                some_expression.satisfies_expression().value(),
                owner_kind,
                owner_name,
            )
        }
    }
}

fn check_list(list: &List, owner_kind: &str, owner_name: &str) -> Result<(), String> {
    for element in list.elements() {
        check_expression(element, owner_kind, owner_name)?;
    }
    Ok(())
}

/// Recursively search an AST for a function body flagged as external.
///
/// Deliberately exhaustive (no wildcard arm): a new `AstNode` variant in a
/// future dsntk release must fail compilation here so its children get audited
/// for reachability of external function bodies.
fn contains_external(node: &AstNode) -> bool {
    match node {
        AstNode::FunctionBody(body, external) => *external || contains_external(body),
        AstNode::At(_)
        | AstNode::Boolean(_)
        | AstNode::ContextEntryKey(_)
        | AstNode::ContextTypeEntryKey(_)
        | AstNode::FeelType(_)
        | AstNode::Irrelevant
        | AstNode::Name(_)
        | AstNode::Null
        | AstNode::Numeric(..)
        | AstNode::ParameterName(_)
        | AstNode::QualifiedNameSegment(_)
        | AstNode::String(_) => false,
        AstNode::EvaluatedExpression(child)
        | AstNode::IntervalEnd(child, _)
        | AstNode::IntervalStart(child, _)
        | AstNode::ListType(child)
        | AstNode::Neg(child)
        | AstNode::RangeType(child)
        | AstNode::Satisfies(child)
        | AstNode::UnaryEq(child)
        | AstNode::UnaryGe(child)
        | AstNode::UnaryGt(child)
        | AstNode::UnaryLe(child)
        | AstNode::UnaryLt(child)
        | AstNode::UnaryNe(child) => contains_external(child),
        AstNode::Add(left, right)
        | AstNode::And(left, right)
        | AstNode::ContextEntry(left, right)
        | AstNode::ContextTypeEntry(left, right)
        | AstNode::Div(left, right)
        | AstNode::Eq(left, right)
        | AstNode::Every(left, right)
        | AstNode::Exp(left, right)
        | AstNode::Filter(left, right)
        | AstNode::For(left, right)
        | AstNode::FormalParameter(left, right)
        | AstNode::FunctionDefinition(left, right)
        | AstNode::FunctionInvocation(left, right)
        | AstNode::FunctionType(left, right)
        | AstNode::Ge(left, right)
        | AstNode::Gt(left, right)
        | AstNode::In(left, right)
        | AstNode::InstanceOf(left, right)
        | AstNode::IterationContextSingle(left, right)
        | AstNode::Le(left, right)
        | AstNode::Lt(left, right)
        | AstNode::Mul(left, right)
        | AstNode::NamedParameter(left, right)
        | AstNode::Nq(left, right)
        | AstNode::Or(left, right)
        | AstNode::Out(left, right)
        | AstNode::Path(left, right)
        | AstNode::QuantifiedContext(left, right)
        | AstNode::Range(left, right)
        | AstNode::Some(left, right)
        | AstNode::Sub(left, right) => contains_external(left) || contains_external(right),
        AstNode::Between(first, second, third)
        | AstNode::If(first, second, third)
        | AstNode::IterationContextInterval(first, second, third) => {
            contains_external(first) || contains_external(second) || contains_external(third)
        }
        AstNode::CommaList(items)
        | AstNode::Context(items)
        | AstNode::ContextType(items)
        | AstNode::ExpressionList(items)
        | AstNode::FormalParameters(items)
        | AstNode::IterationContexts(items)
        | AstNode::List(items)
        | AstNode::NamedParameters(items)
        | AstNode::NegatedList(items)
        | AstNode::ParameterTypes(items)
        | AstNode::PositionalParameters(items)
        | AstNode::QualifiedName(items)
        | AstNode::QuantifiedContexts(items) => items.iter().any(contains_external),
    }
}
