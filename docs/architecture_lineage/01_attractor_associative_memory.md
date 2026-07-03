# Position 1: Attractor / Associative Memory

Anchor:

```text
docs/ARCHITECTURE.md:17
docs/ARCHITECTURE.md:118
```

## Central Work

- Hopfield networks: content-addressable memory where dynamics converge toward
  stable stored states.
- Reference handle: Hopfield, 1982, "Neural networks and physical systems with
  emergent collective computational abilities":
  `https://pmc.ncbi.nlm.nih.gov/articles/PMC346238/`

Classical shape:

```text
partial / noisy input
-> recurrent dynamics
-> nearest stable stored state
```

## Nando Wave Mapping

Nando Wave is not a pure Hopfield network. The local mapping is:

```text
active centers
-> compatibility / conflict / anti-wave
-> bounded settle or gap scoring
-> accept or reject
```

Current L3 field intent:

```text
score(center) =
  motif_votes
+ compatibility(other_centers)
- conflict(other_centers)
- anti_wave
```

Current `WavePredictorHebbianField` exposes the related parts:

```text
base_mass
edges
state_delta_edges
state_delta_role_binding_edges
```

The edge channels are:

```text
compatibility
conflict
anti_wave
```

## Stronger For Our Goal

The project is not only asking for nearest-pattern recall.

Nando Wave's intended upgrade is:

```text
correct attractor strengthens
wrong attractor / trap is explicitly suppressed
```

This matters for reasoning tasks because a near-negative can be very close to
the correct state. A useful system must say:

```text
this is close, but wrong
```

not only:

```text
this is close
```

The second upgrade is the goal shift:

```text
classical recall:
  partial X -> full X

Nando Wave target:
  state_t + rule_action -> state_t+1
```

That means the target is transition memory, not static pattern memory.

## Weak / Not Proven

Current missing pieces:

```text
1. No strict global energy function.
2. No proven attraction basin radius.
3. V3 exposes weak dense rule/slot separation.
4. Current WavePredictor is still closer to margin/readout-field
   than a full recurrent attractor.
```

The v3 result is important:

```text
ordered_sequence_accuracy_milli: 269
flat_gap_parity_mismatches: 0
```

Interpretation:

```text
flat runtime is faithful;
the learned field itself does not yet stabilize the dense rule/slot matrix.
```

## Next Proof / Debt

Take these into work before claiming mature attractor behavior:

```text
basin stability:
  measure which perturbation radius preserves the correct transition.

gap stability:
  require median and p10 gap to stay positive under controlled noise.

ablation stability:
  remove compatibility, conflict, and anti-wave channels separately.

transition stability:
  measure state_t + rule_action -> state_t+1, not only final accuracy.

energy proxy:
  define a monotonic or bounded field-energy proxy.
```

## Status

```text
relation to classic attractor memory: YES
better target for reasoning than plain pattern recall: YES
universally better than Hopfield: NO
proved to maturity: NO
next work: basin / gap / ablation / transition / energy proxy
```

