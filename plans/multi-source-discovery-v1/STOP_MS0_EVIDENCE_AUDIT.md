# STOP-MS0 Multi-Source Evidence Audit

Status: `COMPLETE / MULTI_SOURCE_JOIN_INSUFFICIENT`

Scope: read-only audit of the live opportunity checkpoint, request-learning
checkpoint, and completed relation-frame ledger. No capture, learning,
runtime, admission, registry, or authority state was changed.

## Result

```text
UNEXPLORED_MULTI_SOURCE
├─ intents                              3,635
├─ unique ordinary input tokens      571,174,510
├─ accounting identity                     PASS
├─ preliminary shape identity              PASS
├─ exact decidability reason            0 / 3,635
├─ request-structure join               0 / 3,635
├─ completed RelationFrame join         0 / 3,635
├─ provider-bound joined identity       0 / 3,635
├─ raw text in report                            0
└─ authority                                 false
```

`reason_identity_holds=true` means every intent and token is accounted for
under exactly one reason state. It does not claim exact reasons are available:
all rows are explicitly assigned
`reason_not_persisted_by_opportunity_board_v3`.

## Exact Break

```text
OpportunityBoard
  3,635 multi-source intents / 571,174,510 tokens
  stores class + economics
  does not persist CpuDecidability.reason
          |
          v
TurnIntentId join
          |
          +-- request-learning checkpoint
          |     81 current stored turns
          |     0 reported evictions
          |     0 overlap with opportunity intents
          |     pre-action context atoms not persisted
          |
          `-- completed relation-frame ledger
                62,479 rows scanned
                0 parse errors
                0 overlap with opportunity intents
```

The economic opportunity window survived, but the structural streams did not
retain a joinable representation for the same TurnIntentId population.
Therefore the current 571.2 million-token class cannot be factorized into
multi-source laws from the existing persisted evidence.

This is not a Wave, CEGIS, crystallizer, verifier, or admission failure. None
of those stages received a multi-source work item.

## Missing Fields

All 3,635 intents and all 571,174,510 tokens lack:

```text
decidability_reason persisted with TurnIntentId
bounded pre-action multi-source topology
pre-action context atoms in the durable request checkpoint
completed fragment joined by the same TurnIntentId
```

The current request checkpoint already proves provider-bound identity for rows
that it stores. Its problem is population continuity and missing topology, not
the validity of those 81 rows.

## Decision

Per the preregistered plan, `LearningRequestStructureV2` is now justified.
The next slice must capture one source-neutral descriptor at request time:

```text
TurnIntentId
decidability reason
grounded output count
output-part count
scalar type histogram
collection cardinality classes
temporal output topology
request/context/capability roots
provider-bound identity
```

Raw field names, raw values, request text, teacher response, and post-action
state remain forbidden. V2 must be emitted through the learning-evidence
owner; the economics-only OpportunityBridge must not become a second
structural truth.

Stop before implementing the factorizer. First prove V1/V2 byte and restart
parity, bridge loss zero, hot p99 at most 250 microseconds, and authority
unchanged.

## Reproduction

```bash
nando-multi-source-audit \
  /var/lib/nando-wave/transition/response-online-miner.checkpoint \
  /var/lib/nando-wave/transition/learning-structure-bridge-v2/request-learning-v2.checkpoint.cbor \
  /var/lib/nando-wave/transition/response-relation-frames-v4-verified.jsonl
```

Measured diagnostic cost:

```text
wall time       25.12 s
peak RSS       613,144 KiB
hot-path cost  0
```

Input commitments:

```text
opportunity checkpoint  10be235d7b5902b6d8cd4c76400899ddacf02549ee3a55747b7377250150f5d6
request checkpoint      2ba46c83cabf8038adca3b0556ac38f19332ccb8eec3ee3f9dfd72a5af66e8eb
relation-frame scan     015efbc76d5b92e2d2a8e114a26c4a209081edb91eb323c06cec9d7b45453e83
report                  2b00687d8bd240af3f2ab736b3ae802a174a12c58470bcb131c878dbe917ef5f
```
