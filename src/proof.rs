//! Typed, module- and resource-bound proof attestations.
//!
//! This module authenticates verifier decisions and binds them to a specific
//! compilation/module and resource generation. It does not independently
//! establish physical quantum-state properties: that assurance belongs to the
//! configured verifier implementation.

use crate::resource::{ResourceHandle, ResourceKind};

pub const ATTESTATION_SCHEMA_V1: &str = "qneform-proof-attestation-0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofKind {
    Separable,
    CleanAndSeparable,
    DirtyRestoredAndSeparable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDecision {
    Verified,
    Rejected,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofClaim {
    pub schema: String,
    pub module_fingerprint: [u8; 32],
    pub resource: ResourceHandle,
    pub resource_kind: ResourceKind,
    pub resource_generation: u64,
    pub proof_kind: ProofKind,
    pub evidence_digest: [u8; 32],
    pub verifier_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    Rejected,
    Inconclusive,
    UnsupportedSchema,
    EmptyVerifierIdentity,
    InvalidProofKindForResource,
    BindingMismatch,
}

/// Implementations are trusted verification boundaries. A verifier must
/// validate the evidence represented by the claim before returning Verified.
/// A digest or signature alone is not proof that a physical state has a
/// claimed property.
pub trait ProofVerifier {
    fn verifier_id(&self) -> &str;
    fn verify(&self, claim: &ProofClaim) -> VerificationDecision;
}

/// Opaque receipt. Its fields are private so downstream code cannot directly
/// construct a successful attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAttestation {
    claim: ProofClaim,
}

impl VerifiedAttestation {
    pub fn verify(
        claim: ProofClaim,
        verifier: &impl ProofVerifier,
    ) -> Result<Self, AttestationError> {
        if claim.schema != ATTESTATION_SCHEMA_V1 {
            return Err(AttestationError::UnsupportedSchema);
        }
        if claim.verifier_id.trim().is_empty()
            || claim.verifier_id != verifier.verifier_id()
        {
            return Err(AttestationError::EmptyVerifierIdentity);
        }
        let compatible = matches!(
            (claim.resource_kind, claim.proof_kind),
            (ResourceKind::Qubit, ProofKind::Separable)
                | (ResourceKind::CleanAncilla, ProofKind::CleanAndSeparable)
                | (ResourceKind::DirtyAncilla, ProofKind::DirtyRestoredAndSeparable)
        );
        if !compatible {
            return Err(AttestationError::InvalidProofKindForResource);
        }
        match verifier.verify(&claim) {
            VerificationDecision::Verified => Ok(Self { claim }),
            VerificationDecision::Rejected => Err(AttestationError::Rejected),
            VerificationDecision::Inconclusive => Err(AttestationError::Inconclusive),
        }
    }

    pub fn matches(
        &self,
        module_fingerprint: &[u8; 32],
        resource: ResourceHandle,
        resource_kind: ResourceKind,
        resource_generation: u64,
        proof_kind: ProofKind,
    ) -> bool {
        self.claim.module_fingerprint == *module_fingerprint
            && self.claim.resource == resource
            && self.claim.resource_kind == resource_kind
            && self.claim.resource_generation == resource_generation
            && self.claim.proof_kind == proof_kind
    }

    pub fn verifier_id(&self) -> &str {
        &self.claim.verifier_id
    }

    pub fn evidence_digest(&self) -> &[u8; 32] {
        &self.claim.evidence_digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestVerifier(VerificationDecision);

    impl ProofVerifier for TestVerifier {
        fn verifier_id(&self) -> &str { "test-verifier/1" }
        fn verify(&self, _claim: &ProofClaim) -> VerificationDecision { self.0 }
    }

    fn claim() -> ProofClaim {
        ProofClaim {
            schema: ATTESTATION_SCHEMA_V1.to_owned(),
            module_fingerprint: [7; 32],
            resource: ResourceHandle(4),
            resource_kind: ResourceKind::Qubit,
            resource_generation: 2,
            proof_kind: ProofKind::Separable,
            evidence_digest: [9; 32],
            verifier_id: "test-verifier/1".to_owned(),
        }
    }

    #[test]
    fn verified_receipt_is_bound_to_exact_module_and_resource_generation() {
        let receipt = VerifiedAttestation::verify(
            claim(),
            &TestVerifier(VerificationDecision::Verified),
        ).unwrap();
        assert!(receipt.matches(&[7; 32], ResourceHandle(4), ResourceKind::Qubit, 2, ProofKind::Separable));
        assert!(!receipt.matches(&[8; 32], ResourceHandle(4), ResourceKind::Qubit, 2, ProofKind::Separable));
        assert!(!receipt.matches(&[7; 32], ResourceHandle(4), ResourceKind::Qubit, 3, ProofKind::Separable));
        assert!(!receipt.matches(&[7; 32], ResourceHandle(5), ResourceKind::Qubit, 2, ProofKind::Separable));
    }

    #[test]
    fn rejected_and_inconclusive_verifier_results_never_mint_receipts() {
        assert_eq!(
            VerifiedAttestation::verify(claim(), &TestVerifier(VerificationDecision::Rejected)),
            Err(AttestationError::Rejected)
        );
        assert_eq!(
            VerifiedAttestation::verify(claim(), &TestVerifier(VerificationDecision::Inconclusive)),
            Err(AttestationError::Inconclusive)
        );
    }

    #[test]
    fn incompatible_proof_kind_is_rejected_before_verifier() {
        let mut invalid = claim();
        invalid.resource_kind = ResourceKind::CleanAncilla;
        assert_eq!(
            VerifiedAttestation::verify(invalid, &TestVerifier(VerificationDecision::Verified)),
            Err(AttestationError::InvalidProofKindForResource)
        );
    }
}
