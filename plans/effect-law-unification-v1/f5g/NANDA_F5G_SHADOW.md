# F5-G Shadow Pipeline Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | traffic shadow | consumes | one borrowed canonical input snapshot | F5_RUNTIME_CONVERGENCE_V1.md#incoming-request-contract |
| s2 | structural dispatch | precedes | bounded runtime role binding | F5_RUNTIME_CONVERGENCE_V1.md#structural-dispatch-before-binding |
| s3 | complete binding | precedes | capability and action grounding | F5_RUNTIME_CONVERGENCE_V1.md#binding-and-action-collapse |
| s4 | phase ranking | ranks | only already valid grounded attempts | F5_RUNTIME_CONVERGENCE_V1.md#f5-f-phase-integration |
| s5 | bound action | causes | actor and Operator VM shadow execution | F5_RUNTIME_CONVERGENCE_V1.md#f5-e-actor-operator-vm-shadow |
| s6 | failed upstream boundary | yields | ABSTAIN without phase rescue | F5_RUNTIME_CONVERGENCE_V1.md#f5-f-phase-integration |
| s7 | one shadow attempt | emits | exactly one terminal verdict | F5_RUNTIME_CONVERGENCE_V1.md#f5-g-incoming-traffic-shadow-and-performance |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | traffic shadow | consumes | one borrowed canonical input snapshot | crates/nando-operator-runtime/src/traffic_shadow_v3/input.rs |
| c2 | structural dispatch | precedes | bounded runtime role binding | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c3 | complete binding | precedes | capability and action grounding | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c4 | phase ranking | ranks | only already valid grounded attempts | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c5 | bound action | causes | actor and Operator VM shadow execution | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c6 | failed upstream boundary | yields | ABSTAIN without phase rescue | crates/nando-operator-runtime/src/traffic_shadow_v3/pipeline.rs |
| c7 | one shadow attempt | emits | exactly one terminal verdict | crates/nando-operator-runtime/src/traffic_shadow_v3/receipt.rs |
