# QCM-14 Portable Compilation Attestation v0.1

Status: canonical portable manifest and strict parser implemented; signatures, persistence, and compiler trust policy remain deferred.

## Objective

Turn the QCM-13 in-memory compilation provenance envelope into a portable, independently parseable identity manifest. A receiving process must be able to parse the bytes and compare the claimed source and Semantic IR identities against independently available inputs.

## Scope

QCM-14 defines a deterministic binary wire format for a compilation identity manifest. It does not serialize the complete Semantic IR or source text. It carries their cryptographic identities and compiler/schema metadata. The receiving verifier must obtain the source and IR independently.

## Canonical v1 wire format

All integer fields are unsigned, big-endian. Text is UTF-8. No optional fields or map ordering exist in v1.

| Field | Encoding |
|---|---|
| Magic | Exact bytes `QNEFORM-PORTABLE-ATTESTATION\0` |
| Format version | u16; currently 1 |
| Envelope schema | u16 byte length + UTF-8 |
| Compiler package version | u16 byte length + UTF-8 |
| IR schema version | u16 byte length + UTF-8 |
| Source digest | 32-byte SHA-256 |
| Semantic module fingerprint | 32-byte QCM-10 SHA-256 |
| Obligation fingerprint | 32-byte QCM-12 SHA-256 |

Text fields are limited to 4096 bytes. The envelope schema must be the supported QCM-13 schema. Compiler package version and IR schema version must be non-empty. V1 parsers reject invalid magic, unknown format versions, unsupported envelope schemas, invalid UTF-8, oversized fields, truncation, and trailing bytes.

The fingerprint of a portable manifest is SHA-256 over its complete canonical v1 byte representation. The wire bytes themselves are the canonical serialization; there is no alternate JSON representation or permissive parser in this milestone.

## API

Module: `src/portable_attestation.rs`

- `PortableCompilationAttestation::from_envelope()`: derive a manifest from an internally consistent QCM-13 envelope.
- `to_canonical_bytes()`: encode canonical v1 bytes.
- `from_canonical_bytes()`: strict parser.
- `fingerprint()`: SHA-256 identity of the exact canonical bytes.
- `verify_source()`: compare exact UTF-8 source bytes against the source digest.
- `verify_module()`: compare independently available IR schema, QCM-10 module fingerprint and QCM-12 obligation fingerprint.
- `verify_envelope()`: compare all manifest fields with the original in-memory envelope.

## Verification semantics

A successful parse establishes only that the bytes conform to this format and schema. A successful source check establishes that the supplied bytes match the recorded SHA-256 digest. A successful module check establishes that the supplied IR has the recorded schema and fingerprints.

None of these checks authenticates who created the manifest. An attacker can construct a new manifest and recompute its fingerprint. The manifest is not a signature, timestamp, trusted compiler statement, reproducible-build proof, or evidence that the compiler is correct or uncompromised.

The module fingerprint includes the current complete QCM-10 canonical IR identity. Consequently, a change to the mutable obligation `satisfied` flag changes the module fingerprint and fails exact module verification, even though the separate QCM-12 obligation fingerprint intentionally normalizes that status. Both identities are retained to preserve the distinct contracts.

## Deferred

- Digital signatures and signing algorithm/profile selection.
- Compiler identity registry, trust roots and key-rotation/revocation policy.
- Persistent storage, transparency log and timestamping.
- Reproducible compiler builds and independent compiler implementations.
- Complete portable Semantic IR serialization and independent IR reconstruction.
- Source-to-IR proof or compiler correctness proof.
- Backend authorization or execution.

Signing is deliberately deferred until the byte format and its compatibility rules have been reviewed and stabilized.
