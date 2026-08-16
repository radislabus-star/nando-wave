# R7H Restart Terminals

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | NONCE_COMMITTED without dispatch after restart | terminates as | NONCE_COMMITTED_UNDISPATCHED | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:162 | 1.0 | pre-dispatch crash prefix | immutable terminal | restart | pre-dispatch-terminal-owner |
| t2 | GENERATOR_DISPATCHED without complete split after restart | terminates as | GENERATOR_RESULT_INDETERMINATE | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:168 | 1.0 | post-dispatch crash prefix | immutable terminal | restart | post-dispatch-terminal-owner |
| t3 | existing attempt path | cannot cause | generator redispatch | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:155 | 1.0 | durable attempt prefix | second generator call | restart | replay-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | NONCE_COMMITTED without dispatch after restart | terminates as | NONCE_COMMITTED_UNDISPATCHED | confirm_attempt_journal.rs:278-292 | 1.0 | pre-dispatch crash prefix | immutable terminal | restart | pre-dispatch-terminal-owner |
| c2 | GENERATOR_DISPATCHED without complete split after restart | terminates as | GENERATOR_RESULT_INDETERMINATE | confirm_attempt_journal.rs:294-314 | 1.0 | post-dispatch crash prefix | immutable terminal | restart | post-dispatch-terminal-owner |
| c3 | existing attempt path | cannot cause | generator redispatch | confirm_owner.rs:96-124 | 1.0 | durable attempt prefix | second generator call | restart | replay-owner |

## notes

- The two terminals are distinct and immutable.
- Candidate vocabulary is canonicalized only after direct source inspection; each candidate binds a different source span.
