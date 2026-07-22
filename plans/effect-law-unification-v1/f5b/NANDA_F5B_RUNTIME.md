# F5-B Runtime Context Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | runtime context owner | extracts | one bounded pre-action context per request | F5_RUNTIME_CONVERGENCE_V1.md#f5-b-canonical-runtime-context |
| s2 | runtime context | borrows | provider payload without durable raw values | F5_RUNTIME_CONVERGENCE_V1.md#incoming-request-contract |
| s3 | physical capability symbol | remains | ephemeral request-local binding data | F5_RUNTIME_CONVERGENCE_V1.md#commitment-is-not-executable-payload |
| s4 | exhausted extraction | yields | abstain without semantic authority | F5_RUNTIME_CONVERGENCE_V1.md#runtime-budgets |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | runtime context owner | extracts | one bounded pre-action context per request | crates/nando-operator-runtime/src/runtime_context_v3.rs |
| c2 | runtime context | borrows | provider payload without durable raw values | crates/nando-operator-runtime/src/runtime_context_v3.rs |
| c3 | physical capability symbol | remains | ephemeral request-local binding data | crates/nando-operator-runtime/src/runtime_context_v3.rs |
| c4 | exhausted extraction | yields | abstain without semantic authority | crates/nando-operator-runtime/src/runtime_context_v3.rs |
