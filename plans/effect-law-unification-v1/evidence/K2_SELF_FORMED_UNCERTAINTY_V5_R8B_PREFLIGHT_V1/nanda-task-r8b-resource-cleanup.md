# R8B Resource And Cleanup Evidence

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | sandboxed child | must remain below | 512 MiB RSS | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:260-272 | 1.0 | execution process | resource limit | observation | memory-budget |
| t2 | resource violations | must equal | zero | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:443-456 | 1.0 | measured failures | terminal conjunct | proof | resource-terminal |
| t3 | cleanup verifier | reads | complete attempt root and classified manifests | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:283-290 | 1.0 | proof owner | cleanup evidence | proof | cleanup-proof |
| t4 | cleanup | retains | failure and indeterminate evidence | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:546-564 | 1.0 | mutation owner | evidence policy | cleanup | cleanup-retention |
| t5 | R8B publication failure | preserves | individual remote resource and structural receipts | K2_SELF_FORMED_UNCERTAINTY_IMPLEMENTATION_PREFLIGHT_V5.json:670-683 | 1.0 | failure state | retained evidence | cleanup | publication-failure |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | sandboxed child | must remain below | 512 MiB RSS | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md:197-225 | 1.0 | execution process | resource limit | observation | memory-budget |
| c2 | resource violations | must equal | zero | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md:282-303 | 1.0 | measured failures | terminal conjunct | proof | resource-terminal |
| c3 | cleanup verifier | reads | complete attempt root and classified manifests | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md:177-195 | 1.0 | proof owner | cleanup evidence | proof | cleanup-proof |
| c4 | cleanup | retains | failure and indeterminate evidence | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md:177-195 | 1.0 | mutation owner | evidence policy | cleanup | cleanup-retention |
| c5 | R8B publication failure | preserves | individual remote resource and structural receipts | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V2.md:242-262 | 1.0 | failure state | retained evidence | cleanup | publication-failure |

## notes

- Resource evidence is descendant-inclusive and compilation is outside measurement.
