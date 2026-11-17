//! Opaque compilation provenance envelopes.
//!
//! An envelope binds the exact UTF-8 source bytes to the semantic IR produced
//! by the library compilation pipeline. It is an in-process provenance object,
//! not a signed compiler attestation or proof that the compiler is correct.

use crate::{
    semantic,
    semantic_ir::{self, SemanticIrModule},
    parse_source,
};

pub const COMPILATION_ENVELOPE_SCHEMA_V1: &str = "qneform-compilation-envelope-0.1";

/// Immutable-by-interface record of one source-to-IR compilation event.
///
/// Fields remain private so callers cannot replace the source digest or module
/// fingerprint independently. The contained module is exposed only by shared
/// reference. This is not a persistent or cryptographically signed artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationEnvelope {
    schema: String,
    source_digest: [u8; 32],
    module_fingerprint: [u8; 32],
    obligation_fingerprint: [u8; 32],
    compiler_version: String,
    ir_schema_version: String,
    source_name: Option<String>,
    module: SemanticIrModule,
}

impl CompilationEnvelope {
    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn source_digest(&self) -> &[u8; 32] {
        &self.source_digest
    }

    pub fn module_fingerprint(&self) -> &[u8; 32] {
        &self.module_fingerprint
    }

    pub fn obligation_fingerprint(&self) -> &[u8; 32] {
        &self.obligation_fingerprint
    }

    pub fn compiler_version(&self) -> &str {
        &self.compiler_version
    }

    pub fn ir_schema_version(&self) -> &str {
        &self.ir_schema_version
    }

    pub fn source_name(&self) -> Option<&str> {
        self.source_name.as_deref()
    }

    pub fn module(&self) -> &SemanticIrModule {
        &self.module
    }

    /// Stable envelope identity over source, compiler and both IR identities.
    /// Source-name metadata is intentionally excluded.
    pub fn fingerprint(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut bytes = b"QNEFORM-COMPILATION-ENVELOPE\0".to_vec();
        put_string(&mut bytes, "qneform-compilation-envelope-canonical-v1");
        put_string(&mut bytes, &self.schema);
        bytes.extend_from_slice(&self.source_digest);
        bytes.extend_from_slice(&self.module_fingerprint);
        bytes.extend_from_slice(&self.obligation_fingerprint);
        put_string(&mut bytes, &self.compiler_version);
        put_string(&mut bytes, &self.ir_schema_version);

        let digest = Sha256::digest(bytes);
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&digest);
        fingerprint
    }

    /// Confirm the supplied source bytes and the held IR still match this
    /// envelope. This checks integrity, not compiler correctness.
    pub fn verify_source(&self, source: &str) -> bool {
        self.verify_internal_integrity() && sha256(source.as_bytes()) == self.source_digest
    }

    /// Recompute the module identities rather than trusting stored digest fields.
    pub fn verify_internal_integrity(&self) -> bool {
        self.schema == COMPILATION_ENVELOPE_SCHEMA_V1
            && self.ir_schema_version == self.module.schema_version
            && self.module.fingerprint() == self.module_fingerprint
            && self.module.obligation_fingerprint() == self.obligation_fingerprint
    }
}

/// Parse, semantically validate and lower source into an opaque provenance envelope.
pub fn compile_to_envelope(
    source: &str,
    source_name: Option<&str>,
) -> Result<CompilationEnvelope, String> {
    let program = parse_source(source)?;
    let validated = semantic::analyze(&program).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\\n")
    })?;
    let module = semantic_ir::lower(&program, &validated, source_name)
        .map_err(|error| error.to_string())?;

    Ok(CompilationEnvelope {
        schema: COMPILATION_ENVELOPE_SCHEMA_V1.to_owned(),
        source_digest: sha256(source.as_bytes()),
        module_fingerprint: module.fingerprint(),
        obligation_fingerprint: module.obligation_fingerprint(),
        compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
        ir_schema_version: module.schema_version.clone(),
        source_name: source_name.map(str::to_owned),
        module,
    })
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn put_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u64).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_binds_exact_source_to_both_ir_fingerprints() {
        let source = "qbit q0; allocate q0; measure q0;";
        let envelope = compile_to_envelope(source, Some("program.qn")).unwrap();

        assert_eq!(envelope.schema(), COMPILATION_ENVELOPE_SCHEMA_V1);
        assert!(envelope.verify_source(source));
        assert!(envelope.verify_internal_integrity());
        assert_eq!(*envelope.module_fingerprint(), envelope.module().fingerprint());
        assert_eq!(
            *envelope.obligation_fingerprint(),
            envelope.module().obligation_fingerprint()
        );
        assert_eq!(envelope.source_name(), Some("program.qn"));
    }

    #[test]
    fn changed_source_bytes_do_not_verify_against_existing_envelope() {
        let envelope = compile_to_envelope("let shots := 10;", None).unwrap();
        assert!(!envelope.verify_source("let shots := 11;"));
        assert!(!envelope.verify_source("let shots := 10;\n"));
    }

    #[test]
    fn source_name_is_provenance_not_compilation_identity() {
        let source = "let shots := 10;";
        let first = compile_to_envelope(source, Some("one.qn")).unwrap();
        let second = compile_to_envelope(source, Some("two.qn")).unwrap();

        assert_eq!(first.source_digest(), second.source_digest());
        assert_eq!(first.module_fingerprint(), second.module_fingerprint());
        assert_eq!(first.obligation_fingerprint(), second.obligation_fingerprint());
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_ne!(first.source_name(), second.source_name());
    }

    #[test]
    fn semantic_source_change_changes_envelope_identity() {
        let first = compile_to_envelope("let shots := 10;", None).unwrap();
        let second = compile_to_envelope("let shots := 11;", None).unwrap();

        assert_ne!(first.source_digest(), second.source_digest());
        assert_ne!(first.module_fingerprint(), second.module_fingerprint());
        assert_ne!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn compile_to_ir_and_envelope_share_the_same_lowering_pipeline() {
        let source = "qbit q0; allocate q0; measure q0;";
        let direct = crate::compile_to_ir(source, Some("program.qn")).unwrap();
        let envelope = compile_to_envelope(source, Some("program.qn")).unwrap();
        assert_eq!(&direct, envelope.module());
    }
}
