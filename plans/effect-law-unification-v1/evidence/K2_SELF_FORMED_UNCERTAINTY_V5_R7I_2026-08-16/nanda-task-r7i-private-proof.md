# R7I Private Resolution Execution And Proof

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | one private resolver process | receives only | one case resolver table and one frozen plan ordinal | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:320 | 1.0 | private mapping owner | bounded resolver input | resolver-route | resolver-input-owner |
| t2 | resolver receipt | exposes exactly | one selected effect without alternatives or truth labels | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:323 | 1.0 | private mapping owner | bounded effect receipt | resolver-route | resolver-output-owner |
| t3 | role-specific sandbox matrix | permits only | frozen read-only or writable mounts assigned to that role | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:283-291 | 1.0 | isolation owner | process mount authority | isolation | mount-owner |
| t4 | durable dispatch | precedes | isolated worker mutation and read-only observation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:344-349 | 1.0 | dispatch evidence | execution and observation | execution | execution-order-owner |
| t5 | complete frozen observation vector | precedes | read-only final-truth mount and independent final verification | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:283,441 | 1.0 | observation evidence | proof owner | final-proof | reveal-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | one private resolver process | receives only | one case resolver table and one frozen plan ordinal | confirm_private_resolver.rs:20-123,196-219; k2_self_formed_uncertainty_confirm_r7i_v1.rs:149-189 | 1.0 | private mapping owner | bounded resolver input | resolver-route | resolver-input-owner |
| c2 | resolver receipt | exposes exactly | one selected effect without alternatives or truth labels | confirm_private_resolver.rs:125-193,220-253; k2_self_formed_uncertainty_confirm_r7i_v1.rs:190 | 1.0 | private mapping owner | bounded effect receipt | resolver-route | resolver-output-owner |
| c3 | role-specific sandbox matrix | permits only | frozen read-only or writable mounts assigned to that role | confirm_sandbox.rs:94-168,186-248; k2_self_formed_uncertainty_confirm_r7i_v1.rs:400-421 | 1.0 | isolation owner | process mount authority | isolation | mount-owner |
| c4 | durable dispatch | precedes | isolated worker mutation and read-only observation | k2_self_formed_uncertainty_confirm_r7i_v1.rs:223-325 | 1.0 | dispatch evidence | execution and observation | execution | execution-order-owner |
| c5 | complete frozen observation vector | precedes | read-only final-truth mount and independent final verification | k2_self_formed_uncertainty_confirm_r7i_v1.rs:327-383; final_verifier_v2.rs:28-118 | 1.0 | observation evidence | proof owner | final-proof | reveal-owner |

## notes

- The full route is exercised with generated DevelopmentRehearsal cases only.
- The outer rehearsal sequence is test-owned at R7I; R8B and R9B must bind it into the exact confirm-owner dry-run before a new R10 authorization.
- Authority remains false and no Confirm nonce exists.
