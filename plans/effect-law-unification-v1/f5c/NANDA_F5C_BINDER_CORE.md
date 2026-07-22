# F5-C Binder Core Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | RuntimeRoleBinder | exposes | complete exact structural mapping set before phase pruning | F5_RUNTIME_CONVERGENCE_V1.md#binder-must-not-hide-the-structural-version-space |
| s2 | exact-cap search | reports | complete only when no frontier remains | F5_RUNTIME_CONVERGENCE_V1.md#search-completion-at-the-exact-cap |
| s3 | phase ranking | selects from | already valid structural mappings | F5_RUNTIME_CONVERGENCE_V1.md#wave-position |
| s4 | incomplete role search | yields | exhausted without a partial mapping claim | F5_RUNTIME_CONVERGENCE_V1.md#runtime-budgets |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | RuntimeRoleBinder | exposes | complete exact structural mapping set before phase pruning | crates/nando-core/src/wave/operator_blueprint.rs |
| c2 | exact-cap search | reports | complete only when no frontier remains | crates/nando-core/src/wave/operator_blueprint.rs |
| c3 | phase ranking | selects from | already valid structural mappings | crates/nando-core/src/wave/operator_blueprint.rs |
| c4 | incomplete role search | yields | exhausted without a partial mapping claim | crates/nando-core/src/wave/operator_blueprint.rs |
