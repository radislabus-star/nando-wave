# NANDA Triad Worksheet

task_id: s1c3g-attempt-authority-v1
domain: code
query: Does the S1C-3G terminal packet preserve the frozen one-attempt authority boundary?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | S1C-3G paper | permits | exactly one production transaction | frozen attempt discipline | 1.0 | paper authority | attempt budget | authority | attempt-authority | proof | terminal authority projection | preregistration | one attempt | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:188 | S1C-3G only |
| t2 | S1C-3G state | records | terminal rollback pass | immutable state root and transaction ID | 1.0 | terminal state | attempt result | authority | attempt-authority | evidence | terminal authority projection | sealed state | consumed result | plans/effect-law-unification-v1/evidence/S1C3G_CAPTURE_INSTALLATION_TERMINAL_V1/s1c3g-state.json:1 | S1C-3G only |
| t3 | terminal report | forbids | S1C-3G rerun | one attempt is consumed | 1.0 | terminal result | future attempt | authority | attempt-authority | conclusion | terminal authority projection | terminal report | route closed | plans/effect-law-unification-v1/S1C3G_CAPTURE_INSTALLATION_TERMINAL_REPORT_2026-08-12.md:87 | S1C-3G only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C-3G paper | permits | exactly one production transaction | terminal report boundary | 1.0 | paper authority | attempt budget | authority | attempt-authority | conclusion | terminal authority projection | terminal report | one attempt | candidate_answer:c1 | S1C-3G only |
| c2 | S1C-3G state | records | terminal rollback pass | terminal report boundary | 1.0 | terminal state | attempt result | authority | attempt-authority | conclusion | terminal authority projection | terminal report | consumed result | candidate_answer:c2 | S1C-3G only |
| c3 | terminal report | forbids | S1C-3G rerun | terminal report boundary | 1.0 | terminal result | future attempt | authority | attempt-authority | conclusion | terminal authority projection | terminal report | route closed | candidate_answer:c3 | S1C-3G only |
