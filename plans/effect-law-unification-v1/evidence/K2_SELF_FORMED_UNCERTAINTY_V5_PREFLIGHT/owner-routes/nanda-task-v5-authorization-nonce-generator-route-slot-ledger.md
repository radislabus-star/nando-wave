# NANDA Split Worksheet

split_by: route
split_key: slot-ledger
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-authorization-nonce-generator.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t2 | experiment freeze tuple | admits exactly one | append-only slot claim | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:86 | 1.000 | frozen experiment identity | slot ledger event | slot-ledger | slot-key-owner |
| t3 | durable slot claim | precedes | exclusive attempt-directory creation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:139 | 1.000 | slot authority | attempt container | slot-ledger | slot-mutation-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c2 | experiment freeze tuple | admits exactly one | append-only slot claim | V5 R7H global-slot candidate | 1.000 | frozen experiment identity | slot ledger event | slot-ledger | slot-key-owner |
| c3 | durable slot claim | precedes | exclusive attempt-directory creation | V5 R7H journal-before-mkdir candidate | 1.000 | slot authority | attempt container | slot-ledger | slot-mutation-owner |
