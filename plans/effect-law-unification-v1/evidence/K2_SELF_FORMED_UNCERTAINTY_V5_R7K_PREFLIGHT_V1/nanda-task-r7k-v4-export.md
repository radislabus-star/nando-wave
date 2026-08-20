# R7K V4 Predecessor Export Boundary

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | predecessor drift | requires | new preflight revision | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:51-68 | 1.0 | preservation rule | implementation gate | preflight | revision-owner |
| t2 | exported predecessor packet | cannot substitute for | R7K process evidence | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:67-68 | 1.0 | predecessor evidence | measured child evidence | evidence-separation | process-owner |
| t3 | R7K Development harness | records | actual child process outcomes | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:115-144 | 1.0 | execution harness | measured evidence | control-execution | evidence-owner |
| t4 | R7J control evaluator | owns | K1-K12 aggregate decision | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:81-100 | 1.0 | control proof owner | aggregate control decision | control-proof | control-owner |
| t5 | R7J terminal evaluator | owns | Development terminal disposition | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:81-92 | 1.0 | terminal proof owner | terminal decision | terminal-proof | terminal-owner |
| t6 | R7K | emits only | DevelopmentRehearsalComplete | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md:17-46 | 1.0 | development component | non-scientific result | claim-boundary | result-owner |
| t7 | untrusted export packet | requires | complete manifest validation | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_PREFLIGHT_DRIFT_2026-08-20.md:30-51 | 1.0 | transport evidence | transport proof | export-proof | verifier-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | predecessor drift | requires | new preflight revision | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:131-144 | 1.0 | preservation rule | implementation gate | preflight | revision-owner |
| c2 | exported predecessor packet | cannot substitute for | R7K process evidence | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:97-122 | 1.0 | predecessor evidence | measured child evidence | evidence-separation | process-owner |
| c3 | R7K Development harness | records | actual child process outcomes | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:121-122 | 1.0 | execution harness | measured evidence | control-execution | evidence-owner |
| c4 | R7J control evaluator | owns | K1-K12 aggregate decision | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:26-41 | 1.0 | control proof owner | aggregate control decision | control-proof | control-owner |
| c5 | R7J terminal evaluator | owns | Development terminal disposition | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:26-41 | 1.0 | terminal proof owner | terminal decision | terminal-proof | terminal-owner |
| c6 | R7K | emits only | DevelopmentRehearsalComplete | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:14-24 | 1.0 | development component | non-scientific result | claim-boundary | result-owner |
| c7 | untrusted export packet | requires | complete manifest validation | K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V4.md:68-95 | 1.0 | transport evidence | transport proof | export-proof | verifier-owner |

## notes

- V4 changes predecessor transport only; V3 cleanup and result boundaries stay frozen.
- Exported rows are untrusted transport until the complete manifest passes.
- No R7K result authority exists before the V4 preflight and post-edit gates.
