# F7 Restart Bundle Ownership

## triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| s1 | restart bundle | stores | canonical manifest and canonical artifacts only | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s2 | restart decoder | rebuilds | structural dispatch index from artifacts | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s3 | rebuilt artifact and index roots | must equal | manifest component roots | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s4 | tampered truncated duplicate or oversized bundle | yields | fail-closed rejection | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s5 | restored generation | re-encodes to | byte-identical bundle | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s6 | restart bundle | persists | zero raw request response or teacher payload bytes | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |
| s7 | restored generation | grants | no execution authority | F7_GENERATION_PERSISTENCE_V1.md#f7-a-contract |

## candidate_triads

| id | subject | relation | object | evidence |
|---|---|---|---|---|
| c1 | restart bundle | stores | canonical manifest and canonical artifacts only | crates/nando-operator-runtime/src/generation_persistence_v3/bundle.rs |
| c2 | restart decoder | rebuilds | structural dispatch index from artifacts | crates/nando-operator-runtime/src/generation_persistence_v3/bundle.rs#decode_operator_generation_restart_bundle_v3 |
| c3 | rebuilt artifact and index roots | must equal | manifest component roots | crates/nando-operator-runtime/src/generation_persistence_v3/bundle.rs#validate_manifest_alignment |
| c4 | tampered truncated duplicate or oversized bundle | yields | fail-closed rejection | crates/nando-operator-proof/tests/f7_generation_restart_v3.rs#new_generation_preserves_old_bytes_and_tampering_fails_closed |
| c5 | restored generation | re-encodes to | byte-identical bundle | crates/nando-operator-proof/tests/f7_generation_restart_v3.rs#generation_restart_is_order_invariant_and_converges_with_f6 |
| c6 | restart bundle | persists | zero raw request response or teacher payload bytes | crates/nando-operator-runtime/src/generation_persistence_v3/bundle.rs |
| c7 | restored generation | grants | no execution authority | crates/nando-operator-runtime/src/generation_persistence_v3/bundle.rs#execution_authority |
