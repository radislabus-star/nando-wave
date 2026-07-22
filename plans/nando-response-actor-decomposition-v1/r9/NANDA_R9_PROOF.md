# R9 Proof Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | proof owner | imports | immutable kernel contracts | decomposition-plan#dependency-dag |
| s2 | proof owner | excludes | runtime learning and admission | decomposition-plan#stop-r3 |
| s3 | proof owner | recomputes | expected consequences from bounded evidence | decomposition-plan#canonical-corrections |
| s4 | proof result | does not grant | execution authority | decomposition-plan#authority-owner |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | proof owner | imports | immutable kernel contracts | crates/nando-operator-proof/Cargo.toml |
| c2 | proof owner | excludes | runtime learning and admission | crates/nando-operator-proof/Cargo.toml |
| c3 | proof owner | recomputes | expected consequences from bounded evidence | crates/nando-operator-proof/src/verifier.rs |
| c4 | proof result | does not grant | execution authority | crates/nando-operator-proof/src/lib.rs |
