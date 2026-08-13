# K1 Restart Recovery Bounded Authority Wire Critique V1

Status: `ADVERSARIAL REVIEW APPLIED BEFORE IMPLEMENTATION`

Date: `2026-08-13`

| Priority | Finding | Failure if ignored | Frozen repair |
|---|---|---|---|
| P0 | The generic budget error does not identify the request schema or actual byte count. | The wrong wire is redesigned from a stale report estimate. | Reproduce the failing tick on an isolated production-state copy and record schema plus exact bytes before implementation. |
| P0 | Raising the 4 MiB limit would hide unbounded growth. | A later frontier can exhaust authority memory or block its single connection loop. | Keep the outer limit; use bounded compression with a separate 16 MiB logical ceiling. |
| P0 | Compression can become a zip-bomb path. | Root authority allocates attacker-controlled memory. | Enforce compressed and decompressed ceilings while decoding; reject trailing or incomplete streams. |
| P0 | A client-supplied candidate or root can become selection authority. | The learner can choose a program rather than an evidence domain. | Decode the complete logical request inside authority, rebuild queue derivation, require the first readiness-PASS row, and reseal the freeze. |
| P0 | A stale request can append after registry or scheduler movement. | Generation 556 is selected against the wrong K1 deficit or ledger prefix. | Add explicit certification and scheduler revision/root CAS to V2 and validate them immediately before append. |
| P0 | A retry can append the same generation twice. | Restart changes the proof history. | Preserve exact idempotency: identical active freeze returns the existing projection without append. |
| P1 | The latest frontier report lags failed ticks. | Its 1.50 MB size is mistaken for the failing request size. | Treat the report only as a lower-bound diagnostic; measure the in-memory logical request. |
| P1 | Decompressing before checking the outer envelope weakens the transport budget. | Oversized input reaches expensive code. | Validate schema, encoding, compressed length and checksum before decompression. |
| P1 | A new staging file can silently become recovery authority. | Journal and sidecar disagree after crash. | Do not add a mutable recovery sidecar; signed scheduler journal plus anchor remain the only state authority. |
| P1 | The earlier 20-second readiness timeout can trigger another false rollback. | A correct release is removed before the 883 MiB topology archive is restored. | Separate base health, K1 summary and control checks with 120/240/60 second deadlines. |
| P1 | Restarting only the client leaves authority on the old schema. | V2 requests are rejected while runtime appears deployed. | Install matching transition and authority binaries, restart authority before cold, verify installed SHA values. |
| P1 | Route separation and wire recovery can be conflated with Law #2. | A transport repair is displayed as a scientific pass. | Dashboard keeps K1 transport state separate; Law #2 remains not proved until the full independent cycle completes. |
| P2 | Natural suffixes can advance during verification. | Exact whole-directory equality causes destructive rollback. | Pin immutable report and ledger prefixes; observe and validate append-only suffixes separately. |

## Accepted Design

```text
live evidence -> client builds operator-blind frontier
             -> bounded V2 transport envelope
             -> authority bounded decode
             -> full catalog and queue validation
             -> registry + scheduler + protocol CAS
             -> authority reseals selected freeze
             -> signed append-only CandidateFreeze
             -> restart-identical projection
```

The critique authorizes measurement and, only if the frozen ceilings hold, the
bounded V2 transport implementation. It does not authorize synthetic traffic,
candidate ranking changes, generation cleanup, Law #2 promotion, phase
mutation, hot serving restart, or S1C-4 reopening.
