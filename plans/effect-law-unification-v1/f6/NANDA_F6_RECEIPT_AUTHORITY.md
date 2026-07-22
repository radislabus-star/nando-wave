# F6 Receipt And Authority Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | reference executor | independently computes | expected actor output | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#preserved-frame |
| s2 | verifier receipt | commits to | evidence action output and postconditions | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#stop-f6-gate |
| s3 | verifier receipt | persists | zero raw payloads | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#result |
| s4 | verifier receipt | grants | no execution authority | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#result |
| s5 | live persistence and admission | belong to | F7 and F8 | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#result |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | reference executor | independently computes | expected actor output | crates/nando-operator-proof/src/independent_verifier_v3/reference.rs |
| c2 | verifier receipt | commits to | evidence action output and postconditions | crates/nando-operator-proof/src/independent_verifier_v3/receipt.rs |
| c3 | verifier receipt | persists | zero raw payloads | crates/nando-operator-proof/src/independent_verifier_v3/receipt.rs |
| c4 | verifier receipt | grants | no execution authority | crates/nando-operator-proof/src/independent_verifier_v3/receipt.rs |
| c5 | live persistence and admission | belong to | F7 and F8 | plans/effect-law-unification-v1/STOP_F6_INDEPENDENT_VERIFIER_CONVERGENCE.md |
