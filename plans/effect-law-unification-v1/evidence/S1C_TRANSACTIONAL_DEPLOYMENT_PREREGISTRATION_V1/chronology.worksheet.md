# NANDA Triad Worksheet

task_id: s1c3-chronology
domain: general
query: Does S1C-3 prevent a torn binary/config runtime and attribute exactly one intentional service restart?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| s1 | S1C3 rollback | is_armed_before | transition-serving stop | rollback-arm chronology | 1.0 | recovery guard | service stop | chronology | s1c3-chronology | recovery | S1C3 transaction owner | preparation | rollback armed | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:282 | S1C-3 |
| s2 | transition-serving | is_stopped_before | candidate binary and config swap | reader-stop chronology | 1.0 | runtime process | pair swap | chronology | s1c3-chronology | runtime | S1C3 transaction owner | systemctl stop | no reader | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:284 | S1C-3 |
| s3 | candidate temporary files | are_fsynced_and_verified_before | final rename | candidate-fsync chronology | 1.0 | candidate pair | final install | chronology | s1c3-chronology | persistence | S1C3 transaction owner | temp install | verified pair | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:286 | S1C-3 |
| s4 | intended transition restart | changes_exactly_once_with | unchanged NRestarts | PID-attribution chronology | 1.0 | restart event | process counter | chronology | s1c3-chronology | attribution | S1C3 transaction owner | start and survival | one attributed restart | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md:304 | S1C-3 |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C3 rollback | is_armed_before | transition-serving stop | critique rollback-arm evidence | 1.0 | recovery guard | service stop | chronology | s1c3-chronology | recovery | S1C3 transaction owner | preparation | rollback armed | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:129 | S1C-3 |
| c2 | transition-serving | is_stopped_before | candidate binary and config swap | critique reader-stop evidence | 1.0 | runtime process | pair swap | chronology | s1c3-chronology | runtime | S1C3 transaction owner | systemctl stop | no reader | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:131 | S1C-3 |
| c3 | candidate temporary files | are_fsynced_and_verified_before | final rename | critique candidate-fsync evidence | 1.0 | candidate pair | final install | chronology | s1c3-chronology | persistence | S1C3 transaction owner | temp install | verified pair | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:133 | S1C-3 |
| c4 | intended transition restart | changes_exactly_once_with | unchanged NRestarts | critique PID-attribution evidence | 1.0 | restart event | process counter | chronology | s1c3-chronology | attribution | S1C3 transaction owner | start and survival | one attributed restart | plans/effect-law-unification-v1/S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1_CRITIQUE.md:135 | S1C-3 |

## notes

- Structural PASS is coherence-only and must retain authority_ready false.
