# QnEform Semantic IR v0.2

**Status:** Initial typed IR schema and AST lowering implemented  
**Schema identifier:** \`qneform-ir-0.2\`  
**Scope:** Current parser and Semantic Kernel subset  
**Not claimed:** Canonical serialization, cryptographic source binding, full resource safety, gate execution, or backend conformance

## 1. Pipeline

\`Source → Lexer → Parser/AST → Semantic Kernel → Semantic IR v0.2\`

The public Rust API is \`compile_to_ir(source, source_name)\`. It parses, validates, and lowers the same in-memory AST. A lowering error returns no IR module.

## 2. Identity rules

- \`SymbolId\`, \`ResourceId\`, and \`ScopeId\` are deterministic, module-local ordinal identifiers.
- These IDs are stable only within a particular lowered module.
- They are not globally unique, persistent across source edits, or cryptographic identities.
- \`module_id\` is currently a placeholder (\`module-0\`); content-addressed module identity is deferred until canonical serialization is specified.

## 3. Module structure

The current module contains:

- Schema version and module identifier
- Optional source name
- Symbol records
- Quantum resource records
- Ordered operations
- Deduplicated effect inventory
- Explicit proof-obligation records

## 4. Symbol and resource records

Symbol records preserve identifier, name, inferred type, lexical scope, domain, and source origin.

Quantum declarations create a resource record with:
- Resource kind
- Ownership state
- Lifecycle state
- Entanglement classification
- Owner scope
- Source origin

The present language only declares qubits. Consequently, the current lowering assigns:
- Ownership: \`Unassigned\`
- Lifecycle: \`Declared\`
- Entanglement: \`Unknown\`

It does **not** imply allocation or physical existence. The IR includes future states such as Allocated, Active, Uncomputed, VerifiedClean, and Released, but the current frontend cannot transition resources into those states.

## 5. Operations and effects

Operations preserve source order and carry domain, effects, and source origin.

Current operation kinds:
- EnterDomain / ExitDomain
- DeclareResource
- Measure
- Bind

Current effect annotations:
- QuantumResourceDeclaration
- Measurement
- Observation
- PureClassical

The schema reserves additional effects for unitary operations, channels, allocation/release, classical control, parameter updates, suspend, and restore. Reserved effects are not emitted by the current frontend.

## 6. Proof obligations

Each declared quantum resource receives an unsatisfied resource-ownership proof obligation. This is deliberate fail-closed metadata: the language has not yet defined explicit ownership or allocation syntax.

A proof obligation marked unsatisfied is not a warning that may be ignored and is not evidence of safety. Future execution backends must reject operations requiring unresolved obligations.

## 7. Provenance

Every lowered symbol, resource, and operation retains the AST source span and optional source name. Spans use UTF-8 byte offsets and one-based line/column coordinates.

This provenance is not yet a cryptographic commitment. A later canonical serialization specification must define source normalization, serialization determinism, and the exact hash preimage before any content hash is treated as an integrity identifier.

## 8. Validation and consistency

Lowering requires a semantic module produced from the supplied AST. The implementation performs a consistency check on the number of non-block statements before lowering.

This check is an internal consistency guard, not a proof that two arbitrary objects are cryptographically bound. Callers should use \`compile_to_ir\` rather than independently pairing unrelated AST and semantic-module values.

## 9. Serialization decision

No serialization dependency is introduced in this milestone. JSON or CBOR must not be selected until:
1. The schema and versioning rules are reviewed.
2. Enum representation and unknown-field behavior are specified.
3. Canonical ordering and numeric/string encoding are specified.
4. Round-trip and cross-version compatibility tests exist.

## 10. Next implementation gates

1. Add semantic tests for resource identity and nested-scope resolution.
2. Define explicit allocation/borrow/release syntax and lifecycle transitions.
3. Define measurement-result types and classical/quantum boundary operations.
4. Add effect checking and proof-obligation discharge rules.
5. Specify canonical serialization and source/IR content binding.
6. Add gate syntax only after gate semantics and effects are approved.
