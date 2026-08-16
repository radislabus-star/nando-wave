# V5 Resolver, Execution, Observation And Final-Proof Route

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | confirm supervisor | launches after public coordinator exit | frozen private execution schedule | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:252 | 1.0 | sealed supervisor | immutable schedule | supervisor-handoff | supervisor-owner |
| t2 | one private resolver process | receives only | one case table and one frozen plan step | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:320 | 1.0 | private mapping owner | bounded resolver input | resolver-route | resolver-input-owner |
| t3 | resolver receipt | exposes exactly | selected action effect without alternatives or truth labels | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:323 | 1.0 | private mapping owner | bounded effect receipt | resolver-route | resolver-output-owner |
| t4 | independent safety receipt | precedes | durable case dispatch | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:344 | 1.0 | safety authority | execution permit | safe-execution | safety-owner |
| t5 | durable case dispatch | precedes | isolated worker mutation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:345 | 1.0 | dispatch owner | mutation owner | safe-execution | dispatch-owner |
| t6 | read-only observer | inspects after worker exit | completed isolated workspace | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:348 | 1.0 | observation owner | post-state evidence | observation-parity | observer-owner |
| t7 | worker and observer outputs | require | exact outcome parity before append | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:349 | 1.0 | execution evidence | observation evidence | observation-parity | parity-owner |
| t8 | unmatched durable dispatch | terminates without | redispatch or invented observation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:357 | 1.0 | crash prefix | replacement execution | safe-execution | restart-owner |
| t9 | final private truth file | becomes readable after | complete case observation vector | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:283 | 1.0 | private truth | final verifier | final-proof | truth-mount-owner |
| t10 | independent final verifier | proves | singleton survivor equal to private true class | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:441 | 1.0 | proof owner | final case disposition | final-proof | final-verifier-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | confirm supervisor | launches after public coordinator exit | frozen private execution schedule | V5 R7I supervisor candidate | 1.0 | sealed supervisor | immutable schedule | supervisor-handoff | supervisor-owner |
| c2 | one private resolver process | receives only | one case table and one frozen plan step | V5 R7I resolver-input candidate | 1.0 | private mapping owner | bounded resolver input | resolver-route | resolver-input-owner |
| c3 | resolver receipt | exposes exactly | selected action effect without alternatives or truth labels | V5 R7I resolver-output candidate | 1.0 | private mapping owner | bounded effect receipt | resolver-route | resolver-output-owner |
| c4 | independent safety receipt | precedes | durable case dispatch | V5 R7I safety candidate | 1.0 | safety authority | execution permit | safe-execution | safety-owner |
| c5 | durable case dispatch | precedes | isolated worker mutation | V5 R7I dispatch candidate | 1.0 | dispatch owner | mutation owner | safe-execution | dispatch-owner |
| c6 | read-only observer | inspects after worker exit | completed isolated workspace | V5 R7I observer candidate | 1.0 | observation owner | post-state evidence | observation-parity | observer-owner |
| c7 | worker and observer outputs | require | exact outcome parity before append | V5 R7I parity candidate | 1.0 | execution evidence | observation evidence | observation-parity | parity-owner |
| c8 | unmatched durable dispatch | terminates without | redispatch or invented observation | V5 R7I restart candidate | 1.0 | crash prefix | replacement execution | safe-execution | restart-owner |
| c9 | final private truth file | becomes readable after | complete case observation vector | V5 R7I reveal candidate | 1.0 | private truth | final verifier | final-proof | truth-mount-owner |
| c10 | independent final verifier | proves | singleton survivor equal to private true class | V5 R7I final-verifier candidate | 1.0 | proof owner | final case disposition | final-proof | final-verifier-owner |

## notes

- The public coordinator never receives a resolver effect.
- Every plan ordinal uses a new workspace identity and is never retried.
