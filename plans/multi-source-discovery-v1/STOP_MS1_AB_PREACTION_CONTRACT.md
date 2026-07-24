# STOP-MS1-A/B: Pre-Action Contract And Pure Extractor

Date: 2026-07-24 Europe/Tallinn

Status:

```text
MS0-R-A archive inventory                 PASS
historical raw provider bytes             0 proven
historical authority                      false
MS1-A kernel contract                     PASS
MS1-B pure pre-action extractor           PASS
MS1-C pre-action shadow commitment        PASS
MS1-C durable V2 bridge publication       PASS
MS1-D V3 checkpoint/restart               PASS
MS1-E live deployment                     PASS
production authority change               0
```

## Historical inventory

The live provider capture store is provenance-rich but hash-only. It contains
request roots, lineage roots, capture sequences and immutable receipts, but no
provider payload bytes. The bounded request-learning checkpoint stores phase
and capability atoms, not the original provider request. Codex session JSONL
files are append-only session traces, but no checked contract currently proves
that their bytes equal the exact provider request presented at the learning
boundary.

Therefore the historical population remains:

```text
RECOVERY_UNAUDITED_MULTI_SOURCE
-> LEGACY_UNJOINABLE_MULTI_SOURCE until a separate sealed archive proves bytes
```

No historical row becomes support or future through this inventory.

## New kernel boundary

`LearningRequestStructureV2` owns:

```text
provider-bound TurnIntentId
session lineage roots
existing request/context/capability atoms
provider request root
bounded source-neutral topology
```

`PreActionTopologyCommitV1` seals the topology before action/outcome reveal.
Neither type contains teacher output, post-action state, raw request text,
field names or scalar values.

The extractor receives only the pre-action provider payload and request text.
It emits typed local roles and source-neutral relations. Exceeding a topology
budget censors the whole topology instead of publishing a truncated graph.
JSON-encoded tool outputs are parsed only in memory before source-neutral
projection; their field names and values are not copied into the topology.

The live request owner now computes the V1 structure and the V2 topology from
the same provider-bound request identity. It seals and emits a shadow topology
commit before the provider action is observed. The shadow event has no
authority.

`LearningStructureRecordV3` binds the existing V1 structure, V2 topology,
provider capture receipt and pre-action commitment in one immutable record.
The existing bridge sequence remains the single ordering owner. Its consumer
decodes historical V2 records and new V3 records, while the checkpoint writes
schema V3 and restores V2 checkpoints backward-compatibly. The V3 checkpoint
retains topology and commitment after spool ACK.

## Fixed evidence gates

The audit found two distinct routes:

```text
adaptive natural route
  semantic version-space collapse
  -> immutable freeze
  -> at least one independent post-freeze transfer
  -> crystallized candidate
  -> external admission

legacy relation/subcenter control
  fixed 32 support + 32 future
  -> shadow report only
```

The external controller merges only provenance-bound crystallized candidates.
Legacy relation and uncrystallized collection snapshots remain observable
controls and cannot enter the ACTIVE registry. The number 32 remains legal as
a bounded legacy reservoir/control denominator, never as proof that a natural
operator has been identified.

## Verification

```text
renamed/reordered fields preserve topology       PASS
oversized topology is wholly censored            PASS
focused extractor tests                          2/2 PASS
V3 record canonical roundtrip                    PASS
V2/V3 single-consumer restart                    2/2 PASS
nando-response-actor all-target compile          PASS
online::stream public re-export regression       NOT PRESENT
authority                                        false
```

Live shadow after deployment:

```text
ordinary provider captures                         8
V3 topologies retained after checkpoint ACK        1
producer/consumer sequence gaps                     0
pending records                                     0
bridge failures                                     0
request-learning evictions                          0
raw payloads persisted                              0
extractor debug-build p99                          12 us
ACTIVE packages preserved                           2
false accepts                                       0
runtime parity failures                             0
composite live gate                              PASS
deployed binary SHA-256
bfbb49c590a255beb4081cd449dc27ee7345949c66f9097290dab9d7cebdbc37
```

## Next

Freeze the fresh structural evidence epoch at the first V3 capture sequence,
then implement the MS2 blind-then-reveal join. Historical rows remain
unjoinable unless a separately sealed raw provider archive is proven.
