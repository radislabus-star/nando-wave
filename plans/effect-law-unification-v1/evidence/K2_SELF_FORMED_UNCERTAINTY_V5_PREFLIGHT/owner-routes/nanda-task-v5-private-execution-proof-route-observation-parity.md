# NANDA Split Worksheet

split_by: route
split_key: observation-parity
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-private-execution-proof.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t6 | read-only observer | inspects after worker exit | completed isolated workspace | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:348 | 1.000 | observation owner | post-state evidence | observation-parity | observer-owner |
| t7 | worker and observer outputs | require | exact outcome parity before append | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:349 | 1.000 | execution evidence | observation evidence | observation-parity | parity-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c6 | read-only observer | inspects after worker exit | completed isolated workspace | V5 R7I observer candidate | 1.000 | observation owner | post-state evidence | observation-parity | observer-owner |
| c7 | worker and observer outputs | require | exact outcome parity before append | V5 R7I parity candidate | 1.000 | execution evidence | observation evidence | observation-parity | parity-owner |
