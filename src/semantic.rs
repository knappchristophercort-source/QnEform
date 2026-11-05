//! Semantic validation and the first typed semantic representation.
//!
//! This stage validates the current syntax subset. It does not allocate,
//! execute, measure, or simulate physical or simulated quantum resources.

use crate::ast::{Expression, Program, Statement, StatementKind};
use crate::lexer::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Hybrid,
    Quantum,
    Classical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Qubit,
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticInstruction {
    EnterDomain(Domain),
    ExitDomain(Domain),
    DeclareQubit { name: String },
    Allocate { target: String },
    Release { target: String },
    Borrow { target: String, alias: String },
    ReturnBorrow { alias: String },
    Measure { target: String },
    Bind { name: String, value_type: ValueType, source: BoundValue },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundValue {
    NumberLiteral(String),
    StringLiteral(String),
    BooleanLiteral(bool),
    Reference(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    pub name: String,
    pub value_type: ValueType,
    pub declared_at: Span,
    pub domain: Domain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticModule {
    pub ir_version: String,
    pub instructions: Vec<SemanticInstruction>,
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} at {}:{}", self.code, self.message, self.span.line, self.span.column)
    }
}

impl std::error::Error for SemanticError {}

#[derive(Debug, Clone)]
struct Symbol {
    info: SymbolInfo,
}

struct Analyzer {
    scopes: Vec<HashMap<String, Symbol>>,
    symbols: Vec<SymbolInfo>,
    instructions: Vec<SemanticInstruction>,
    errors: Vec<SemanticError>,
    domain: Domain,
    allocated: HashMap<String, bool>,
    released: HashMap<String, bool>,
    borrows: HashMap<String, String>,
    borrow_counts: HashMap<String, usize>,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            symbols: Vec::new(),
            instructions: Vec::new(),
            errors: Vec::new(),
            domain: Domain::Hybrid,
            allocated: HashMap::new(),
            released: HashMap::new(),
            borrows: HashMap::new(),
            borrow_counts: HashMap::new(),
        }
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.errors.push(SemanticError { code, message: message.into(), span });
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn declare(&mut self, name: &str, value_type: ValueType, span: Span) -> bool {
        // Shadowing is intentionally rejected in v0.1. This keeps resource
        // identity and name resolution unambiguous until explicit scopes and
        // ownership transfer are part of the language contract.
        if self.lookup(name).is_some() {
            self.error(
                "SEM_DUPLICATE_DECLARATION",
                format!("'{name}' is already declared in an enclosing or current scope"),
                span,
            );
            return false;
        }

        let info = SymbolInfo {
            name: name.to_owned(),
            value_type,
            declared_at: span,
            domain: self.domain,
        };
        self.scopes.last_mut().expect("analyzer always has a scope").insert(
            name.to_owned(),
            Symbol { info: info.clone() },
        );
        self.symbols.push(info);
        true
    }

    fn analyze_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.analyze_statement(statement);
        }
    }

    fn analyze_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::QubitDeclaration { name } => {
                if self.domain == Domain::Classical {
                    self.error(
                        "SEM_QUBIT_IN_CLASSICAL_DOMAIN",
                        format!("quantum resource '{name}' cannot be declared inside a classical block"),
                        statement.span,
                    );
                    return;
                }
                if self.declare(name, ValueType::Qubit, statement.span) {
                    self.allocated.insert(name.clone(), false);
                    self.released.insert(name.clone(), false);
                    self.borrow_counts.insert(name.clone(), 0);
                    self.instructions.push(SemanticInstruction::DeclareQubit { name: name.clone() });
                }
            }
            StatementKind::Allocate { target } => {
                if self.domain == Domain::Classical {
                    self.error("SEM_RESOURCE_OPERATION_IN_CLASSICAL_DOMAIN", "allocation is not permitted in a classical block", statement.span);
                    return;
                }
                if self.lookup(target).map(|s| s.info.value_type) != Some(ValueType::Qubit) {
                    self.error("SEM_UNRESOLVED_QUANTUM_RESOURCE", format!("allocation target '{target}' is not a declared qubit"), statement.span);
                    return;
                }
                if self.allocated.get(target).copied().unwrap_or(false) || self.released.get(target).copied().unwrap_or(false) {
                    self.error("SEM_INVALID_RESOURCE_ALLOCATION", format!("'{target}' is already allocated or has been released"), statement.span);
                    return;
                }
                self.allocated.insert(target.clone(), true);
                self.instructions.push(SemanticInstruction::Allocate { target: target.clone() });
            }
            StatementKind::Release { target } => {
                if self.domain == Domain::Classical {
                    self.error("SEM_RESOURCE_OPERATION_IN_CLASSICAL_DOMAIN", "release is not permitted in a classical block", statement.span);
                    return;
                }
                if self.lookup(target).map(|s| s.info.value_type) != Some(ValueType::Qubit) {
                    self.error("SEM_UNRESOLVED_QUANTUM_RESOURCE", format!("release target '{target}' is not a declared qubit"), statement.span);
                    return;
                }
                if !self.allocated.get(target).copied().unwrap_or(false) || self.released.get(target).copied().unwrap_or(false) {
                    self.error("SEM_RESOURCE_NOT_ACTIVE", format!("'{target}' must be allocated and live before release"), statement.span);
                    return;
                }
                let count = self.borrow_counts.get(target).copied().unwrap_or(0);
                if count != 0 {
                    self.error("SEM_RESOURCE_HAS_OUTSTANDING_BORROWS", format!("'{target}' has {count} outstanding borrow(s)"), statement.span);
                    return;
                }
                self.released.insert(target.clone(), true);
                self.allocated.insert(target.clone(), false);
                self.instructions.push(SemanticInstruction::Release { target: target.clone() });
            }
            StatementKind::Borrow { target, alias } => {
                if self.domain == Domain::Classical {
                    self.error("SEM_RESOURCE_OPERATION_IN_CLASSICAL_DOMAIN", "borrow is not permitted in a classical block", statement.span);
                    return;
                }
                if self.lookup(target).map(|s| s.info.value_type) != Some(ValueType::Qubit)
                    || !self.allocated.get(target).copied().unwrap_or(false)
                    || self.released.get(target).copied().unwrap_or(false) {
                    self.error("SEM_BORROW_REQUIRES_ACTIVE_RESOURCE", format!("'{target}' must be allocated and live before it can be borrowed"), statement.span);
                    return;
                }
                if self.lookup(alias).is_some() || self.borrows.contains_key(alias) {
                    self.error("SEM_DUPLICATE_BORROW_ALIAS", format!("borrow alias '{alias}' is already in use"), statement.span);
                    return;
                }
                self.borrows.insert(alias.clone(), target.clone());
                *self.borrow_counts.entry(target.clone()).or_insert(0) += 1;
                self.instructions.push(SemanticInstruction::Borrow { target: target.clone(), alias: alias.clone() });
            }
            StatementKind::ReturnBorrow { alias } => {
                let Some(target) = self.borrows.remove(alias) else {
                    self.error("SEM_UNKNOWN_BORROW", format!("'{alias}' is not an active borrow alias"), statement.span);
                    return;
                };
                let count = self.borrow_counts.entry(target).or_insert(0);
                *count = count.saturating_sub(1);
                self.instructions.push(SemanticInstruction::ReturnBorrow { alias: alias.clone() });
            }
            StatementKind::Measurement { target } => {
                if self.domain == Domain::Classical {
                    self.error(
                        "SEM_MEASUREMENT_IN_CLASSICAL_DOMAIN",
                        "measurement is not permitted inside a classical block",
                        statement.span,
                    );
                    return;
                }
                if !self.allocated.get(target).copied().unwrap_or(false) || self.released.get(target).copied().unwrap_or(false) {
                    self.error("SEM_RESOURCE_NOT_ACTIVE", format!("measurement target '{target}' must be allocated and live"), statement.span);
                    return;
                }
                match self.lookup(target).map(|symbol| symbol.info.value_type) {
                    None => self.error(
                        "SEM_UNRESOLVED_QUANTUM_RESOURCE",
                        format!("measurement target '{target}' has not been declared"),
                        statement.span,
                    ),
                    Some(ValueType::Qubit) => self.instructions.push(
                        SemanticInstruction::Measure { target: target.clone() },
                    ),
                    Some(_) => self.error(
                        "SEM_MEASUREMENT_TARGET_NOT_QUBIT",
                        format!("'{target}' is not a quantum resource"),
                        statement.span,
                    ),
                }
            }
            StatementKind::LetBinding { name, value } => {
                if self.domain == Domain::Quantum {
                    self.error(
                        "SEM_CLASSICAL_BINDING_IN_QUANTUM_DOMAIN",
                        format!("classical binding '{name}' is not permitted directly inside a quantum block"),
                        statement.span,
                    );
                    return;
                }

                let (value_type, bound_value) = match value {
                    Expression::Number(number) => (
                        ValueType::Number,
                        BoundValue::NumberLiteral(number.clone()),
                    ),
                    Expression::String(string) => (
                        ValueType::String,
                        BoundValue::StringLiteral(string.clone()),
                    ),
                    Expression::Boolean(value) => (
                        ValueType::Boolean,
                        BoundValue::BooleanLiteral(*value),
                    ),
                    Expression::Identifier(reference) => {
                        match self.lookup(reference).map(|symbol| symbol.info.value_type) {
                            None => {
                                self.error(
                                    "SEM_UNRESOLVED_IDENTIFIER",
                                    format!("'{reference}' has not been declared"),
                                    statement.span,
                                );
                                return;
                            }
                            Some(ValueType::Qubit) => {
                                self.error(
                                    "SEM_QUANTUM_VALUE_IN_CLASSICAL_BINDING",
                                    format!("quantum resource '{reference}' cannot be copied into a classical binding"),
                                    statement.span,
                                );
                                return;
                            }
                            Some(value_type) => (
                                value_type,
                                BoundValue::Reference(reference.clone()),
                            ),
                        }
                    }
                };

                if self.declare(name, value_type, statement.span) {
                    self.instructions.push(SemanticInstruction::Bind {
                        name: name.clone(),
                        value_type,
                        source: bound_value,
                    });
                }
            }
            StatementKind::QuantumBlock { statements } => {
                if self.domain == Domain::Classical {
                    self.error(
                        "SEM_INVALID_DOMAIN_NESTING",
                        "a quantum block cannot be nested inside a classical block",
                        statement.span,
                    );
                    return;
                }
                self.analyze_block(Domain::Quantum, statements);
            }
            StatementKind::ClassicalBlock { statements } => {
                if self.domain == Domain::Quantum {
                    self.error(
                        "SEM_INVALID_DOMAIN_NESTING",
                        "a classical block cannot be nested inside a quantum block",
                        statement.span,
                    );
                    return;
                }
                self.analyze_block(Domain::Classical, statements);
            }
        }
    }

    fn analyze_block(&mut self, next_domain: Domain, statements: &[Statement]) {
        let previous = self.domain;
        self.scopes.push(HashMap::new());
        self.domain = next_domain;
        self.instructions.push(SemanticInstruction::EnterDomain(next_domain));
        self.analyze_statements(statements);
        self.instructions.push(SemanticInstruction::ExitDomain(next_domain));
        self.domain = previous;
        self.scopes.pop();
    }
}

