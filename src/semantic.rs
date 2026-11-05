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
}

impl Analyzer {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            symbols: Vec::new(),
            instructions: Vec::new(),
            errors: Vec::new(),
            domain: Domain::Hybrid,
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
                    self.instructions.push(SemanticInstruction::DeclareQubit {
                        name: name.clone(),
                    });
                }
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
            "qbit q0; classical { let shots := 100; let label := \"run\"; let enabled := true; let count := shots; } quantum { measure q0; }"
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
