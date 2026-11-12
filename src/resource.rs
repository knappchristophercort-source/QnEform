//! Fail-closed quantum resource lifecycle state machine.
//!
//! This module models compiler/runtime bookkeeping only. It does not allocate
//! physical qubits or inspect quantum states. Cleanliness must be supplied as
//! a proof result by a trusted analysis/backend; it is never inferred from a
//! measurement result.

use std::collections::HashMap;

use crate::proof::{ProofKind, VerifiedAttestation};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_EPHEMERAL_LEDGER_ID: AtomicU64 = AtomicU64::new(1);

/// Stable ledger namespace. Persist this value with a durable ledger; the
/// default constructor creates an ephemeral process-local identity only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LedgerId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Qubit,
    CleanAncilla,
    DirtyAncilla,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Owned,
    Borrowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Allocated,
    Active,
    Uncomputed,
    VerifiedClean,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entanglement {
    KnownSeparable,
    PossiblyEntangled,
    KnownEntangled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cleanliness {
    NotApplicable,
    Unverified,
    VerifiedClean,
    VerifiedRestored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceState {
    pub handle: ResourceHandle,
    /// Monotonic allocation generation within this ledger namespace.
    pub generation: u64,
    pub name: String,
    pub kind: ResourceKind,
    pub ownership: Ownership,
    pub lifecycle: Lifecycle,
    pub entanglement: Entanglement,
    pub cleanliness: Cleanliness,
    pub active_borrows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceError {
    DuplicateName(String),
    UnknownResource(ResourceHandle),
    UnknownBorrow(BorrowHandle),
    AlreadyReleased(String),
    BorrowRequiresOwned(String),
    BorrowAlreadyActive(String),
    BorrowedResourceCannotBeReleased(String),
    OutstandingBorrows(String),
    ReleaseRequiresOwned(String),
    ReleaseRequiresSeparable(String),
    ReleaseRequiresCleanAncilla(String),
    InvalidTransition {
        name: String,
        from: Lifecycle,
        action: &'static str,
    },
    CleanlinessProofRequired(String),
    DirtyAncillaRestorationProofRequired(String),
    MissingOrMismatchedAttestation,
}

impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ResourceError {}

#[derive(Debug, Clone)]
struct BorrowRecord {
    resource: ResourceHandle,
}

#[derive(Debug)]
pub struct ResourceLedger {
    ledger_id: LedgerId,
    next_resource: u64,
    next_borrow: u64,
    by_name: HashMap<String, ResourceHandle>,
    resources: HashMap<ResourceHandle, ResourceState>,
    borrows: HashMap<BorrowHandle, BorrowRecord>,
}

impl Default for ResourceLedger {
    fn default() -> Self {
        let nonce = NEXT_EPHEMERAL_LEDGER_ID.fetch_add(1, Ordering::Relaxed);
        Self::new_with_id(LedgerId(nonce as u128))
    }
}

impl ResourceLedger {
    /// Create a ledger with an explicitly supplied identity. Durable stores
    /// must persist and restore this identity; do not reuse it for a new ledger.
    pub fn new_with_id(ledger_id: LedgerId) -> Self {
        Self {
            ledger_id,
            next_resource: 0,
            next_borrow: 0,
            by_name: HashMap::new(),
            resources: HashMap::new(),
            borrows: HashMap::new(),
        }
    }

    /// Create an ephemeral ledger with a process-local identity. This identity
    /// is not restart-stable and must not be used as a durable replay boundary.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ledger_id(&self) -> LedgerId {
        self.ledger_id
    }

    pub fn allocate(
        &mut self,
        name: impl Into<String>,
        kind: ResourceKind,
    ) -> Result<ResourceHandle, ResourceError> {
        let name = name.into();
        if self.by_name.contains_key(&name) {
            return Err(ResourceError::DuplicateName(name));
        }
        let handle = ResourceHandle(self.next_resource);
        self.next_resource += 1;
        let state = ResourceState {
            handle,
            generation: handle.0,
            name: name.clone(),
            kind,
            ownership: Ownership::Owned,
            lifecycle: Lifecycle::Allocated,
            entanglement: Entanglement::Unknown,
            cleanliness: match kind {
                ResourceKind::CleanAncilla => Cleanliness::Unverified,
                ResourceKind::DirtyAncilla => Cleanliness::Unverified,
                ResourceKind::Qubit => Cleanliness::NotApplicable,
            },
            active_borrows: 0,
        };
        self.by_name.insert(name, handle);
        self.resources.insert(handle, state);
        Ok(handle)
    }

    pub fn state(&self, handle: ResourceHandle) -> Result<&ResourceState, ResourceError> {
        self.resources
            .get(&handle)
            .ok_or(ResourceError::UnknownResource(handle))
    }

    fn state_mut(&mut self, handle: ResourceHandle) -> Result<&mut ResourceState, ResourceError> {
        self.resources
            .get_mut(&handle)
            .ok_or(ResourceError::UnknownResource(handle))
    }

    pub fn mark_active(&mut self, handle: ResourceHandle) -> Result<(), ResourceError> {
        let state = self.state_mut(handle)?;
        if state.lifecycle != Lifecycle::Allocated {
            return Err(ResourceError::InvalidTransition {
                name: state.name.clone(),
                from: state.lifecycle,
                action: "mark_active",
            });
        }
        state.lifecycle = Lifecycle::Active;
        Ok(())
    }

    /// Mark conservative entanglement knowledge. Unknown is not separable.
    pub fn set_entanglement(
        &mut self,
        handle: ResourceHandle,
        status: Entanglement,
    ) -> Result<(), ResourceError> {
        self.state_mut(handle)?.entanglement = status;
        Ok(())
    }

    /// Borrow an owned, live resource. Borrow handles are linear: each must be
    /// returned exactly once before the owner may release the resource.
    pub fn borrow(&mut self, handle: ResourceHandle) -> Result<BorrowHandle, ResourceError> {
        let state = self.state_mut(handle)?;
        if state.lifecycle == Lifecycle::Released {
            return Err(ResourceError::AlreadyReleased(state.name.clone()));
        }
        if state.ownership != Ownership::Owned {
            return Err(ResourceError::BorrowRequiresOwned(state.name.clone()));
        }
        state.active_borrows += 1;
        let borrow = BorrowHandle(self.next_borrow);
        self.next_borrow += 1;
        self.borrows
            .insert(borrow, BorrowRecord { resource: handle });
        Ok(borrow)
    }

    pub fn return_borrow(&mut self, borrow: BorrowHandle) -> Result<(), ResourceError> {
        let record = self
            .borrows
            .remove(&borrow)
            .ok_or(ResourceError::UnknownBorrow(borrow))?;
        let state = self.state_mut(record.resource)?;
        state.active_borrows = state
            .active_borrows
            .checked_sub(1)
            .expect("borrow ledger and resource count must remain consistent");
        Ok(())
    }

    /// Release only with a verifier-issued receipt bound to this ledger,
    /// exact resource allocation, resource kind, generation and module.
    /// The fingerprint is supplied by the caller pending canonical IR hashing.
    pub fn release(
        &mut self,
        handle: ResourceHandle,
        module_fingerprint: &[u8; 32],
        attestation: &VerifiedAttestation,
    ) -> Result<(), ResourceError> {
        let state = self.state(handle)?.clone();
        if state.lifecycle == Lifecycle::Released {
            return Err(ResourceError::AlreadyReleased(state.name));
        }
        if state.ownership != Ownership::Owned {
            return Err(ResourceError::ReleaseRequiresOwned(state.name));
        }
        if state.active_borrows != 0 {
            return Err(ResourceError::OutstandingBorrows(state.name));
        }
        if state.entanglement != Entanglement::KnownSeparable {
            return Err(ResourceError::ReleaseRequiresSeparable(state.name));
        }
        let required_proof = match state.kind {
            ResourceKind::Qubit => ProofKind::Separable,
            ResourceKind::CleanAncilla => ProofKind::CleanAndSeparable,
            ResourceKind::DirtyAncilla => ProofKind::DirtyRestoredAndSeparable,
        };
        if !attestation.matches(
            module_fingerprint,
            self.ledger_id,
            handle,
            state.kind,
            state.generation,
            required_proof,
        ) {
            return Err(ResourceError::MissingOrMismatchedAttestation);
        }
        let released_name = {
            let state = self.state_mut(handle)?;
            state.lifecycle = Lifecycle::Released;
            state.name.clone()
        };
        self.by_name.remove(&released_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{ProofClaim, ProofVerifier, VerificationDecision, ATTESTATION_SCHEMA_V1};

    const MODULE: [u8; 32] = [7; 32];

    struct TestVerifier;
    impl ProofVerifier for TestVerifier {
        fn verifier_id(&self) -> &str { "resource-ledger-test/1" }
        fn verify(&self, _claim: &ProofClaim) -> VerificationDecision {
            VerificationDecision::Verified
        }
    }

    fn receipt(ledger: &ResourceLedger, handle: ResourceHandle) -> VerifiedAttestation {
        let state = ledger.state(handle).unwrap();
        let proof_kind = match state.kind {
            ResourceKind::Qubit => ProofKind::Separable,
            ResourceKind::CleanAncilla => ProofKind::CleanAndSeparable,
            ResourceKind::DirtyAncilla => ProofKind::DirtyRestoredAndSeparable,
        };
        VerifiedAttestation::verify(ProofClaim {
            schema: ATTESTATION_SCHEMA_V1.to_owned(),
            module_fingerprint: MODULE,
            ledger_id: ledger.ledger_id(),
            resource: handle,
            resource_kind: state.kind,
            resource_generation: state.generation,
            proof_kind,
            evidence_digest: [9; 32],
            verifier_id: "resource-ledger-test/1".to_owned(),
        }, &TestVerifier).unwrap()
    }

    fn separable(ledger: &mut ResourceLedger, handle: ResourceHandle) {
        ledger.set_entanglement(handle, Entanglement::KnownSeparable).unwrap();
    }

    #[test]
    fn explicit_ledger_identity_is_preserved_for_restore() {
        let ledger = ResourceLedger::new_with_id(LedgerId(0xA11CE));
        let restored = ResourceLedger::new_with_id(ledger.ledger_id());
        assert_eq!(restored.ledger_id(), ledger.ledger_id());
    }

    #[test]
    fn ephemeral_ledgers_receive_distinct_process_local_identities() {
        assert_ne!(ResourceLedger::new().ledger_id(), ResourceLedger::new().ledger_id());
    }

    #[test]
    fn allocates_owned_resource_without_claiming_cleanliness() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        let state = ledger.state(q).unwrap();
        assert_eq!(state.ownership, Ownership::Owned);
        assert_eq!(state.lifecycle, Lifecycle::Allocated);
        assert_eq!(state.entanglement, Entanglement::Unknown);
        assert_eq!(state.generation, q.0);
    }

    #[test]
    fn rejects_duplicate_live_names() {
        let mut ledger = ResourceLedger::new();
        ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        assert_eq!(ledger.allocate("q0", ResourceKind::Qubit), Err(ResourceError::DuplicateName("q0".into())));
    }

    #[test]
    fn release_requires_matching_receipt_and_rejects_cross_ledger_replay() {
        let mut first = ResourceLedger::new();
        let q = first.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut first, q);
        let proof = receipt(&first, q);
        let mut second = ResourceLedger::new();
        let q2 = second.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut second, q2);
        assert_eq!(second.release(q2, &MODULE, &proof), Err(ResourceError::MissingOrMismatchedAttestation));
        first.release(q, &MODULE, &proof).unwrap();
    }

    #[test]
    fn release_rejects_wrong_module_fingerprint() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        let proof = receipt(&ledger, q);
        assert_eq!(ledger.release(q, &[8; 32], &proof), Err(ResourceError::MissingOrMismatchedAttestation));
    }

    #[test]
    fn borrow_must_be_returned_before_receipt_gated_release() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        let proof = receipt(&ledger, q);
        let borrow = ledger.borrow(q).unwrap();
        assert_eq!(ledger.release(q, &MODULE, &proof), Err(ResourceError::OutstandingBorrows("q0".into())));
        ledger.return_borrow(borrow).unwrap();
        ledger.release(q, &MODULE, &proof).unwrap();
    }

    #[test]
    fn rejects_double_return_and_unknown_borrow() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        let borrow = ledger.borrow(q).unwrap();
        ledger.return_borrow(borrow).unwrap();
        assert_eq!(ledger.return_borrow(borrow), Err(ResourceError::UnknownBorrow(borrow)));
    }

    #[test]
    fn rejects_release_when_entanglement_is_unknown_or_possible() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        let proof = receipt(&ledger, q);
        assert_eq!(ledger.release(q, &MODULE, &proof), Err(ResourceError::ReleaseRequiresSeparable("q0".into())));
        ledger.set_entanglement(q, Entanglement::PossiblyEntangled).unwrap();
        assert_eq!(ledger.release(q, &MODULE, &proof), Err(ResourceError::ReleaseRequiresSeparable("q0".into())));
    }

    #[test]
    fn clean_ancilla_requires_clean_and_separable_receipt() {
        let mut ledger = ResourceLedger::new();
        let a = ledger.allocate("work", ResourceKind::CleanAncilla).unwrap();
        separable(&mut ledger, a);
        let proof = receipt(&ledger, a);
        ledger.release(a, &MODULE, &proof).unwrap();
    }

    #[test]
    fn dirty_ancilla_requires_restored_and_separable_receipt() {
        let mut ledger = ResourceLedger::new();
        let a = ledger.allocate("scratch", ResourceKind::DirtyAncilla).unwrap();
        separable(&mut ledger, a);
        let proof = receipt(&ledger, a);
        ledger.release(a, &MODULE, &proof).unwrap();
    }

    #[test]
    fn wrong_proof_kind_cannot_release_resource() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        let mut claim = ProofClaim {
            schema: ATTESTATION_SCHEMA_V1.to_owned(),
            module_fingerprint: MODULE,
            ledger_id: ledger.ledger_id(),
            resource: q,
            resource_kind: ResourceKind::Qubit,
            resource_generation: ledger.state(q).unwrap().generation,
            proof_kind: ProofKind::CleanAndSeparable,
            evidence_digest: [9; 32],
            verifier_id: "resource-ledger-test/1".to_owned(),
        };
        // The verifier constructor itself rejects the incompatible claim.
        assert!(VerifiedAttestation::verify(claim.clone(), &TestVerifier).is_err());
        claim.proof_kind = ProofKind::Separable;
        let proof = VerifiedAttestation::verify(claim, &TestVerifier).unwrap();
        ledger.release(q, &MODULE, &proof).unwrap();
    }

    #[test]
    fn rejects_use_after_release_and_double_release() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        let proof = receipt(&ledger, q);
        ledger.release(q, &MODULE, &proof).unwrap();
        assert_eq!(ledger.mark_active(q), Err(ResourceError::InvalidTransition {
            name: "q0".into(), from: Lifecycle::Released, action: "mark_active",
        }));
        assert_eq!(ledger.release(q, &MODULE, &proof), Err(ResourceError::AlreadyReleased("q0".into())));
    }
}
