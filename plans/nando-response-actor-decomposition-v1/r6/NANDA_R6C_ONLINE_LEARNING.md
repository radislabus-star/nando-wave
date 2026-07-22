# R6-C Online Learning Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | verified transition evidence | updates | bounded learning state | decomposition-plan#r6 |
| s2 | CEGIS winner | freezes | support and future generation | decomposition-plan#rollover |
| s3 | positive and negative atoms | induce | learned Wave route | decomposition-plan#wave-learning |
| s4 | learning checkpoint and reports | exclude | execution authority | decomposition-plan#authority |
| s5 | learning owner | excludes | runtime and admission imports | decomposition-plan#dependency-dag |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | verified transition evidence | updates | bounded learning state | crates/nando-operator-learning/src/cegis.rs#CegisCoordinator |
| c2 | CEGIS winner | freezes | support and future generation | crates/nando-operator-learning/src/rollover.rs#freeze_generation |
| c3 | positive and negative atoms | induce | learned Wave route | crates/nando-operator-learning/src/wave_route_learning.rs#learned_wave_route_from_support_medoid |
| c4 | learning checkpoint and reports | exclude | execution authority | crates/nando-operator-learning/src/lib.rs#contract |
| c5 | learning owner | excludes | runtime and admission imports | crates/nando-operator-learning/Cargo.toml#dependencies |
