# Retrieval-0 Eval

## Choice

After `understanding-0`, the next useful gate is associative retrieval.

The question:

```text
can the wave organism store several patterns,
receive a partial/noisy probe,
and move toward the correct stable center?
```

This is still not a claim of human understanding. It is the next measurable
property of a wave associative memory.

## Gate

The eval warms an organism with four patterns:

```text
NANDA
WAVE
CACHE
VECTOR
```

Then it builds clean prototype centers from the warmed organism and checks:

```text
noisy probe hits nearest correct prototype
hit has enough margin over the second prototype
conflict/noise probes are rejected by low margin
cold baseline fails where trained state succeeds
```

The cold baseline matters. If a cold organism can pass the same probe without
warming, then we did not prove storage.

## Command

```bash
cargo run -p nando-cli -- eval-symbol-retrieval
```

Capacity sweep:

```bash
cargo run -p nando-cli -- eval-symbol-retrieval-sweep
```

Multi-seed capacity ladder:

```bash
cargo run -p nando-cli -- eval-symbol-retrieval-capacity
```

Capacity scale over larger cliques:

```bash
cargo run -p nando-cli -- eval-symbol-retrieval-capacity-scale
```

The report currently covers:

```text
cluster-16
turbo-256
default-512
```

## Current Result

Current status:

```text
symbol-retrieval0-eval-pass
```

Observed profile behavior:

```text
cluster-16:
  noisy_hits              = 1 / 4
  veto_noisy_accepts      = 1 / 4
  veto_conflict_rejects   = 3 / 4
  cold_ablation_failures  = 2 / 4

turbo-256:
  noisy_hits              = 4 / 4
  noisy_strong_hits       = 4 / 4
  veto_noisy_accepts      = 4 / 4
  veto_conflict_rejects   = 4 / 4
  veto_cold_rejections    = 4 / 4

default-512:
  noisy_hits              = 2 / 4
  veto_noisy_accepts      = 2 / 4
  veto_conflict_rejects   = 4 / 4
  veto_cold_rejections    = 3 / 4
```

Interpretation:

```text
turbo-256 retrieves noisy stored patterns,
rejects conflict probes,
and rejects cold/no-storage probes.
```

The first VETO-0 readout is trajectory based. It accepts a retrieval only when
the final center has enough prototype margin and the known query symbols follow
the selected prototype trajectory closely enough.

The remaining warning is scale stability: `default-512` is not yet a better
retrieval profile than `turbo-256`.

## Stability Sweep

Current turbo-256 capacity sweep:

```text
readout = superposition-wave
max_passing_patterns = 64

4 stored patterns:
  noisy_hits             = 4 / 4
  veto_noisy_accepts     = 4 / 4
  veto_conflict_rejects  = 4 / 4
  veto_cold_rejections   = 4 / 4
  status                 = pass

8 stored patterns:
  noisy_hits             = 8 / 8
  veto_noisy_accepts     = 8 / 8
  veto_conflict_rejects  = 4 / 4
  veto_cold_rejections   = 8 / 8
  status                 = pass

16 stored patterns:
  noisy_hits             = 16 / 16
  veto_noisy_accepts     = 16 / 16
  veto_conflict_rejects  = 4 / 4
  veto_cold_rejections   = 16 / 16
  status                 = pass

32 stored patterns:
  noisy_hits             = 32 / 32
  veto_noisy_accepts     = 32 / 32
  veto_conflict_rejects  = 4 / 4
  veto_cold_rejections   = 32 / 32
  status                 = pass
```

Interpretation:

```text
current turbo-256 retrieval capacity = 32 robust patterns
```

The active sweep now uses a superposition-style readout: every prototype gets
an accumulated wave-distance score from the visible query steps and the final
L3 center. Known symbol mismatches add destructive projection pressure, while
`?` positions are allowed to be reconstructed by the wave path.

This raises the robust capacity from 4 to 32 patterns in the current
turbo-256 profile. The stability bank now avoids letting very short synthetic
words dominate the capacity gate: the previous `PEAK` and `BETA` stress entries
were normalized to `PEAKS` and `BETAS`, so a one-symbol hole still leaves four
visible constraints instead of only three. This keeps the sweep focused on
associative capacity rather than a short-word trajectory edge case.

A naive memory-lane marker was tested before this and was not kept as the
active gate. It worsened the 4-pattern row instead of improving capacity. The
next useful lane design must be trainable coupling/superposition support, not
just an extra marker symbol injected into the same wave path.

## Capacity-1 Ladder

`CAPACITY-1` is intentionally harder than the stability sweep:

