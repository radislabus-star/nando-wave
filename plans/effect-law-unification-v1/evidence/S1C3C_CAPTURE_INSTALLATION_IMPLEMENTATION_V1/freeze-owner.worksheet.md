# NANDA Triad Worksheet

task_id: s1c3c-implementation-freeze-owner-v1
domain: code
query: Does the implementation freeze owner bind committed source and uploaded successor bytes before measurement?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | implementation freeze verifier | binds | bundle-derived source commit tree frozen paper receipts config and five successor files | create implementation freeze and bundle identity | 1.0 | freeze decision owner | immutable implementation identity | freeze | freeze-owner | proof | implementation freeze verifier | create_implementation_freeze | rooted freeze receipt | ops/remote-backend/verify_s1c3c_transaction_v1.py:148 | successor only |
| t2 | implementation freeze verifier | rejects | missing changed or foreign successor bytes | verify implementation freeze | 1.0 | freeze decision owner | tamper input | freeze | freeze-owner | proof | implementation freeze verifier | verify_implementation_freeze | fail closed | ops/remote-backend/verify_s1c3c_transaction_v1.py:140 | successor only |
| t3 | implementation freeze verifier | supplies | implementation root to authority envelope | build envelope binding | 1.0 | freeze decision owner | operational verifier input | freeze | freeze-owner | proof | implementation freeze verifier | verified freeze | exact root | ops/remote-backend/verify_s1c3c_transaction_v1.py:242 | successor only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | implementation freeze verifier | binds | bundle-derived source commit tree frozen paper receipts config and five successor files | candidate claim c1 | 1.0 | freeze decision owner | immutable implementation identity | freeze | freeze-owner | conclusion | implementation freeze verifier | create_implementation_freeze | rooted freeze receipt | candidate_answer:c1 | successor only |
| c2 | implementation freeze verifier | rejects | missing changed or foreign successor bytes | candidate claim c2 | 1.0 | freeze decision owner | tamper input | freeze | freeze-owner | conclusion | implementation freeze verifier | verify_implementation_freeze | fail closed | candidate_answer:c2 | successor only |
| c3 | implementation freeze verifier | supplies | implementation root to authority envelope | candidate claim c3 | 1.0 | freeze decision owner | operational verifier input | freeze | freeze-owner | conclusion | implementation freeze verifier | verified freeze | exact root | candidate_answer:c3 | successor only |
