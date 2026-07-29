# MS3 Remote Learning Closed Loop Checkpoint

Date: 2026-07-29

## Verdict

The source and remote cold learner now enforce the missing evidence route
between local Codex experience and the remote learner. The local Evidence Agent
and full-receipt remote decoder are deployed. The route-receipt connector binary
is staged and waiting for the three current client connections to drain; it has
not interrupted them. This checkpoint does not grant authority and does not
claim a natural-law future pass.

```text
remote CPU serving                              PASS
remote cold learner                             DEPLOYED
remote gateway control                          DEPLOYED
local Evidence Agent                            DEPLOYED
remote full-receipt validation                  DEPLOYED
local connector process                         OLD VERSION, LIVE
route-receipt connector activation              WAITING FOR DRAIN
legacy remote frames                            79 UNPROVEN
route-bound remote frames                       0
learning_closed_loop_ready                      false
learning blocker                                NO_ROUTE_BOUND_REMOTE_EVIDENCE
raw Codex payload persisted remotely            false
G16 ineligible-row adjudication                 PASS
successor cursor preserves overflow             PASS
typed ambiguity                                 PASS
learning closed-loop source/integration         PASS
learning closed-loop live evidence              NOT YET OBSERVED
MS3 natural-law authority                       false
phase mutation                                  false
```

## Architecture

```text
Codex Responses request on client
  |
  v
Local Connector
  |- records only turn/session identity hashes
  |- binds the exact request-body digest
  |- records request observation before forwarding
  |- commits only after confirmed remote 200 or 418
  |- seals request and confirmation timestamps separately
  `- keeps a bounded 64 MiB hash-only route ledger
  |
  +-----------------------------+
                                |
Codex session journal on client |
  |
  v
Incremental Session Stream
  |- starts existing files at current EOF
  |- reads only newly appended rows
  |- uses existing source-neutral extractor
  |- emits only independently verified RelationFrames
  `- requires an exact pre-action route receipt --------+
  |
  v
Local Durable Evidence Outbox
  |- 65,536-frame / 512 MiB hard bounds
  |- fsync before frame becomes sendable
  |- no raw session rows
  `- compacts only after durable ACK state
  |
  v
RemoteEvidenceBatchV1
  |- at most 32 frames / 256 KiB
  |- per-client monotonic sequence
  |- previous batch root
  |- carries the complete canonical route receipt, not only its root
  |- request HMAC with fresh transport timestamp
  |- server ACK HMAC
  |- authority_ready=false
  `- phase_mutation_allowed=false
  |
  v
Private LAN Nginx exact route
  `- /_nando/evidence/v1/batches -> cold learner :18790
  |
  v
Remote Durable Spool
  |- independently validates route receipt root, identity, status and time order
  |- pending batch fsync
  |- verified frame archive append
  |- immutable batch receipt publish
  |- durable client head
  `- restart-idempotent duplicate ACK
  |
  v
Existing MS3 Join / Acquisition / Version Space
```

The connector carries provider traffic and fallback and proves that a specific
local turn actually crossed the Nando route. The Evidence Agent carries compact
post-action proof only when that pre-action route receipt exists. The remote
spool requires
`request_observed_at <= route_confirmed_at <= action_frame_observed_at` and
rechecks the full receipt against the frame turn/session identity. Historical
root-only frames remain decodable but can never become route-bound evidence.
Neither component owns admission or an `AuthorityLease`.

The route-ledger cursor advances only after canonical decode and chain
validation. A complete invalid JSONL record remains blocking across every poll;
it cannot be skipped by a second refresh. Connector append performs `sync_data`
before publishing the receipt to the in-process head.

## G16 Repair

The frozen live-state fixture proves the original deadlock was a denominator
error, not negative scientific evidence.

```text
G16 raw rows scanned                             256
eligible rows                                    155
terminal eligible rows                           155
relevant verified frames                          72
linked frames                                      0
provider identity unproven                        12
topology censored                                 89
verdict                   CENSORED_INELIGIBLE_PROBE
consumed topology cursor                       13,231
consumed capture sequence                      24,492
authority_ready                                  false
phase_update_allowed                             false
```

The successor begins at the exact consumed topology cursor. Rows already
captured beyond that cursor are not discarded.

## Acquisition Contract V2

```text
raw scanned denominator
  |- eligible for linked evidence
  |  |- terminal within receipt SLO
  |  `- relevant verified frame
  `- ineligible
     |- provider identity unproven
     `- topology censored
