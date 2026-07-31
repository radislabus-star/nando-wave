# MS3 G26 Rollover Checkpoint - 2026-07-31

## Verdict

`MS3_TERMINAL_ROLLOVER_PASS`

The G26 natural-law candidate remains an immutable contradiction with
`physical_transition_mismatch`. The cold learner now advances to fresh
successor generations without changing authority or phase memory.

## Root Cause

The topology archive contained one bridge epoch and 19,948 unique rows, but its
historical bootstrap was appended in backfill order. That produced 910 internal
sequence inversions. The old successor cursor incorrectly required every row in
the append prefix to be strictly ordered by bridge sequence.

G26 itself had a clean scientific boundary:

- closure bridge sequence: `29,934`
- exact archive cursor: `18,673`
- future rows before the cursor: `0`
- pre-closure rows after the cursor: `0`

The cursor now permits internal backfill order only while the closure still
defines one exact prefix. A missing or duplicate closure sequence, or any
future row crossing that prefix, remains fail-closed.

## Source And Binary

- source commit: `134b3774d9032ce53882a9834495e4563011f090`
- installed binary SHA-256:
  `a5163aef208d091ad40c157a4f0b38e428fd6b2d3195da7aed2e82c27151874c`
- previous binary SHA-256:
  `5dee23eb7d96122ee36d12010cbc640e83761b8cfcec0b9a8db81485c603bc3c`
- deployment receipt:
  `/var/lib/nando-wave/transition/deployment-receipts/ms3-rollover-20260731T083137Z.json`
- rollback backup:
  `/var/backups/nando-wave/ms3-rollover-20260731T083137Z`

The release binary was built on the mini-PC from a clean detached worktree at
the exact source commit.

## Verification

- `cargo fmt --all -- --check`: PASS
- focused topology archive tests: 7 PASS
- contradiction successor/restart test: PASS
- `cargo test -p nando-transition-serving --all-targets`:
  183 PASS, 0 FAIL, 8 ignored fixture/performance gates
- `cargo clippy -p nando-transition-serving --all-targets -- -D warnings`:
  PASS
- graphify AST update: PASS, 31,588 nodes and 73,337 edges
- cold learner deployment with automatic binary/state rollback: PASS
- post-rollover restart parity: G31 -> G31 PASS
- hot serving PID unchanged: PASS
- transport gateway PID unchanged: PASS
- connector uptime preserved: PASS
- connector relay failures: 0
- connector fallback failures: 0
- false accepts: 0

## Live Tree

```text
G26 UNIQUE_LAW_FROZEN
└─ independent future
   └─ CONTRADICTION
      ├─ blocker: physical_transition_mismatch
      ├─ authority: false
      └─ phase mutation: false

terminal rollover
├─ G27 acquisition fail
├─ G28 acquisition fail
├─ G29 acquisition fail
├─ G30 acquisition fail
└─ G31 collecting
   ├─ watermark: 19,697
   ├─ raw scanned: 257
   ├─ eligible: 174
   ├─ terminal: 255
   ├─ linked: 0
   ├─ blocker: linked_frame_pending
   ├─ authority: false
   └─ phase mutation: false
```

The transport closed loop is live: the remote spool has accepted more than
2,200 batches and more than 2,100 route-bound frames from one authenticated
client. G31 is a scientific acquisition gate, not a serving outage.

## Next Gate

No threshold, classifier, guard, authority, or phase-memory change is allowed
while G31 is collecting.

```text
fresh route-bound linked frame
-> immutable three-root binding
-> version space
-> semantic quotient
-> unique law freeze
-> independent future
-> BundleV4
-> external admission
-> ordinary CPU accept
```

If the bounded denominator closes with zero linked rows, the autonomous
lifecycle must preserve the failure receipt and open the next generation from
the consumed cursor.
