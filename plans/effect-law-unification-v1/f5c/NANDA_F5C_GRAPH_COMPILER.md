# F5-C Graph Compiler Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | executable mode artifact | compiles into | existing RoleGraph and OperatorCircuit vocabulary | F5_RUNTIME_CONVERGENCE_V1.md#f5-c-mode-to-role-compilation |
| s2 | selector predicate | becomes | typed structural role and relation constraint | F5_RUNTIME_CONVERGENCE_V1.md#binding-and-action-collapse |
| s3 | tampered or contradictory artifact | yields | reject before index publication | F5_RUNTIME_CONVERGENCE_V1.md#f5-c-mode-to-role-compilation |
| s4 | graph compiler | grants | no execution authority | F5_RUNTIME_CONVERGENCE_V1.md#canonical-ownership-boundary |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | executable mode artifact | compiles into | existing RoleGraph and OperatorCircuit vocabulary | crates/nando-operator-runtime/src/mode_to_role_v3/compiler.rs |
| c2 | selector predicate | becomes | typed structural role and relation constraint | crates/nando-operator-runtime/src/mode_to_role_v3/constraint.rs |
| c3 | tampered or contradictory artifact | yields | reject before index publication | crates/nando-operator-runtime/src/mode_to_role_v3/compiler.rs |
| c4 | graph compiler | grants | no execution authority | crates/nando-operator-runtime/src/mode_to_role_v3/mod.rs |
