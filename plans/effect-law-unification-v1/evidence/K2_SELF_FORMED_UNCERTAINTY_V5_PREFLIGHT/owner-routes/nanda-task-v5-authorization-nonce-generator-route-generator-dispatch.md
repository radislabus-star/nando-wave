# NANDA Split Worksheet

split_by: route
split_key: generator-dispatch
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-authorization-nonce-generator.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t6 | GENERATOR_DISPATCHED event | precedes | first anonymous-pipe request byte | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:151 | 1.000 | irreversible dispatch marker | generator input | generator-dispatch | dispatch-owner |
| t7 | dispatched generator without complete split result | terminates as | generator-result indeterminate without rerun | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:168 | 1.000 | crash prefix | terminal result | generator-dispatch | restart-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c6 | GENERATOR_DISPATCHED event | precedes | first anonymous-pipe request byte | V5 R7H irreversible-send candidate | 1.000 | irreversible dispatch marker | generator input | generator-dispatch | dispatch-owner |
| c7 | dispatched generator without complete split result | terminates as | generator-result indeterminate without rerun | V5 R7H restart-projection candidate | 1.000 | crash prefix | terminal result | generator-dispatch | restart-owner |
