# Current Core Decision

Date: 2026-06-20

## Decision

Nando Wave now has two generations of core code in the repository:

```text
legacy/control: Stage2 Cell32 / Organ128
current direction: SymbolCell8 / SymbolWaveCluster / SymbolL3Organism
```

The project must not treat the legacy Stage2 status as the main NANDA core.
Stage2 remains useful as a control baseline, benchmark, and historical
evidence path. The active research direction is the SymbolL3/Cell8 wave
organism.

## Why

The SymbolL3 path is closer to the intended NANDA architecture:

```text
symbol projection
-> local modes
-> transition memory
-> interference slots
-> compact wave advice
-> cluster center
-> L3 center
```

This is structurally different from a phrase cache or lookup wrapper. A valid
NANDA integration must use the wave field and stable peaks, not only store
text continuations behind keys.

## Current evidence

Recent local checks:

```text
nando-cli status
  still reports: stage-2-fixed-wave-tick

nando-cli eval-symbol-l3
  mode_status: symbol-l3-eval-pass

nando-cli eval-symbol-understanding
  mode_status: symbol-understanding0-eval-watch

nando-cli eval-symbol-retrieval
  mode_status: symbol-retrieval0-eval-pass
```

Important nuance:

```text
retrieval0 currently passes mainly through turbo-256
default-512 is not yet the strongest profile
```

So the new core is real, but the project has not yet promoted it cleanly to the
main route.

## Next architectural move

Do not delete Stage2 blindly.

Instead:

```text
1. Mark Stage2 / Organ128 as legacy controls.
2. Make SymbolL3 / Cell8 the explicit current NANDA core.
3. Update status/help/docs/checks to show this split.
4. Keep old evals as controls, not as the default identity of the project.
5. Only then port the native SymbolL3 mechanism into lay.
```

## Rule for lay integration

Do not integrate a text-cache wrapper as "NANDA".

For lay, the useful future route is:

```text
lay candidates / typing context
-> SymbolL3/Cell8 wave organism
-> stable peak / reflection / veto signal
-> safe replacement pipeline
```

If the integration cannot expose wave evidence, stable peak state, or
reflection/veto behavior, it is not the NANDA core. It is only a helper layer.
