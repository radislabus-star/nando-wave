# NANDA Triad Worksheet

task_id: s1c3h-install-rollback-implementation-v1
domain: code
query: Does the implemented S1C-3H transaction install or restore one coherent runtime-authority unit and preserve diagnostics before rollback?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | execution staging | owned_and_traversable_by | service user e only | dedicated state-dir staging with uid gid mode tests | 1.0 | staging owner | production service user | preparation | staging | operations | S1C-3H transaction | reset_staging | off-path authority execution | ops/remote-backend/s1c3h_remote_transaction_v1.py:443 | preparation |
| t2 | candidate pair | built_from | one frozen source commit | build receipt binds source tree binary hashes and runtime contracts | 1.0 | build producer | candidate compatibility unit | build | pair-build | operations | orchestrator | create-build-receipt | pair contract equality | ops/remote-backend/verify_s1c3h_transaction_v1.py:188 | deployment |
| t3 | candidate authority generation | published_before | pointer and final admission | generation copy is first and admission is last | 1.0 | generation producer | authority publication | installation | authority-install | runtime | response admission owner | install_staged_authority | coherent sidecars | ops/remote-backend/s1c3h_remote_transaction_v1.py:605 | production |
| t4 | generation copy | preserves | exact ownership and mode | chown chmod and manifest parity are applied recursively | 1.0 | installer | service-readable immutable generation | installation | generation-owner | persistence | transaction owner | copy_tree_atomic | exact tree | ops/remote-backend/s1c3h_remote_transaction_v1.py:123 | production |
| t5 | candidate failure | persists_before | rollback mutation | diagnostic or minimal diagnostic is fsynced before rollback call | 1.0 | failure observer | rollback evidence | rollback | diagnostic-order | proof | transaction owner | execute exception | durable blocker | ops/remote-backend/s1c3h_remote_transaction_v1.py:1083 | failure |
| t6 | rollback | restores | old binary config sidecars and generation | byte fault matrix restores three mixed states | 1.0 | rollback owner | complete old unit | rollback | complete-rollback | operations | transaction owner | rollback | old pair READY | ops/remote-backend/test_s1c3h_transaction_v1.py:243 | failure |
| t7 | early pre-mutation failure | does_not_rewrite | production compatibility files | ROLLBACK_ARMED path restores triggers without installs | 1.0 | early failure | live production | rollback | no-mutation | operations | transaction owner | rollback | old service remains active | ops/remote-backend/s1c3h_remote_transaction_v1.py:908 | failure |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | execution staging | owned_and_traversable_by | service user e only | candidate c1 | 1.0 | staging owner | production service user | preparation | staging | operations | S1C-3H transaction | reset_staging | off-path authority execution | candidate_answer:c1 | preparation |
| c2 | candidate pair | built_from | one frozen source commit | candidate c2 | 1.0 | build producer | candidate compatibility unit | build | pair-build | operations | orchestrator | create-build-receipt | pair contract equality | candidate_answer:c2 | deployment |
| c3 | candidate authority generation | published_before | pointer and final admission | candidate c3 | 1.0 | generation producer | authority publication | installation | authority-install | runtime | response admission owner | install_staged_authority | coherent sidecars | candidate_answer:c3 | production |
| c4 | generation copy | preserves | exact ownership and mode | candidate c4 | 1.0 | installer | service-readable immutable generation | installation | generation-owner | persistence | transaction owner | copy_tree_atomic | exact tree | candidate_answer:c4 | production |
| c5 | candidate failure | persists_before | rollback mutation | candidate c5 | 1.0 | failure observer | rollback evidence | rollback | diagnostic-order | proof | transaction owner | execute exception | durable blocker | candidate_answer:c5 | failure |
| c6 | rollback | restores | old binary config sidecars and generation | candidate c6 | 1.0 | rollback owner | complete old unit | rollback | complete-rollback | operations | transaction owner | rollback | old pair READY | candidate_answer:c6 | failure |
| c7 | early pre-mutation failure | does_not_rewrite | production compatibility files | candidate c7 | 1.0 | early failure | live production | rollback | no-mutation | operations | transaction owner | rollback | old service remains active | candidate_answer:c7 | failure |
