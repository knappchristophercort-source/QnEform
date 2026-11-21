# QCM-15 Signed Compilation Attestation and Trust Policy — Design v0.1

Status: design decision record; no signing code or cryptographic dependency has been added.

## Objective

Add authenticity and explicit signer authorization to the QCM-14 portable compilation manifest without conflating a valid signature with compiler correctness or physical execution truth.

QCM-14 remains the canonical unsigned payload. QCM-15 wraps its exact canonical bytes in a versioned signature envelope and evaluates the signature against a verifier-supplied trust policy.

## Frozen compatibility boundary

- The QCM-14 v1 byte format is the signed payload and is not modified by QCM-15.
- Signatures cover the exact QCM-14 canonical bytes, not a re-serialized object or a display representation.
- Existing QCM-14 parsing and verification remain usable without keys or trust configuration.
- Unknown versions and algorithms fail closed. There is no algorithm fallback or downgrade.
- The signed wrapper is a new format with its own magic and version.

## Proposed initial cryptographic profile

Use Ed25519 for the initial signature profile, subject to selecting and pinning a maintained Rust implementation after reviewing its current security status, supported Rust version, feature flags, and lockfile impact. Do not implement curve or signature arithmetic in QnEform.

The signature input is a domain-separated, unambiguous byte string:

`QNEFORM-SIGNED-PORTABLE-ATTESTATION\\0`
|| u16_be(profile_version)
|| u16_be(algorithm_id)
|| u16_be(key_id_byte_length)
|| key_id_utf8
|| u32_be(manifest_byte_length)
|| exact_QCM14_manifest_bytes

The initial algorithm identifier is reserved for Ed25519. The key ID is included in the signed input so a wrapper cannot substitute a different signer label without invalidating the signature. The manifest length and exact bytes are included to prevent concatenation ambiguity and payload substitution.

The wrapper carries:
- wrapper magic and version;
- algorithm identifier;
- bounded UTF-8 key ID;
- bounded length-prefixed exact QCM-14 manifest bytes;
- fixed-width Ed25519 signature.

All lengths are checked before allocation. Parsers reject invalid magic, unsupported versions/algorithms, invalid UTF-8, empty or oversized key IDs, truncated fields, invalid signature length, and trailing bytes.

## Trust is separate from cryptography

A cryptographically valid signature proves only that the private key corresponding to the supplied public key signed the defined message. It does not by itself authorize that key to speak for QnEform or a compiler.

A trust policy must explicitly map key IDs to:
- public key bytes and algorithm;
- authorized signer role (for example, compiler release signer);
- permitted compiler identity/package scope;
- activation and expiry boundaries;
- revocation state and, where supported, revocation effective time;
- policy version or identifier.

Unknown keys, unauthorized roles, compiler-scope mismatches, expired keys, revoked keys, and policy mismatches are rejected. No trust policy is implicit, global, or silently inferred from a valid signature.

Verification should return distinct outcomes for malformed encoding, unsupported profile, invalid signature, unknown key, unauthorized signer, expired key, revoked key, and compiler-scope mismatch. Callers must not collapse these into a successful verification state.

## Time and revocation semantics

The verifier supplies the evaluation time and trust policy. A timestamp written by the signer into the artifact is only a claim unless independently timestamped by a trusted service.

Without a trusted timestamp or transparency-log inclusion proof, a verifier cannot establish that a signature was created before a key's later revocation or expiry. QCM-15 must not imply historical validity from an untrusted signing-time field.

Key rotation introduces a new key ID and explicit policy entry. Revocation is an explicit policy event; the verifier's configured policy determines whether it is effective immediately or at a declared effective time. A missing or unavailable policy is a verification failure, not an invitation to trust the key.

## Replay and application context

A signed artifact may legitimately be copied and verified more than once. QCM-15 therefore does not claim that a signature alone prevents replay.

Where a consuming application needs freshness or request binding, it must provide an expected context (such as a challenge, deployment identifier, or transaction identifier) and require that context to be included in the signed payload under a separately versioned extension. A receiver must not invent a challenge after signing and then claim the old signature proves it.

## API direction

The implementation should expose separate operations for:
1. signing an already constructed QCM-14 manifest using an explicitly supplied signing key and key ID;
2. parsing and canonically re-encoding the signed wrapper;
3. cryptographic signature verification against an explicitly supplied public key;
4. authorization evaluation against an explicitly supplied trust policy and evaluation time.

The result type must keep signature validity and policy authorization distinct. A successful signature check must never automatically produce an authorized/trusted result.

Private key material must not be serialized into manifests, debug output, errors, logs, or repository fixtures. Production key custody, hardware-backed signing, secret storage, and operational key ceremonies are outside the library's scope.

## Required negative tests before release

- one-bit mutation of manifest bytes;
- one-bit mutation of signature bytes;
- wrong public key;
- changed key ID or algorithm identifier;
- unknown key and unsupported algorithm;
- malformed, truncated, oversized, or trailing wrapper bytes;
- expired and revoked key;
- role and compiler-scope mismatch;
- missing trust policy;
- policy version mismatch;
- attempted algorithm downgrade;
- deterministic canonical wrapper encoding;
- round-trip parsing preserves the exact signed payload bytes.

Tests must use published Ed25519 test vectors or well-reviewed crate APIs, never a custom implementation.

## Explicit non-claims and deferred work

QCM-15 does not prove:
- compiler correctness or absence of compiler compromise;
- reproducible builds or source-to-IR semantic equivalence;
- correctness of a quantum simulator or physical quantum hardware;
- that the signer is a person or organization beyond the configured key policy;
- trusted signing time, non-repudiation in a legal sense, or transparency-log inclusion.

Trusted timestamping, append-only transparency logs, public compiler registries, key ceremony procedures, hardware security modules, post-quantum signatures, and complete portable IR serialization remain separate milestones.

## Exit criteria

QCM-15 implementation may begin only after:
1. the QCM-14 wire format and compatibility boundary are accepted;
2. the Rust signature dependency is reviewed and pinned in Cargo.toml and Cargo.lock;
3. the signed wrapper and trust-policy semantics are reviewed;
4. signature validity and trust authorization are represented as separate API results;
5. all required positive and negative tests pass in CI.
