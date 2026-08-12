# NANDA Triad Worksheet

task_id: s1c3c-terminal-outcome-v1
domain: code
query: Does the S1C-3C postmortem preserve the consumed operational RESOURCE_VETO?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | sole S1C-3C attempt | terminates_as | RESOURCE_VETO | durable transaction state and resource root | 1.0 | transaction attempt | terminal outcome | postmortem | terminal-outcome | evidence | resource mechanism owner | frozen resource gate | veto | plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_POSTMORTEM_V1/POSTMORTEM_VERIFICATION_V1.json:1 | exact attempt |
| t2 | resource receipt | records | two settlement-p99 failures and parity byte-identity failure | exact bounded failure rows | 1.0 | resource mechanism | resource failures | postmortem | terminal-outcome | evidence | resource mechanism owner | frozen resource gate | veto reasons | plans/effect-law-unification-v1/evidence/S1C3C_CAPTURE_INSTALLATION_POSTMORTEM_V1/POSTMORTEM_VERIFICATION_V1.json:1 | exact attempt |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | sole S1C-3C attempt | terminates_as | RESOURCE_VETO | terminal report | 1.0 | transaction attempt | terminal outcome | postmortem | terminal-outcome | conclusion | resource mechanism owner | terminal report | veto | candidate_answer:c1 | exact attempt |
| c2 | resource receipt | records | two settlement-p99 failures and parity byte-identity failure | terminal report | 1.0 | resource mechanism | resource failures | postmortem | terminal-outcome | conclusion | resource mechanism owner | terminal report | veto reasons | candidate_answer:c2 | exact attempt |
