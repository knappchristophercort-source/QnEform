//! QnEform language implementation.
//!
//! The current frontend includes a source lexer, parser, syntax AST, and
//! semantic validation for a deliberately small language subset. It does not
//! yet execute or simulate quantum programs.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod resource;
pub mod semantic;
pub mod semantic_ir;

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

/// Parse and semantically validate source text.
///
/// The returned module is the initial typed semantic representation. It is
/// not executable bytecode or a quantum circuit.
pub fn validate_source(source: &str) -> Result<semantic::SemanticModule, String> {
    let program = parse_source(source)?;
    semantic::analyze(&program).map_err(|errors| {
        errors.into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
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

    #[test]
    fn validates_semantics_after_parsing() {
        assert!(super::validate_source("qbit q0; measure q0;").is_ok());
        assert!(super::validate_source("measure missing;").is_err());
    }
}

/// Parse, semantically validate, and lower source into QnEform Semantic IR.
///
/// The source name is optional provenance metadata. This API does not
/// serialize the IR or execute it.
pub fn compile_to_ir(
    source: &str,
    source_name: Option<&str>,
) -> Result<semantic_ir::SemanticIrModule, String> {
    let program = parse_source(source)?;
    let validated = semantic::analyze(&program).map_err(|errors| {
        errors.into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\\n")
    })?;
    semantic_ir::lower(&program, &validated, source_name)
        .map_err(|error| error.to_string())
}
