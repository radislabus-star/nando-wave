# R6-B Learning Isolation

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | learning owner | imports | kernel core and proof | decomposition-plan#dependency-dag |
| s2 | learning owner | excludes | hot runtime and admission | decomposition-plan#hard-vetoes |
| s3 | response facade | preserves | public compiler paths | decomposition-plan#compatibility |
| s4 | learning candidate | does not grant | execution authority | decomposition-plan#r6 |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | learning owner | imports | kernel core and proof | crates/nando-operator-learning/Cargo.toml#dependencies |
| c2 | learning owner | excludes | hot runtime and admission | crates/nando-operator-learning/Cargo.toml#dependencies |
| c3 | response facade | preserves | public compiler paths | crates/nando-response-actor/src/protocol_mode.rs#facade |
| c4 | learning candidate | does not grant | execution authority | crates/nando-operator-learning/src/lib.rs#contract |
