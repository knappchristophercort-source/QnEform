//! QnEform language implementation.
//!
//! This crate is an intentionally small, dependency-free baseline.
//! It does not yet implement quantum semantics, a parser, a simulator,
//! QEC, hardware compilation, or the Semantic IR.

/// Current crate version, kept available to library consumers.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the implementation version.
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert!(!super::version().trim().is_empty());
    }
}
