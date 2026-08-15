# K2 Safe Inquiry Execution V1

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | exact eligibility gate | vetoes | unsafe ambiguous delayed unknown probes | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:133 | 1.0 | authorization owner | forbidden probes | safety | fail-closed |
| t2 | dispatch request | binds | verified selected probe and private resolved effect | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:134 | 1.0 | dispatch owner | worker input | authority | dispatch-binding |
| t3 | sandbox worker | mutates only | one disposable generated work tree | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:135 | 1.0 | mutation owner | isolated state | execution | one-workspace |
| t4 | observer executable | scans independently | read-only post-state after worker exit | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:136 | 1.0 | observation owner | post-state manifest | observation | separate-observer |
| t5 | observer request | excludes | models predictions effects and worker stdout | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:137 | 1.0 | observation owner | forbidden inputs | observation | observer-exclusion |
| t6 | cleanup owner | removes | workspace only after observer publication | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_CRITIQUE_V1.md:138 | 1.0 | cleanup owner | disposable state | cleanup | cleanup-order |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | exact eligibility gate | vetoes | unsafe ambiguous delayed unknown probes | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:189 | 1.0 | authorization owner | forbidden probes | safety | fail-closed |
| c2 | dispatch request | binds | verified selected probe and private resolved effect | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:273 | 1.0 | dispatch owner | worker input | authority | dispatch-binding |
| c3 | sandbox worker | mutates only | one disposable generated work tree | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:267 | 1.0 | mutation owner | isolated state | execution | one-workspace |
| c4 | observer executable | scans independently | read-only post-state after worker exit | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:268 | 1.0 | observation owner | post-state manifest | observation | separate-observer |
| c5 | observer request | excludes | models predictions effects and worker stdout | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:276 | 1.0 | observation owner | forbidden inputs | observation | observer-exclusion |
| c6 | cleanup owner | removes | workspace only after observer publication | plans/effect-law-unification-v1/K2_SELF_CHOSEN_SAFE_INQUIRY_PREREGISTRATION_V1.md:279 | 1.0 | cleanup owner | disposable state | cleanup | cleanup-order |

## notes

- Observation and execution are separate process routes.
