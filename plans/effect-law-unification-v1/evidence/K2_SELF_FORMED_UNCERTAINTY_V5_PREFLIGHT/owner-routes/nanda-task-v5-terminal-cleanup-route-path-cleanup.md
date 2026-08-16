# NANDA Split Worksheet

split_by: route
split_key: path-cleanup
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-terminal-cleanup.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t8 | terminal owner | freezes before cleanup | complete classified path manifest | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:548 | 1.000 | terminal owner | cleanup census | path-cleanup | classification-owner |
| t9 | cleanup owner | deletes only | disposable classified paths | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:557 bounded deletion | 1.000 | mutation owner | disposable artifacts | path-cleanup | deletion-owner |
| t13 | cleanup mutation owner | remains distinct from | read-only cleanup verifier | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:231 cleanup-owner boundary | 1.000 | cleanup mutation owner | cleanup proof owner | path-cleanup | cleanup-owner-separation |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c8 | terminal owner | freezes before cleanup | complete classified path manifest | V5 R7K classification candidate | 1.000 | terminal owner | cleanup census | path-cleanup | classification-owner |
| c9 | cleanup owner | deletes only | disposable classified paths | V5 R7K deletion candidate | 1.000 | mutation owner | disposable artifacts | path-cleanup | deletion-owner |
| c13 | cleanup mutation owner | remains distinct from | read-only cleanup verifier | V5 R7K cleanup-owner candidate | 1.000 | cleanup mutation owner | cleanup proof owner | path-cleanup | cleanup-owner-separation |
