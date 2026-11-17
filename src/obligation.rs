//! Authenticated receipts for semantic proof obligations.
//!
//! Receipts attest that a configured verifier returned Verified for a claim
//! bound to a stable semantic-module identity and one exact proof obligation.
//! They do not independently establish verifier correctness or physical truth.

use crate::semantic_ir::{ProofObligation, ProofObligationKind, ResourceId, SemanticIrModule};

pub const OBLIGATION_ATTESTATION_SCHEMA_V1: &str = "qneform-obligation-attestation-0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationVerificationDecision {
    Verified,
    Rejected,
    Inconclusive,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationClaim {
    pub schema: String,
    pub module_fingerprint: [u8; 32],
    pub obligation_id: u32,
    pub kind: ProofObligationKind,
    pub subject_resource: Option<ResourceId>,
    pub required: bool,
    pub description: String,
    pub evidence_digest: [u8; 32],
    pub verifier_id: String,
}

impl ObligationClaim {
    /// Construct a claim from the exact module and obligation to be verified.
    pub fn for_obligation(
        module: &SemanticIrModule,
        obligation: &ProofObligation,
        evidence_digest: [u8; 32],
        verifier_id: impl Into<String>,
    ) -> Self {
        Self {
            schema: OBLIGATION_ATTESTATION_SCHEMA_V1.to_owned(),
            module_fingerprint: module.obligation_fingerprint(),
            obligation_id: obligation.id,
            kind: obligation.kind,
            subject_resource: obligation.subject_resource,
            required: obligation.required,
            description: obligation.description.clone(),
            evidence_digest,
            verifier_id: verifier_id.into(),
        }
    }
}

/// Trusted verifier boundary for semantic proof-obligation evidence.
pub trait ObligationVerifier {
    fn verifier_id(&self) -> &str;
    fn verify(&self, claim: &ObligationClaim) -> ObligationVerificationDecision;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObligationAttestationError {
    UnsupportedSchema,
    EmptyOrMismatchedVerifierIdentity,
    Rejected,
    Inconclusive,
    Unsupported,
}

/// Opaque proof receipt. Its claim is private and can only be populated by a
/// verifier that returns the explicit Verified decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedObligationReceipt {
    claim: ObligationClaim,
}

impl VerifiedObligationReceipt {
    pub fn verify(
        claim: ObligationClaim,
        verifier: &impl ObligationVerifier,
    ) -> Result<Self, ObligationAttestationError> {
        if claim.schema != OBLIGATION_ATTESTATION_SCHEMA_V1 {
            return Err(ObligationAttestationError::UnsupportedSchema);
        }
        if claim.verifier_id.trim().is_empty() || claim.verifier_id != verifier.verifier_id() {
            return Err(ObligationAttestationError::EmptyOrMismatchedVerifierIdentity);
        }
        match verifier.verify(&claim) {
            ObligationVerificationDecision::Verified => Ok(Self { claim }),
            ObligationVerificationDecision::Rejected => Err(ObligationAttestationError::Rejected),
            ObligationVerificationDecision::Inconclusive => {
                Err(ObligationAttestationError::Inconclusive)
            }
            ObligationVerificationDecision::Unsupported => {
                Err(ObligationAttestationError::Unsupported)
            }
        }
    }

    pub fn obligation_id(&self) -> u32 {
        self.claim.obligation_id
    }

    pub fn verifier_id(&self) -> &str {
        &self.claim.verifier_id
    }

    pub fn evidence_digest(&self) -> &[u8; 32] {
        &self.claim.evidence_digest
    }

    pub fn matches(&self, module: &SemanticIrModule, obligation: &ProofObligation) -> bool {
        self.claim.module_fingerprint == module.obligation_fingerprint()
            && self.claim.obligation_id == obligation.id
            && self.claim.kind == obligation.kind
            && self.claim.subject_resource == obligation.subject_resource
            && self.claim.required == obligation.required
            && self.claim.description == obligation.description
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedReadinessError {
    pub missing_or_mismatched_obligations: Vec<u32>,
    pub duplicate_obligation_ids: Vec<u32>,
    pub duplicate_receipt_ids: Vec<u32>,
    pub unexpected_receipt_ids: Vec<u32>,
}

impl std::fmt::Display for AuthenticatedReadinessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "authenticated backend readiness blocked: {} missing or mismatched obligation(s), {} duplicate obligation ID(s), {} duplicate receipt ID(s), {} unexpected receipt(s)",
            self.missing_or_mismatched_obligations.len(),
            self.duplicate_obligation_ids.len(),
            self.duplicate_receipt_ids.len(),
            self.unexpected_receipt_ids.len()
        )
    }
}

