# K2 V4 Execution And Journal Route

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | all sixteen census dispositions | enter | immutable all-case barrier | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:212 | 1.0 | denominator owner | batch barrier | temporal | no-case-drop |
| t2 | successful closure census | produces | exactly one immutable ordered plan | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:213 | 1.0 | census owner | plan | persistence | plan-freeze |
| t3 | complete plan dispatch | binds before workers | all safety worker observer ordinal and executable roots | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:236 | 1.0 | dispatch owner | execution identities | temporal | pre-outcome |
| t4 | probe ordinal | derives | fresh workspace identity and exact initial manifest | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:243 | 1.0 | identity owner | isolated workspace | execution | no-carry-over |
| t5 | probe one dispatch | precedes | every accepted probe zero observation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:241 | 1.0 | dispatch owner | observation owner | temporal | no-adaptation |
| t6 | append-only case journal | preserves | every legal execution and observation prefix | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:256 | 1.0 | journal owner | crash projection | durability | exact-prefix |
| t7 | unmatched durable dispatch | terminates as | indeterminate execution without redispatch | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:270 | 1.0 | crash owner | terminal state | fail-closed | no-invention |
| t8 | cleanup owner | waits_for | case terminal and outer models-updated publication | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V4.md:275 | 1.0 | cleanup owner | proof publication | temporal | cleanup-after-proof |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | all sixteen census dispositions | enter | immutable all-case barrier | V4 execution implementation contract | 1.0 | denominator owner | batch barrier | temporal | no-case-drop |
| c2 | successful closure census | produces | exactly one immutable ordered plan | V4 execution implementation contract | 1.0 | census owner | plan | persistence | plan-freeze |
| c3 | complete plan dispatch | binds before workers | all safety worker observer ordinal and executable roots | V4 execution implementation contract | 1.0 | dispatch owner | execution identities | temporal | pre-outcome |
| c4 | probe ordinal | derives | fresh workspace identity and exact initial manifest | V4 execution implementation contract | 1.0 | identity owner | isolated workspace | execution | no-carry-over |
| c5 | probe one dispatch | precedes | every accepted probe zero observation | V4 execution implementation contract | 1.0 | dispatch owner | observation owner | temporal | no-adaptation |
| c6 | append-only case journal | preserves | every legal execution and observation prefix | V4 execution implementation contract | 1.0 | journal owner | crash projection | durability | exact-prefix |
| c7 | unmatched durable dispatch | terminates as | indeterminate execution without redispatch | V4 execution implementation contract | 1.0 | crash owner | terminal state | fail-closed | no-invention |
| c8 | cleanup owner | waits_for | case terminal and outer models-updated publication | V4 execution implementation contract | 1.0 | cleanup owner | proof publication | temporal | cleanup-after-proof |

## notes

- Dispatch, execution, observation, proof, and cleanup remain different owners.
- Every mutation is development-only and authority remains false.
