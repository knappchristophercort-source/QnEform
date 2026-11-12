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