impl std::error::Error for AuthenticatedReadinessError {}

/// Require exactly one matching verifier-issued receipt for every required
/// obligation. Mutable satisfied-status values are never consulted.
pub fn validate_backend_readiness(
    module: &SemanticIrModule,
    receipts: &[VerifiedObligationReceipt],
) -> Result<(), AuthenticatedReadinessError> {
    let mut missing_or_mismatched_obligations = Vec::new();
    let mut duplicate_obligation_ids = Vec::new();
    let mut duplicate_receipt_ids = Vec::new();
    let mut unexpected_receipt_ids = Vec::new();

    for (index, obligation) in module.proof_obligations.iter().enumerate() {
        if module.proof_obligations[..index]
            .iter()
            .any(|previous| previous.id == obligation.id)
        {
            duplicate_obligation_ids.push(obligation.id);
        }

        if obligation.required {
            let matches = receipts
                .iter()
                .filter(|receipt| receipt.matches(module, obligation))
                .count();
            if matches != 1 {
                missing_or_mismatched_obligations.push(obligation.id);
                if matches > 1 {
                    duplicate_receipt_ids.push(obligation.id);
                }
            }
        }
    }

    for (index, receipt) in receipts.iter().enumerate() {
        if receipts[..index]
            .iter()
            .any(|previous| previous.obligation_id() == receipt.obligation_id())
            && !duplicate_receipt_ids.contains(&receipt.obligation_id())
        {
            duplicate_receipt_ids.push(receipt.obligation_id());
        }

        let belongs_to_required_obligation = module
            .proof_obligations
            .iter()
            .any(|obligation| obligation.required && receipt.matches(module, obligation));
        if !belongs_to_required_obligation {
            unexpected_receipt_ids.push(receipt.obligation_id());
        }
    }

    if missing_or_mismatched_obligations.is_empty()
        && duplicate_obligation_ids.is_empty()
        && duplicate_receipt_ids.is_empty()
        && unexpected_receipt_ids.is_empty()
    {
        Ok(())
    } else {
        Err(AuthenticatedReadinessError {
            missing_or_mismatched_obligations,
            duplicate_obligation_ids,
            duplicate_receipt_ids,
            unexpected_receipt_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_ir;

    struct TestVerifier(ObligationVerificationDecision);

    impl ObligationVerifier for TestVerifier {
        fn verifier_id(&self) -> &str {
            "qneform-test-verifier/1"
        }

        fn verify(&self, _claim: &ObligationClaim) -> ObligationVerificationDecision {
            self.0
        }
    }

    fn module() -> SemanticIrModule {
        compile_to_ir("qbit q0;", Some("test.qn")).unwrap()
    }

    fn receipt(module: &SemanticIrModule, obligation_index: usize) -> VerifiedObligationReceipt {
        let obligation = &module.proof_obligations[obligation_index];
        let claim = ObligationClaim::for_obligation(
            module,
            obligation,
            [0xA5; 32],
            "qneform-test-verifier/1",
        );
        VerifiedObligationReceipt::verify(
            claim,
            &TestVerifier(ObligationVerificationDecision::Verified),
        )
        .unwrap()
    }

    #[test]
    fn obligation_fingerprint_ignores_mutable_satisfied_status() {
        let first = module();
        let mut second = first.clone();
        second.proof_obligations[0].satisfied = true;
        assert_eq!(
            first.obligation_fingerprint(),
            second.obligation_fingerprint()
        );
        assert_ne!(first.fingerprint(), second.fingerprint());
    }

    #[test]
    fn obligation_fingerprint_changes_when_requirement_changes() {
        let first = module();
        let mut second = first.clone();
        second.proof_obligations[0].required = false;
        assert_ne!(
            first.obligation_fingerprint(),
            second.obligation_fingerprint()
        );
    }

    #[test]
    fn verified_receipt_satisfies_exact_required_obligation() {
        let ir = module();
        let receipt = receipt(&ir, 0);
        assert_eq!(validate_backend_readiness(&ir, &[receipt]), Ok(()));
    }

    #[test]
    fn mutable_satisfied_flag_cannot_bypass_missing_receipt() {
        let mut ir = module();
        ir.proof_obligations[0].satisfied = true;
        let error = validate_backend_readiness(&ir, &[]).unwrap_err();
        assert_eq!(error.missing_or_mismatched_obligations, vec![0]);
    }

    #[test]
    fn receipt_does_not_match_different_module_or_obligation() {
        let ir = module();
        let verified = receipt(&ir, 0);

        let other_module = compile_to_ir("qbit q1;", Some("test.qn")).unwrap();
        assert!(!verified.matches(&other_module, &other_module.proof_obligations[0]));

        let mut changed_obligation = ir.proof_obligations[0].clone();
        changed_obligation.description.push_str(" changed");
        assert!(!verified.matches(&ir, &changed_obligation));
    }

    #[test]
    fn rejected_inconclusive_and_unsupported_decisions_never_mint_receipts() {
        let ir = module();
        let claim = ObligationClaim::for_obligation(
            &ir,
            &ir.proof_obligations[0],
            [0xA5; 32],
            "qneform-test-verifier/1",
        );
        for (decision, expected) in [
            (
                ObligationVerificationDecision::Rejected,
                ObligationAttestationError::Rejected,
            ),
            (
                ObligationVerificationDecision::Inconclusive,
                ObligationAttestationError::Inconclusive,
            ),
            (
                ObligationVerificationDecision::Unsupported,
                ObligationAttestationError::Unsupported,
            ),
        ] {
            assert_eq!(
                VerifiedObligationReceipt::verify(claim.clone(), &TestVerifier(decision)),
                Err(expected)
            );
        }
    }

    #[test]
    fn verifier_identity_mismatch_and_unsupported_schema_fail_closed() {
        let ir = module();
        let mut claim = ObligationClaim::for_obligation(
            &ir,
            &ir.proof_obligations[0],
            [0xA5; 32],
            "different-verifier/1",
        );
        assert_eq!(
            VerifiedObligationReceipt::verify(
                claim.clone(),
                &TestVerifier(ObligationVerificationDecision::Verified)
            ),
            Err(ObligationAttestationError::EmptyOrMismatchedVerifierIdentity)
        );

        claim.verifier_id = "qneform-test-verifier/1".to_owned();
        claim.schema = "unsupported-schema".to_owned();
        assert_eq!(
            VerifiedObligationReceipt::verify(
                claim,
                &TestVerifier(ObligationVerificationDecision::Verified)
            ),
            Err(ObligationAttestationError::UnsupportedSchema)
        );
    }

    #[test]
    fn source_provenance_does_not_change_obligation_fingerprint() {
        let first = module();
        let mut second = first.clone();
        second.source_name = Some("another/location.qn".to_owned());
        second.proof_obligations[0].source.as_mut().unwrap().line += 5;
        assert_eq!(first.obligation_fingerprint(), second.obligation_fingerprint());
    }

    #[test]
    fn duplicate_obligation_ids_fail_closed() {
        let mut ir = module();
        ir.proof_obligations.push(ir.proof_obligations[0].clone());
        let error = validate_backend_readiness(&ir, &[]).unwrap_err();
        assert!(error.duplicate_obligation_ids.contains(&0));
    }

    #[test]
    fn optional_unresolved_obligations_need_no_receipt() {
        let mut ir = module();
        ir.proof_obligations[0].required = false;
        assert_eq!(validate_backend_readiness(&ir, &[]), Ok(()));
    }

    #[test]
    fn duplicate_receipts_fail_closed() {
        let ir = module();
        let verified = receipt(&ir, 0);
        let error = validate_backend_readiness(&ir, &[verified.clone(), verified]).unwrap_err();
        assert!(error.duplicate_receipt_ids.contains(&0));
    }

    #[test]
    fn receipt_for_another_module_is_unexpected_and_does_not_satisfy_obligation() {
        let ir = module();
        let other = compile_to_ir("qbit q1;", Some("test.qn")).unwrap();
        let foreign_receipt = receipt(&other, 0);
        let error = validate_backend_readiness(&ir, &[foreign_receipt]).unwrap_err();
        assert_eq!(error.missing_or_mismatched_obligations, vec![0]);
        assert_eq!(error.unexpected_receipt_ids, vec![0]);
    }
}
