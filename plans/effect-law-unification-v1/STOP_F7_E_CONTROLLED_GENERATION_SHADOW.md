# STOP-F7-E Controlled Generation Shadow

Date: `2026-07-22`

Verdict: `COMPLETE_CONTROLLED_SHADOW_PASS`

Authority: `false`

## Result

```text
canonical generation checkpoint             PASS
  +-- support/future ledger                  PASS
  +-- exact F6/F7 receipts                   PASS
  `-- two-slot restart                       PASS
             |
             v
separate provider-request capture index      PASS
  +-- sequence/lineage/event/request join    EXACT
  +-- missing or tampered relation           BLOCK
  `-- session JSONL used as HTTP evidence    NO
             |
             v
HTTP bind before cold restore                PASS
  +-- immutable registry install             PASS
  +-- monotonic generation swap              PASS
  +-- request pins one generation            PASS
  +-- empty store clears registry            PASS
  `-- bounded try_send queue                 48 max
             |
             v
F5 structural runtime + actor                PASS
             |
             v
independent F6 verifier                      PASS
             |
             v
hash-only shadow accounting                  PASS
  +-- raw payload persisted                  0
  +-- false accepts                          0
  +-- local accepts                          0
  `-- execution authority                    false
             |
             v
external admission / ACTIVE                  F8 BLOCK
```

The capture index is an explicit contract for the owner that observed the
exact provider request bytes. `StreamingEvidenceLedger` remains unchanged and
cannot satisfy that contract because its payload is a complete Codex session
event, not the HTTP request independently reconstructed by F6.

## Runtime Properties

The feature is disabled by default. Enabling it does not delay HTTP bind: the
loader and one bounded worker start afterward. The request path performs one
existing request hash, creates a small typed ingress record, pins the currently
loaded generation, and uses `try_send`. Missing generation, overload, invalid
input, or a disconnected worker are censored outcomes and never become Wave
negative evidence.

The queue maximum is 48. At the F6 maximum of 256 KiB provider bytes plus
16 KiB request text, queued payload ownership remains bounded below 13 MiB,
excluding allocator overhead.

## Performance

Remote release measurements on `e-MEGA-MINI-M1-13th`:

```text
F5  no-match p99              241,282 ns  PASS
F5  matched p99               493,988 ns  PASS
F6  no-match p99               34,284 ns  PASS
F6  matched p99               288,692 ns  PASS
F7E full no-match p99         192,028 ns  PASS
F7E full matched p99          616,290 ns  PASS
F7E full hard max             719,743 ns  PASS
F5  2048-operator RSS delta    49.6 MiB    WATCH
```

The end-to-end latency budget passes. The older F5-G registry RSS target of
16 MiB does not pass and remains an explicit F8 blocker; STOP-F7-E does not
relabel it.

## Verification

```text
generation capture index                 3 / 3 PASS
checkpoint/capture join                  2 / 2 PASS
F5 traffic shadow                        8 PASS / 1 perf ignored
F7E serving integration                  5 PASS / 1 perf ignored
F7E release performance                  1 / 1 PASS
six-crate bounded baseline             354 PASS / 3 perf ignored
known serving failures excluded          3 / 3 baseline-identical
gateway-control receipt UI               18 / 18 PASS
gateway-control Clippy -D warnings        PASS
Clippy -D warnings                        PASS
changed-file rustfmt                      PASS
git diff --check                          PASS
NANDA composite gate                      PASS
eligible for local accept                 false
Graphify nodes / edges / communities      27372 / 61264 / 1269
services restarted                        NO
deployment changed                        NO
```

The control page reads the F5, F6 and F7 receipts directly. It renders the
controlled path through F7-E and the F8 blockers separately from the live
support/future/admission route; no live counter is promoted into this proof.

Graphify resolves the direct capture join
`load_generation_shadow_snapshot_v3 -> join_generation_checkpoint_to_capture_index_v3`
and the execution route
`run_generation_shadow_worker_v3 -> evaluate_generation_shadow_request_v3 -> verify_operator_result_v3`.

## Honest Boundary

The capture format, exact join, loader, registry, queue and execution path are
complete in controlled shadow. A live producer of the exact capture index has
not been deployed. F8 must provide that producer, independently reconstruct
admission, preserve the same performance and safety metrics, and remain shadow
until a separate authority decision.
