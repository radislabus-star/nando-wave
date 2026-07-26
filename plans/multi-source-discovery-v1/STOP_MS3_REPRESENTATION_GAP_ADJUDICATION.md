# STOP-MS3 Representation Gap Adjudication

Date: 2026-07-26 Europe/Tallinn.

## Verdict

```text
Representation Gap A/B/C/D classifier       CODE PASS
source-neutral gap policy                    PASS
durable pre-action topology archive          LIVE PASS
durable verified RelationFrame archive       LIVE PASS
durable transport terminal archive           LIVE PASS
archive restart byte identity                PASS
current natural bound gap rows               0
natural A/B/C/D classification               NOT EVALUATED
new execution authority                      false
MS4                                          BLOCKED
```

The implementation closes an evidence-retention defect. It does not claim a
new natural operator.

## Canonical Route

```text
LearningStructureRecordV3
-> RequestLearningIndex
-> append-only PreActionTopology archive
-> durable bridge ACK

verified source-neutral RelationFrame
-> append-only RelationFrame archive

Nginx request terminal
-> append-only terminal receipt archive

three immutable inputs
-> TransportBindingLedgerV1
-> selected_role_witness_missing
-> backward derivability adjudication
   ├─ A capture gap
   ├─ B transform gap
   ├─ C post-action only
   └─ D free generation
```

The endpoint, classifier and archives cannot compile, promote or authorize an
operator. `phase_update_allowed=false` and `authority_ready=false` for every
gap result.

## Classification Policy

```text
A capture gap
  allow a source-neutral pre-action representation change
  bump schema/extractor epoch
  require new support freeze and post-freeze future

B transform gap
  allow a DSL change only after one transform is supported by
  at least three independent lineages
  require new support freeze and post-freeze future

C post-action only
  permanent ABSTAIN

D free generation
  permanent ABSTAIN
```

One row cannot create an opcode, selector or applicability rule. Missing or
censored evidence is not negative operator evidence.

## Live Deployment

```text
source commit                         39c97815af00da2b0fa416fe71eae5c2904df991
installed cold-learner SHA-256        dc02d1b14c0127beb1f0c1260e41b018c5d630071da789de2c8ee0f07a16c513
cold bind                             127.0.0.1:18790
cold invocation                       d60d86d6122e4a3aadc3173a98bee966
cold NRestarts                        0
hot serving PID                       3061210
hot serving restarted                no
```

Current durable corpus:

```text
pre-action topology rows              1832
terminal receipts for those rows      1832
relevant completed RelationFrames        0
censored topology                        3
missing completed observation          1829
bound representation-gap rows             0
authority_ready                       false
```

Topology archive restart receipt:

```text
bytes before / after restart          16,454,474 / 16,454,474
SHA-256 before / after                73dad431ba82c6e3cd8276d6478dc06a2ea69e0c500c2f09954b6bf5927e1e45
                                      73dad431ba82c6e3cd8276d6478dc06a2ea69e0c500c2f09954b6bf5927e1e45
```

## Denominator Correction

The historical `78,598` tokens belonged to one selected MS3 opportunity
class. They were never the denominator of all `UNEXPLORED_MULTI_SOURCE`
traffic. The corresponding bound evidence row was evicted before durable
archives existed, so it cannot be reconstructed from teacher or post-action
data and cannot authorize a representation change.

The current proof denominator is therefore:

```text
fresh durable transport-bound gap rows = 0
```

New ordinary traffic will populate all three archives automatically. The first
fresh bound gap row will be classified without changing authority.

## Verification

```text
nando-operator-learning               276 / 276 PASS
nando-transition-serving              120 / 120 PASS, 1 perf ignored
F7 generation shadow                    7 / 7 PASS, 1 perf ignored
representation-gap focused              3 / 3 PASS
topology archive focused                2 / 2 PASS
bridge archive-before-ACK               1 / 1 PASS
Clippy -D warnings                          PASS
rustfmt / diff check                        PASS
NANDA composite                             PASS
ACTIVE response packages                       2
false accepts                                  0
runtime parity failures                        0
M3                                        WATCH 2/3
```

## Next Live Gate

```text
fresh ordinary pre-action topology
+ matching successful terminal
+ independently verified completed RelationFrame
-> immutable transport binding
-> A / B / C / D classification
```

Only an A or independently repeated B result can open a new representation
epoch. C and D remain permanent ABSTAIN. MS3 remains open until a fresh natural
case reaches a unique source-neutral law, independent future, BundleV4,
external admission and an ordinary CPU receipt.
