# NANDA Triad Worksheet

task_id: s1c3e-rollback-v1
domain: code
query: Does S1C-3E rollback preserve both production and any natural row?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | durable rollback state | precedes | directory and binary mutation | transaction boundary | 1.0 | recovery owner | production mutation | rollback | recovery | transaction | root | ROLLBACK_ARMED | crash recovery | plans/effect-law-unification-v1/S1C3E_JOURNAL_OWNERSHIP_REPAIR_PREREGISTRATION_V1.md:126 | forward |
| t2 | rollback cleanup | removes | exact three zero-byte startup segments only | empty-journal cleanup clause | 1.0 | recovery owner | empty operational artifact | rollback | cleanup-empty | transaction | root | failed forward | clean baseline | plans/effect-law-unification-v1/S1C3E_JOURNAL_OWNERSHIP_REPAIR_PREREGISTRATION_V1.md:143 | empty journal |
| t3 | rollback cleanup | preserves | any natural append or foreign entry | natural-row preservation clause | 1.0 | recovery owner | natural evidence | rollback | preserve-nonempty | transaction | root | nonempty journal | evidence retained | plans/effect-law-unification-v1/S1C3E_JOURNAL_OWNERSHIP_REPAIR_PREREGISTRATION_V1.md:145 | nonempty journal |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | durable rollback state | precedes | directory and binary mutation | candidate c1 | 1.0 | recovery owner | production mutation | rollback | recovery | conclusion | root | ROLLBACK_ARMED | crash recovery | candidate_answer:c1 | forward |
| c2 | rollback cleanup | removes | exact three zero-byte startup segments only | candidate c2 | 1.0 | recovery owner | empty operational artifact | rollback | cleanup-empty | conclusion | root | failed forward | clean baseline | candidate_answer:c2 | empty journal |
| c3 | rollback cleanup | preserves | any natural append or foreign entry | candidate c3 | 1.0 | recovery owner | natural evidence | rollback | preserve-nonempty | conclusion | root | nonempty journal | evidence retained | candidate_answer:c3 | nonempty journal |
