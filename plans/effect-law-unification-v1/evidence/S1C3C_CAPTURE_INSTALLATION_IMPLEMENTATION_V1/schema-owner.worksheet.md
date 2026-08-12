# NANDA Triad Worksheet

task_id: s1c3c-implementation-schema-owner-v1
domain: code
query: Does the pure S1C-3C schema owner close parser and evaluator drift before side effects?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | schema preflight owner | validates | all four regex field and type tuples | complete SPECS and parse_metric | 1.0 | pure decision owner | metric schema | preflight | schema-owner | proof | schema preflight owner | run_preflight | rooted schema receipt | ops/remote-backend/s1c3c_schema_preflight_v1.py:30 | local only |
| t2 | schema preflight owner | exercises | complete evaluator and each retained field | validate_spec mutations | 1.0 | pure decision owner | evaluator contract | preflight | schema-owner | proof | schema preflight owner | validate_spec | mutation roots | ops/remote-backend/s1c3c_schema_preflight_v1.py:196 | local only |
| t3 | schema preflight owner | reports | no side effects and no remote attempt | run_preflight receipt | 1.0 | pure decision owner | authority boundary | preflight | schema-owner | proof | schema preflight owner | run_preflight | authority false receipt | ops/remote-backend/s1c3c_schema_preflight_v1.py:232 | local only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | schema preflight owner | validates | all four regex field and type tuples | candidate claim c1 | 1.0 | pure decision owner | metric schema | preflight | schema-owner | conclusion | schema preflight owner | run_preflight | rooted schema receipt | candidate_answer:c1 | local only |
| c2 | schema preflight owner | exercises | complete evaluator and each retained field | candidate claim c2 | 1.0 | pure decision owner | evaluator contract | preflight | schema-owner | conclusion | schema preflight owner | validate_spec | mutation roots | candidate_answer:c2 | local only |
| c3 | schema preflight owner | reports | no side effects and no remote attempt | candidate claim c3 | 1.0 | pure decision owner | authority boundary | preflight | schema-owner | conclusion | schema preflight owner | run_preflight | authority false receipt | candidate_answer:c3 | local only |
