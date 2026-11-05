# QnEform

QnEform is a project to develop a native language for classical, quantum, and hybrid computation.

## Current implementation status

This branch contains an initial Rust package, a frontend syntax subset, Semantic Kernel v0.1, and initial Semantic IR v0.2 lowering. It is **not yet a quantum compiler or simulator**.

Implemented:
- Rust library and command-line executable
- Source-positioned lexer with line/block comments and escaped strings
- AST for qubit declarations, measurement, literal/name bindings, and quantum/classical blocks
- Recursive-descent parser with structured syntax errors
- `qneform check <file>` syntax-check command
- Semantic validation for duplicate declarations, unresolved identifiers, domain restrictions, and quantum/classical type boundaries
- Initial typed semantic module (`qneform-semantic-0.1`)
- Versioned Semantic IR schema (`qneform-ir-0.2`) with module-local IDs, resource records, effects, proof obligations, and source provenance
- Public `compile_to_ir` pipeline
- Unit tests for syntax and semantic acceptance/rejection
- GitHub Actions build/test and formatting workflow

Not implemented:
- Canonical Semantic IR serialization and cryptographic source/IR binding
- Explicit resource allocation, borrowing, release, and complete lifecycle enforcement
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

quantum {
    qbit q1;
    measure q1;
}

classical {
    let shots := 100;
    let label := "baseline";
    let ready := true;
}
```

The parser currently accepts these declarations and blocks only. Gate statements such as `H q0;` are intentionally rejected until gate grammar and semantics are specified. `qneform check` performs parsing and v0.1 semantic validation; it does not execute or simulate a quantum program.

## Architecture direction

The planned pipeline is:

`Source → Lexer → Parser/AST → Semantic Kernel → Semantic IR v0.2 → future reference simulator and/or backend`

The semantic model must distinguish classical, quantum, hybrid, boundary, and abstract lifecycle domains. Quantum resources require explicit ownership and lifecycle rules. Measurement is explicit and state-altering. Simulator behavior must not be presented as physical hardware behavior.

See [QCM-06 repository reconciliation](docs/architecture/QCM-06-repository-reconciliation.md) for the audit baseline, [frontend syntax specification](docs/language/frontend-syntax-v0.1.md), [Semantic Kernel v0.1](docs/architecture/semantic-kernel-v0.1.md), and [Semantic IR v0.2](docs/architecture/semantic-ir-v0.2.md).

## Project boundaries

QnEform is a separate project from Level 8 Protocol (L8P). Their codebases and architectural responsibilities must remain separate.
