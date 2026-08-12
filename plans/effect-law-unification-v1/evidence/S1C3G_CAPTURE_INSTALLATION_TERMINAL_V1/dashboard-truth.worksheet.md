# NANDA Triad Worksheet

task_id: s1c3g-dashboard-truth-v1
domain: code
query: Does the dashboard projection report the rooted S1C-3G terminal facts without broadening authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | rooted terminal sidecar | supplies | S1C-3G rollback facts | canonical root and pinned receipt roots | 1.0 | status source | dashboard projection | dashboard | truth | evidence | dashboard truth projection | status file | exact facts | plans/effect-law-unification-v1/S1C3G_CAPTURE_INSTALLATION_TERMINAL_STATUS.json:1 | S1C-3G only |
| t2 | gateway parser | rejects | altered or authority-bearing sidecar | exact fields root and negative flags required | 1.0 | fail-closed parser | forged status | dashboard | truth | implementation | dashboard truth projection | snapshot parser | status unavailable | crates/nando-gateway-control/src/main.rs:1874 | dashboard only |
| t3 | live dashboard | displays | rollback pass baseline restored and S1C-4 closed | dynamic status projection replaces stale hardcoded verdict | 1.0 | UI renderer | user-visible status | dashboard | truth | conclusion | dashboard truth projection | dashboard snapshot | honest terminal state | crates/nando-gateway-control/src/live_dashboard.rs:646 | dashboard only |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | rooted terminal sidecar | supplies | S1C-3G rollback facts | dashboard change | 1.0 | status source | dashboard projection | dashboard | truth | conclusion | dashboard truth projection | dashboard parser | exact facts | candidate_answer:c1 | dashboard only |
| c2 | gateway parser | rejects | altered or authority-bearing sidecar | dashboard change | 1.0 | fail-closed parser | forged status | dashboard | truth | conclusion | dashboard truth projection | dashboard parser | status unavailable | candidate_answer:c2 | dashboard only |
| c3 | live dashboard | displays | rollback pass baseline restored and S1C-4 closed | dashboard change | 1.0 | UI renderer | user-visible status | dashboard | truth | conclusion | dashboard truth projection | dashboard renderer | honest terminal state | candidate_answer:c3 | dashboard only |
