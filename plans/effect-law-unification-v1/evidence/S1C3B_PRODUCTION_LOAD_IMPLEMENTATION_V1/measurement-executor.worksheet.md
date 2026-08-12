# NANDA Triad Worksheet

task_id: s1c3b-implementation-measurement-v1
domain: code
query: Does the S1C-3B measurement executor preserve the frozen denominator and absolute resource boundary?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | measurement executor | runs | exactly three ordered rounds on CPU 4 | executor lines 688-732 | 1.0 | evidence producer | frozen denominator | measurement | measurement-executor | proof | measurement executor | evaluate_measurement | complete ordered rows | ops/remote-backend/s1c3b_remote_transaction_v1.py:688 | proof only |
| t2 | measurement executor | applies | unchanged absolute resource thresholds | executor lines 734-813 | 1.0 | evidence producer | product boundary | measurement | measurement-executor | proof | measurement executor | metric rows | resource failures | ops/remote-backend/s1c3b_remote_transaction_v1.py:734 | product safety |
| t3 | filesystem floor | contributes | diagnostic evidence without verdict authority | executor floor evaluation lines 703-738 | 1.0 | diagnostic producer | authority boundary | measurement | measurement-executor | proof | measurement executor | floor probe | diagnostic rows | ops/remote-backend/s1c3b_remote_transaction_v1.py:703 | proof only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | measurement executor | runs | exactly three ordered rounds on CPU 4 | candidate claim c1 | 1.0 | evidence producer | frozen denominator | measurement | measurement-executor | proof | measurement executor | evaluate_measurement | complete ordered rows | candidate_answer:c1 | proof only |
| c2 | measurement executor | applies | unchanged absolute resource thresholds | candidate claim c2 | 1.0 | evidence producer | product boundary | measurement | measurement-executor | proof | measurement executor | metric rows | resource failures | candidate_answer:c2 | product safety |
| c3 | filesystem floor | contributes | diagnostic evidence without verdict authority | candidate claim c3 | 1.0 | diagnostic producer | authority boundary | measurement | measurement-executor | proof | measurement executor | floor probe | diagnostic rows | candidate_answer:c3 | proof only |