```

Only eligible rows spend the scientific row budget. Ineligible rows receive an
immutable operational censor and cannot create anti-centers, phase updates, or
authority. A stalled or exhausted generation closes through the existing
generation registry and opens one successor from its consumed cursor.

## Typed Ambiguity

Repeated structural mentions no longer destroy the entire topology. The
extractor preserves a bounded ordinal candidate set. Source-neutral T1 expands
that set into hypotheses, and runtime still abstains until the semantic
quotient proves one action-equivalent class.

No tool name, field name, exact value, package identity, or teacher output was
added as runtime authority.

## Durability Order

Client:

```text
verified frame
-> outbox append + fsync
-> immutable pending batch + fsync
-> signed request
-> verify signed server ACK
-> advance local head + fsync
-> remove pending + directory fsync
-> compact fully acknowledged outbox
```

Server:

```text
verify timestamp + client key + request HMAC
-> validate canonical batch and every verifier receipt
-> pending batch + fsync
-> frame archive append
-> immutable receipt publish + directory fsync
-> client head atomic replace + fsync
-> signed ACK
```

A crash at any boundary causes replay or fail-closed recovery, not a
restart-valid phantom future.

## Receipt Health Denominator

The live G25 audit exposed 264 raw rows, 66 eligible rows, and 180 stale rows
classified as `terminal_receipt_unavailable`. Receipt health incorrectly
filtered those 180 rows before measuring the topology-to-terminal edge and
therefore reported `COMPLETE`.

The operational monitor now measures every structurally eligible row in the
frozen raw scan. On the same immutable G25 contract it reports those missing
terminal receipts as `RECEIPT_STALLED`; it does not alter the acquisition
report, watermark, deadline, authority, or phase memory.

Capture-health schema V2 also propagates that receipt status to its top-level
status and `operational_repair_allowed` flag. A healthy topology counter can no
longer mask a stalled topology-to-terminal edge as generic `CAPTURE_PROGRESS`.

New generations use acquisition contract V3:

```text
provider-bound, uncensored topology
-> scientific eligible denominator
-> terminal receipt health
   |- below SLO        IN_FLIGHT
   |- at/above SLO     RECEIPT_STALLED
   `- present          terminally covered
```

V1 and V2 contracts retain their original byte identity and selection
semantics. An active V2 generation is loaded from disk rather than silently
recomputed under V3.

## Verification

All Rust compilation, tests, and Clippy for the changed crates ran on the
20-core mini-PC at
`e@192.168.3.94:/home/e/build/nando-wave-ms3-closed-loop`.

```text
cargo fmt --all -- --check                         PASS
cargo check --workspace --all-targets               PASS
cargo test -p nando-operator-learning
  316 unit + 1 integration                          PASS
cargo test -p nando-transition-serving
  161 unit PASS / 4 explicit ignored
  5 Evidence Agent tests                            PASS
  remaining integration targets                    PASS
connector tests 9/9                                PASS
client route-receipt tests 6/6                     PASS
gateway-control tests 48/48                        PASS
G16 frozen live-state replay                        PASS
full cargo test --workspace --all-targets           PASS / 0 FAIL
changed-crate Clippy --all-targets -D warnings      PASS
Shellcheck for deployment scripts                   PASS
LAN edge transaction + rollback                     PASS
local agent transaction + rollback                  PASS
remote spool transaction + rollback                 PASS
gateway-control transaction + rollback              PASS
drain-aware connector transaction + rollback        PASS
```

Rust 1.97 full-workspace Clippy still exposes 62 pre-existing `nando-cli`
lint errors outside this change. The five changed crates pass strict Clippy;
the unrelated CLI debt was not rewritten inside the MS3 rollout.

The isolated LAN integration accepted sequence `2 -> 3` with four fresh
verified frames, zero authentication failures, signed ACK verification, and
post-ACK outbox compaction to zero frames. Restart and outage replay were also
tested against the same durable client/server state. Test-only services were
stopped afterward.

The old production `/health` path recomputed
`StreamingSelfTrainingState::report()` and took 5.4-5.8 seconds on the first
request after a pause. A live stack trace identified the exact path. Health now
uses the miner worker's published status. Two post-deployment pause-and-probe
series returned the full 72.5 KiB status in 3.8-6.6 milliseconds. The cold
installer still keeps a bounded 10-second startup probe; hot health remains
independently required before and after restart.

## Deployment Boundary

The production order is intentionally narrow:

```text
release build on mini-PC                         DONE
-> transactional cold learner binary/state install
                                                DONE
-> verify learner health with authority=false   DONE
-> install local Evidence Agent                 DONE
-> deploy full route-receipt decoder            DONE
-> update remote control scope                  DONE
-> wait for active connector connections = 0    ACTIVE
-> transactional connector activation           QUEUED
-> observe first ordinary route-bound frame      PENDING
-> expose G25 stale terminal gaps as RECEIPT_STALLED
                                                READY TO DEPLOY
-> create V3 eligibility contract only at next successor
                                                PENDING
-> preserve authority=false until normal MS3 future proof
                                                ENFORCED
```

The installed cold learner binary is SHA-256
`fc14c05bb1a1ef2e60c7ff1deedd0694567dd9f34f3d91bec677195dc9db3b0b`.
The installed local Evidence Agent is SHA-256
`a849b3578ecaa87cf233082a543c8465cc1d0737b94a07a824b51c3536a1a667`.
The staged static connector is SHA-256
`af597b3ac93f06d2ba8d9968a62ce1b341b81514248c8908b07bd0c137c0b760`.
The active old connector PID remained `161091` throughout this checkpoint.

Any failed learner, control, agent, or connector health check restores the
previous component. The drain-aware connector job returns without modification
when active connections remain and activates only after two zero-connection
samples. Hot serving, active connector connections, and current Codex windows
stay online throughout.
