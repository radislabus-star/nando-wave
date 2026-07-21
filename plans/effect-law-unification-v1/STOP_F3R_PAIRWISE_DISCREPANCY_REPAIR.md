# STOP-F3R: Pairwise Discrepancy Repair

Date: 2026-07-21

```text
pairwise discrepancies accounted   6 / 6 (100%)
adversarial false explanation      BLOCKED
protocol-only merge fixture         NOT PROVEN
all future merges                   WATCH until proven
golden JSON parity                  PASS
execution authority                false
production callers                 0
F4                                 NOT STARTED
```

## Repair

The former group-level rule accepted a merge whenever one V3 law contained
more than one protocol-facet root. It has been removed.

Every discrepancy now contains `DiscrepancyWitness` for each class pair:

```text
direction
left and right class commitments
shared class commitment
left and right effect-facet roots
left and right protocol-facet-set roots
effect facets identical
protocol facets distinct
supporting fixture proven
pair-specific reasons
explained
```

For a V1-to-V3 split, each V3-law pair must retain a concrete committed
effect-significant difference. A reason observed in a different pair cannot
explain it.

For a V3 merge across V1 signatures, matching effect facets plus different
protocol facets are still insufficient today: no label-free protocol-only
merge fixture exists. The classifier therefore marks every such pair
unexplained and returns WATCH.

## Adversarial Control

The repair test constructs three legacy classes under one diagnostic V3 law:

```text
V1-A -> protocol-1
V1-B -> protocol-1
V1-C -> protocol-2
```

The old aggregate rule saw two protocol roots and explained the whole merge.
The pairwise rule emits three witnesses; A/B has no protocol difference and is
unexplained. The merge is blocked.

Synthetic labels in this unit-level adversarial control never authorize a
protocol merge. They only prove rejection behavior.

## Golden Artifact

The focused test recomputes the deterministic report, canonicalizes both it
and the checked-in JSON, and requires byte equality:

```text
plans/effect-law-unification-v1/STOP_F3_DUAL_CLASSIFICATION_V1_V3.json
schema      nando.effect-law-dual-classification-report.v1-v3.r1
file sha256 c1de712eb4b1f43e40e38d092fca6565202e5ef6a625cdf8c54eeff254f3880c
```

## Verification

```text
F3R focused tests             12 / 12 PASS
Canonical F2                 28 / 28 PASS
Historical F2                15 / 15 PASS
cargo check                  PASS
F3-aware Clippy              PASS
git diff --check             PASS
pairwise structural gate     PASS
production/runtime/admission unchanged
authority                    false
```

Work stops at STOP-F3R. The next allowed stage remains B1 label-free binding
evidence acquisition, not F4 selector or ProtocolMode compilation.