/// Validate a parsed program and lower it to the initial typed semantic module.
///
/// A module is returned only when every semantic check succeeds. No partial
/// module is exposed on failure.
pub fn analyze(program: &Program) -> Result<SemanticModule, Vec<SemanticError>> {
    let mut analyzer = Analyzer::new();
    analyzer.analyze_statements(&program.statements);

    if !analyzer.borrows.is_empty() {
        for alias in analyzer.borrows.keys() {
            analyzer.errors.push(SemanticError { code: "SEM_BORROW_NOT_RETURNED", message: format!("borrow alias '{alias}' was not returned"), span: program.statements.last().map(|s| s.span).unwrap_or(Span { start: 0, end: 0, line: 1, column: 1 }) });
        }
    }

    if analyzer.errors.is_empty() {
        Ok(SemanticModule {
            ir_version: "qneform-semantic-0.1".to_owned(),
            instructions: analyzer.instructions,
            symbols: analyzer.symbols,
        })
    } else {
        Err(analyzer.errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn analyze_source(source: &str) -> Result<SemanticModule, Vec<SemanticError>> {
        let tokens = Lexer::new(source).tokenize().expect("fixture must lex");
        let program = Parser::new(tokens).parse().expect("fixture must parse");
        analyze(&program)
    }

    fn error_codes(source: &str) -> Vec<&'static str> {
        analyze_source(source)
            .expect_err("fixture should fail semantic analysis")
            .into_iter()
            .map(|error| error.code)
            .collect()
    }

    #[test]
    fn accepts_valid_hybrid_program_and_lowers_typed_instructions() {
        let module = analyze_source(
            "qbit q0; allocate q0; classical { let shots := 100; let label := \"run\"; let enabled := true; let count := shots; } quantum { measure q0; }"
        ).unwrap();

        assert_eq!(module.ir_version, "qneform-semantic-0.1");
        assert!(module.instructions.iter().any(|instruction| matches!(
            instruction,
            SemanticInstruction::DeclareQubit { name } if name == "q0"
        )));
        assert!(module.instructions.iter().any(|instruction| matches!(
            instruction,
            SemanticInstruction::Measure { target } if target == "q0"
        )));
        assert!(module.instructions.iter().any(|instruction| matches!(
            instruction,
            SemanticInstruction::Bind { name, value_type: ValueType::Number, .. } if name == "shots"
        )));
    }

    #[test]
    fn rejects_duplicate_declarations_and_shadowing() {
        assert!(error_codes("qbit q0; qbit q0;").contains(&"SEM_DUPLICATE_DECLARATION"));
        assert!(error_codes("let item := 1; classical { let item := 2; }")
            .contains(&"SEM_DUPLICATE_DECLARATION"));
    }

    #[test]
    fn rejects_unresolved_identifiers_and_measurement_targets() {
        assert!(error_codes("let result := missing;").contains(&"SEM_UNRESOLVED_IDENTIFIER"));
        assert!(error_codes("measure q404;").contains(&"SEM_UNRESOLVED_QUANTUM_RESOURCE"));
    }

    #[test]
    fn rejects_quantum_resource_as_classical_value() {
        assert!(error_codes("qbit q0; let copied := q0;")
            .contains(&"SEM_QUANTUM_VALUE_IN_CLASSICAL_BINDING"));
    }

    #[test]
    fn rejects_classical_operations_in_quantum_domain() {
        assert!(error_codes("quantum { let count := 3; }")
            .contains(&"SEM_CLASSICAL_BINDING_IN_QUANTUM_DOMAIN"));
    }

    #[test]
    fn rejects_quantum_operations_in_classical_domain() {
        let codes = error_codes("classical { qbit q0; measure q0; }");
        assert!(codes.contains(&"SEM_QUBIT_IN_CLASSICAL_DOMAIN"));
        assert!(codes.contains(&"SEM_MEASUREMENT_IN_CLASSICAL_DOMAIN"));
    }

    #[test]
    fn rejects_cross_domain_block_nesting() {
        assert!(error_codes("quantum { classical { let x := 1; } }")
            .contains(&"SEM_INVALID_DOMAIN_NESTING"));
        assert!(error_codes("classical { quantum { qbit q0; } }")
            .contains(&"SEM_INVALID_DOMAIN_NESTING"));
    }

    #[test]
    fn block_local_declarations_do_not_escape() {
        assert!(error_codes("quantum { qbit local_q; } measure local_q;")
            .contains(&"SEM_UNRESOLVED_QUANTUM_RESOURCE"));
    }

    #[test]
    fn classical_references_are_allowed_but_qubit_references_are_not() {
        assert!(analyze_source("let first := 4; let second := first;").is_ok());
        assert!(analyze_source("qbit q0; let second := q0;").is_err());
    }
}
