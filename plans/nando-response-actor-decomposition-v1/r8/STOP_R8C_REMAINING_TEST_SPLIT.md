# STOP-R8C Remaining Test Split

The embedded test tails of `online.rs`, `online_collection.rs`, and the
learning-owned `synthesis.rs` moved to explicit test files. Function-name
inventories remain identical:

```text
online                         37 / 37
online_collection              49 / 49
synthesis                      14 / 14
total                         100 / 100
```

```text
online.rs                       5,336 -> 3,814 lines
online_collection.rs            9,877 -> 6,674 lines
synthesis.rs                    2,709 -> 2,231 lines
learning tests / Clippy                    PASS / PASS
response historical fingerprint                   PASS
new failures                                          0
deployment                                            no
restart                                               no
authority                                          false
```

The two online production modules still exceed the hard budget and remain
explicit R8 production-cut work.
