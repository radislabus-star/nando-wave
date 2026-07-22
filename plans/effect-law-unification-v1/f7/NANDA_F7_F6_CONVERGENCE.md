# F7 And F6 Root Convergence

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | F6 verifier artifact set | shares | kernel-owned artifact-set digest function | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s2 | F7 generation manifest | commits | exact F6 artifact-set root | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s3 | artifact order | cannot change | artifact-set root or restart bytes | F7_GENERATION_PERSISTENCE_V1.md#stop-f7 |
| s4 | F7-A controlled proof | excludes | live ledger admission and production authority | F7_GENERATION_PERSISTENCE_V1.md#owner-slices |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | F6 verifier artifact set | shares | kernel-owned artifact-set digest function | crates/nando-operator-proof/src/independent_verifier_v3/artifact_set.rs |
| c2 | F7 generation manifest | commits | exact F6 artifact-set root | crates/nando-operator-proof/tests/f7_generation_restart_v3.rs#generation_restart_is_order_invariant_and_converges_with_f6 |
| c3 | artifact order | cannot change | artifact-set root or restart bytes | crates/nando-operator-proof/tests/f7_generation_restart_v3.rs#generation_restart_is_order_invariant_and_converges_with_f6 |
| c4 | F7-A controlled proof | excludes | live ledger admission and production authority | crates/nando-operator-kernel/src/operator_generation.rs#execution_authority |
