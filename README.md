# QnEform

QnEform is a project to develop a native language for classical, quantum, and hybrid computation.

## Repository status

This branch establishes the initial buildable Rust package baseline. It is **not yet a quantum compiler**.

Implemented in this baseline:
- Rust library target and command-line executable
- Version reporting and basic command-line help
- Dependency-free package configuration
- One minimal library unit test

Not yet implemented:
- QnEform lexer, parser, AST, type system, or Semantic IR
- Quantum gate semantics or reference simulator
- Quantum resource ownership and ancilla lifecycle enforcement
- Reversible execution and adjoint generation
- Noise/decoherence simulation, QEC, ASH/Phoenix runtime
- OpenQASM 3 export or physical hardware backends

## Build and test

Requires the Rust toolchain (Cargo).

```sh
cargo test
cargo run -- --help
cargo run -- --version
```

## Architecture direction

The planned pipeline is:

`Source → Parser/AST → Semantic IR → validation → reference simulator and/or backend`

The semantic model must distinguish classical, quantum, hybrid, boundary, and abstract lifecycle domains. Quantum resources require explicit ownership and lifecycle rules. Measurement is explicit and state-altering. Simulator behavior must not be presented as physical hardware behavior.

See [QCM-06 repository reconciliation](docs/architecture/QCM-06-repository-reconciliation.md) for the audit baseline and implementation sequence.

## Project boundaries

QnEform is a separate project from Level 8 Protocol (L8P). Their codebases and architectural responsibilities must remain separate.
