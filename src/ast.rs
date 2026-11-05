//! Abstract syntax tree for the deliberately small v0.1 source subset.

use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    QubitDeclaration { name: String },
    Measurement { target: String },
    LetBinding { name: String, value: Expression },
    QuantumBlock { statements: Vec<Statement> },
    ClassicalBlock { statements: Vec<Statement> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(String),
    String(String),
    Boolean(bool),
}
