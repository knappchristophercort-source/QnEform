# QCM-03 Resource Proof Hardening v0.2

Status: implementation increment in progress. This document defines the next safety boundary; it does not claim a quantum backend exists.

## 1. Implemented in this increment

- Borrow aliases are scoped to the lexical block in which they are created.
- A borrow must be returned in that same scope.
- A borrow that reaches the end of its creation scope is rejected.
- A borrow cannot be returned from a parent or child scope.
- The analyzer remains fail-closed: any such error prevents a semantic module from being returned.

These are compiler-level lifetime checks. They do not prove anything about a physical quantum state.

## 2. Typed proof attestations — required design

The current `ResourceLedger` accepts boolean values in `attest_cleanliness` and `attest_dirty_restored`. Those booleans are assertions from the caller, not evidence. They must not be described as cryptographic, independently verified, or provenance-bound proofs.

Replace the boolean boundary in a later implementation with an opaque attestation value whose fields cannot be freely constructed by downstream callers. The minimum conceptual record is:

- schema/version identifier;
- module identity and canonical module fingerprint;
- resource identity and resource kind;
- proof kind (clean-and-separable, or dirty-state-and-correlation restoration);
- verifier identity and verifier implementation/version;
- evidence digest and evidence format/version;
- verification result;
- verification time or execution epoch, where the backend can provide a trustworthy source;
- scope/lineage binding sufficient to reject replay against another resource, module, or execution.

An attestation is acceptable only when its bindings match the exact resource and module being checked. A valid signature or digest authenticates a statement; it does not establish that the underlying quantum claim is physically true. That requires a trusted analysis or backend verification procedure with a declared assurance level.

## 3. Proof acceptance contract

A future verifier boundary should return a typed result, not a caller-supplied boolean:

- `Verified(attestation)`
- `Rejected(reason)`
- `Inconclusive(reason)`

Only `Verified` may satisfy a required proof obligation. `Rejected` and `Inconclusive` both block execution and release. The distinction remains visible for diagnostics and audit.

The verifier must check, at minimum:

1. the attestation schema is supported;
2. module fingerprint and module identity match;
3. resource ID, kind, and lifecycle generation match;
4. proof kind is appropriate for the resource kind;
5. evidence digest matches the supplied evidence;
6. verifier identity and assurance policy are accepted;
7. the attestation has not been replayed for another resource or execution;
8. all required proof components are present.

Do not add a public constructor that lets arbitrary callers mint a `Verified` attestation. Keep construction behind the verifier boundary. Any test-only constructor must be gated to tests.

## 4. Release and discard are different operations

- `release` means deallocation under the language's safe-release contract. It requires ownership, no outstanding borrows, and evidence that the resource is separable from the retained system. Clean ancilla additionally requires proof of the required clean state. Dirty ancilla additionally requires restoration of its arbitrary input state and correlations.
- `discard` is an explicit irreversible operation corresponding to tracing out / relinquishing a subsystem. It must carry a non-unitary effect and explicit semantics for the retained subsystem. It is not a weaker spelling of `release`, and it is not implemented by this increment.

Unknown or possibly-entangled state must never be silently treated as separable.

## 5. Backend execution gate

The compiler may produce an analysis artifact containing unresolved proof obligations. No execution-capable backend may consume it unless every required obligation is satisfied by an accepted attestation bound to that exact module and resource generation.

A backend must fail closed on:
- missing or mismatched attestations;
- unsupported proof schema;
- stale/replayed evidence;
- unresolved required proof obligations;
- unknown or possibly-entangled resources at safe release;
- ownership/lifecycle mismatch;
- outstanding borrows.

The current project has no physical allocator or execution backend. This gate is a contract for future backends, not a claim that runtime enforcement is already implemented.

## 6. Required conformance tests before declaring this complete

- alias escape at block exit is rejected;
- return from a different lexical scope is rejected;
- return in the creation scope is accepted;
- forged, missing, stale, replayed, or resource-mismatched attestations are rejected;
- module-fingerprint mismatch is rejected;
- verifier rejection and inconclusive results block release;
- clean ancilla requires clean-state and separability evidence;
- dirty ancilla requires restoration of arbitrary input state and correlations;
- unknown/possibly-entangled resource blocks safe release;
- discard is separately typed and marked non-unitary before it is implemented;
- backend refuses any unresolved required proof obligation.

## 7. Current limitations

This increment implements lexical borrow-scope enforcement and regression tests only. Typed proof attestations, canonical module fingerprints, verifier assurance policy, discard semantics, and backend enforcement remain future work. The existing boolean attestation methods remain provisional and must not be used as a security boundary.
