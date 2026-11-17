# QCM-13 Compilation Provenance Envelope v0.1

Status: initial implementation present; external signatures, persistence and independently reproducible compiler attestations remain deferred.

## Objective

Bind the exact UTF-8 source bytes supplied to the QnEform compilation pipeline to the Semantic IR produced by that same pipeline. QCM-12 authenticates verifier decisions about a semantic module; QCM-13 adds an in-process provenance object that records which exact source input produced the module.

## Implemented API

Module: `src/compilation.rs`

- `CompilationEnvelope`: opaque-by-interface compilation result with private fields.
- `compile_to_envelope(source, source_name)`: parses, semantically validates and lowers the exact source input, then constructs the envelope.
- `CompilationEnvelope::verify_source(source)`: verifies the exact source digest and recomputes the stored module identities.
- `CompilationEnvelope::verify_internal_integrity()`: recomputes both module fingerprints and checks the IR schema.
- `CompilationEnvelope::fingerprint()`: computes a separately versioned envelope identity.
- `CompilationEnvelope::module()`: exposes the contained IR through an immutable shared reference.
- `compile_to_ir()`: now delegates to the envelope pipeline and returns a clone of its IR module, preventing two independent source-to-IR implementations from drifting.

## Binding model

The envelope records:

- schema identifier `qneform-compilation-envelope-0.1`;
- SHA-256 of the exact UTF-8 source bytes;
- QCM-10 semantic module fingerprint;
- QCM-12 status-independent obligation fingerprint;
- crate package version;
- Semantic IR schema version;
- optional source name as provenance metadata;
- the resulting Semantic IR module behind a private field.

The envelope fingerprint uses domain separation and an explicit canonical version tag. It binds the source digest, both IR fingerprints, envelope schema, compiler package version and IR schema version. Source name is excluded from semantic identity.

## Invariants

1. Source digest is computed from the exact input bytes, including whitespace and line endings.
2. Module and obligation fingerprints are computed only after semantic validation and IR lowering succeed.
3. Envelope fields cannot be independently replaced through the public API.
4. The contained IR is exposed immutably.
5. Internal integrity checks recompute the stored fingerprints rather than trusting the stored digest values.
6. The legacy QCM-10 fingerprint and QCM-12 obligation fingerprint algorithms remain unchanged.
7. A failed parse, semantic validation or lowering step produces no envelope.

## Security boundary and limitations

This is an in-process compilation provenance envelope, not a digital signature or externally verifiable compiler attestation. It records that this library pipeline constructed a module from the supplied source input; it does not prove the compiler implementation is correct, uncompromised, deterministic across compiler builds, or equivalent to another compiler implementation.

The envelope is not yet serialized or persisted. There is no signing key, trusted compiler registry, build reproducibility statement, AST canonical hash, remote verifier protocol or source-to-IR proof. A future serialized envelope must define canonical encoding, schema migration, signature/key rotation and verification policy before being treated as portable evidence.

The source digest is not encryption and does not preserve the source text. Callers must retain the original source separately if later re-verification is required.

## Tests

- exact source digest and both IR fingerprints are bound;
- changed source bytes fail verification;
- source-name metadata does not alter envelope identity;
- semantic source changes alter envelope identity;
- direct IR compilation and envelope compilation use the same lowering result.

No simulator, backend execution, physical allocation or hardware behavior is introduced.
