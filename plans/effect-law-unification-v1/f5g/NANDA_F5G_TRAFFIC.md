# F5-G Traffic And Privacy Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | metadata-only ordinary row | yields | censored payload-unavailable verdict | F5_RUNTIME_CONVERGENCE_V1.md#f5-g-incoming-traffic-shadow-and-performance |
| s2 | missing raw payload | prohibits | invented replay evidence | F5_RUNTIME_CONVERGENCE_V1.md#incoming-request-contract |
| s3 | caller-owned try-send | classifies | enqueue, queue-full, or disconnected | F5_RUNTIME_CONVERGENCE_V1.md#live-process-and-traffic-boundary |
| s4 | queue-full or disconnected | yields | accounted censored outcome without waiting | F5_RUNTIME_CONVERGENCE_V1.md#runtime-budgets |
| s5 | terminal shadow receipt | stores | hashes, verdict, generation, and timing only | F5_RUNTIME_CONVERGENCE_V1.md#observability-contract |
| s6 | Responses and Chat projections | preserve | one canonical action law before rendering | F5_RUNTIME_CONVERGENCE_V1.md#action-equivalence-must-precede-rendering |
| s7 | F5 traffic shadow | grants | zero local accepts and zero authority | F5_RUNTIME_CONVERGENCE_V1.md#canonical-ownership-boundary |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | metadata-only ordinary row | yields | censored payload-unavailable verdict | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c2 | missing raw payload | prohibits | invented replay evidence | crates/nando-operator-runtime/src/traffic_shadow_v3/input.rs |
| c3 | caller-owned try-send | classifies | enqueue, queue-full, or disconnected | crates/nando-operator-runtime/src/traffic_shadow_v3/handoff.rs |
| c4 | queue-full or disconnected | yields | accounted censored outcome without waiting | crates/nando-operator-runtime/src/traffic_shadow_v3/handoff.rs |
| c5 | terminal shadow receipt | stores | hashes, verdict, generation, and timing only | crates/nando-operator-runtime/src/traffic_shadow_v3/receipt.rs |
| c6 | Responses and Chat projections | preserve | one canonical action law before rendering | crates/nando-operator-runtime/src/traffic_shadow_v3/tests/controls.rs |
| c7 | F5 traffic shadow | grants | zero local accepts and zero authority | crates/nando-operator-runtime/src/traffic_shadow_v3/receipt.rs |
