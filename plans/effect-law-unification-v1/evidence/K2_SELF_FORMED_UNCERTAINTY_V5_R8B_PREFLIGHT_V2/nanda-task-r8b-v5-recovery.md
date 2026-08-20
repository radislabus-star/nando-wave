# R8B V5 Recovery State Machine

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | Development attempt lock | serializes | one live owner | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:66-66 | 1.0 | concurrency owner | live attempt | ownership | single-writer |
| t2 | concurrent owner | preserves | journal and files unchanged | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:67-67 | 1.0 | rejected contender | durable state | failure | owner-busy |
| t3 | pre-dispatch restart | permits | exactly one generator dispatch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:68-68 | 1.0 | recovery state | bounded effect | recovery | before-dispatch |
| t4 | post-dispatch incomplete split | forbids | generator redispatch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:69-69 | 1.0 | recovery state | repeated side effect | recovery | after-dispatch |
| t5 | complete split recovery | requires | exact response reconstruction | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V4_CRITIQUE.md:51-52 | 1.0 | recovery state | parity proof | proof | split-recovery |
| t6 | durable owner receipt | returns | byte identical receipt | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V3_CRITIQUE.md:17-17 | 1.0 | recovered state | stable output | recovery | owner-replay |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | Development attempt lock | serializes | one live owner | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:341-347 | 1.0 | concurrency owner | live attempt | ownership | single-writer |
| c2 | concurrent owner | preserves | journal and files unchanged | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:344-347 | 1.0 | rejected contender | durable state | failure | owner-busy |
| c3 | pre-dispatch restart | permits | exactly one generator dispatch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:362-363 | 1.0 | recovery state | bounded effect | recovery | before-dispatch |
| c4 | post-dispatch incomplete split | forbids | generator redispatch | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:365-391 | 1.0 | recovery state | repeated side effect | recovery | after-dispatch |
| c5 | complete split recovery | requires | exact response reconstruction | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:369-379 | 1.0 | recovery state | parity proof | proof | split-recovery |
| c6 | durable owner receipt | returns | byte identical receipt | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:380-387 | 1.0 | recovered state | stable output | recovery | owner-replay |

## notes

- `GeneratorDispatched` is the irreversible boundary even if the child produced no usable response.
