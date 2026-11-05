# QCM-06 — QnEform Repository Reconciliation Report

- **Report ID:** QCM-06
- **Audit date:** 2026-11-04
- **Repository:** `knappchristophercort-source/QnEform`
- **Audited branch:** `main`
- **Working branch:** `qcm06/reconciliation-baseline`
- **Status:** Baseline audit recorded; implementation not yet restored or tested
- **Authority:** Observed repository contents and prior project decisions available at audit time

## 1. Executive finding

The canonical QnEform repository is currently a placeholder repository, not an implementation repository. Its default branch contains a minimal README and an Apache License 2.0 file. The inspected source paths for a Rust crate, Python package, and JavaScript package were absent at the time of inspection.

A separate repository, `cortlandcoppernicus-cmd/qneform`, contains a partial Rust bootstrap. It is not the canonical repository and must be treated as a reference source until its provenance, intended ownership, and relationship to the QnEform prototype are reconciled.

No source code was changed as part of this audit. This report is committed to a working branch; `main` remains untouched.

## 2. Canonical repository findings

Repository: `knappchristophercort-source/QnEform`

- Default branch: `main`
- Visibility: public
- License file: Apache License 2.0
- README content at audit: `# QnEform`
- Repository metadata reported push permission, but the authoritative operational check was branch creation. Creation of `qcm06/reconciliation-baseline` from `main` succeeded and the branch was subsequently found in the branch list.
- The following paths were checked on `main` and returned not found:
  - `Cargo.toml`
  - `src/lib.rs`
  - `src/main.rs`
  - `src/lexer.rs`
  - `src/parser.rs`
  - `src/ast.rs`
  - `pyproject.toml`
  - `package.json`

The absence of those paths establishes only that those files were not present at the checked paths on the inspected branch. It does not establish that no other project artifacts exist elsewhere (for example, in the Lovable project, another repository, or user-held archives).

## 3. Separate Rust bootstrap findings

Repository: `cortlandcoppernicus-cmd/qneform`

