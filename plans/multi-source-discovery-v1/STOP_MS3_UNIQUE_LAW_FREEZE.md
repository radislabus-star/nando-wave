# STOP-MS3 Unique Law Freeze

Date: 2026-07-26 Europe/Tallinn.

## Verdict

```text
NO_GAP three-root evidence                 PASS
bounded source-neutral candidate search    COMPLETE
semantic quotient                          1 class
existing identification machine            USED
unique law freeze                          SEALED
two-watermark partition                    PASS
pre-action prediction persistence          LIVE
restart byte identity                      PASS
independent future                         PENDING
new authority                              false
phase mutation                             false
MS4                                        BLOCKED
```

This STOP closes the immutable natural-law candidate freeze. It does not close
MS3 transfer and does not authorize CPU execution.

## Signal Route

```text
fresh pre-action topology
-> verified completed RelationFrame
-> transport terminal receipt
-> immutable three-root NO_GAP binding
-> source-neutral candidate enumeration
-> exact support replay
-> semantic quotient
-> OperatorIdentificationMachineV1
-> one action-equivalent class
-> FrozenVersionSpaceContractV1
-> pre-action future prediction ledger
-> INDEPENDENT_LINEAGE_PENDING
```

No second MS3 identifier was introduced. The frozen checkpoint is the existing
`OperatorIdentificationMachineV1`, and future evidence is applied back to that
checkpoint.

## Frozen Evidence

```text
acquisition report root
  67b9b4dc5c17570b9203d2db4400afbaba17610e6d59ee56b4d7f575e8c7982f

linked NO_GAP receipt
  d836338f0efccd1ff82edb9c1b536ace7758c073a935100117d42e146f70672d

topology root
  c064a43115e0d0a4998c9bc86d8ecb60e0521d2f4eb1db930828bfdf1abc9ec3

RelationFrame root
  6babf3b923a1f6d2e5904f85f5e22860933be0f2d1f05f9ede396e9cc7e40642

terminal root
  0b4f5db42ee6faf8cb49ce0a1ea3d886dd9667e4c4330a3ba65d0dfee89f8776

transport binding root
  e264a88f11517f1950085abedcd6e11deb4cca735ef6af70a5d67b679f2e7d34
```

## Version Space

```text
contract root
  23f002035fb5090827a7815fc24d2fdc633df2126a32c978b5347ae69410ba81

candidate program root
  4036f3216731a4797f3190d6de08de5748fdb90b09b826f029254186173d5c9a

semantic class root
  aae14454b996f05d90c8dc441a14d160f008e8ecb08b04a0d3eba7e0f354df08

semantic quotient root
  187554c2145772565db96212a906b37505dbc0784fa50e63fc0f8bf49f8a0179

candidate freeze root
  a1266a3197ba4c0f72b3f0eac3f0a4cec566796a7d983beaee269e036a2e19a2

machine checkpoint SHA-256
  2c24c7f149a175f1e9adfe67a4d1d6539419df21981c7befc92c713a79f85f5b

machine checkpoint bytes
  11,457
```

Zero-class outcomes have explicit fail-closed reasons:

```text
PROGRAM_ALGEBRA_GAP
UNSUPPORTED_RENDERER
SELF_REPLAY_INCONSISTENCY
INVALID_HYPOTHESIS_GENERATION
PERMANENT_ABSTAIN
```

`REPRESENTATION_GAP` cannot be reopened without invalidating the frozen NO_GAP
binding.

## Two-Watermark Partition

```text
support_watermark                  13116
contract_watermark                 13420
future_min_sequence                13421
pre-freeze buffer span               304
pre-freeze buffer disposition      PRE_FREEZE_BUFFER_EXCLUDED
```

The contract watermark is captured only after candidate enumeration, exact
replay and semantic quotient. Rows in the 304-sequence interval are neither
support nor future.

The live pre-action prediction ledger opened at sequence 13611:

```text
prediction_min_sequence            13612
ledger SHA-256
  addaef06dcee22405b48bdb0707406c2bff627368a23d34fc634f815ba3dc292
```

Rows from `13421` through `13611` were already present before the deployed
prediction owner could commit a pre-action prediction. They remain excluded
and cannot be replayed into future evidence after seeing their outcomes.

## Persistence

```text
frozen envelope
  /var/lib/nando-wave/transition/multi-source-live-v2/
  linked-frame-acquisition-v1/version-space-v1/
  frozen-version-space-v1.cbor

bytes                           14,201
SHA-256
  cfeade1afa5c82c03765b917b05399493e11a7dfa03a4bfe0edab5efe62733c1

future prediction ledger
  /var/lib/nando-wave/transition/multi-source-live-v2/
  linked-frame-acquisition-v1/version-space-v1/
  future-predictions-v1.cbor

bytes                              281
SHA-256
  addaef06dcee22405b48bdb0707406c2bff627368a23d34fc634f815ba3dc292
```

Both files retained byte-identical SHA-256 values across cold-learner restart.
The frozen contract is loaded from disk and is not recomputed on restart.

## Live Future Boundary

At the recorded snapshot:

```text
post-open topology rows              86
support-lineage reuse                85
independent topology rows             1
independent lineages                  1
structurally applicable rows          0
committed predictions                 0
future receipt                     NONE
```

The exact blocker is `APPLICABLE_INDEPENDENT_TOPOLOGY_PENDING`. Repeated
observations from the support lineage are not future evidence. The independent
but structurally inapplicable row is also not negative evidence.

The next admissible transition is automatic:

```text
fresh topology from a new SessionLineageId
-> source-neutral applicability check
-> prediction persisted before terminal outcome
-> independently verified completed frame
-> apply_future on the frozen identification machine
-> PASS or CONTRADICTION receipt
```

No manual session relabeling, historical replay, threshold change, phase
update or authority change is allowed.

## Deployment Scope

```text
cold learner bind                   127.0.0.1:18790
cold invocation                     b49f5e8615c14a3fb29115708fcbffb4
cold restarts                       0
installed binary SHA-256
  d2b4f667cdf7b1b8ddb66da8a0ed9b0815056fa7954bcd38f133d7e1c0cd8118

hot serving invocation              aec3a81260c648ed84538066131715c6
hot serving restarted               no
```

The cold learner has no local execution authority. Existing hot packages and
their admission state were not changed by this MS3 freeze.

## Verification

```text
nando-operator-learning             285 / 285 PASS
learning integration                  1 / 1 PASS
nando-transition-serving            133 PASS / 1 perf ignored
strict Clippy, both owner crates     PASS
rustfmt / diff check                 PASS
Graphify                             30,493 nodes / 57,016 edges
NANDA composite gate                PASS
existing ACTIVE response packages      2
active false accepts                   0
active runtime parity failures         0
M3                                  WATCH
```

The composite gate's M3 ledger contains two completed passing windows. The
current deduplicated saving share at the gate snapshot was 49.4%, below the
50% threshold, and this frozen MS3 candidate has no execution authority.

## Remaining Gate

MS3 remains open until all of the following are observed from ordinary live
traffic:

```text
new independent lineage             >= 1
pre-action prediction committed     PASS
independent future receipt          PASS
wrong role bindings                    0
negative accepts                       0
runtime parity failures                0
authority before external admission false
```

Only then may the route continue to CanonicalOperatorIR, BundleV4, external
admission and the first ordinary multi-source CPU receipt.
