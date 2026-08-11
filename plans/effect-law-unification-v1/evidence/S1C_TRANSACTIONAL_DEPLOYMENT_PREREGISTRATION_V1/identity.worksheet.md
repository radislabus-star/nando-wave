# NANDA Triad Worksheet

task_id: s1c3-identity
domain: general
query: Does S1C-3 bind one accepted source, one candidate binary, the current production identity, and an exact rollback source?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 candidate source | is_exactly | commit a3ea27a and tree 670d9c4 | frozen source identity | 1.0 | deployment source | source identity | identity | s1c3-identity | proof | S1C3 identity owner | source freeze | candidate source | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:60 | S1C-3 |
| s2 | S1C3 candidate binary hash | is_assigned_once_before | production mutation | immutable build identity | 1.0 | binary identity | mutation boundary | identity | s1c3-identity | chronology | S1C3 identity owner | clean release build | preparation receipt | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:94 | S1C-3 |
| s3 | paper-time production baseline | must_be_revalidated_before | production mutation | stale baseline rule | 1.0 | production identity | mutation boundary | identity | s1c3-identity | safety | S1C3 identity owner | preflight | stale or current | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:132 | S1C-3 |
| s4 | S1C3 rollback source | is_exactly | current deployed source 6639590 plus old binary bytes | rollback identity | 1.0 | rollback source | deployed identity | identity | s1c3-identity | recovery | S1C3 identity owner | rollback preparation | exact old pair | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:376 | S1C-3 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 candidate source | is_exactly | commit a3ea27a and tree 670d9c4 | critique source repair | 1.0 | deployment source | source identity | identity | s1c3-identity | proof | S1C3 identity owner | source freeze | candidate source | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:22 | S1C-3 |
| c2 | S1C3 candidate binary hash | is_assigned_once_before | production mutation | critique build binding | 1.0 | binary identity | mutation boundary | identity | s1c3-identity | chronology | S1C3 identity owner | clean release build | preparation receipt | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:22 | S1C-3 |
| c3 | paper-time production baseline | must_be_revalidated_before | production mutation | critique stale baseline repair | 1.0 | production identity | mutation boundary | identity | s1c3-identity | safety | S1C3 identity owner | preflight | stale or current | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:39 | S1C-3 |
| c4 | S1C3 rollback source | is_exactly | current deployed source 6639590 plus old binary bytes | critique rollback binding | 1.0 | rollback source | deployed identity | identity | s1c3-identity | recovery | S1C3 identity owner | rollback preparation | exact old pair | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:25 | S1C-3 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
