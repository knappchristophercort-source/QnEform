//! QnEform language implementation.
//!
//! The current frontend supports a deliberately small source subset:
//! qubit declarations, measurement, literal/name bindings, and explicit
//! quantum/classical blocks. Quantum semantics and execution are not yet
//! implemented.

pub mod ast;
pub mod lexer;
pub mod parser;

/// Current crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> &'static str {
    VERSION
}

/// Parse source text into the initial QnEform AST.
pub fn parse_source(source: &str) -> Result<ast::Program, String> {
    let tokens = lexer::Lexer::new(source)
        .tokenize()
        .map_err(|error| error.to_string())?;
    parser::Parser::new(tokens)
        .parse()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert!(!super::version().trim().is_empty());
    }

    #[test]
    fn parses_minimal_qubit_program() {
        let program = super::parse_source("qbit q0; measure q0;").unwrap();
        assert_eq!(program.statements.len(), 2);
    }

    #[test]
    fn rejects_unknown_syntax() {
        assert!(super::parse_source("quantum { H q0; }").is_err());
    }
}
