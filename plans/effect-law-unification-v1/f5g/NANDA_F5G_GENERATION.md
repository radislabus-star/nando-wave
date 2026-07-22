# F5-G Generation Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | cold generation builder | produces | immutable dispatch generation | F5_RUNTIME_CONVERGENCE_V1.md#live-process-and-traffic-boundary |
| s2 | one shadow request | pins | one generation before execution | F5_RUNTIME_CONVERGENCE_V1.md#incoming-request-contract |
| s3 | generation registry | swaps | whole generations monotonically | F5_RUNTIME_CONVERGENCE_V1.md#f5-g-incoming-traffic-shadow-and-performance |
| s4 | terminal receipt | commits | sequence, generation root, and index root | F5_RUNTIME_CONVERGENCE_V1.md#observability-contract |
| s5 | F5-G generation owner | grants | no execution authority | F5_RUNTIME_CONVERGENCE_V1.md#canonical-ownership-boundary |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | cold generation builder | produces | immutable dispatch generation | crates/nando-operator-runtime/src/traffic_shadow_v3/generation.rs |
| c2 | one shadow request | pins | one generation before execution | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c3 | generation registry | swaps | whole generations monotonically | crates/nando-operator-runtime/src/traffic_shadow_v3/generation.rs |
| c4 | terminal receipt | commits | sequence, generation root, and index root | crates/nando-operator-runtime/src/traffic_shadow_v3/receipt.rs |
| c5 | F5-G generation owner | grants | no execution authority | crates/nando-operator-runtime/src/traffic_shadow_v3/generation.rs |
