# NANDA Triad Worksheet

task_id: s1c3b-attempt-authority-v1
domain: code
query: Does the terminal report preserve the frozen one-attempt authority boundary?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | frozen S1C-3B paper | permits | exactly one remote transaction | terminal outcomes and attempt budget | 1.0 | paper authority | attempt budget | authority | attempt-authority | proof | paper owner | frozen contract | one attempt | plans/effect-law-unification-v1/S1C3B_PRODUCTION_LOAD_ABSOLUTE_GATE_PREREGISTRATION_V1.md:309 | S1C-3B only |
| t2 | remote transaction directory | records | one S1C-3B attempt | unique frozen transaction identifier | 1.0 | consumed attempt | attempt budget | authority | attempt-authority | evidence | paper owner | durable transaction state | count one | plans/effect-law-unification-v1/evidence/S1C3B_PRODUCTION_LOAD_ATTEMPT_V1/20260812T093629Z-36ffc0cbf56b-s1c3b-v1/transaction-state.json:1 | S1C-3B only |
| t3 | terminal attempt | forbids | retry or automatic S1C-3C | no automatic next stage in frozen paper | 1.0 | consumed attempt | future transaction | authority | attempt-authority | proof | paper owner | terminal result | route closed | plans/effect-law-unification-v1/S1C3B_PRODUCTION_LOAD_ABSOLUTE_GATE_PREREGISTRATION_V1.md:328 | S1C-3B only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | frozen S1C-3B paper | permits | exactly one remote transaction | terminal report boundary | 1.0 | paper authority | attempt budget | authority | attempt-authority | conclusion | paper owner | terminal report | one attempt | candidate_answer:c1 | S1C-3B only |
| c2 | remote transaction directory | records | one S1C-3B attempt | terminal report boundary | 1.0 | consumed attempt | attempt budget | authority | attempt-authority | conclusion | paper owner | terminal report | count one | candidate_answer:c2 | S1C-3B only |
| c3 | terminal attempt | forbids | retry or automatic S1C-3C | terminal report boundary | 1.0 | consumed attempt | future transaction | authority | attempt-authority | conclusion | paper owner | terminal report | route closed | candidate_answer:c3 | S1C-3B only |
