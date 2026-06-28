# NANDA Literature Notes

## Purpose

These notes exist to prevent NANDA from rediscovering known mistakes.

The current target is:

```text
wave associative memory
where storage, search, propagation, and understanding are one stabilization process
```

The literature does not give this exact architecture, but it gives strong warnings
and useful mechanisms.

## Sources Read In This Pass

Primary and near-primary sources used:

- John J. Hopfield, "Neural networks and physical systems with emergent
  collective computational abilities", PNAS, 1982.
  https://www.pnas.org/doi/10.1073/pnas.79.8.2554
- Gail A. Carpenter and Stephen Grossberg, "A Massively Parallel Architecture
  for a Self-Organizing Neural Pattern Recognition Machine", Computer Vision,
  Graphics, and Image Processing, 1987.
  https://sites.bu.edu/steveg/files/2016/06/CarGro1987CVGIP.pdf
- Tony A. Plate, "Holographic Reduced Representations", IEEE Transactions on
  Neural Networks, 1995.
  https://redwood.berkeley.edu/wp-content/uploads/2020/08/Plate-HRR-IEEE-TransNN.pdf
- Pentti Kanerva, "Hyperdimensional Computing: An Introduction to Computing in
  Distributed Representation with High-Dimensional Random Vectors", Cognitive
  Computation, 2009.
  https://redwood.berkeley.edu/wp-content/uploads/2018/01/kanerva2009hyperdimensional.pdf
- Takashi Nishikawa, Ying-Cheng Lai, and Frank C. Hoppensteadt, "Capacity of
  Oscillatory Associative-Memory Networks with Error-Free Retrieval", Physical
  Review Letters, 2004.
  https://doi.org/10.1103/PhysRevLett.92.108101
- M. Jimenez-Traves, M. A. Avedillo, B. Linares-Barranco, and J. Nunez,
  "Learning algorithms for oscillatory neural networks as associative memory for
  pattern recognition", Frontiers in Neuroscience, 2023.
  https://pmc.ncbi.nlm.nih.gov/articles/PMC10716297/

## 1. Hopfield / Attractor Memory

### Useful For NANDA

Hopfield gives the cleanest warning and the cleanest inspiration:

```text
memory can be a dynamically stable attractor
```

For NANDA, this supports:

```text
stable peak = attractor-like wave state
```

It also gives a discipline for eval:

```text
input starts corrupted/noisy
system should settle to a stored/stable state
```

### Risk

Attractor systems can create unwanted stable states:

```text
spurious attractors
limit cycles
wrong but stable states
```

NANDA must never treat "stable" alone as "true".

### NANDA Rule

Stable peak acceptance must require:

```text
energy
separation
coherence
persistence
transition support
veto check
ablation survival
```

Stability alone is not enough.

## 2. Adaptive Resonance Theory

### Useful For NANDA

ART directly addresses the stability-plasticity problem:

```text
learn new patterns without destroying old stable codes
```

The important mechanisms for NANDA:

```text
top-down expectation
match / mismatch
vigilance
reset
search for a finer category
```

This maps well to:

```text
carrier feedback
accept / veto
reflection
stable peak threshold
```

### Risk

If vigilance is too low:

```text
different meanings collapse into one broad peak
```

If vigilance is too high:

```text
the system fragments into too many narrow peaks
```

### NANDA Rule

NANDA needs a dynamic vigilance equivalent:

```text
low conflict -> allow broad stable peak
high mismatch -> raise vigilance and search finer
```

Reflection should not be just rejection. It should behave like an orienting/reset
signal:

```text
mismatch -> reflected wave -> cluster tries another center
```

## 3. Holographic Reduced Representations

### Useful For NANDA

HRR shows how complex structures can be represented in a fixed-width vector:

```text
binding by circular convolution
superposition by addition
cleanup memory for noisy reconstruction
```

This is close to NANDA's:

```text
projection + mode bank + interference
```

The key idea:

```text
associations can live in the same-dimensional space as items
```

For NANDA, this means transition/mode relations should not require an expanding
external table.

### Risk

HRR reconstructions are noisy and require cleanup.

In NANDA, cleanup memory could accidentally become:

```text
hidden lookup table
```

### NANDA Rule

Cleanup is allowed only if it is treated as a wave operation:

```text
cleanup = stabilization into nearest known wave mode
not direct answer retrieval
```

Eval must include:

```text
ablate direct projection lane
verify peak degrades but survives via distributed support
```

## 4. Hyperdimensional Computing / Sparse Distributed Memory

### Useful For NANDA

Kanerva/HDC gives two important pieces:

1. High-dimensional random representations are robust.
2. Content-addressable memory can retrieve from noisy addresses.

This supports NANDA's idea that:

