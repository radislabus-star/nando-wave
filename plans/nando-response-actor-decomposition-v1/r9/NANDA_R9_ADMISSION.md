# R9 Admission Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | admission owner | imports | kernel and proof only | decomposition-plan#dependency-dag |
| s2 | admission owner | excludes | runtime and mutable learning | decomposition-plan#stop-r5 |
| s3 | admission owner | validates | immutable proof receipts | decomposition-plan#target-architecture |
| s4 | admission owner | alone grants | authority lease | decomposition-plan#authority-owner |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | admission owner | imports | kernel and proof only | crates/nando-operator-admission/Cargo.toml |
| c2 | admission owner | excludes | runtime and mutable learning | crates/nando-operator-admission/Cargo.toml |
| c3 | admission owner | validates | immutable proof receipts | crates/nando-operator-admission/src/package_policy.rs |
| c4 | admission owner | alone grants | authority lease | crates/nando-operator-admission/src/authority.rs |
