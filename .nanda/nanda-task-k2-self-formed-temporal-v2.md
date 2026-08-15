# K2 Self-Formed Uncertainty Temporal V2

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | confirm nonce | is created after | source binary test and contract freeze | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:143 | 1.0 | nonce owner | frozen artifacts | temporal | post-freeze-nonce |
| t2 | nonce commitment | precedes | generator execution | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:152 | 1.0 | journal owner | generator action | temporal | nonce-before-generation |
| t3 | all-case precommit | precedes | first worker dispatch | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:154 | 1.0 | batch owner | mutation owner | temporal | batch-barrier |
| t4 | outcome from early case | cannot influence | later case selection | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:40 | 1.0 | observed future | frozen selection | leakage | cross-case-firewall |
| t5 | post-nonce failure | consumes | sole scientific attempt | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:159 | 1.0 | attempt owner | terminal chronology | temporal | one-attempt |
| t6 | durable dispatch without observation | forbids | same-identity redispatch | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_CRITIQUE_V1.md:194 | 1.0 | dispatch record | mutation retry | crash | no-redispatch |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | confirm nonce | is created after | source binary test and contract freeze | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:490 | 1.0 | nonce owner | frozen artifacts | temporal | post-freeze-nonce |
| c2 | nonce commitment | precedes | generator execution | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:503 | 1.0 | journal owner | generator action | temporal | nonce-before-generation |
| c3 | all-case precommit | precedes | first worker dispatch | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:554 | 1.0 | batch owner | mutation owner | temporal | batch-barrier |
| c4 | outcome from early case | cannot influence | later case selection | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:571 | 1.0 | observed future | frozen selection | leakage | cross-case-firewall |
| c5 | post-nonce failure | consumes | sole scientific attempt | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:513 | 1.0 | attempt owner | terminal chronology | temporal | one-attempt |
| c6 | durable dispatch without observation | forbids | same-identity redispatch | plans/effect-law-unification-v1/K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V2.md:604 | 1.0 | dispatch record | mutation retry | crash | no-redispatch |

## notes

- Custody is local procedural evidence only; it is not external attestation.
