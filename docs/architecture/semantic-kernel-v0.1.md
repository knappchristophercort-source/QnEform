# QnEform Semantic Kernel v0.1

**Status:** Implemented initial semantic validation and typed semantic module  
**Version identifier:** \`qneform-semantic-0.1\`  
**Scope:** The current AST subset only  
**Not claimed:** Full QCM-02 Semantic IR, executable quantum IR, proof system, simulator, or hardware semantics

## 1. Purpose

The Semantic Kernel sits after parsing and before backend lowering. It determines whether a syntactically valid program is meaningful under the currently supported language subset.

\`Source → Lexer → Parser/AST → Semantic Kernel → Typed Semantic Module\`

A syntax pass is not a semantic pass. A semantic pass is not execution.

## 2. Root and block domains

The root program has the \`Hybrid\` domain. Explicit blocks establish narrower domains:

- \`quantum { ... }\` establishes the Quantum domain.
- \`classical { ... }\` establishes the Classical domain.

The v0.1 rules are deliberately conservative:

| Construct | Hybrid root | Quantum block | Classical block |
|---|---:|---:|---:|
| Declare qubit | Allowed | Allowed | Rejected |
| Measure qubit | Allowed | Allowed | Rejected |
| Declare classical binding | Allowed | Rejected | Allowed |
| Enter quantum block | Allowed | Allowed | Rejected |
| Enter classical block | Allowed | Rejected | Allowed |

Cross-domain block nesting is rejected. Sequential quantum and classical blocks at the hybrid root are supported. These restrictions avoid implying a control-flow or synchronization model that has not yet been specified.

## 3. Names, scopes, and declarations

- Declarations become visible after their declaration statement.
- A reference must resolve to a declaration in the current or an enclosing scope.
- A block creates a lexical scope; declarations made inside it do not escape it.
- Parent-scope declarations are visible inside nested blocks.
- Duplicate declarations and shadowing are rejected in v0.1.
- Declaration order matters; forward references are not supported.
- Qubit names identify quantum resources; they are not ordinary copyable classical values.

Shadowing is rejected as a conservative initial rule. A future language revision may define explicit shadowing or resource rebinding, but it must do so without creating ambiguous quantum-resource identity.

## 4. Initial types

The first semantic pass infers only these types:

- \`Qubit\`
- \`Number\`
- \`String\`
- \`Boolean\`

Numeric literals are retained as source strings. This version does not define integer widths, floating-point precision, overflow, units, or arithmetic semantics.

Identifier expressions may reference previously declared classical values. They may not copy a Qubit resource into a classical binding.

## 5. Measurement and resource limits

A measurement target must resolve to a declared Qubit. The current analyzer emits a semantic \`Measure\` instruction only; it does not perform a measurement.

The current subset does not yet model:
- Allocation or deallocation
- Ownership transfer or borrowing
- Entanglement state
- Ancilla cleanliness
- Reset or discard
- Measurement result binding
- Shot counts or aggregate statistics

Those features require the richer resource model in QCM-01/QCM-02 and must not be inferred from the present syntax.

## 6. Typed semantic module

Successful analysis produces a module with:

- \`ir_version\`: currently \`qneform-semantic-0.1\`
- Ordered semantic instructions
- A symbol inventory with inferred type, declaration span, and declaration domain

Instruction variants currently include:
- \`EnterDomain\` / \`ExitDomain\`
- \`DeclareQubit\`
- \`Measure\`
- \`Bind\` with a typed literal or classical reference

This is a minimal typed semantic representation, not yet the full versioned QCM-02 IR. It is intentionally backend-neutral and does not contain executable gate operations.

## 7. Diagnostics and fail-closed behavior

The analyzer accumulates semantic diagnostics and returns no module if any error exists. Initial diagnostic codes:

- \`SEM_DUPLICATE_DECLARATION\`
- \`SEM_UNRESOLVED_IDENTIFIER\`
- \`SEM_UNRESOLVED_QUANTUM_RESOURCE\`
- \`SEM_MEASUREMENT_TARGET_NOT_QUBIT\`
- \`SEM_QUBIT_IN_CLASSICAL_DOMAIN\`
- \`SEM_MEASUREMENT_IN_CLASSICAL_DOMAIN\`
- \`SEM_CLASSICAL_BINDING_IN_QUANTUM_DOMAIN\`
- \`SEM_QUANTUM_VALUE_IN_CLASSICAL_BINDING\`
- \`SEM_INVALID_DOMAIN_NESTING\`

The CLI command \`qneform check <file>\` now runs both parsing and this semantic pass. Success means the source is valid under the current syntax and semantic subset only. It does not mean a quantum circuit was compiled or executed.

## 8. Conformance tests

The implementation includes tests for:
- A valid hybrid program with classical bindings and a quantum measurement
- Duplicate declarations and shadowing
- Unresolved classical identifiers
- Unresolved measurement targets
- Prohibition on copying a qubit into a classical binding
- Classical operations inside quantum blocks
- Quantum operations inside classical blocks
- Cross-domain block nesting
- Block-local declaration visibility
- Valid classical references

## 9. Next required evolution

The next semantic increment should expand this module toward QCM-02:

1. Add explicit typed values and stable symbol/resource identifiers.
2. Define resource ownership, borrowing, release, and lifecycle states.
3. Add measurement-result semantics and explicit classical/quantum boundary operations.
4. Add effect annotations and proof obligations.
5. Define a versioned, serializable IR schema with source provenance.
6. Add canonical serialization and content hashing only after the schema is stable.
7. Add gate syntax only after gate types, parameters, domains, and operation effects are defined.

Do not claim complete resource safety, reversible correctness, or quantum execution from Semantic Kernel v0.1.
