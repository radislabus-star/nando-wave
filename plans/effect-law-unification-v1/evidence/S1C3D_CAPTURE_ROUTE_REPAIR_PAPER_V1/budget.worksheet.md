# NANDA Triad Worksheet

task_id: s1c3d-budget-v1
domain: code
query: Does S1C-3D distinguish performance targets from hard operational safety?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | resource verifier | classifies | p99 above 5 ms as optimization watch | three-axis verdict contract | 1.0 | resource classification owner | performance target | budget | target-safety | proof | resource verifier | measured p99 | OPTIMIZATION_WATCH | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:111 | optimization |
| t2 | resource verifier | rejects | per-operation hard maximum above 20 ms | operational safety contract | 1.0 | resource classification owner | hard safety ceiling | budget | target-safety | proof | resource verifier | measured hard max | SAFETY_VETO | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:98 | deployment safety |
| t3 | resource verifier | preserves | every exact target value and deviation ratio | reporting contract | 1.0 | resource classification owner | optimization evidence | budget | target-safety | reporting | resource verifier | resource receipt | complete optimization baseline | plans/effect-law-unification-v1/S1C3D_CAPTURE_ROUTE_REPAIR_PREREGISTRATION_V1.md:119 | all repair epochs |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | resource verifier | classifies | p99 above 5 ms as optimization watch | candidate claim c1 | 1.0 | resource classification owner | performance target | budget | target-safety | conclusion | resource verifier | measured p99 | OPTIMIZATION_WATCH | candidate_answer:c1 | optimization |
| c2 | resource verifier | rejects | per-operation hard maximum above 20 ms | candidate claim c2 | 1.0 | resource classification owner | hard safety ceiling | budget | target-safety | conclusion | resource verifier | measured hard max | SAFETY_VETO | candidate_answer:c2 | deployment safety |
| c3 | resource verifier | preserves | every exact target value and deviation ratio | candidate claim c3 | 1.0 | resource classification owner | optimization evidence | budget | target-safety | conclusion | resource verifier | resource receipt | complete optimization baseline | candidate_answer:c3 | all repair epochs |
