# QCM-12 Authenticated Proof-Obligation Resolution v0.1

Status: architecture specification; implementation and backend authorization are not yet complete.

## Objective

Replace trust in mutable `ProofObligation.satisfied` flags with opaque verifier-issued receipts. A required obligation is resolved only when a trusted verifier has returned `Verified` for a claim bound to the exact canonical semantic module and exact obligation identity.

QCM-12 is an attestation protocol for static/module-scoped proof obligations. It does not establish physical quantum-state truth independently of the configured verifier.

## Existing boundary

QCM-11's `validate_backend_readiness()` is a diagnostic preflight. It reads public mutable IR status and is not an execution authorization token. A caller can mutate `satisfied`; therefore that field must never be accepted as authenticated proof.

QCM-08's resource attestation is a separate protocol: it binds a resource lifecycle proof to a ledger, allocation generation, resource kind and module fingerprint. QCM-12 must not overload that receipt or weaken its resource-specific checks.

## Proposed claim and receipt

Introduce a separately versioned obligation-attestation schema and opaque `VerifiedObligationReceipt`.

The verifier claim must bind, at minimum:

- attestation schema/version;
- canonical semantic module fingerprint;
- obligation ID;
- obligation kind;
- optional subject ResourceId;
- evidence digest;
- verifier identity.

The receipt's claim fields remain private. A receipt can be minted only by a verification function after schema, verifier identity and obligation binding checks, and only when the configured verifier returns `Verified`.

Decision handling is fail-closed:

- `Verified`: mint an opaque receipt;
- `Rejected`: reject the claim; no receipt;
- `Inconclusive`: no receipt;
- unsupported schema or proof profile: no receipt and an explicit unsupported error;
- verifier identity mismatch: no receipt.

A digest is an evidence identifier, not proof by itself. Verifier soundness, evidence acquisition, verifier key management and physical validity remain explicit trust assumptions.

## Stable fingerprint requirement

The current canonical IR encoding includes `ProofObligation.satisfied`. That status is mutable diagnostic metadata and must not be part of the semantic module fingerprint used by QCM-12 attestations.

QCM-12 implementation must revise the canonical encoding version (rather than silently changing v1 semantics) so that the attested fingerprint includes obligation identity and semantic requirements—ID, kind, subject, required flag and description—but excludes mutable satisfaction status. This prevents the act of recording a result from changing the module identity to which that result is bound.

Any future canonical encoding change requires an explicit version/tag and known-answer/regression tests. The legacy fingerprint version must remain distinguishable.

## Readiness API

Add a receipt-based readiness validator, conceptually:

`validate_backend_readiness_with_receipts(&[VerifiedObligationReceipt])`

It must:

1. Recompute the module fingerprint from the current IR.
2. Enumerate every required obligation in deterministic IR order.
3. Require exactly one matching verified receipt for each required obligation.
4. Reject missing, duplicate, mismatched, stale-module, wrong-kind, wrong-subject or unsupported receipts.
5. Ignore `satisfied` as an authorization signal.
6. Reject extraneous receipts unless a documented policy explicitly permits them.
7. Return all blocking issues deterministically; never partially authorize a module.

Optional obligations do not require receipts for readiness. A module with no required obligations can pass this static obligation gate, but this is not a general backend authorization.

## Separation of concerns

- QCM-11 status preflight remains diagnostic and must be clearly named/documented as non-authoritative.
- QCM-12 receipts attest a verifier's decision for a module obligation.
- QCM-08/09 resource receipts continue to govern resource release and remain bound to ledger identity, resource handle, resource kind, generation and module fingerprint.
- Backend-specific conformance and runtime/dynamic obligations require separate profiles and evidence.
- A passing static obligation gate does not prove backend conformance, runtime safety, hardware execution, or physical quantum-state truth.

## Required tests

- only the configured verifier can mint receipts;
- Verified mints; Rejected, Inconclusive and unsupported profiles never mint;
- verifier identity mismatch fails;
- receipt matches exact module fingerprint, obligation ID, kind and subject;
- source path/span changes do not change semantic identity;
- changing semantic IR does change fingerprint;
- changing only mutable satisfaction status does not change the QCM-12 fingerprint;
- missing, duplicate, stale, wrong-kind and wrong-subject receipts block;
- every required obligation is checked and all issues are reported deterministically;
- optional unresolved obligations do not block;
- receipt from one module cannot authorize another;
- QCM-08/09 resource-release receipt checks remain unchanged.

## Deferred

- Backend execution authorization and backend integration.
- Runtime/dynamic proof-obligation evidence.
- Durable verifier identity/key registry, revocation and rotation.
- Durable receipt storage, replay policy and restart-safe ledger identity.
- Concrete proof profiles for each obligation kind.
- Simulator, OpenQASM 3 export and physical hardware execution.

QCM-12 is not complete until the receipt type, canonical fingerprint versioning, readiness API and tests are implemented and CI passes. No backend execution is introduced by this specification.
