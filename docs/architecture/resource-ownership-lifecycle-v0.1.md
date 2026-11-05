# QCM-03 Resource Ownership and Lifecycle v0.1

**Status:** Initial resource lifecycle ledger implemented; source-language integration is pending  
**Scope:** Compiler/runtime bookkeeping API and fail-closed transition rules  
**Not claimed:** Physical qubit allocation, quantum-state inspection, or complete static ownership proof

## 1. Core rule

A quantum resource is not an ordinary copyable value. Every resource has one identity, one ownership mode, one lifecycle state, conservative entanglement knowledge, and explicit proof requirements.

The ledger in \`src/resource.rs\` is a bookkeeping model. Calling \`allocate\` creates a logical record only; it does not allocate a physical QPU qubit.

## 2. Resource kinds

- \`Qubit\`: ordinary quantum resource.
- \`CleanAncilla\`: resource whose required initial condition must be established by a valid proof.
- \`DirtyAncilla\`: arbitrary initial state and correlations must be preserved/restored by the operation using it.

Clean ancilla condition for resource \(a\) and remainder \(R\):

\[
\rho_{aR}=|0\rangle\langle 0|_a\otimes\rho_R
\]

This requires both the zero state and separability from the remainder. A zero measurement outcome or zero probability of observing one is not a sufficient proof.

## 3. Ownership and borrowing

- Allocation creates an \`Owned\` logical resource.
- Borrowing creates a unique linear borrow handle and increments the outstanding-borrow count.
- Every borrow handle must be returned exactly once.
- The owner cannot release a resource while any borrow remains outstanding.
- A borrow handle cannot be duplicated through the API's identity model.
- Borrowing does not transfer ownership.
- Unknown or stale handles fail closed.

The initial API uses explicit borrow handles rather than aliasing mutable references to represent borrow lifetime. Source syntax and lexical borrow scopes are a later integration step.

## 4. Lifecycle

The ledger supports these lifecycle labels:

\`Allocated → Active → Uncomputed → VerifiedClean → Released\`

Not every resource follows every state. A normal qubit may be released only when ownership is retained, all borrows are returned, and entanglement is known separable. Clean ancilla additionally requires a cleanliness attestation. Dirty ancilla requires a restoration attestation.

The current API intentionally does not infer that a resource is separable. The caller must supply a conservative entanglement status. \`Unknown\`, \`PossiblyEntangled\`, and \`KnownEntangled\` all block release.

## 5. Clean and dirty ancilla proof contracts

### Clean ancilla

Release requires:
1. A resource of kind \`CleanAncilla\`.
2. A positive cleanliness attestation from a separately trusted analysis/backend.
3. \`KnownSeparable\` entanglement status.

The ledger records the attestation; it does not independently establish the physics.

### Dirty ancilla

A dirty ancilla may begin in an arbitrary state and may be correlated with other resources. Release requires an explicit restoration attestation. The proof obligation is restoration of the arbitrary input state and its relevant correlations, not merely return to \(|0\rangle\).

## 6. Release restrictions

Release is rejected if:
- The resource handle is unknown or already released.
- Ownership is not \`Owned\`.
- Any borrow remains outstanding.
- Entanglement is not known separable.
- A clean ancilla lacks a positive cleanliness attestation.
- A dirty ancilla lacks a positive restoration attestation.

A released handle remains invalid. Reusing a released handle is rejected. A later allocation with the same name receives a new handle only after the previous name has been removed from the live-name map.

## 7. Trust boundary

The following are separate claims and must not be conflated:

- **Ledger bookkeeping:** records declared transitions and attestations.
- **Static proof:** compiler proves a property from program structure and assumptions.
- **Simulator evidence:** a particular simulator validates a model under stated assumptions.
- **Hardware evidence:** device/backend-specific evidence supports a physical claim.

The ledger's \`attest_cleanliness\` and \`attest_dirty_restored\` methods accept proof results. They do not generate those proofs. Production integration must replace unconstrained booleans with typed, provenance-bearing proof objects before hardware use.

## 8. Implemented tests

The current module tests:
- Owned allocation and conservative initial state
- Duplicate live names
- Outstanding borrow blocks release
- Borrow handles returned exactly once
- Unknown/possible entanglement blocks release
- Clean ancilla requires separability and cleanliness attestation
- Dirty ancilla requires restoration attestation
- Entangled clean ancilla cannot be released
- Use-after-release and double release rejection

## 9. Next implementation gates

1. Replace boolean attestations with typed proof records bound to resource identity and program/IR version.
2. Integrate resource declarations and transitions into Semantic IR operations.
3. Add source syntax for allocation, borrowing, return, and release.
4. Add lexical borrow-scope checking and reject borrow escape.
5. Add conservative entanglement propagation through operations.
6. Add ancilla compute/uncompute contracts and adversarial tests.
7. Only then permit gate operations to consume or modify resources.
