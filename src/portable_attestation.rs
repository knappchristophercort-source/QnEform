//! Portable, canonical compilation-attestation manifests.
//!
//! This wire format carries source and Semantic IR identities. It is not
//! signed, does not contain source text or the complete IR, and does not prove
//! compiler correctness. A verifier must independently obtain the source and
//! IR, then call the corresponding verification methods.

use crate::{
    compilation::{CompilationEnvelope, COMPILATION_ENVELOPE_SCHEMA_V1},
    semantic_ir::SemanticIrModule,
};

const MAGIC: &[u8] = b"QNEFORM-PORTABLE-ATTESTATION\0";
const FORMAT_VERSION: u16 = 1;
const MAX_TEXT_FIELD_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortableAttestationError {
    Truncated,
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedEnvelopeSchema,
    InvalidUtf8,
    EmptyField(&'static str),
    FieldTooLong,
    TrailingBytes,
}

impl std::fmt::Display for PortableAttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(f, "portable attestation is truncated"),
            Self::InvalidMagic => write!(f, "portable attestation magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported portable attestation version: {version}")
            }
            Self::UnsupportedEnvelopeSchema => {
                write!(f, "unsupported compilation envelope schema")
            }
            Self::InvalidUtf8 => write!(f, "attestation text field is not valid UTF-8"),
            Self::EmptyField(name) => write!(f, "attestation field '{name}' must not be empty"),
            Self::FieldTooLong => write!(f, "attestation text field exceeds size limit"),
            Self::TrailingBytes => write!(f, "portable attestation contains trailing bytes"),
        }
    }
}

impl std::error::Error for PortableAttestationError {}

/// Portable compilation identity record. Private fields prevent accidental
/// mutation after parsing; the canonical byte representation is the transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortableCompilationAttestation {
    envelope_schema: String,
    compiler_package_version: String,
    ir_schema_version: String,
    source_digest: [u8; 32],
    module_fingerprint: [u8; 32],
    obligation_fingerprint: [u8; 32],
}

impl PortableCompilationAttestation {
    pub fn from_envelope(envelope: &CompilationEnvelope) -> Result<Self, PortableAttestationError> {
        if !envelope.verify_internal_integrity() {
            return Err(PortableAttestationError::UnsupportedEnvelopeSchema);
        }
        Ok(Self {
            envelope_schema: envelope.schema().to_owned(),
            compiler_package_version: envelope.compiler_package_version().to_owned(),
            ir_schema_version: envelope.ir_schema_version().to_owned(),
            source_digest: *envelope.source_digest(),
            module_fingerprint: *envelope.module_fingerprint(),
            obligation_fingerprint: *envelope.obligation_fingerprint(),
        })
    }

    pub fn envelope_schema(&self) -> &str {
        &self.envelope_schema
    }

    pub fn compiler_package_version(&self) -> &str {
        &self.compiler_package_version
    }

    pub fn ir_schema_version(&self) -> &str {
        &self.ir_schema_version
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

    /// Canonical v1 bytes: fixed magic/version, u16-length-prefixed UTF-8
    /// metadata fields, then three fixed-width SHA-256 digests.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut output = MAGIC.to_vec();
        output.extend_from_slice(&FORMAT_VERSION.to_be_bytes());
        put_text(&mut output, &self.envelope_schema);
        put_text(&mut output, &self.compiler_package_version);
        put_text(&mut output, &self.ir_schema_version);
        output.extend_from_slice(&self.source_digest);
        output.extend_from_slice(&self.module_fingerprint);
        output.extend_from_slice(&self.obligation_fingerprint);
        output
    }

