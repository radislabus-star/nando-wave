# R8B V7 Ledger Ownership

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | actual spawn owner | fsyncs before spawn | request-bound ChildStarted | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:16-17 | 1.0 | mutation owner | durable intent | execution | intent-first |
| t2 | M01 | journals only | M02 generator | confirm_owner.rs:218-241 | 1.0 | nested spawn owner | generator process | execution | owner-allowlist |
| t3 | M10 | journals only | M03-M09 descendants | confirm_public_coordinator.rs:31-197 | 1.0 | nested spawn owner | public child processes | execution | coordinator-allowlist |
| t4 | M24 | journals | direct manifested children | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V2.md:12-12 | 1.0 | root or child orchestrator | direct process set | execution | direct-allowlist |
| t5 | typed validation failure | leaves | started-without-finished indeterminate suffix | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V6_IMPLEMENTATION_DISCREPANCY_2026-08-21.md:42-45 | 1.0 | failed process result | retained evidence | failure | fail-closed |
| t6 | M24 root process | is excluded from | its descendant ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7_CRITIQUE_V1.md:20-21 | 1.0 | root orchestrator | descendant process ledger | denominator | no-self-child |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | actual spawn owner | fsyncs before spawn | request-bound ChildStarted | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:76-92 | 1.0 | mutation owner | durable intent | execution | intent-first |
| c2 | M01 | journals only | M02 generator | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:99-107 | 1.0 | nested spawn owner | generator process | execution | owner-allowlist |
| c3 | M10 | journals only | M03-M09 descendants | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:99-107 | 1.0 | nested spawn owner | public child processes | execution | coordinator-allowlist |
| c4 | M24 | journals | direct manifested children | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:99-107 | 1.0 | root or child orchestrator | direct process set | execution | direct-allowlist |
| c5 | typed validation failure | leaves | started-without-finished indeterminate suffix | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:84-92 | 1.0 | failed process result | retained evidence | failure | fail-closed |
| c6 | M24 root process | is excluded from | its descendant ledger | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md:71-75 | 1.0 | root orchestrator | descendant process ledger | denominator | no-self-child |

## notes

- One actual spawn has exactly one intent writer.
- Structural PASS cannot prove source instrumentation.
