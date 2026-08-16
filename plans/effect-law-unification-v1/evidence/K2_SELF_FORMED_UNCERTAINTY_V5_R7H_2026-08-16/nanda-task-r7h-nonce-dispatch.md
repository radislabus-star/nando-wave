# R7H Nonce And Dispatch Order

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | ARTIFACTS_FROZEN journal event | precedes | operating-system CSPRNG read | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:147 | 1.0 | durable journal event | nonce source | nonce-commit | nonce-owner |
| t2 | retained nonce mode 0400 | commits as | hash-only NONCE_COMMITTED event | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:149 | 1.0 | private nonce artifact | public journal event | nonce-commit | nonce-artifact-owner |
| t3 | GENERATOR_DISPATCHED journal event | precedes | first anonymous-pipe request byte | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:152 | 1.0 | irreversible journal event | generator input | generator-dispatch | dispatch-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | ARTIFACTS_FROZEN journal event | precedes | operating-system CSPRNG read | confirm_owner.rs:65-92,195-199 | 1.0 | durable journal event | nonce source | nonce-commit | nonce-owner |
| c2 | retained nonce mode 0400 | commits as | hash-only NONCE_COMMITTED event | confirm_nonce.rs:77-107; confirm_owner.rs:199-211 | 1.0 | private nonce artifact | public journal event | nonce-commit | nonce-artifact-owner |
| c3 | GENERATOR_DISPATCHED journal event | precedes | first anonymous-pipe request byte | confirm_owner.rs:230-248 | 1.0 | irreversible journal event | generator input | generator-dispatch | dispatch-owner |

## notes

- The generator request is sent once through anonymous stdin after the durable dispatch marker.
- Candidate vocabulary is canonicalized only after direct source inspection; code evidence is independent of the paper span.
