# QCM-09 Ledger-Enforced Typed Attestations v0.1

Status: design contract; implementation not yet complete.

## Objective

Make a verifier-issued `VerifiedAttestation` a mandatory input to resource release. The resource ledger, not the caller, is authoritative for the live resource identity, kind, allocation generation, outstanding borrows, and lifecycle state.

This increment is a design gate. It does not claim that the implementation already enforces these rules.

## Required release contract

A release operation must receive:

- the live `ResourceHandle`;
- the exact module fingerprint produced by the canonical IR pipeline;
- an opaque `VerifiedAttestation` minted by a configured `ProofVerifier`.

The ledger must compare the receipt against its own state. The caller must not choose the authoritative resource kind or generation at release time.

| Resource kind | Required proof kind |
|---|---|
| Qubit | Separable |
| CleanAncilla | CleanAndSeparable |
| DirtyAncilla | DirtyRestoredAndSeparable |

Reject release if any of the following holds:

- receipt absent;
- module fingerprint mismatch;
- resource handle, kind, or allocation generation mismatch;
- proof kind incompatible with the ledger's resource kind;
- resource is not owned or has outstanding borrows;
- resource is already released;
- verifier decision was Rejected or Inconclusive;
- proof's evidence digest is absent/invalid under the eventual evidence schema.

Only after all checks pass may the ledger transition the resource to Released.

## Identity and replay requirements

Resource handles are currently module/ledger-local counters. They are not globally unique identifiers. Before claiming cross-ledger replay resistance, the design must bind an explicit ledger identity into both the proof claim and the ledger record.

Allocation generation must be ledger-issued and monotonic within the ledger. A receipt from an earlier allocation generation must not authorize a later allocation, even if a display name is reused.

The module fingerprint must be derived from a canonical, versioned IR encoding. Caller-supplied 32-byte values are placeholders until canonical encoding and SHA-256 computation are implemented and tested.

## API migration

- Replace boolean `attest_cleanliness(handle, true)` and `attest_dirty_restored(handle, true)` with typed verifier receipts.
- Remove any release overload that bypasses receipt validation.
- Keep the ledger's conservative entanglement/lifecycle checks as independent invariants; a receipt must not silently override outstanding borrows or invalid lifecycle state.
- Keep `discard` separate from `release`. Discard is a non-unitary trace-out operation and requires its own explicit effect and backend semantics; it is not part of this increment.

## Backend enforcement

No execution-capable backend may execute a module with unresolved required proof obligations. The gate must distinguish:
- static proof accepted;
- backend/runtime proof accepted;
- rejected;
- inconclusive;
- unsupported proof profile.

A source-level or IR-level declaration is not itself a verified proof.

## Test matrix

Required negative tests include:
1. no receipt;
2. rejected verifier;
3. inconclusive verifier;
4. wrong module fingerprint;
5. wrong ledger identity;
6. wrong resource handle;
7. stale allocation generation;
8. wrong resource kind;
9. wrong proof kind;
10. outstanding borrow;
11. double release;
12. receipt replay after release;
13. receipt replay across two ledger instances;
14. verifier identity mismatch.

Positive tests must cover each of the three resource kinds with a matching receipt and no outstanding borrows.

## Trust boundary

A typed receipt proves that the configured verifier returned Verified for a specific claim. It does not prove that the verifier is correct, that evidence is physically sound, or that a simulator corresponds to hardware. Assurance profiles and verifier implementations must be reviewed separately.

## Acceptance criteria

- All boolean attestation bypasses are removed.
- Ledger release is receipt-gated and checks its own state.
- Ledger identity and allocation generation are bound into claims.
- Canonical IR fingerprinting is deterministic and versioned.
- Required proof obligations are enforced before backend execution.
- Positive and negative test matrix passes.
- CI runs for the actual QCM branch and pull-request base.


## Implementation progress (2026-11-11)

Implemented on the QCM-09 branch:
- `LedgerId(u128)` namespace type.
- `ResourceLedger::new_with_id` for explicit identity restoration.
- `ResourceLedger::ledger_id` accessor.
- Ephemeral `ResourceLedger::new` identities generated from a process-local atomic counter.
- `ProofClaim` now includes `ledger_id`.
- `VerifiedAttestation::matches` checks ledger identity in addition to module fingerprint, resource handle, resource kind, allocation generation, and proof kind.
- Unit tests cover explicit identity restoration, distinct ephemeral identities, and cross-ledger receipt mismatch.

Not yet implemented:
- Durable persistence of ledger identity and allocation-generation counters.
- Ledger-owned receipt validation in `release`.
- Removal of boolean cleanliness/restoration attestation methods.
- Canonical IR fingerprint generation.
- Backend proof-obligation gate.

The process-local atomic identity is a convenience namespace only. It is not a cryptographic identifier, is not stable across process restarts, and must not be treated as a durable replay-prevention mechanism.


## Receipt-gated release implementation (2026-11-11)

Implemented in `src/resource.rs`:
- `ResourceState` records an allocation generation assigned from the ledger's monotonically increasing resource handle counter.
- `ResourceLedger::release` now requires both the expected module fingerprint and an opaque `VerifiedAttestation`.
- The ledger checks its own ledger identity, resource handle, resource kind, allocation generation, module fingerprint, and required proof kind before changing lifecycle to Released.
- Required proof mapping is enforced by the ledger: Qubit → Separable; CleanAncilla → CleanAndSeparable; DirtyAncilla → DirtyRestoredAndSeparable.
- The legacy public boolean methods `attest_cleanliness(handle, bool)` and `attest_dirty_restored(handle, bool)` have been removed.
- Existing fail-closed checks for ownership, outstanding borrows, and known separability remain independent of the receipt.
- Tests cover missing/incorrect module binding, cross-ledger receipt replay, borrow blocking, proof-kind compatibility, clean/dirty resource paths, and double release.

Known limitations remain:
- The module fingerprint is still caller-supplied; canonical versioned IR encoding and hashing are not implemented.
- The generation counter is in-memory. Durable storage must persist and restore the ledger identity, counter, resources and release state atomically to resist restart/replay.
- The configured verifier is a trust boundary; the receipt proves only that it returned Verified for the bound claim.
- No `discard` operation is introduced.


## QCM-10 foundation: canonical semantic IR encoding (2026-11-11)

Implemented in `src/semantic_ir.rs`:
- `SemanticIrModule::canonical_bytes()` emits an explicit, versioned binary encoding with a domain-separation prefix.
- Integer fields use fixed-width big-endian encoding; strings use UTF-8 with 64-bit byte-length prefixes.
- Enum variants have explicit stable numeric tags.
- Ordered declaration, operation, effect and proof-obligation sequences are preserved.
- Source paths and source spans are excluded from semantic identity; they remain provenance metadata.
- Tests assert deterministic encoding, provenance independence, and changed bytes when a literal changes.

This is the canonical-encoding foundation only. It does **not** yet compute a cryptographic digest. A reviewed SHA-256 implementation/dependency and lockfile update are required before this encoding can replace caller-supplied module fingerprints in proof claims and ledger release. Do not use a non-cryptographic standard-library hasher as a substitute.
