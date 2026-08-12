# NANDA Triad Worksheet

task_id: s1c3b-implementation-rollback-v1
domain: code
query: Does the S1C-3B runner preserve one-attempt chronology and rollback coverage?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | transaction runner | requires_before | committed pushed bytes and zero prior attempts | runner lines 71-119 | 1.0 | transaction orchestrator | attempt boundary | rollback | rollback-runner | operations | transaction runner | git and deployment state | one attempt | ops/remote-backend/run_s1c3b_transaction_v1.sh:71 | deployment only |
| t2 | transaction runner | arms_before | remote execute mutation | runner lines 229-275 | 1.0 | transaction orchestrator | rollback boundary | rollback | rollback-runner | operations | transaction runner | verified preparation | armed trap | ops/remote-backend/run_s1c3b_transaction_v1.sh:229 | production safety |
| t3 | transaction runner | retains_arm_during | unexpected post-mutation state | runner lines 280-297 | 1.0 | transaction orchestrator | failure state | rollback | rollback-runner | operations | transaction runner | state read | emergency rollback | ops/remote-backend/run_s1c3b_transaction_v1.sh:280 | production safety |
| t4 | transaction runner | disarms_after | verified seal and remote COMPLETE | runner lines 313-337 | 1.0 | transaction orchestrator | terminal state | rollback | rollback-runner | operations | transaction runner | two final verifiers | complete state | ops/remote-backend/run_s1c3b_transaction_v1.sh:313 | production safety |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | transaction runner | requires_before | committed pushed bytes and zero prior attempts | candidate claim c1 | 1.0 | transaction orchestrator | attempt boundary | rollback | rollback-runner | operations | transaction runner | git and deployment state | one attempt | candidate_answer:c1 | deployment only |
| c2 | transaction runner | arms_before | remote execute mutation | candidate claim c2 | 1.0 | transaction orchestrator | rollback boundary | rollback | rollback-runner | operations | transaction runner | verified preparation | armed trap | candidate_answer:c2 | production safety |
| c3 | transaction runner | retains_arm_during | unexpected post-mutation state | candidate claim c3 | 1.0 | transaction orchestrator | failure state | rollback | rollback-runner | operations | transaction runner | state read | emergency rollback | candidate_answer:c3 | production safety |
| c4 | transaction runner | disarms_after | verified seal and remote COMPLETE | candidate claim c4 | 1.0 | transaction orchestrator | terminal state | rollback | rollback-runner | operations | transaction runner | two final verifiers | complete state | candidate_answer:c4 | production safety |
