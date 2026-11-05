# QnEform

QnEform is a project to develop a native language for classical, quantum, and hybrid computation.

## Current implementation status

This branch contains an initial Rust package and a small, tested frontend syntax subset. It is **not yet a quantum compiler or simulator**.

Implemented:
- Rust library and command-line executable
- Source-positioned lexer with line/block comments and escaped strings
- AST for qubit declarations, measurement, literal/name bindings, and quantum/classical blocks
- Recursive-descent parser with structured syntax errors
- `qneform check <file>` syntax-check command
- Unit tests for supported syntax and rejection of unsupported syntax
- GitHub Actions build/test and formatting workflow

Not implemented:
- Semantic analysis, type checking, or QnEform Semantic IR
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

The parser currently accepts these declarations and blocks only. Gate statements such as `H q0;` are intentionally rejected until gate grammar and semantics are specified. A successful syntax check means only that the source matches the supported grammar; it does not mean that a quantum program has been type-checked or executed.

## Architecture direction

The planned pipeline is:

`Source → Lexer → Parser/AST → Semantic IR → validation → reference simulator and/or backend`

The semantic model must distinguish classical, quantum, hybrid, boundary, and abstract lifecycle domains. Quantum resources require explicit ownership and lifecycle rules. Measurement is explicit and state-altering. Simulator behavior must not be presented as physical hardware behavior.

See [QCM-06 repository reconciliation](docs/architecture/QCM-06-repository-reconciliation.md) for the audit baseline and implementation sequence.

## Project boundaries

QnEform is a separate project from Level 8 Protocol (L8P). Their codebases and architectural responsibilities must remain separate.
