# F7 Checkpoint Composition

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | generation checkpoint | contains | canonical bundle ledger and exact F6/F7 receipt bytes | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s2 | every ledger row | joins exactly one | generation receipt pair | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s3 | missing duplicate or extra receipt | invalidates | generation checkpoint | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s4 | checkpoint decoder | reopens | each component through its owning validator | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s5 | raw request response or teacher payload | is excluded from | generation checkpoint | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |
| s6 | generation checkpoint | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-d-persistence |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | generation checkpoint | contains | canonical bundle ledger and exact F6/F7 receipt bytes | crates/nando-operator-persistence/src/checkpoint/wire.rs |
| c2 | every ledger row | joins exactly one | generation receipt pair | crates/nando-operator-persistence/src/checkpoint/validate.rs#join_receipts_to_ledger |
| c3 | missing duplicate or extra receipt | invalidates | generation checkpoint | crates/nando-operator-persistence/tests/f7_atomic_store_v3.rs#ledger_receipt_omission_and_full_corruption_fail_closed |
| c4 | checkpoint decoder | reopens | each component through its owning validator | crates/nando-operator-persistence/src/checkpoint/validate.rs#validate_checkpoint_wire_v3 |
| c5 | raw request response or teacher payload | is excluded from | generation checkpoint | crates/nando-operator-persistence/tests/f7_atomic_store_v3.rs#checkpoint_has_no_raw_runtime_payload_and_never_grants_authority |
| c6 | generation checkpoint | grants | no execution authority | crates/nando-operator-persistence/src/checkpoint/types.rs#execution_authority |