```text
projection may be random-ish
meaning can still be recovered by distributed similarity
```

It also supports a cleanup stage, but with the same warning as HRR.

### Risk

HDC can become too vector-symbolic and stop being wave-dynamic.

If NANDA turns into:

```text
hypervector lookup + nearest neighbor
```

then the wave theorem is lost.

### NANDA Rule

NANDA can borrow:

```text
near-orthogonal random projections
similarity robustness
binding / permutation ideas
cleanup as stabilization
```

But the active decision must still come from:

```text
interference + propagation + carrier feedback + stable peak
```

## 5. Oscillatory Associative Memory

### Useful For NANDA

Oscillatory neural networks validate the general direction:

```text
information can be encoded in phase relationships
phase locking can implement associative memory
```

This matches:

```text
cluster center of mass
insight as phase-lock transition
```

### Critical Warning

Nishikawa, Lai, and Hoppensteadt show that ordinary oscillator associative
memory can have unstable error-free retrieval states and near-zero error-free
capacity unless the coupling function is modified.

This is a major warning for NANDA:

```text
phase synchronization alone is not enough
```

The stabilizer matters.

### NANDA Rule

NANDA needs an explicit stability control term.

Candidate equivalents:

```text
second harmonic / second-order mode
coherence gate
vigilance / reset
transition support
carrier feedback
veto against spurious stable peaks
```

The stable peak detector must distinguish:

```text
desired stable peak
spurious stable peak
limit cycle
unstable phase lock
```

## 6. Modern ONN Learning Lessons

### Useful For NANDA

The ONN learning paper shows practical constraints matter:

```text
symmetric coupling
low precision weights
online/local updates
correlated patterns are hard
spurious states and limit cycles must be counted
```

This maps strongly to `SymbolCell8`:

```text
i8 weights
u8 damping
bounded fanout
local updates
fixed 8 KB
```

### Risk

One-shot Hebbian learning is often weak, especially with correlated patterns.

### NANDA Rule

Do not start with a single Hebbian update and call it learning.

Use evals that classify endings:

```text
correct stable peak
wrong stable peak
spurious peak
limit cycle
no stable peak
```

## Design Changes For NANDA

### Add Vigilance

Add a dynamic vigilance-like field:

```text
vigilance: u8
```

It should rise when:

```text
mismatch
false_positive
high reflection
low separation
```

It should fall when:

```text
stable repeated peak
low conflict
high coherence
```

### Add Peak Outcome Types

Stable peak should not be boolean.

Use:

```text
Accepted
Supported
Reflected
Vetoed
Spurious
LimitCycle
Unstable
```

### Add Cluster Reset

When a cluster center has high energy but low coherence:

```text
raise vigilance
emit reset/reflection
try another center
```

### Add Second-Order Stabilization

Because oscillator associative memory can be unstable, NANDA needs a stabilizing
term beyond first-order phase alignment.

Possible implementation:

```text
mode.role includes harmonic role
cluster carrier includes first and second harmonic
stable_score includes second-order agreement
```

### Add Cleanup Carefully

Cleanup is allowed, but only as:

```text
nearest stable wave mode
```

Forbidden:

```text
direct answer table
exact string lookup
hidden corpus search during active tick
```

## Required Evals Before More Runtime

### Stable Peak Eval

```text
same input + same seed -> same peak
repeated coherent input -> stable peak
one-tick spike -> rejected
```

### Spurious Attractor Eval

```text
random / mixed inputs
must not become accepted stable peaks
```

### Limit Cycle Eval

```text
detect p(t) -> q(t+1) -> p(t+2)
```

### Vigilance Eval

```text
same input family, low mismatch -> shared broad center
same input family, high mismatch -> split finer centers
```

### Perturbation Eval

```text
small carrier noise -> same peak region
large noise -> uncertainty, not confident wrong answer
```

### Ablation Eval

```text
remove direct projection lane
peak weakens but does not disappear if distributed support exists
```

### Correlated Pattern Eval

```text
similar sequences should not collapse into one false peak
```

### L3 Traffic Eval

```text
fanout 4
8 B advice messages
reflection bounded to 1 per cell
measure hot working set stays within L3 budget
```

## What We Should Not Do Next

Do not immediately build a large bus.

Do not train on text.

Do not add a cold memory lookup to make examples pass.

Do not accept a peak just because it is high energy.

Do not treat phase lock as proof of meaning.

## Recommended Next Step

Implement the theory-facing evals before expanding runtime:

```text
SymbolCell8 layout
SymbolCell8 tick
StablePeakScore
PeakOutcome
Vigilance
second-order stability component
spurious / limit-cycle / ablation tests
```

Only after those pass should `SymbolWaveCluster` be built.
