# NANDA Triad Worksheet

task_id: s1c3g-projection-v1
domain: code
query: Does S1C-3G compare endpoint-owned stable projections instead of whole runtime objects?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | endpoint projection owner | selects | endpoint-specific stable fields | frozen endpoint projections | 1.0 | comparison owner | stable runtime state | health projection | stable fields | proof | S1C-3G | health snapshot | exact projection | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:43 | all endpoints |
| t2 | endpoint projection owner | excludes from equality | raw hashes counters timestamps and transition profile telemetry | observed-only clauses | 1.0 | comparison owner | dynamic observations | health projection | observed only | proof | S1C-3G | health snapshot | no false mismatch | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:58 | dynamic fields |
| t3 | projection contract | forbids | whole-object equality and wildcard projection | comparison prohibition | 1.0 | contract | unsafe comparison | health projection | fail closed | proof | S1C-3G | preflight | blocker | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:99 | implementation |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | endpoint projection owner | selects | endpoint-specific stable fields | candidate c1 | 1.0 | comparison owner | stable runtime state | health projection | stable fields | conclusion | S1C-3G | health snapshot | exact projection | candidate_answer:c1 | all endpoints |
| c2 | endpoint projection owner | excludes from equality | raw hashes counters timestamps and transition profile telemetry | candidate c2 | 1.0 | comparison owner | dynamic observations | health projection | observed only | conclusion | S1C-3G | health snapshot | no false mismatch | candidate_answer:c2 | dynamic fields |
| c3 | projection contract | forbids | whole-object equality and wildcard projection | candidate c3 | 1.0 | contract | unsafe comparison | health projection | fail closed | conclusion | S1C-3G | preflight | blocker | candidate_answer:c3 | implementation |
