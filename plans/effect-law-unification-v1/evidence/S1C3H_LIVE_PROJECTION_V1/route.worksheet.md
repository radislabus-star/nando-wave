# NANDA Triad Worksheet

task_id: s1c3h-live-projection-v1
domain: code
query: Does the control plane expose installed decision capture without promoting installation into K2 authority?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | immutable S1C-3H terminal state | authorizes | installed capture projection | exact transaction and rooted receipt fields are required | 1.0 | evidence owner | operational status | authority | receipt-admission | proof | S1C-3H verifier | s1c3_operational_snapshot | capture installed | crates/nando-gateway-control/src/main.rs | operations |
| t2 | control parser | rejects | altered terminal state | root or exact field mismatch returns unavailable and false authority | 1.0 | authorization owner | forged status | authority | fail-closed-parser | safety | gateway control | s1c3_operational_snapshot | status unavailable | crates/nando-gateway-control/src/main.rs | operations |
| t3 | dashboard API | passes_to | HTML renderer | admitted fields remain separate from decision census | 1.0 | producer | render owner | observation | status-render | presentation | gateway control | dashboard_snapshot | factual panel | crates/nando-gateway-control/src/live_dashboard.rs | UI |
| t4 | S1C-3H installation PASS | enables_only | S1C-4 COLLECTING | zero natural records and scientific authority false remain visible | 1.0 | operational prerequisite | bounded census state | claim | collection-boundary | science | S1C owner | installed receipt | collecting only | plans/effect-law-unification-v1/S1C3H_LIVE_PROJECTION_PREREGISTRATION_V1.md | science |
| t5 | K2 grounded meaning | remains | CLOSED | no goal alternative outcome or complete natural decision episode exists yet | 1.0 | scientific claim | missing evidence | claim | k2-boundary | science | K2 verifier | natural census | no K2 claim | plans/effect-law-unification-v1/S1C3H_LIVE_PROJECTION_PREREGISTRATION_V1.md | science |
| t6 | control-plane deployment | preserves | transition Nginx connector and decision journal | only status sidecar and gateway-control may change | 1.0 | mutation owner | data plane | deployment | runtime-preservation | operations | installer | rollback transaction | serving unchanged | plans/effect-law-unification-v1/S1C3H_LIVE_PROJECTION_PREREGISTRATION_V1.md | production |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | immutable S1C-3H terminal state | authorizes | installed capture projection | candidate c1 | 1.0 | evidence owner | operational status | authority | receipt-admission | proof | S1C-3H verifier | s1c3_operational_snapshot | capture installed | candidate_answer:c1 | operations |
| c2 | control parser | rejects | altered terminal state | candidate c2 | 1.0 | authorization owner | forged status | authority | fail-closed-parser | safety | gateway control | s1c3_operational_snapshot | status unavailable | candidate_answer:c2 | operations |
| c3 | dashboard API | passes_to | HTML renderer | candidate c3 | 1.0 | producer | render owner | observation | status-render | presentation | gateway control | dashboard_snapshot | factual panel | candidate_answer:c3 | UI |
| c4 | S1C-3H installation PASS | enables_only | S1C-4 COLLECTING | candidate c4 | 1.0 | operational prerequisite | bounded census state | claim | collection-boundary | science | S1C owner | installed receipt | collecting only | candidate_answer:c4 | science |
| c5 | K2 grounded meaning | remains | CLOSED | candidate c5 | 1.0 | scientific claim | missing evidence | claim | k2-boundary | science | K2 verifier | natural census | no K2 claim | candidate_answer:c5 | science |
| c6 | control-plane deployment | preserves | transition Nginx connector and decision journal | candidate c6 | 1.0 | mutation owner | data plane | deployment | runtime-preservation | operations | installer | rollback transaction | serving unchanged | candidate_answer:c6 | production |
