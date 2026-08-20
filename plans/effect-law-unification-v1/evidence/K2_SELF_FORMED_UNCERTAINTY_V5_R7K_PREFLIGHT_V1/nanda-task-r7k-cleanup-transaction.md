# R7K Cleanup Transaction

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | classified path manifest | precedes | cleanup authorization | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:548-555 | 1.0 | immutable classification | mutation permission | cleanup-order | authorization-owner |
| t2 | cleanup owner | deletes only | disposable class | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:557-558 | 1.0 | mutation owner | bounded disposable paths | cleanup-mutation | mutation-owner |
| t3 | cleanup failure | preserves | frozen terminal evidence | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:467-474 | 1.0 | operational failure | scientific evidence | failure-boundary | terminal-owner |
| t4 | restart | projects | one legal durable prefix | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:532-535 | 1.0 | restart observer | immutable state | crash-route | journal-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | classified path manifest | precedes | cleanup authorization | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:210-277 | 1.0 | immutable classification | mutation permission | cleanup-order | authorization-owner |
| c2 | cleanup owner | deletes only | disposable class | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:284-301 | 1.0 | mutation owner | bounded disposable paths | cleanup-mutation | mutation-owner |
| c3 | cleanup failure | preserves | frozen terminal evidence | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:314-316 | 1.0 | operational failure | scientific evidence | failure-boundary | terminal-owner |
| c4 | restart | projects | one legal durable prefix | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:303-312 | 1.0 | restart observer | immutable state | crash-route | journal-owner |

## notes

- Cleanup control-plane state is outside the governed tree.
- Intent is durable before every deletion; broad recursive deletion is forbidden.
