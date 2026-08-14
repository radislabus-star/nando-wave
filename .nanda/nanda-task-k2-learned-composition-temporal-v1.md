# K2 Learned Composition Temporal And Journal V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | private mapping artifact | publishes durably before | experiment freeze event | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:41 | 1.0 | private persistence owner | experiment identity | temporal | mapping-before-freeze |
| t2 | learned laws | freeze before | target and goal enter planner | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:150 | 1.0 | learning evidence owner | target reveal | temporal | laws-before-target |
| t3 | target and goal | freeze before | planning request and output | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:151 | 1.0 | goal owner | selected program | temporal | goal-before-plan |
| t4 | verified plan | freezes before | private mapping reopen and execution | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:152 | 1.0 | plan precommit owner | hidden execution binding | temporal | plan-before-mapping |
| t5 | execution dispatch | publishes durably before | sandbox process creation | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:153 | 1.0 | journal owner | external side effect | persistence | dispatch-order |
| t6 | unobserved published dispatch | forbids | same-identity retry | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:154 | 1.0 | restart owner | duplicate action | crash | no-rerun |
| t7 | deterministic projector | replays exactly | all legal prefixes and cross-event roots | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:155 | 1.0 | projection owner | temporal evidence | persistence | restart-parity |
| t8 | terminal outcome | binds acyclically after | twenty-nine typed events | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_CRITIQUE_V1.md:156 | 1.0 | terminal owner | full evidence chain | proof | terminal-seal |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | private mapping artifact | publishes durably before | experiment freeze event | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:190-198 | 1.0 | private persistence owner | experiment identity | temporal | mapping-before-freeze |
| c2 | learned laws | freeze before | target and goal enter planner | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:185-204 | 1.0 | learning evidence owner | target reveal | temporal | laws-before-target |
| c3 | target and goal | freeze before | planning request and output | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:136-152 | 1.0 | goal owner | selected program | temporal | goal-before-plan |
| c4 | verified plan | freezes before | private mapping reopen and execution | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:339-367 | 1.0 | plan precommit owner | hidden execution binding | temporal | plan-before-mapping |
| c5 | execution dispatch | publishes durably before | sandbox process creation | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:339-367 | 1.0 | journal owner | external side effect | persistence | dispatch-order |
| c6 | unobserved published dispatch | forbids | same-identity retry | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:357-365 | 1.0 | restart owner | duplicate action | crash | no-rerun |
| c7 | deterministic projector | replays exactly | all legal prefixes and cross-event roots | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:339-365 | 1.0 | projection owner | temporal evidence | persistence | restart-parity |
| c8 | terminal outcome | binds acyclically after | twenty-nine typed events | plans/effect-law-unification-v1/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md:341-357 | 1.0 | terminal owner | full evidence chain | proof | terminal-seal |

## notes

- This packet checks temporal ordering and crash semantics only.
- Journal existence never grants K2 or execution authority.