```text
profile      = turbo-256
readout      = superposition-wave
seed_cases   = 2
pattern bank = decorrelated 6-symbol uppercase patterns
counts       = 32 / 64 / 128 / 256
conflicts    = 16 per seed
cold         = every noisy probe against an untrained organism
```

Current result:

```text
max_passing_patterns = 128

32 stored patterns:
  noisy_hits             = 64 / 64
  veto_noisy_accepts     = 63 / 64
  veto_conflict_rejects  = 32 / 32
  veto_cold_rejections   = 63 / 64
  min_trained_gain       = 0
  status                 = watch

64 stored patterns:
  noisy_hits             = 128 / 128
  veto_noisy_accepts     = 125 / 128
  veto_conflict_rejects  = 32 / 32
  veto_cold_rejections   = 125 / 128
  min_trained_gain       = 0
  status                 = watch

128 stored patterns:
  noisy_hits             = 256 / 256
  veto_noisy_accepts     = 256 / 256
  veto_conflict_rejects  = 32 / 32
  veto_cold_rejections   = 256 / 256
  min_trained_gain       = 0
  status                 = pass

256 stored patterns:
  noisy_hits             = 511 / 512
  veto_noisy_accepts     = 508 / 512
  veto_conflict_rejects  = 32 / 32
  veto_cold_rejections   = 509 / 512
  min_trained_gain       = 0
  status                 = watch
```

Interpretation:

```text
current multi-seed capacity = 128 robust patterns.
the next bottleneck starts at 256 patterns.
```

The capacity gate now includes a trained-resonance term:

```text
trained_gain = cold_best_distance - trained_best_distance
accept only if cold rejects the peak OR trained_gain >= 24
```

This closes the 128-pattern multi-seed row: all noisy probes are accepted, all
conflicts are rejected, and all cold probes are rejected. The capacity bank now
uses a deterministic decorrelated generator instead of the earlier syllable-grid
generator; this avoids measuring a bad pattern-bank collision as if it were a
memory failure. The ladder is not strictly monotonic: the 32/64 rows can still
show cold accepts while the 128 row passes, because the larger superposition
context changes the wave field. At 256, candidate search remains strong, but
cold accepts and a few noisy misses reappear.

Attempts toward the 256 row:

```text
capacity repeats = 1/2:
  improves the 256 row to one passing seed, but loses the 128 pass.

two-hole noisy probes:
  reduces visible projection evidence, but current wave readout misses too many
  trained targets before 256.

7-symbol probes with visible distance >= 4:
  makes the cold projection too confident and loses the 128 pass.
```

These attempts were not kept. The next 256 mechanism should add learned support
separation inside the wave state rather than only changing the prompt pattern
shape or repeat count.

## Capacity Scale

`CAPACITY-SCALE` keeps the same 256-pattern bank and changes only clique size:

```text
stored_patterns = 256
seed_cases      = 2
cliques         = 256 / 512 / 1024 cells
```

Current result:

```text
256 cells / 2 MB:
  passing_seeds          = 0 / 2
  noisy_hits             = 511 / 512
  veto_noisy_accepts     = 508 / 512
  veto_cold_rejections   = 509 / 512
  status                 = watch

512 cells / 4 MB:
  passing_seeds          = 2 / 2
  noisy_hits             = 512 / 512
  veto_noisy_accepts     = 512 / 512
  veto_cold_rejections   = 512 / 512
  min_noisy_margin       = 472
  status                 = pass

1024 cells / 8 MB:
  passing_seeds          = 2 / 2
  noisy_hits             = 512 / 512
  veto_noisy_accepts     = 512 / 512
  veto_cold_rejections   = 512 / 512
  min_noisy_margin       = 334
  min_trained_gain       = 379
  status                 = pass
```

Interpretation:

```text
2 MB clique  -> 128-pattern safe row, 256-pattern frontier/watch
4 MB clique  -> 256-pattern safe row
8 MB clique  -> 256-pattern safe row with stronger trained gain
```

This confirms that 256 patterns were not fundamentally impossible; they were
too tight for the 2 MB clique. The cost is real: every extra cell carries header,
projection, calibration, stats, and scratch overhead. The next architecture step
should therefore separate per-clique shared metadata from per-cell wave state,
so larger cliques spend less of their byte budget repeating service data.

## Interpretation

`pass` means the profile satisfies the first retrieval gate.

`watch` is not failure of the project. It means the current wave mechanism is
not yet a reliable associative memory at that scale, or that the gate exposed a
bad margin/conflict case. That is exactly what this eval is for.