    /// Strict parser. Rejects unknown versions, oversized/empty metadata,
    /// invalid UTF-8, truncation and any trailing bytes.
    pub fn from_canonical_bytes(
        bytes: &[u8],
    ) -> Result<Self, PortableAttestationError> {
        let mut cursor = Cursor::new(bytes);
        if cursor.read_exact(MAGIC.len())? != MAGIC {
            return Err(PortableAttestationError::InvalidMagic);
        }
        let version = cursor.read_u16()?;
        if version != FORMAT_VERSION {
            return Err(PortableAttestationError::UnsupportedVersion(version));
        }

        let envelope_schema = cursor.read_text()?;
        let compiler_package_version = cursor.read_text()?;
        let ir_schema_version = cursor.read_text()?;
        if envelope_schema != COMPILATION_ENVELOPE_SCHEMA_V1 {
            return Err(PortableAttestationError::UnsupportedEnvelopeSchema);
        }
        if compiler_package_version.is_empty() {
            return Err(PortableAttestationError::EmptyField("compiler_package_version"));
        }
        if ir_schema_version.is_empty() {
            return Err(PortableAttestationError::EmptyField("ir_schema_version"));
        }

        let source_digest = cursor.read_digest()?;
        let module_fingerprint = cursor.read_digest()?;
        let obligation_fingerprint = cursor.read_digest()?;
        if !cursor.is_finished() {
            return Err(PortableAttestationError::TrailingBytes);
        }

        Ok(Self {
            envelope_schema,
            compiler_package_version,
            ir_schema_version,
            source_digest,
            module_fingerprint,
            obligation_fingerprint,
        })
    }

    /// SHA-256 identity of the exact canonical manifest bytes.
    pub fn fingerprint(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(self.to_canonical_bytes());
        let mut output = [0u8; 32];
        output.copy_from_slice(&digest);
        output
    }

    /// Check source bytes independently of the compiler's in-memory envelope.
    pub fn verify_source(&self, source: &str) -> bool {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(source.as_bytes());
        digest.as_slice() == &self.source_digest[..]
    }

    /// Check an independently available IR module against both recorded IDs.
    pub fn verify_module(&self, module: &SemanticIrModule) -> bool {
        module.schema_version == self.ir_schema_version
            && module.fingerprint() == self.module_fingerprint
            && module.obligation_fingerprint() == self.obligation_fingerprint
    }

    /// Compare every manifest field with the original in-memory envelope.
    pub fn verify_envelope(&self, envelope: &CompilationEnvelope) -> bool {
        envelope.verify_internal_integrity()
            && self.envelope_schema == envelope.schema()
            && self.compiler_package_version == envelope.compiler_package_version()
            && self.ir_schema_version == envelope.ir_schema_version()
            && self.source_digest == *envelope.source_digest()
            && self.module_fingerprint == *envelope.module_fingerprint()
            && self.obligation_fingerprint == *envelope.obligation_fingerprint()
    }
}

