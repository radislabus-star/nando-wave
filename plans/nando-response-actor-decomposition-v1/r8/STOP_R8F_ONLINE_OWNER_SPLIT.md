# STOP-R8F: Online Owner Split

Status: `PASS / MOVE_ONLY / AUTHORITY_FALSE`

Date: 2026-07-21

## Boundary

```text
online.rs             miner state machine and report assembly
online/stream.rs      source tail and checkpoint persistence
online/evidence.rs    canonical evidence and bounded bucket storage
online/admission.rs   candidate assembly and fail-closed prechecks
```

`online/admission.rs` does not grant authority. Final package authority remains
owned by `nando-operator-admission`; this split changes neither thresholds nor
admission semantics.

## File Budget

```text
before online.rs             3814
after online.rs              2118
after online/stream.rs        840
after online/admission.rs     602
after online/evidence.rs      289
hard production violations     0
```

## Proof

```text
AST functions and methods               101/101
nando-response-actor frozen fingerprint PASS
compile                                 PASS
new remote background builds               0
execution authority                    false
deploy/restart                         not run
```

Machine receipt: `R8F_ONLINE_OWNER_SPLIT_STOP.json`.

This STOP is a physical ownership and file-budget result only. F5-B remains
frozen until STOP-R9.
