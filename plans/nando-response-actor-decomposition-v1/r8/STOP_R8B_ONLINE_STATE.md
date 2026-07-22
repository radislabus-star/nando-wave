# STOP-R8B Online State Owner Split

`online_state.rs` now owns only the streaming state machine and its mutation
route. Two derived, non-authoritative routes moved to explicit modules:

```text
online_generation_evidence.rs  semantic evidence classes and generation parity
online_signal_tree.rs          read-only stage scoring and blocker reporting
```

The moved code cannot grant authority and does not own persistence, runtime
execution, or admission.

```text
online_state.rs before             2,788 lines
online_state.rs after              2,477 lines
hard production-file budget       <=2,500 lines
focused online_state tests         15/15 PASS
historical response fingerprint          PASS
new Clippy diagnostics                       0
new background build processes              0
deployment                                  no
restart                                     no
authority                                false
```

Machine receipt: `R8B_ONLINE_STATE_STOP.json`.
