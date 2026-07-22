# STOP-F7-B Generation Evidence Ledger

Date: `2026-07-22`

Verdict: `COMPLETE_CONTROLLED_PROOF_PASS`

Authority: `false`

## Result

```text
CanonicalGenerationId
        |
        v
support-open ledger                      PASS
        |
        v
immutable support freeze + watermark     PASS
        |
        v
future-open ledger                       PASS
        |
        +-- old support lineage           REJECT
        +-- pre-watermark row             REJECT
        +-- duplicate event/request/root  REJECT
        `-- censored semantic update      NONE
        |
        v
canonical evidence root                  PASS
        |
        v
byte-identical restart                   PASS
        |
        v
generation-bound F6 receipt              F7-C / NOT STARTED
        |
        v
admission                                F8 / BLOCKED
```

The ledger is owned by `nando-operator-learning`, not by the response actor.
Its support and future records are separate hash chains under one immutable
generation ID. Future growth changes the evidence root but cannot rewrite the
generation manifest or support freeze.

Each row stores only capture sequence plus privacy-safe lineage, event,
request, and verifier-receipt roots. No request text, response, provider
payload, teacher label, or actor-selected value is serialized.

## Outcome Contract

```text
VerifiedPass          -> PositiveReinforcement
ApplicabilityNegative -> ApplicabilityCounterWave
HardContradiction     -> StructuralRevision
Censored(reason)      -> no semantic update
```

Censored rows remain accounted evidence; they are not silently dropped and
cannot form positive centers or anti-centers.

## Budgets

```text
rows per partition  <= 2048
canonical bytes     <= 2 MiB
raw payload bytes   = 0
authority           = false
```

## Verification

```text
F7-B causal tests             6 / 6 PASS
learning crate baseline       204 / 204 PASS
learning Clippy -D warnings   PASS
changed-file rustfmt          PASS
git diff --check              PASS
production callers            0
services restarted            NO
deployment changed            NO
```

Next boundary: F7-C opaque envelope binding an F6 verifier receipt to the
generation, partition, lineage, event and post-freeze watermark.
