# QnEform

QnEform is a project to develop a native language for classical, quantum, and hybrid computation.

## Current implementation status

This branch contains an initial Rust package, a frontend syntax subset, Semantic Kernel v0.1, and initial Semantic IR v0.2 lowering. It is **not yet a quantum compiler or simulator**.

Implemented:
- Rust library and command-line executable
- Source-positioned lexer with line/block comments and escaped strings
- AST for qubit declarations, allocation, borrowing/return, release, measurement, literal/name bindings, and quantum/classical blocks
- Recursive-descent parser with structured syntax errors
- `qneform check <file>` syntax-check command
- Semantic validation for duplicate declarations, unresolved identifiers, domain restrictions, quantum/classical type boundaries, allocation order, borrow balance, and release safety
- Initial typed semantic module (`qneform-semantic-0.1`)
- Versioned Semantic IR schema (`qneform-ir-0.2`) with module-local IDs, resource records, effects, proof obligations, and source provenance
- Public `compile_to_ir` pipeline routed through an opaque QCM-13 compilation-provenance envelope
- QCM-13 exact-source SHA-256 binding, QCM-10/QCM-12 module identity capture, immutable-by-interface IR access and envelope integrity checks
- QCM-14 canonical portable compilation-attestation manifest with strict versioned binary parser, exact-source verification and independent Semantic IR fingerprint checks
- QCM-11 fail-closed diagnostic backend-readiness preflight; it is not authenticated proof or execution authorization
- QCM-12 status-independent obligation fingerprint and opaque verifier-issued proof-obligation receipts; receipt-gated readiness requires exactly one matching receipt per required obligation
- Unit tests for syntax and semantic acceptance/rejection
- GitHub Actions build/test and formatting workflow

Not implemented:
- Complete entanglement-aware static ownership proofs
- Concrete proof profiles, evidence acquisition and verifier trust/key management; QCM-12 authenticates the configured verifier decision but does not independently establish verifier soundness or physical quantum-state truth
- Digital signatures, persistent attestations, compiler trust/key management, complete portable IR serialization and independently reproducible source-to-IR compiler attestations; QCM-13/QCM-14 provide source/IR identity manifests, not proof of compiler correctness
- Production backend authorization and backend-specific conformance evidence
- Physical allocation and quantum execution
- Measurement-result semantics, gate semantics, simulator, and hardware backends
- Quantum gate syntax or gate semantics
- Quantum state simulation or physical hardware execution
- Quantum resource ownership and ancilla lifecycle enforcement
- Reversible execution and adjoint generation
- Noise/decoherence simulation, QEC, ASH/Phoenix runtime
- OpenQASM 3 export or hardware backends

## Build and test

Requires the stable Rust toolchain (Cargo).

```sh
cargo fmt --all -- --check
cargo test --locked
cargo run -- --help
cargo run -- check examples/minimal.qn
```

## First supported syntax subset

```text
qbit q0;
allocate q0;

quantum {
    qbit q1;
    allocate q1;
    measure q1;
    release q1;
}

classical {
    let shots := 100;
    let label := "baseline";
    let ready := true;
}
```

The parser accepts the resource lifecycle statements shown above. Gate statements such as `H q0;` are intentionally rejected until gate grammar and semantics are specified. `qneform check` performs parsing and v0.1 semantic validation; it does not execute or simulate a quantum program.

## Architecture direction

The planned pipeline is:

`Source → Lexer → Parser/AST → Semantic Kernel → Semantic IR v0.2 → future reference simulator and/or backend`

The semantic model must distinguish classical, quantum, hybrid, boundary, and abstract lifecycle domains. Quantum resources require explicit ownership and lifecycle rules. QCM-03 resource lifecycle statements are now parsed, semantically checked, and lowered to typed IR operations. The separate ledger remains a bookkeeping API; no physical allocation or backend execution is implied. Measurement is explicit and state-altering. Simulator behavior must not be presented as physical hardware behavior.

See [QCM-14 portable compilation attestation](docs/architecture/qcm-14-portable-compilation-attestation-v0.1.md), [QCM-13 compilation provenance envelope](docs/architecture/qcm-13-compilation-provenance-envelope-v0.1.md), [QCM-12 authenticated proof-obligation resolution](docs/architecture/qcm-12-authenticated-proof-obligation-resolution-v0.1.md), [QCM-11 backend proof-obligation preflight](docs/architecture/qcm-11-backend-proof-obligation-preflight-v0.1.md), [QCM-06 repository reconciliation](docs/architecture/QCM-06-repository-reconciliation.md) for the audit baseline, [frontend syntax specification](docs/language/frontend-syntax-v0.1.md), [Semantic Kernel v0.1](docs/architecture/semantic-kernel-v0.1.md), and [Semantic IR v0.2](docs/architecture/semantic-ir-v0.2.md).

## Project boundaries

QnEform is a separate project from Level 8 Protocol (L8P). Their codebases and architectural responsibilities must remain separate.
