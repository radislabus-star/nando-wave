# R9 Kernel Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | operator kernel | owns | immutable operator contracts | decomposition-plan#target-architecture |
| s2 | operator kernel | excludes | learning runtime proof and admission | decomposition-plan#dependency-dag |
| s3 | operator kernel | performs | no filesystem network or authority side effects | decomposition-plan#stop-r2 |
| s4 | response facade | re-exports | kernel compatibility paths | decomposition-plan#r7 |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | operator kernel | owns | immutable operator contracts | crates/nando-operator-kernel/src/lib.rs |
| c2 | operator kernel | excludes | learning runtime proof and admission | crates/nando-operator-kernel/Cargo.toml |
| c3 | operator kernel | performs | no filesystem network or authority side effects | crates/nando-operator-kernel/src |
| c4 | response facade | re-exports | kernel compatibility paths | crates/nando-response-actor/src/lib.rs |