fn put_text(output: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    output.extend_from_slice(bytes);
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, length: usize) -> Result<&'a [u8], PortableAttestationError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(PortableAttestationError::Truncated)?;
        let slice = self
            .bytes
            .get(self.offset..end)
            .ok_or(PortableAttestationError::Truncated)?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, PortableAttestationError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_text(&mut self) -> Result<String, PortableAttestationError> {
        let length = self.read_u16()? as usize;
        if length > MAX_TEXT_FIELD_BYTES {
            return Err(PortableAttestationError::FieldTooLong);
        }
        let bytes = self.read_exact(length)?;
        let value = std::str::from_utf8(bytes).map_err(|_| PortableAttestationError::InvalidUtf8)?;
        Ok(value.to_owned())
    }

    fn read_digest(&mut self) -> Result<[u8; 32], PortableAttestationError> {
        let bytes = self.read_exact(32)?;
        let mut digest = [0u8; 32];
        digest.copy_from_slice(bytes);
        Ok(digest)
    }

    fn is_finished(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation::compile_to_envelope;

    #[test]
    fn canonical_manifest_round_trips_and_verifies_source_module_and_envelope() {
        let source = "qbit q0; allocate q0; measure q0;";
        let envelope = compile_to_envelope(source, Some("portable.qn")).unwrap();
        let attestation = PortableCompilationAttestation::from_envelope(&envelope).unwrap();
        let bytes = attestation.to_canonical_bytes();
        let decoded = PortableCompilationAttestation::from_canonical_bytes(&bytes).unwrap();

        assert_eq!(decoded, attestation);
        assert_eq!(decoded.to_canonical_bytes(), bytes);
        assert!(decoded.verify_source(source));
        assert!(decoded.verify_module(envelope.module()));
        assert!(decoded.verify_envelope(&envelope));
    }

    #[test]
    fn exact_source_bytes_are_required() {
        let source = "let shots := 10;";
        let envelope = compile_to_envelope(source, None).unwrap();
        let manifest = PortableCompilationAttestation::from_envelope(&envelope).unwrap();
        assert!(manifest.verify_source(source));
        assert!(!manifest.verify_source("let shots := 10;\n"));
        assert!(!manifest.verify_source("let shots := 11;"));
    }

    #[test]
    fn changed_ir_or_obligation_identity_fails_module_verification() {
        let source = "qbit q0;";
        let envelope = compile_to_envelope(source, None).unwrap();
        let manifest = PortableCompilationAttestation::from_envelope(&envelope).unwrap();

        let changed_module = compile_to_envelope("qbit q1;", None).unwrap();
        assert!(!manifest.verify_module(changed_module.module()));

        let mut changed_status = envelope.module().clone();
        changed_status.proof_obligations[0].satisfied = true;
        assert!(!manifest.verify_module(&changed_status));
    }

    #[test]
    fn parser_rejects_trailing_bytes_and_truncation() {
        let envelope = compile_to_envelope("let shots := 10;", None).unwrap();
        let manifest = PortableCompilationAttestation::from_envelope(&envelope).unwrap();
        let mut bytes = manifest.to_canonical_bytes();
        bytes.push(0);
        assert_eq!(
            PortableCompilationAttestation::from_canonical_bytes(&bytes),
            Err(PortableAttestationError::TrailingBytes)
        );
        assert_eq!(
            PortableCompilationAttestation::from_canonical_bytes(
                &manifest.to_canonical_bytes()[..10]
            ),
            Err(PortableAttestationError::Truncated)
        );
    }

    #[test]
    fn parser_rejects_unknown_envelope_schema() {
        let envelope = compile_to_envelope("let shots := 10;", None).unwrap();
        let manifest = PortableCompilationAttestation::from_envelope(&envelope).unwrap();
        let mut bytes = manifest.to_canonical_bytes();
        let schema_start = MAGIC.len() + 2 + 2;
        let schema_length = bytes[MAGIC.len() + 2] as usize * 256
            + bytes[MAGIC.len() + 3] as usize;
        bytes[schema_start..schema_start + schema_length].fill(b'x');
        assert_eq!(
            PortableCompilationAttestation::from_canonical_bytes(&bytes),
            Err(PortableAttestationError::UnsupportedEnvelopeSchema)
        );
    }

    #[test]
    fn parser_rejects_unknown_version_and_invalid_magic() {
        let envelope = compile_to_envelope("let shots := 10;", None).unwrap();
        let manifest = PortableCompilationAttestation::from_envelope(&envelope).unwrap();
        let mut bytes = manifest.to_canonical_bytes();
        let version_offset = MAGIC.len();
        bytes[version_offset + 1] = 2;
        assert_eq!(
            PortableCompilationAttestation::from_canonical_bytes(&bytes),
            Err(PortableAttestationError::UnsupportedVersion(2))
        );

        let mut bad_magic = manifest.to_canonical_bytes();
        bad_magic[0] ^= 0xff;
        assert_eq!(
            PortableCompilationAttestation::from_canonical_bytes(&bad_magic),
            Err(PortableAttestationError::InvalidMagic)
        );
    }

    #[test]
    fn manifest_fingerprint_is_deterministic_and_content_sensitive() {
        let first = compile_to_envelope("let shots := 10;", None).unwrap();
        let second = compile_to_envelope("let shots := 11;", None).unwrap();
        let first_manifest = PortableCompilationAttestation::from_envelope(&first).unwrap();
        let first_copy = PortableCompilationAttestation::from_canonical_bytes(
            &first_manifest.to_canonical_bytes(),
        )
        .unwrap();
        let second_manifest = PortableCompilationAttestation::from_envelope(&second).unwrap();

        assert_eq!(first_manifest.fingerprint(), first_copy.fingerprint());
        assert_ne!(first_manifest.fingerprint(), second_manifest.fingerprint());
    }
}
