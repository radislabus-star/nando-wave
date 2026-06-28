# Understanding-0 Eval

## Choice

The next proof target is not a stored answer and not raw phase coherence.

The first useful target is:

```text
same immediate symbol context
different earlier wave history
same probe symbol
different stable group center
```

This is the smallest practical "understanding-0" gate for NANDA.

## Why

Markov-1 can only see the immediate previous state. If two probes have the same
immediate suffix, a Markov-1 baseline should collapse them.

The wave organism may pass only if earlier context survives inside the cluster
state and changes the final center.

## Gate

For each probe pair:

```text
left context  = different earlier history
right context = different earlier history
last context symbol is the same
probe symbol is the same
```

Required:

```text
both full contexts produce a centered wave
full contexts split into different centers
suffix-only control collapses to the same center
exact replay is stable
```

This does not prove human understanding. It proves a smaller property:

```text
the group center carries context that is not reducible to the last symbol
```

That is the first measurable place to look for a primitive cell mind.

## Command

```bash
cargo run -p nando-cli -- eval-symbol-understanding
```

Current scale result:

```text
cluster-16  pass
micro-64    pass
small-128   watch
turbo-256   pass
default-512 pass
```

The overall status is currently:

```text
symbol-understanding0-eval-watch
```

This is intentional. The 128-cell scale exposes a non-monotonic blind spot: one
probe pair collapses instead of splitting. The 256-cell turbo profile and the
512-cell default profile pass this first gate, but the scale sweep warns us that
more cells do not automatically mean a stronger primitive mind.

## Next Gate

After this passes, the next harder gate is retrieval:

```text
store several patterns
probe with partial/noisy context
select the correct stable center
reject conflicting/noise probes
fail under ablation
```
