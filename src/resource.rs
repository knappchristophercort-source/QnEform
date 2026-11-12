//! Fail-closed quantum resource lifecycle state machine.
//!
//! This module models compiler/runtime bookkeeping only. It does not allocate
//! physical qubits or inspect quantum states. Cleanliness must be supplied as
//! a proof result by a trusted analysis/backend; it is never inferred from a
//! measurement result.

use std::collections::HashMap;
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

    /// Record a proof result from static analysis or a backend-specific
    /// verification procedure. This method does not itself prove cleanliness.
    pub fn attest_cleanliness(
        &mut self,
        handle: ResourceHandle,
        verified: bool,
    ) -> Result<(), ResourceError> {
        let state = self.state_mut(handle)?;
        if !matches!(state.kind, ResourceKind::CleanAncilla) {
            return Err(ResourceError::CleanlinessProofRequired(state.name.clone()));
        }
        if !verified {
            state.cleanliness = Cleanliness::Unverified;
            return Err(ResourceError::CleanlinessProofRequired(state.name.clone()));
        }
        if state.entanglement != Entanglement::KnownSeparable {
            return Err(ResourceError::ReleaseRequiresSeparable(state.name.clone()));
        }
        state.cleanliness = Cleanliness::VerifiedClean;
        state.lifecycle = Lifecycle::VerifiedClean;
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

    /// Dirty ancilla must be proven restored to its arbitrary borrowed input
    /// state and correlations before it can be released.
    pub fn attest_dirty_restored(
        &mut self,
        handle: ResourceHandle,
        verified: bool,
    ) -> Result<(), ResourceError> {
        let state = self.state_mut(handle)?;
        if state.kind != ResourceKind::DirtyAncilla {
            return Err(ResourceError::DirtyAncillaRestorationProofRequired(
                state.name.clone(),
            ));
        }
        if !verified {
            return Err(ResourceError::DirtyAncillaRestorationProofRequired(
                state.name.clone(),
            ));
        }
        state.cleanliness = Cleanliness::VerifiedRestored;
        state.lifecycle = Lifecycle::Uncomputed;
        Ok(())
    }

    pub fn release(&mut self, handle: ResourceHandle) -> Result<(), ResourceError> {
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
        if !matches!(state.entanglement, Entanglement::KnownSeparable) {
            return Err(ResourceError::ReleaseRequiresSeparable(state.name));
        }
        match state.kind {
            ResourceKind::CleanAncilla if state.cleanliness != Cleanliness::VerifiedClean => {
                return Err(ResourceError::ReleaseRequiresCleanAncilla(state.name))
            }
            ResourceKind::DirtyAncilla if state.cleanliness != Cleanliness::VerifiedRestored => {
                return Err(ResourceError::DirtyAncillaRestorationProofRequired(
                    state.name,
                ))
            }
            _ => {}
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

    fn separable(ledger: &mut ResourceLedger, handle: ResourceHandle) {
        ledger
            .set_entanglement(handle, Entanglement::KnownSeparable)
            .unwrap();
    }

    #[test]
    fn allocates_owned_resource_without_claiming_cleanliness() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        let state = ledger.state(q).unwrap();
        assert_eq!(state.ownership, Ownership::Owned);
        assert_eq!(state.lifecycle, Lifecycle::Allocated);
        assert_eq!(state.entanglement, Entanglement::Unknown);
    }

    #[test]
    fn rejects_duplicate_live_names() {
        let mut ledger = ResourceLedger::new();
        ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        assert_eq!(
            ledger.allocate("q0", ResourceKind::Qubit),
            Err(ResourceError::DuplicateName("q0".into()))
        );
    }

    #[test]
    fn borrow_must_be_returned_before_release() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        let borrow = ledger.borrow(q).unwrap();
        assert_eq!(
            ledger.release(q),
            Err(ResourceError::OutstandingBorrows("q0".into()))
        );
        ledger.return_borrow(borrow).unwrap();
        ledger.release(q).unwrap();
    }

    #[test]
    fn rejects_double_return_and_unknown_borrow() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        let borrow = ledger.borrow(q).unwrap();
        ledger.return_borrow(borrow).unwrap();
        assert_eq!(
            ledger.return_borrow(borrow),
            Err(ResourceError::UnknownBorrow(borrow))
        );
    }

    #[test]
    fn rejects_release_when_entanglement_is_unknown_or_possible() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        assert_eq!(
            ledger.release(q),
            Err(ResourceError::ReleaseRequiresSeparable("q0".into()))
        );
        ledger
            .set_entanglement(q, Entanglement::PossiblyEntangled)
            .unwrap();
        assert_eq!(
            ledger.release(q),
            Err(ResourceError::ReleaseRequiresSeparable("q0".into()))
        );
    }

    #[test]
    fn clean_ancilla_requires_separability_and_explicit_cleanliness_attestation() {
        let mut ledger = ResourceLedger::new();
        let a = ledger.allocate("work", ResourceKind::CleanAncilla).unwrap();
        separable(&mut ledger, a);
        assert_eq!(
            ledger.release(a),
            Err(ResourceError::ReleaseRequiresCleanAncilla("work".into()))
        );
        ledger.attest_cleanliness(a, true).unwrap();
        ledger.release(a).unwrap();
    }

    #[test]
    fn dirty_ancilla_requires_restoration_attestation() {
        let mut ledger = ResourceLedger::new();
        let a = ledger
            .allocate("scratch", ResourceKind::DirtyAncilla)
            .unwrap();
        separable(&mut ledger, a);
        assert_eq!(
            ledger.release(a),
            Err(ResourceError::DirtyAncillaRestorationProofRequired(
                "scratch".into()
            ))
        );
        ledger.attest_dirty_restored(a, true).unwrap();
        ledger.release(a).unwrap();
    }

    #[test]
    fn cannot_release_entangled_clean_ancilla_even_with_clean_attestation() {
        let mut ledger = ResourceLedger::new();
        let a = ledger.allocate("work", ResourceKind::CleanAncilla).unwrap();
        ledger
            .set_entanglement(a, Entanglement::KnownEntangled)
            .unwrap();
        assert_eq!(
            ledger.attest_cleanliness(a, true),
            Err(ResourceError::ReleaseRequiresSeparable("work".into()))
        );
    }

    #[test]
    fn rejects_use_after_release_and_double_release() {
        let mut ledger = ResourceLedger::new();
        let q = ledger.allocate("q0", ResourceKind::Qubit).unwrap();
        separable(&mut ledger, q);
        ledger.release(q).unwrap();
        assert_eq!(
            ledger.mark_active(q),
            Err(ResourceError::InvalidTransition {
                name: "q0".into(),
                from: Lifecycle::Released,
                action: "mark_active",
            })
        );
        assert_eq!(
            ledger.release(q),
            Err(ResourceError::AlreadyReleased("q0".into()))
        );
    }
}
