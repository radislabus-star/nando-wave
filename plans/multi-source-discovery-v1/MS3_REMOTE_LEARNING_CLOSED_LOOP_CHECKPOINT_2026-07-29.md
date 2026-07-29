# MS3 Remote Learning Closed Loop Checkpoint

Date: 2026-07-29

## Verdict

The source and isolated LAN integration now close the missing evidence route
between local Codex experience and the remote learner. This checkpoint does not
grant authority and does not claim a natural-law future pass.

```text
remote CPU serving                              PASS
local connector                                 unchanged
remote post-action evidence source              IMPLEMENTED
raw Codex payload persisted remotely            false
G16 ineligible-row adjudication                 PASS
successor cursor preserves overflow             PASS
typed ambiguity                                 PASS
learning closed-loop integration                PASS
MS3 natural-law authority                       false
phase mutation                                  false
production deployment                           NOT YET PERFORMED
```

## Architecture

```text
Codex session journal on client
  |
  v
Incremental Session Stream
  |- starts existing files at current EOF
  |- reads only newly appended rows
  |- uses existing source-neutral extractor
  `- emits only independently verified RelationFrames
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
  |- pending batch fsync
  |- verified frame archive append
  |- immutable batch receipt publish
  |- durable client head
  `- restart-idempotent duplicate ACK
  |
  v
Existing MS3 Join / Acquisition / Version Space
```

The connector carries provider traffic and fallback. The Evidence Agent carries
compact post-action proof. Neither component owns admission or an
`AuthorityLease`.

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

## Verification

All Rust compilation, tests, and Clippy for the changed crates ran on the
20-core mini-PC at
`e@192.168.3.94:/home/e/build/nando-wave-ms3-closed-loop`.

```text
cargo fmt --all -- --check                         PASS
cargo check --workspace --all-targets               PASS
cargo test -p nando-operator-learning
  314 unit + 1 integration                          PASS
cargo test -p nando-transition-serving
  157 unit PASS / 3 explicit ignored
  4 Evidence Agent tests                            PASS
  remaining integration targets                    PASS
G16 frozen live-state replay                        PASS
full cargo test --workspace --all-targets           PASS / 0 FAIL
changed-crate Clippy --all-targets -D warnings      PASS
Shellcheck for deployment scripts                   PASS
LAN edge transaction + rollback                     PASS
local agent transaction + rollback                  PASS
remote spool transaction + rollback                 PASS
```

Rust 1.97 full-workspace Clippy still exposes 62 pre-existing `nando-cli`
lint errors outside this change. The five changed crates pass strict Clippy;
the unrelated CLI debt was not rewritten inside the MS3 rollout.

The isolated LAN integration accepted sequence `2 -> 3` with four fresh
verified frames, zero authentication failures, signed ACK verification, and
post-ACK outbox compaction to zero frames. Restart and outage replay were also
tested against the same durable client/server state. Test-only services were
stopped afterward.

The production cold learner can legitimately take several seconds to answer
while holding a learning-state lock. Its transactional installer therefore
uses a separate bounded 10-second health probe instead of the hot serving
timeout; hot health remains independently required before and after restart.

## Deployment Boundary

The production order is intentionally narrow:

```text
release build on mini-PC
-> transactional cold learner binary/state install
-> verify learner health with authority=false
-> graceful Nginx reload for the exact evidence route
-> install local Evidence Agent without touching connector
-> observe first ordinary fresh frame
-> verify G16 immutable censor and G17 exact-cursor successor
-> preserve authority=false until normal MS3 future proof
```

Any failed learner, edge, agent, or evidence health check restores the previous
component. Hot serving, active connector connections, and current Codex windows
must remain online throughout.
