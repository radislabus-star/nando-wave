# STOP-R8E: Independent Verifier Owner Split

Status: `PASS / MOVE_ONLY / AUTHORITY_FALSE`

Date: 2026-07-21

## Boundary

The independent response verifier was physically split without changing proof
semantics:

```text
verifier.rs             proof orchestration and response comparison
verifier/input.rs       immutable request observation
verifier/selection.rs   independent operand and scalar derivation
verifier/collection.rs  independent collection execution and rendering
```

The selection and collection owners do not import `nando-operator-runtime` and
do not call the actor implementation. Their expected values remain independently
derived from bounded provider evidence and immutable operator contracts.

## File Budget

```text
before verifier.rs             3956
after verifier.rs              1784
after verifier/selection.rs    1503
after verifier/collection.rs    666
after verifier/input.rs          38
hard production violations       0
```

## Proof

```text
nando-operator-proof compile/test/clippy   PASS
nando-response-actor frozen fingerprint    PASS
AST function inventory                     95/95
git diff --check                           PASS
new remote background builds                  0
execution authority                       false
deploy/restart                            not run
```

Machine receipts:

- `R8E_PROOF_VERIFIER_STOP.json`
- `R8E_PROOF_VERIFIER_RESPONSE_STOP.json`

This STOP closes only the verifier file-budget and ownership split. It grants
no package authority and does not unlock F5-B before STOP-R9.
