//! Recursive-descent parser for the initial QnEform source subset.

use crate::ast::{Expression, Program, Statement, StatementKind};
use crate::lexer::{Keyword, Span, Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.span.line, self.span.column)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.cursor).or_else(|| self.tokens.last())
            .expect("lexer always emits an EOF token")
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(&token.kind, TokenKind::Eof) {
            self.cursor += 1;
        }
        token
    }

    fn error_here(&self, message: impl Into<String>) -> ParseError {
        ParseError { message: message.into(), span: self.current().span }
    }

    fn expect_identifier(&mut self) -> Result<(String, Span), ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok((name, token.span)),
            other => Err(ParseError {
                message: format!("expected identifier, found {other:?}"),
                span: token.span,
            }),
        }
    }

    fn expect_kind(&mut self, expected: TokenKind, description: &str) -> Result<Token, ParseError> {
        let token = self.advance();
        if std::mem::discriminant(&token.kind) == std::mem::discriminant(&expected) {
            Ok(token)
        } else {
            Err(ParseError {
                message: format!("expected {description}, found {:?}", token.kind),
                span: token.span,
            })
        }
    }

    fn expect_keyword(&mut self, expected: Keyword, description: &str) -> Result<Token, ParseError> {
        let token = self.advance();
        if token.kind == TokenKind::Keyword(expected) {
            Ok(token)
        } else {
            Err(ParseError {
                message: format!("expected {description}, found {:?}", token.kind),
                span: token.span,
            })
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok(Expression::Identifier(name)),
            TokenKind::Number(value) => Ok(Expression::Number(value)),
            TokenKind::String(value) => Ok(Expression::String(value)),
            TokenKind::Keyword(Keyword::True) => Ok(Expression::Boolean(true)),
            TokenKind::Keyword(Keyword::False) => Ok(Expression::Boolean(false)),
            other => Err(ParseError {
                message: format!("expected literal or identifier, found {other:?}"),
                span: token.span,
            }),
        }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        while !matches!(&self.current().kind, TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.current().kind.clone() {
            TokenKind::Keyword(Keyword::Qbit) => {
                let start = self.advance().span;
                let (name, _) = self.expect_identifier()?;
                self.expect_kind(TokenKind::Semicolon, "';'")?;
                Ok(Statement { kind: StatementKind::QubitDeclaration { name }, span: start })
            }
            TokenKind::Keyword(Keyword::Measure) => {
                let start = self.advance().span;
                let (target, _) = self.expect_identifier()?;
                self.expect_kind(TokenKind::Semicolon, "';'")?;
                Ok(Statement { kind: StatementKind::Measurement { target }, span: start })
            }
            TokenKind::Keyword(Keyword::Let) => {
                let start = self.advance().span;
                let (name, _) = self.expect_identifier()?;
                self.expect_kind(TokenKind::Assign, "':='")?;
                let value = self.parse_expression()?;
                self.expect_kind(TokenKind::Semicolon, "';'")?;
                Ok(Statement { kind: StatementKind::LetBinding { name, value }, span: start })
            }
            TokenKind::Keyword(Keyword::Quantum) => {
                let start = self.advance().span;
                let statements = self.parse_block()?;
                Ok(Statement { kind: StatementKind::QuantumBlock { statements }, span: start })
            }
            TokenKind::Keyword(Keyword::Classical) => {
                let start = self.advance().span;
                let statements = self.parse_block()?;
                Ok(Statement { kind: StatementKind::ClassicalBlock { statements }, span: start })
            }
            _ => Err(self.error_here("expected qbit, measure, let, quantum, or classical")),
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect_kind(TokenKind::LBrace, "'{'")?;
        let mut statements = Vec::new();
        while !matches!(&self.current().kind, TokenKind::RBrace | TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }
        self.expect_kind(TokenKind::RBrace, "'}'")?;
        Ok(statements)
    }

    #[allow(dead_code)]
    fn _expect_keyword_example(&mut self) -> Result<Token, ParseError> {
        self.expect_keyword(Keyword::Let, "'let'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program, String> {
        let tokens = Lexer::new(source).tokenize().map_err(|error| error.to_string())?;
        Parser::new(tokens).parse().map_err(|error| error.to_string())
    }

    #[test]
    fn parses_mixed_program_and_nested_blocks() {
        let program = parse(
            "qbit q0; quantum { qbit q1; measure q1; } classical { let shots := 100; let ready := true; }"
        ).unwrap();
        assert_eq!(program.statements.len(), 3);
        assert!(matches!(&program.statements[0].kind, StatementKind::QubitDeclaration { .. }));
        assert!(matches!(&program.statements[1].kind, StatementKind::QuantumBlock { .. }));
        assert!(matches!(&program.statements[2].kind, StatementKind::ClassicalBlock { .. }));
    }

    #[test]
    fn rejects_missing_semicolon_and_unclosed_block() {
        assert!(parse("qbit q0").is_err());
        assert!(parse("quantum { qbit q0;").is_err());
    }

    #[test]
    fn rejects_unsupported_gate_syntax_instead_of_silently_accepting_it() {
        assert!(parse("quantum { H q0; }").is_err());
    }

    #[test]
    fn parses_string_literal_with_escape() {
        let program = parse("let note := \"a\\tb\";").unwrap();
        assert!(matches!(
            &program.statements[0].kind,
            StatementKind::LetBinding { value: Expression::String(value), .. } if value == "a\tb"
        ));
    }
}
