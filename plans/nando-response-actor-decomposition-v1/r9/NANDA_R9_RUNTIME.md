# R9 Runtime Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | runtime owner | imports | immutable kernel contracts | decomposition-plan#dependency-dag |
| s2 | runtime owner | excludes | learning proof and admission | decomposition-plan#stop-r4 |
| s3 | runtime owner | binds and executes | operator artifact | decomposition-plan#target-architecture |
| s4 | runtime result | requires | independent verifier before authority | decomposition-plan#canonical-corrections |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | runtime owner | imports | immutable kernel contracts | crates/nando-operator-runtime/Cargo.toml |
| c2 | runtime owner | excludes | learning proof and admission | crates/nando-operator-runtime/Cargo.toml |
| c3 | runtime owner | binds and executes | operator artifact | crates/nando-operator-runtime/src/runtime.rs |
| c4 | runtime result | requires | independent verifier before authority | crates/nando-response-actor/src/crystallized_operator.rs |
