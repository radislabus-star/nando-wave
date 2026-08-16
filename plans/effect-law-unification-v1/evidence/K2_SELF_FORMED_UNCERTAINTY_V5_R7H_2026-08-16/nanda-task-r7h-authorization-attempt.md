# R7H Authorization And Attempt Start

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | exact successor-root user authorization | freezes as | denied-authority V2-V5 authorization receipt | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:61 | 1.0 | authorization source | authorization receipt | authorization | exact-root-owner |
| t2 | frozen experiment tuple | admits exactly one | durable global slot claim | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:88 | 1.0 | frozen identity | slot claim | slot-ledger | slot-owner |
| t3 | durable global slot claim | precedes | exclusive attempt-directory creation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:141 | 1.0 | slot authority | attempt container | attempt-start | attempt-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | exact successor-root user authorization | freezes as | denied-authority V2-V5 authorization receipt | confirm_authorization.rs:94-147 | 1.0 | authorization source | authorization receipt | authorization | exact-root-owner |
| c2 | frozen experiment tuple | admits exactly one | durable global slot claim | confirm_authorization.rs:283-335 | 1.0 | frozen identity | slot claim | slot-ledger | slot-owner |
| c3 | durable global slot claim | precedes | exclusive attempt-directory creation | confirm_owner.rs:51-64 | 1.0 | slot authority | attempt container | attempt-start | attempt-owner |

## notes

- Structural coherence only; no nonce or sealed-attempt authority.
- Candidate vocabulary is canonicalized to the paper vocabulary only after direct source inspection; candidate evidence remains source-owned.
