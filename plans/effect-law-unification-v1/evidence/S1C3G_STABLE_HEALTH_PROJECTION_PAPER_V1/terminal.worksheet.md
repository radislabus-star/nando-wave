# NANDA Triad Worksheet

task_id: s1c3g-terminal-v1
domain: code
query: Does S1C-3G preserve its parent and close every mutating failure path?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | S1C-3G identity | preserves | terminal S1C-3F roots | immutable parent | 1.0 | repair owner | historical evidence | terminal lifecycle | parent preservation | paper | S1C-3G | parent roots | unchanged parent | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:7 | parent |
| t2 | post-arming failure | transitions to | rollback pass or veto | terminal transition clause | 1.0 | failure transition | terminal state | terminal lifecycle | rollback | transaction | root | failure | terminal receipt | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:143 | mutation |
| t3 | rollback owner | preserves | opening prefixes and natural valid suffixes | journal preservation clause | 1.0 | recovery owner | evidence bytes | terminal lifecycle | evidence preservation | transaction | rollback | no evidence loss | plans/effect-law-unification-v1/S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md:145 | journal |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | S1C-3G identity | preserves | terminal S1C-3F roots | candidate c1 | 1.0 | repair owner | historical evidence | terminal lifecycle | parent preservation | conclusion | S1C-3G | parent roots | unchanged parent | candidate_answer:c1 | parent |
| c2 | post-arming failure | transitions to | rollback pass or veto | candidate c2 | 1.0 | failure transition | terminal state | terminal lifecycle | rollback | conclusion | root | failure | terminal receipt | candidate_answer:c2 | mutation |
| c3 | rollback owner | preserves | opening prefixes and natural valid suffixes | candidate c3 | 1.0 | recovery owner | evidence bytes | terminal lifecycle | evidence preservation | conclusion | rollback | no evidence loss | candidate_answer:c3 | journal |
