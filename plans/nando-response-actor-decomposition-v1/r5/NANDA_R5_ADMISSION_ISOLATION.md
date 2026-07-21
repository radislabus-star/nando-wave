# R5 Admission Isolation

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | admission owner | imports | kernel and proof only | decomposition-plan#dependency-dag |
| s2 | admission owner | excludes | runtime and mutable learning | decomposition-plan#hard-vetoes |
| s3 | response facade | derives | immutable package projection | decomposition-plan#r5 |
| s4 | admission owner | decides | authority binding | decomposition-plan#owner-contract |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | admission owner | imports | kernel and proof only | crates/nando-operator-admission/Cargo.toml#dependencies |
| c2 | admission owner | excludes | runtime and mutable learning | crates/nando-operator-admission/Cargo.toml#dependencies |
| c3 | response facade | derives | immutable package projection | crates/nando-response-actor/src/authority.rs#validate_response_authority |
| c4 | admission owner | decides | authority binding | crates/nando-operator-admission/src/authority.rs#build_composite_response_admission |
