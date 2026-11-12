# QCM-08 Typed Proof Attestation Boundary v0.1

Status: initial typed verifier boundary implemented; ledger/release integration is not yet complete.

## Implemented

The new `src/proof.rs` introduces:

- versioned attestation schema identifier;
- typed proof kinds for separability, clean-and-separable, and dirty-restored-and-separable claims;
- explicit verifier outcomes: Verified, Rejected, Inconclusive;
- claim bindings for module fingerprint, resource handle, resource kind, resource generation, proof kind, evidence digest, and verifier identity;
- opaque `VerifiedAttestation` receipts with private fields;
- verifier-mediated receipt construction;
- exact binding comparison through `matches`;
- tests proving that mismatched module fingerprints, resource handles, and generations do not match, and that rejected/inconclusive outcomes cannot mint receipts.

## Trust boundary

`ProofVerifier` is an explicit trusted boundary. Its implementation is responsible for evaluating evidence and returning a decision. The type system prevents ordinary downstream code from directly constructing a successful receipt, but it cannot prevent a malicious or defective verifier implementation from lying.

The evidence digest and module fingerprint are supplied as 32-byte values. This increment does not implement canonical module serialization, SHA-256 hashing, signature verification, verifier attestation, trusted execution, or physical quantum-state inspection. Do not interpret these fields as cryptographic proof until canonical hashing and evidence verification are implemented and independently tested.

## Not yet enforced

This increment does not yet connect receipts to `ResourceLedger::release`, does not remove the legacy boolean cleanliness/restoration methods, and does not provide an execution backend gate. Therefore it must not be described as complete release enforcement or a production security boundary.

## Next required steps

1. Derive module fingerprints from a specified canonical IR encoding.
2. Bind resource generations to ledger-issued lifecycle generations rather than caller-selected integers.
3. Store and validate typed receipts inside the ledger at release time.
4. Remove/deprecate boolean attestation entry points once typed migration is complete.
5. Add replay, stale-generation, cross-module, wrong-resource, and wrong-proof-kind tests at the ledger boundary.
6. Make all execution-capable backends reject unresolved required proof obligations.
7. Define verifier assurance profiles and document which claims can be established by static analysis versus a concrete backend.

## Safety rule

A cryptographically authentic statement is not necessarily a true statement about a quantum state. The verifier's method, evidence, assumptions, and assurance level must be explicit.
