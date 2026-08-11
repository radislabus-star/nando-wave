# NANDA Triad Worksheet

task_id: s1c3-rollback-evidence
domain: general
query: Does S1C-3 restore exact old serving bytes without deleting or promoting forward journal evidence?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 rollback | restores_exactly | old binary and old role config | rollback order | 1.0 | recovery action | old runtime pair | rollback | s1c3-rollback | recovery | S1C3 rollback owner | rollback | old service | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:355 | S1C-3 |
| s2 | S1C3 rollback | preserves | every forward journal byte prefix | evidence preservation | 1.0 | recovery action | forward evidence | rollback | s1c3-rollback | durability | S1C3 rollback owner | rollback manifest | nonshrinking journal | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:366 | S1C-3 |
| s3 | post-start journal tree root | has_scope | operational deployment observation only | claim boundary | 1.0 | journal observation | evidence scope | rollback | s1c3-rollback | authority | S1C3 rollback owner | journal manifest | non-scientific root | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:337 | S1C-3 |
| s4 | rollback with lost journal bytes | yields | S1C3_VETO | failure classification | 1.0 | failed rollback | terminal verdict | rollback | s1c3-rollback | safety | S1C3 rollback owner | rollback verifier | VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:371 | S1C-3 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 rollback | restores_exactly | old binary and old role config | critique rollback binding | 1.0 | recovery action | old runtime pair | rollback | s1c3-rollback | recovery | S1C3 rollback owner | rollback | old service | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:25 | S1C-3 |
| c2 | S1C3 rollback | preserves | every forward journal byte prefix | critique evidence repair | 1.0 | recovery action | forward evidence | rollback | s1c3-rollback | durability | S1C3 rollback owner | rollback manifest | nonshrinking journal | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:26 | S1C-3 |
| c3 | post-start journal tree root | has_scope | operational deployment observation only | critique root boundary | 1.0 | journal observation | evidence scope | rollback | s1c3-rollback | authority | S1C3 rollback owner | journal manifest | non-scientific root | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:32 | S1C-3 |
| c4 | rollback with lost journal bytes | yields | S1C3_VETO | critique fail-closed repair | 1.0 | failed rollback | terminal verdict | rollback | s1c3-rollback | safety | S1C3 rollback owner | rollback verifier | VETO | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:26 | S1C-3 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
