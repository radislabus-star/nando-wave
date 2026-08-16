# NANDA Split Worksheet

split_by: route
split_key: authorization
source: plans/effect-law-unification-v1/evidence/K2_SELF_FORMED_UNCERTAINTY_V5_PREFLIGHT/nanda-task-v5-authorization-nonce-generator.md

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| t1 | exact successor-root user authorization | freezes as | denied-authority authorization receipt | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:61 | 1.000 | authorization source | authorization receipt | authorization | authorization-owner |
| t10 | old-freeze authorization | cannot authorize | successor freeze slot | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:80 | 1.000 | superseded authorization | successor attempt | authorization | superseded-auth-owner |


## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|----|----|----|----|----|----|----|----|----|----|
| c1 | exact successor-root user authorization | freezes as | denied-authority authorization receipt | V5 R7H authorization-owner candidate | 1.000 | authorization source | authorization receipt | authorization | authorization-owner |
| c10 | old-freeze authorization | cannot authorize | successor freeze slot | V5 R7H root-mismatch candidate | 1.000 | superseded authorization | successor attempt | authorization | superseded-auth-owner |
