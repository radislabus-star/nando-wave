# Position 2: Sparse Distributed Memory

Anchor:

```text
docs/ARCHITECTURE.md:17
```

## Central Work

- Kanerva Sparse Distributed Memory: high-dimensional memory where read/write
  is driven by similarity rather than exact symbolic address.
- Reference handle: `https://mitpress.mit.edu/9780262514699/sparse-distributed-memory/`

Classical shape:

```text
high-dimensional address
-> nearby hard locations participate
-> distributed write/read
-> similarity-based recall
```

## Nando Wave Mapping

Nando Wave has several SDM-like ingredients:

```text
SurfaceWave4096
L1 surface centers
L2 motif centers
sparse active fringes
center ids as compact reusable handles
heldout/corrupt coverage tests
```

The current L1 path is not literal Kanerva SDM. It is closer to:

```text
surface atoms / 4-grams / boundary atoms / service atoms
-> sparse signed projection
-> reusable centers and residuals
```

L2 then compresses L1 center sequences into motifs:

```text
L1 center refs
-> repeated local motifs
-> reusable L2 handles
```

## Stronger For Our Goal

Nando Wave does not want nearest-neighbor recall to be the final result.

The stronger target is:

```text
surface similarity supports entry into the field,
but proof requires heldout transfer, trap rejection, and ablation.
```

That matters because a pure SDM-like system can be fooled by:

```text
surface family shortcut
nearest neighbor copy
bag-of-tokens similarity
```

The project already treats those as shortcut gates rather than success.

Useful local upgrade:

```text
hot centers are not just frequent words;
they must be center-forming and useful under coverage/stability gates.
```

## Weak / Not Proven

Current weak points:

```text
1. No full SDM-style capacity curve for L1/L2/L3 together.
2. Center collision and residual behavior are measured only in slices.
3. L1/L2 similarity can still become shortcut authority if not gated.
4. Dense v3 L3 failure shows that compact handles alone are not enough.
5. Hot/cold center policy still needs stronger center-stability evidence.
```

The central danger:

```text
similarity becomes answer authority
```

The project must keep the line:

```text
similarity can seed candidates;
it cannot by itself prove operator understanding.
```

## Next Proof / Debt

Take these into work:

```text
address-radius sweep:
  measure how far surface perturbations can move before center recall breaks.

center collision audit:
  count collisions that merge unrelated L1/L2 motifs.

hot/cold split proof:
  show hot centers improve coverage/stability without deleting cold residual recall.

capacity curve:
  measure patterns/operators vs false positive rate and memory bytes.

nearest-neighbor guard:
  keep L2-neighbor / surface-family baselines in every L3 proof.
```

## Status

```text
relation to sparse distributed memory: YES
literal Kanerva SDM implementation: NO
useful high-dimensional sparse-memory idea: YES
operator proof from SDM alone: NO
next work: capacity / collision / hot-cold / nearest-neighbor guards
```

