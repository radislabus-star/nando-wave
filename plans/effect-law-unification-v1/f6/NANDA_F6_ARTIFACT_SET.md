# F6 Artifact Set Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | immutable executable artifacts | validate into | cold digest-bound verifier set | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#objective |
| s2 | duplicate or invalid artifact | prevents | verifier set construction | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#stop-f6-gate |
| s3 | per-request verifier | consumes | prevalidated immutable set | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#objective |
| s4 | proof crate normal dependencies | exclude | operator runtime | F6_INDEPENDENT_VERIFIER_CONVERGENCE_V1.md#objective |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | immutable executable artifacts | validate into | cold digest-bound verifier set | crates/nando-operator-proof/src/independent_verifier_v3/artifact_set.rs |
| c2 | duplicate or invalid artifact | prevents | verifier set construction | crates/nando-operator-proof/src/independent_verifier_v3/artifact_set.rs |
| c3 | per-request verifier | consumes | prevalidated immutable set | crates/nando-operator-proof/src/independent_verifier_v3/input.rs |
| c4 | proof crate normal dependencies | exclude | operator runtime | crates/nando-operator-proof/Cargo.toml |
