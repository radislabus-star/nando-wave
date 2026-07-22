# R9 Learning Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | learning owner | imports | kernel and proof | decomposition-plan#dependency-dag |
| s2 | learning owner | excludes | runtime and admission | decomposition-plan#stop-r6 |
| s3 | learning owner | emits | untrusted operator candidates | decomposition-plan#target-architecture |
| s4 | learning candidate | does not grant | execution authority | decomposition-plan#pause-contract |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | learning owner | imports | kernel and proof | crates/nando-operator-learning/Cargo.toml |
| c2 | learning owner | excludes | runtime and admission | crates/nando-operator-learning/Cargo.toml |
| c3 | learning owner | emits | untrusted operator candidates | crates/nando-operator-learning/src/operator_generation.rs |
| c4 | learning candidate | does not grant | execution authority | crates/nando-operator-learning/src/lib.rs |
