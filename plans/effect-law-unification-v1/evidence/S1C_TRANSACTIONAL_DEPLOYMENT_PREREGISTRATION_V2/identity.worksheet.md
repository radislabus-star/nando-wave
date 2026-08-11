# NANDA Triad Worksheet

task_id: s1c3-v2-identity
domain: general
query: Does S1C-3 V2 preserve the exact candidate, baseline, executable identities, and stale-before-deployment boundary?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 V2 candidate | is_exactly | commit a3ea27a and tree 670d9c4 | frozen identity | 1.0 | deployment source | source identity | identity | s1c3-v2-identity | proof | S1C3 identity owner | source freeze | one candidate | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:40 | S1C-3 V2 |
| s2 | S1C3 V2 production baseline | must_be_revalidated_before | any production mutation | stale baseline rule | 1.0 | deployed runtime | mutation boundary | identity | s1c3-v2-identity | safety | S1C3 identity owner | preflight | current or stale | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:75 | S1C-3 V2 |
| s3 | S1C3 V2 executable set | is_bound_before | quiescence eligibility | executable hash contract | 1.0 | measurement executables | eligibility boundary | identity | s1c3-v2-identity | measurement | S1C3 identity owner | clean builds | frozen executables | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:100 | S1C-3 V2 |
| s4 | changed frozen executable | yields | VETO before measurement | immutable executable rule | 1.0 | executable drift | terminal guard | identity | s1c3-v2-identity | safety | S1C3 identity owner | hash verification | VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md:182 | S1C-3 V2 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 V2 candidate | is_exactly | commit a3ea27a and tree 670d9c4 | critique same-candidate verdict | 1.0 | deployment source | source identity | identity | s1c3-v2-identity | proof | S1C3 identity owner | source freeze | one candidate | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:78 | S1C-3 V2 |
| c2 | S1C3 V2 production baseline | must_be_revalidated_before | any production mutation | critique owner boundary | 1.0 | deployed runtime | mutation boundary | identity | s1c3-v2-identity | safety | S1C3 identity owner | preflight | current or stale | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:29 | S1C-3 V2 |
| c3 | S1C3 V2 executable set | is_bound_before | quiescence eligibility | critique prebuild repair | 1.0 | measurement executables | eligibility boundary | identity | s1c3-v2-identity | measurement | S1C3 identity owner | clean builds | frozen executables | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:18 | S1C-3 V2 |
| c4 | changed frozen executable | yields | VETO before measurement | critique rebuild rejection | 1.0 | executable drift | terminal guard | identity | s1c3-v2-identity | safety | S1C3 identity owner | hash verification | VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md:24 | S1C-3 V2 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
