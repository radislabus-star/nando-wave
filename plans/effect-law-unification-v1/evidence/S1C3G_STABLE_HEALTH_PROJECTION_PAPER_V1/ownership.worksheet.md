# NANDA Triad Worksheet

task_id: s1c3g-ownership-v1
domain: code
query: Does S1C-3G keep stable health, process replacement and capture availability under separate owners?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | stable projection | verifies | serving admission and route invariants | endpoint contract | 1.0 | parity owner | stable service state | runtime ownership | stable parity | proof | projection function | health endpoints | equality verdict | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:43 | health |
| t2 | service lifecycle owner | verifies | candidate PID replacement and survival | process replacement clause | 1.0 | process owner | process identity | runtime ownership | process lifecycle | transaction | systemd checks | PID verdict | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:103 | process |
| t3 | capture owner | verifies | environment writer journal and startup log | capture availability clause | 1.0 | capture owner | capture availability | runtime ownership | capture install | transaction | candidate runtime | availability verdict | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:104 | capture |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | stable projection | verifies | serving admission and route invariants | candidate c1 | 1.0 | parity owner | stable service state | runtime ownership | stable parity | conclusion | projection function | health endpoints | equality verdict | candidate_answer:c1 | health |
| c2 | service lifecycle owner | verifies | candidate PID replacement and survival | candidate c2 | 1.0 | process owner | process identity | runtime ownership | process lifecycle | conclusion | systemd checks | PID verdict | candidate_answer:c2 | process |
| c3 | capture owner | verifies | environment writer journal and startup log | candidate c3 | 1.0 | capture owner | capture availability | runtime ownership | capture install | conclusion | candidate runtime | availability verdict | candidate_answer:c3 | capture |
