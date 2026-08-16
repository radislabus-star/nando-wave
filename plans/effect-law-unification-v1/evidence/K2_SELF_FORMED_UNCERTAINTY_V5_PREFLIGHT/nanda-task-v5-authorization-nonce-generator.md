# V5 Authorization, Slot, Nonce And Generator Route

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | exact successor-root user authorization | freezes as | denied-authority authorization receipt | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:61 | 1.0 | authorization source | authorization receipt | authorization | authorization-owner |
| t2 | experiment freeze tuple | admits exactly one | append-only slot claim | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:86 | 1.0 | frozen experiment identity | slot ledger event | slot-ledger | slot-key-owner |
| t3 | durable slot claim | precedes | exclusive attempt-directory creation | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:139 | 1.0 | slot authority | attempt container | slot-ledger | slot-mutation-owner |
| t4 | frozen artifact descriptor | precedes | operating-system CSPRNG read | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:145 | 1.0 | frozen descriptor | nonce source | nonce-commit | nonce-source-owner |
| t5 | retained nonce file | commits as | hash-only NONCE_COMMITTED event | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:148 | 1.0 | private nonce artifact | public journal commitment | nonce-commit | nonce-artifact-owner |
| t6 | GENERATOR_DISPATCHED event | precedes | first anonymous-pipe request byte | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:151 | 1.0 | irreversible dispatch marker | generator input | generator-dispatch | dispatch-owner |
| t7 | dispatched generator without complete split result | terminates as | generator-result indeterminate without rerun | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:168 | 1.0 | crash prefix | terminal result | generator-dispatch | restart-owner |
| t8 | Confirm generator request | uses | separate closed Confirm wire schema | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:104 | 1.0 | confirm request | typed generator contract | generator-split | request-schema-owner |
| t9 | validated Confirm generator response | separates into | public batch resolver tables and per-case truth files | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:176 | 1.0 | generator result | isolated artifact classes | generator-split | output-split-owner |
| t10 | old-freeze authorization | cannot authorize | successor freeze slot | K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md:80 | 1.0 | superseded authorization | successor attempt | authorization | superseded-auth-owner |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | exact successor-root user authorization | freezes as | denied-authority authorization receipt | V5 R7H authorization-owner candidate | 1.0 | authorization source | authorization receipt | authorization | authorization-owner |
| c2 | experiment freeze tuple | admits exactly one | append-only slot claim | V5 R7H global-slot candidate | 1.0 | frozen experiment identity | slot ledger event | slot-ledger | slot-key-owner |
| c3 | durable slot claim | precedes | exclusive attempt-directory creation | V5 R7H journal-before-mkdir candidate | 1.0 | slot authority | attempt container | slot-ledger | slot-mutation-owner |
| c4 | frozen artifact descriptor | precedes | operating-system CSPRNG read | V5 R7H nonce-owner candidate | 1.0 | frozen descriptor | nonce source | nonce-commit | nonce-source-owner |
| c5 | retained nonce file | commits as | hash-only NONCE_COMMITTED event | V5 R7H nonce-secrecy candidate | 1.0 | private nonce artifact | public journal commitment | nonce-commit | nonce-artifact-owner |
| c6 | GENERATOR_DISPATCHED event | precedes | first anonymous-pipe request byte | V5 R7H irreversible-send candidate | 1.0 | irreversible dispatch marker | generator input | generator-dispatch | dispatch-owner |
| c7 | dispatched generator without complete split result | terminates as | generator-result indeterminate without rerun | V5 R7H restart-projection candidate | 1.0 | crash prefix | terminal result | generator-dispatch | restart-owner |
| c8 | Confirm generator request | uses | separate closed Confirm wire schema | V5 R7G split-schema candidate | 1.0 | confirm request | typed generator contract | generator-split | request-schema-owner |
| c9 | validated Confirm generator response | separates into | public batch resolver tables and per-case truth files | V5 R7H output-publication candidate | 1.0 | generator result | isolated artifact classes | generator-split | output-split-owner |
| c10 | old-freeze authorization | cannot authorize | successor freeze slot | V5 R7H root-mismatch candidate | 1.0 | superseded authorization | successor attempt | authorization | superseded-auth-owner |

## notes

- This worksheet checks procedural coherence only; it grants no nonce authority.
- The old R10 authorization remains bound to the historical freeze and cannot satisfy c1.
