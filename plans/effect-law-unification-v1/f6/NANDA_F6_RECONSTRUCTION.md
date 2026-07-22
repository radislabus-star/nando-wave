# F6 Reconstruction Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | verifier reconstruction | rebuilds | selector role capability and action | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s2 | actor-selected mapping | serves as | comparison claim only | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s3 | multiple candidate paths | may collapse to | one physical action class | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#independence-contract |
| s4 | multiple physical action classes | yield | ABSTAIN | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#stop-f6-gate |
| s5 | unique reconstructed action | precedes | independent reference execution | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#objective |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | verifier reconstruction | rebuilds | selector role capability and action | crates/nando-operator-proof/src/independent_verifier_v3/reconstruct/mod.rs |
| c2 | actor-selected mapping | serves as | comparison claim only | crates/nando-operator-proof/src/independent_verifier_v3/reconstruct/matching.rs |
| c3 | multiple candidate paths | may collapse to | one physical action class | crates/nando-operator-proof/src/independent_verifier_v3/reconstruct/mod.rs |
| c4 | multiple physical action classes | yield | ABSTAIN | crates/nando-operator-proof/src/independent_verifier_v3/reconstruct/mod.rs |
| c5 | unique reconstructed action | precedes | independent reference execution | crates/nando-operator-proof/src/independent_verifier_v3/mod.rs |
