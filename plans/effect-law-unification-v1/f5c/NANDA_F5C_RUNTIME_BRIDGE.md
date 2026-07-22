# F5-C Runtime Bridge Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | immutable dispatch index | selects | at most 32 structurally relevant modes | F5_RUNTIME_CONVERGENCE_V1.md#structural-dispatch-before-binding |
| s2 | overfull dispatch bucket | yields | abstain without order truncation | F5_RUNTIME_CONVERGENCE_V1.md#runtime-budgets |
| s3 | runtime bridge | consumes | CanonicalRuntimeRequestV3 without provider payload rescan | F5_RUNTIME_CONVERGENCE_V1.md#incoming-request-contract |
| s4 | binding report | separates | source-candidate count, complete mappings, and phase winners | F5_RUNTIME_CONVERGENCE_V1.md#binder-must-not-hide-the-structural-version-space |
| s5 | F5-C runtime bridge | grants | no capability binding, action, verifier, or authority | F5_RUNTIME_CONVERGENCE_V1.md#canonical-ownership-boundary |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | immutable dispatch index | selects | at most 32 structurally relevant modes | crates/nando-operator-runtime/src/mode_to_role_v3/dispatch_index.rs |
| c2 | overfull dispatch bucket | yields | abstain without order truncation | crates/nando-operator-runtime/src/mode_to_role_v3/dispatch.rs |
| c3 | runtime bridge | consumes | CanonicalRuntimeRequestV3 without provider payload rescan | crates/nando-operator-runtime/src/mode_to_role_v3/binding.rs |
| c4 | binding report | separates | source-candidate count, complete mappings, and phase winners | crates/nando-operator-runtime/src/mode_to_role_v3/report.rs |
| c5 | F5-C runtime bridge | grants | no capability binding, action, verifier, or authority | crates/nando-operator-runtime/src/mode_to_role_v3/mod.rs |
