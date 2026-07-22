# F7 Atomic Generation Store

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | inactive slot temporary | is written and synced before | atomic rename | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s2 | slot rename | is followed by | directory fsync | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s3 | previous valid slot | remains available during | next slot publication | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s4 | corrupt newest slot | yields | quarantine and previous valid checkpoint | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s5 | same-generation checkpoint | must extend | immutable evidence prefix | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s6 | next generation | must name | exact previous generation as parent | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s7 | atomic store | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | inactive slot temporary | is written and synced before | atomic rename | crates/nando-operator-persistence/src/store/io.rs#write_slot_atomically |
| c2 | slot rename | is followed by | directory fsync | crates/nando-operator-persistence/src/store/io.rs#write_slot_atomically |
| c3 | previous valid slot | remains available during | next slot publication | crates/nando-operator-persistence/src/store/recovery.rs#publish |
| c4 | corrupt newest slot | yields | quarantine and previous valid checkpoint | crates/nando-operator-persistence/tests/f7_atomic_store_v3.rs#stale_temporary_and_corrupt_new_slot_recover_previous_generation |
| c5 | same-generation checkpoint | must extend | immutable evidence prefix | crates/nando-operator-persistence/src/store/recovery.rs#same_generation_evidence_extends |
| c6 | next generation | must name | exact previous generation as parent | crates/nando-operator-persistence/src/store/recovery.rs#validate_transition |
| c7 | atomic store | grants | no execution authority | crates/nando-operator-persistence/src/store/types.rs#execution_authority |
