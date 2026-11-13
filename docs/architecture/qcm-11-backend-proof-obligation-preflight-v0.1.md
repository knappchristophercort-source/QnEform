# QCM-11 Backend Proof-Obligation Preflight v0.1

Status: implemented preflight API; no execution backend exists yet.

## Objective

Provide a common fail-closed readiness check that a future execution backend can call before accepting a module. A module is blocked whenever any proof obligation marked required remains unsatisfied.

This milestone implements preflight validation only. It does not execute, simulate, allocate, or authorize physical quantum operations.

## API

`SemanticIrModule::validate_backend_readiness()` returns:

- `Ok(())` when no required proof obligation remains unresolved;
- `BackendReadinessError` containing every unresolved required obligation otherwise.

Each issue preserves the obligation ID, kind, optional resource subject, and description. The check does not stop at the first failure, so callers receive a complete deterministic diagnostic list in IR order.

Optional unresolved obligations do not block readiness. A module with no proof obligations passes this preflight.

## Fail-closed rule

For every obligation:

- `required == true && satisfied == false`: block;
- `required == true && satisfied == true`: does not block this preflight;
- `required == false`: does not block this preflight.

The current lowering pipeline emits an unsatisfied required ResourceOwnership obligation for each declared qubit. Such modules therefore fail readiness until a later, trusted proof-resolution pipeline exists. This is intentional.

## Security and trust boundary

The current `ProofObligation.satisfied` field is public mutable IR data. This API checks the declared status; it does not authenticate who changed that status or prove that an obligation was actually discharged.

Therefore:

1. This preflight must not be treated as a verifier-issued receipt.
2. A production backend must additionally require trusted, provenance-bound verifier evidence before executing quantum work.
3. The backend must bind that evidence to the exact canonical IR fingerprint and applicable resource identity.
4. Static proof acceptance, runtime/backend proof acceptance, rejection, inconclusive results, and unsupported proof profiles must remain distinct outcomes.
5. A software readiness result does not establish physical quantum-state truth or hardware conformance.

No backend execution is introduced by QCM-11.

## Test coverage

- all required obligations satisfied: pass;
- one or more required obligations unresolved: reject and report all;
- optional unresolved obligations: pass;
- no obligations: pass;
- multiple failures preserve deterministic IR order.

## Deferred work

- Immutable compilation envelope and authenticated proof-resolution workflow.
- Backend-specific proof profiles and conformance receipts.
- Runtime evidence for dynamic obligations.
- Actual reference simulator or hardware backend integration.
- Durable persistence and restart-safe ledger replay protection.

`release` remains distinct from `discard`; QCM-11 does not change either operation.
