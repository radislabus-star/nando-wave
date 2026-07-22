# STOP-R8A Test Module Split

This checkpoint is a move-only extraction of six embedded test modules. No
production algorithm, schema, CLI contract, authority route, or threshold was
changed.

## Inventory Parity

```text
collection_synthesis             47 / 47 test functions
online_state                     20 / 20 test functions
operator_live_shadow             32 / 32 test functions
online_admission                 16 / 16 test functions
operator_runtime                  6 /  6 test functions
response_miner                   22 / 22 test functions
total                           143 / 143 test functions
```

The sorted function-name inventory is identical before and after extraction.

## File Budget Effect

```text
collection_synthesis.rs          4,154 -> 2,453 lines
online_state.rs                  3,945 -> 2,788 lines
operator_live_shadow.rs          4,539 -> 3,404 lines
online_admission.rs              2,736 -> 2,056 lines
operator-runtime/runtime.rs      3,888 -> 3,708 lines
response_miner/app.rs            5,125 -> 3,727 lines
```

`collection_synthesis.rs` and `online_admission.rs` are now below the 2,500
line hard production-file budget. The remaining four production files still
require explicit owner cuts before STOP-R8.

## Proof

```text
response historical fingerprint       PASS
runtime tests / Clippy                 PASS / PASS
miner bin baseline                     16 PASS / 2 known FAIL
new test failures                      0
new background build processes         0
deployment                             no
restart                                no
authority                              false
```

The focused response failures were compared against the exact clean parent:
all ten names and assertions are unchanged.
