# Position 5: Superposition / Sparse Features

Anchor:

```text
docs/ARCHITECTURE.md:17
```

## Central Work

- Anthropic, "Toy Models of Superposition".
- Reference handles:

```text
https://transformer-circuits.pub/2022/toy_model/index.html
https://arxiv.org/abs/2209.10652
```

Known lesson:

```text
models can represent more sparse features than dimensions,
but this creates interference and polysemanticity/collision pressure.
```

## Nando Wave Mapping

Nando Wave has several superposition-like pressure points:

```text
SurfaceWave4096 lanes
folded slot projection
sparse active centers
compressed role/action edges
many rules sharing compact center space
```

The v3 pressure gate made this concrete:

```text
lengths up to 8
48 proof_rule_ids
8 output slots
folded projection
same-bag wrong candidates
```

The static diagnostics already show collision/interference pressure:

```text
some different rule actions have similarity_milli = 1000
folded multi-role / wrong-role / missing-true-role signals are nonzero
```

## Stronger For Our Goal

Nando Wave does not treat superposition as only an interpretability problem.

The local goal is:

```text
use compact sparse representations,
but measure when compression becomes destructive interference.
```

This is why v3 is valuable even as a failure:

```text
it finds where compact role/action features stop separating.
```

That is more useful than a green test that hides collisions behind an easy
shortcut.

## Weak / Not Proven

Current weak points:

```text
1. No full collision budget per layer.
2. Folded projection pressure is measured but not fully isolated.
3. Action motifs can collide completely under current signatures.
4. No learned de-superposition / disentangling mechanism exists yet.
5. No capacity law connects feature count, slot count, rule count, and accuracy.
```

The central danger:

```text
compression makes the model smaller,
but also makes different operators indistinguishable.
```

That is exactly the v3 suspicion:

```text
not enough action/operator separability under dense matrix pressure.
```

## Next Proof / Debt

Take these into work:

```text
collision budget:
  per layer, count how often unrelated features share lanes/centers.

feature capacity sweep:
  vary rules, lengths, slots, surfaces, and folded span.

de-superposition test:
  test learned output phase centers or learned action centers only after
  collision diagnostics show the need.

ablation by collision class:
  separate clean failures from folded-collision failures.

operator separability metric:
  require different rule actions to have lower similarity than same-rule actions.
```

## Literature Update: Superposition Decoding

Date:

```text
2026-07-02
```

Relevant classical / VSA answer:

```text
HRR and VSA systems expect bound-symbol superpositions to decode noisily.
The usual fix is not to pretend the raw readout is clean; it is cleanup memory,
winner selection, no-decision margins, or multi-step decoding/explaining-away.
```

Current mapping:

```text
Nando Wave v4 conditional:
  sequence energy can choose the correct whole transform;
  strict slot readout still fails under ru_words/network surface pressure;
  sign-aware matching leaves 871-932 milli same-sign residual collision.
```

Proof debt:

```text
Treat the remaining red gate as a superposition decoding problem:
  measure same-sign residual collision against runtime gap;
  test cleanup/readout candidates against ablations;
  do not add local_out_t or proof_rule_id authority.
```

## Status

```text
relation to sparse feature superposition: YES
superposition fully controlled: NO
v3 failure informative: YES
folding proven main cause: NO
next work: collision budget / capacity sweep / de-superposition proof
```
