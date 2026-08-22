# R8B V7 Causal Authority

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | P06 packet | freezes before | M25 authorization | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V6_IMPLEMENTATION_DISCREPANCY_2026-08-21.md:61-74 | 1.0 | immutable evidence | authorization owner | chronology | no-cycle |
| t2 | M25 | consumes only | P06 closed packet | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:28-29 | 1.0 | authorization owner | prior evidence | authority | authorizer-input |
| t3 | M26 | publishes exactly | M25 authorization bytes | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:379-381 | 1.0 | mutation owner | authorization receipt | authority | publication |
| t4 | M25 and M26 outcomes | remain outside | P06 pre-authorization ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V6_IMPLEMENTATION_DISCREPANCY_2026-08-21.md:67-74 | 1.0 | future process outcomes | prior evidence packet | chronology | future-exclusion |
| t5 | post-authorization audit | grants no | decision or publication authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:30-30 | 1.0 | diagnostic evidence | authority | claim | audit-only |
| t6 | R8B_FROZEN | grants no | K2, runtime, deployment or LawCertificate authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:20-37 | 1.0 | scoped Development result | forbidden claims | claim | boundary |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | P06 packet | freezes before | M25 authorization | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:40-69 | 1.0 | immutable evidence | authorization owner | chronology | no-cycle |
| c2 | M25 | consumes only | P06 closed packet | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:60-69 | 1.0 | authorization owner | prior evidence | authority | authorizer-input |
| c3 | M26 | publishes exactly | M25 authorization bytes | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:60-65 | 1.0 | mutation owner | authorization receipt | authority | publication |
| c4 | M25 and M26 outcomes | remain outside | P06 pre-authorization ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:143-144 | 1.0 | future process outcomes | prior evidence packet | chronology | future-exclusion |
| c5 | post-authorization audit | grants no | decision or publication authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:377-380 | 1.0 | diagnostic evidence | authority | claim | audit-only |
| c6 | R8B_FROZEN | grants no | K2, runtime, deployment or LawCertificate authority | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:12-27 | 1.0 | scoped Development result | forbidden claims | claim | boundary |

## notes

- Future process outcomes cannot be prerequisites for their own execution.
- Structural PASS retains authority_ready=false.
