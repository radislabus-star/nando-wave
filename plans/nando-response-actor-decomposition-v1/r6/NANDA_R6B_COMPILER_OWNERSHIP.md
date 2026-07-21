# R6-B Compiler Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | accepted binding evidence | constrains | protocol mode search | decomposition-plan#r6 |
| s2 | canonical effect law | binds | protocol mode identity | decomposition-plan#effect-law |
| s3 | bounded compiler | emits | immutable candidate artifact or ABSTAIN | decomposition-plan#fail-closed |
| s4 | compiler output | requires | later independent admission | decomposition-plan#authority |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | accepted binding evidence | constrains | protocol mode search | crates/nando-operator-learning/src/protocol_mode.rs#compile_protocol_modes_for_effect_law_v3 |
| c2 | canonical effect law | binds | protocol mode identity | crates/nando-operator-learning/src/protocol_mode.rs#effect_law_id |
| c3 | bounded compiler | emits | immutable candidate artifact or ABSTAIN | crates/nando-operator-learning/src/executable_protocol_mode/compiler.rs#compile_executable_protocol_mode_artifact_v3 |
| c4 | compiler output | requires | later independent admission | crates/nando-operator-learning/src/executable_protocol_mode/mod.rs#artifact |