The inspected `main` branch contains a partial Rust project with `Cargo.toml`, `src/lib.rs`, `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, README, and LICENSE.

Static inspection found these gaps:

- `src/lib.rs` declares `typecheck`, `codegen`, and `qir` modules whose source files were absent at the inspected paths.
- `Cargo.toml` expects `src/main.rs`, which was absent on the inspected `main` branch.
- A separate `ci/add-github-actions` branch included a placeholder `src/main.rs` and CI workflow, but did not resolve the missing typecheck/codegen/QIR modules.
- The AST covered a narrow set of constructs: qubit declaration, measurement, let binding, quantum block, and classical block.
- The parser did not cover several constructs shown in prototype-level examples, including common gates, rotation gates, allocation, burn/rebirth, and archive operations.
- Compiler output included success-style messages for areas described as TODO. Those messages are not evidence that unitary proof, error correction, or pulse compilation are implemented.

No Cargo build or test command was run during this static repository audit. Therefore, this report makes no claim that the bootstrap currently builds or passes tests.

## 4. Preserve these architectural decisions

The following project concepts are retained as design constraints, not asserted as implemented features:

1. QnEform is a native quantum-computing language intended to support classical, quantum, and hybrid computation.
2. QnEform remains a separate project from Level 8 Protocol (L8P). Do not merge their codebases, governance, or product identities.
3. Preserve the existing lexer/parser/AST as reference material when they are recovered; do not perform a grammar rewrite before reconciliation.
4. Use a backend-independent semantic layer between source processing and execution backends.
5. Distinguish classical, quantum, hybrid, boundary, and abstract lifecycle domains.
6. Model quantum resources with explicit ownership and lifecycle rules. Ordinary classical copying semantics must not silently duplicate quantum resources.
7. Treat measurement as an explicit, state-altering operation. Distinguish individual shots from aggregate statistics.
8. Distinguish clean ancillas from dirty or borrowed quantum resources. A clean ancilla requires both the required reference state and separability from the rest of the system; a zero-valued observation alone is not proof of cleanliness.
9. Reversible execution must reject measurement, reset, non-unitary channels, and other operations without valid inverse semantics inside a reversible region.
10. Preserve the distinction between simulator behavior and physical quantum hardware. A simulator is a computational model, not a physical quantum system.
11. Noise, decoherence, physical time evolution, observables, and hardware limitations must be represented explicitly when modeled; do not imply that ideal circuit semantics establish physical behavior.
12. Error correction is a hybrid quantum/classical process.
13. ASH/history records execution observations and provenance; it must not be represented as a stored copy of an unknown quantum state.
14. Phoenix/adaptive execution and burn/rebirth lifecycle concepts must not be confused with physical time evolution, measurement, or reset.

## 5. Existing specification work to preserve

The following specifications have been drafted in project discussions. Their status is draft/proposed unless separately ratified and implemented:

- **QCM-01 — Computational Model v0.1:** computational entities, execution domains, state semantics, affine quantum-resource ownership, measurement, ancilla classes, and backend independence.
- **QCM-02 — Semantic IR v0.1:** versioned semantic module, typed values/resources/operations, effects, control flow, provenance, proof obligations, validation, and backend contract.
- **QCM-03 — Ancilla lifecycle:** clean/dirty resource distinctions, lifecycle states, proof requirements, and negative tests.
- **QCM-04 — Reversible Execution Engine v0.1:** reversibility analysis, adjoint generation, inverse gate rules, global-phase handling, and rejection conditions.
- **QCM-05 — Conformance and Verification Suite v0.1:** layered conformance testing, mathematical checks, backend tests, adversarial tests, deterministic seeds, and PASS/FAIL/REJECT/INCONCLUSIVE result semantics.

These drafts are not evidence that the corresponding software modules exist or that their tests have passed. Before implementation, recover the complete source artifacts and reconcile any differences against this report.

## 6. Implementation status matrix

| Area | Audit status | Required action |
|---|---|---|
| Canonical repository | Placeholder | Establish canonical source tree |
| Lexer/parser/AST | Absent in canonical repo; partial in separate bootstrap | Recover and reconcile before changing grammar |
| Semantic kernel / semantic IR | Not found in inspected repositories | Specify then implement minimal typed IR |
| Reference simulator | Not found in inspected repositories | Implement a small mathematically testable simulator |
| Gate semantics | Partial examples only in bootstrap | Define supported gate set and exact semantics |
| Reversible engine | Specification draft only | Implement after IR and gate semantics |
| Ancilla manager | Specification draft only | Implement lifecycle and fail-closed validation |
| Noise / hardware model | Not verified | Add explicit model boundaries and conformance |
| QEC | TODO/claims in bootstrap; not verified | Do not expose as implemented until tests substantiate it |
| ASH / Phoenix / Rebirth | Conceptual/draft status | Recover requirements and define bounded semantics |
| OpenQASM 3 / hardware backends | Not verified in canonical repo | Add only after stable IR/backend contract |
| Automated CI | Not verified in canonical repo | Establish build and test gates |

## 7. Recommended implementation sequence

1. Keep `main` unchanged while establishing the baseline on a working branch.
2. Recover and inventory the Lovable prototype and all user-held QnEform source archives. Record source provenance and version before porting.
3. Inspect the separate Rust bootstrap in full, including its history and branches. Decide explicitly whether it is suitable to port, selectively reuse, or replace.
4. Establish one buildable canonical project with a minimal executable and library target. Avoid placeholder success messages.
5. Add lexer/parser fixtures and tests for the syntax actually supported. Unsupported syntax must fail clearly rather than being silently accepted.
6. Implement a minimal vertical slice: source text → parser/AST → validated semantic IR → reference simulator → mathematically checked result.
7. Add the QCM-04 reversible engine only after the semantic IR and supported gate semantics are stable.
8. Implement ancilla lifecycle enforcement and QCM-05 conformance tests.
9. Add noise models, QEC, ASH/Phoenix lifecycle behavior, OpenQASM 3 export, and hardware adapters as independently testable increments.
10. Require CI to build and run tests; do not mark a capability complete based on a placeholder, demonstration message, or unexecuted specification.

## 8. Acceptance criteria for the next milestone

The next milestone is a **buildable canonical baseline**, not a claim of a complete quantum compiler.

It is accepted only when:

- The canonical repository contains a documented project structure and explicit language/toolchain decision.
- The project builds from a clean checkout using documented commands.
- A minimal executable runs.
- Unit tests cover lexer/parser behavior for the implemented subset.
- Unsupported syntax returns structured errors.
- No TODO or placeholder path reports successful compilation, proof, QEC, or hardware execution.
- CI runs the same build and test commands.
- The README clearly distinguishes implemented features, experimental features, and planned features.

## 9. Audit limitations and change control

This is a static inspection report based on repository paths and file contents observed on 2026-11-04. It is not a full history audit, security audit, build verification, physics validation, or proof of completeness across external project stores.

Any later discovery of source files, archives, or prototype behavior should update this report through a dated addendum rather than silently overwriting the baseline. Changes to `main` should occur only after review of the working branch and its test results.

---

**Current decision:** retain the canonical repository as the authoritative destination; preserve the separate Rust bootstrap as reference-only pending provenance review; implement in small, testable increments; do not claim unimplemented quantum, error-correction, or hardware capabilities.
