# NANDA Wave Theorem

## Purpose

NANDA must prove that it does not merely store an answer behind a key.

The target claim is stronger:

```text
the wave structure itself selects a stable peak
```

This document defines what that sentence means before the runtime grows more
code paths.

## Core Statement

A NANDA cell is a bounded wave medium. An input symbol does not directly select
an output. It excites a projection of local modes. Those modes interfere with
the previous state, transition memory, and global carrier feedback.

A response is valid only when the system produces a stable peak.

## Stable Peak

A stable peak is not the largest number in one tick.

A stable peak is a local maximum in the wave field that survives enough tests to
be treated as structure rather than noise.

In practical terms, a peak is stable when all of the following are true:

1. It has enough energy.

   The peak must rise above the excitation threshold:

   ```text
   energy(p) >= excite
   ```

2. It has enough separation.

   The peak must be clearly stronger than nearby or competing peaks:

   ```text
   energy(p) - energy(second_best) >= margin
   ```

3. It has phase coherence.

   The real and imaginary parts must point in a consistent direction instead of
   cancelling each other:

   ```text
   coherence(p) >= coherence_min
   ```

4. It persists across ticks.

   The same peak, or a nearby compatible peak, must reappear after decay and the
   next transition:

   ```text
   peak(t) ~= peak(t + 1) ~= peak(t + 2)
   ```

5. It survives perturbation.

   Small changes in carrier phase, damping, input encoding, or neighboring
   state must not immediately move the decision somewhere unrelated:

   ```text
   perturb(input) -> same_peak_region
   ```

6. It is supported by transition memory.

   The current peak must be compatible with the previous symbol/cell state:

   ```text
   transition(prev_peak, current_peak) > 0
   ```

7. It is accepted by calibration.

   The peak must pass accept/veto thresholds:

   ```text
   energy >= accept
   veto_signal < veto
   ```

8. It is not only a memorized lookup.

   Removing one direct projection lane must weaken the peak, but not completely
   destroy it if surrounding modal, transition, and carrier support remains:

   ```text
   ablate(direct_lane) -> degraded_but_recognizable_peak
   ```

9. It is not a spurious attractor.

   A peak can be stable and still wrong. Stability alone is not truth:

   ```text
   stable_wrong_peak -> Spurious
   ```

10. It is not a limit cycle.

    If the system alternates between two or more peaks instead of settling, the
    state is not accepted:

    ```text
    p(t) -> q(t + 1) -> p(t + 2) = LimitCycle
    ```

11. It has second-order support.

    First-order phase alignment is not enough. Oscillatory associative memory
    work shows that phase-locked retrieval can be unstable without an additional
    stabilizing term. NANDA therefore requires a second-order or harmonic support
    component:

    ```text
    second_order_score(p) >= second_order_min
    ```

## Cell Requirements

If NANDA is a wave associative memory, then a cell is not a passive storage
block. It must satisfy the following requirements.

### 1. Projection Requirement

A cell must turn an external symbol into an internal wave excitation without
using a direct answer table.

```text
symbol -> projection -> modal excitation
```

The projection may be deterministic, but it must not be the whole memory. If
projection alone is enough to answer, the system has collapsed into lookup.

### 2. Modal Requirement

A cell must contain many independent modes.

Each mode contributes phase, amplitude, damping, and role:

```text
mode = frequency + sin/cos weight + amplitude + phase + damping + role
```

The cell must be able to activate several modes for one symbol. One symbol must
not equal one mode.

### 3. Interference Requirement

A cell must accumulate real interference state.

```text
interference = real + imag + energy + coherence
```

The current decision must depend on interference between modes, not only on the
strongest individual mode.

### 4. Transition Requirement

A cell must be sensitive to sequence.

The same symbol after different previous symbols may produce different stable
peaks:

```text
peak("a" after "n") != peak("a" after "r")
```

This is required for memory and understanding. Without transition dependence,
the cell only recognizes isolated symbols.

### 5. Decay Requirement

A cell must forget unsupported excitation.

```text
field(t + 1) = field(t) * decay + excitation(t + 1)
```

Noise must disappear. Structure may remain only if it is re-excited or supported
by transition/carrier feedback.

### 6. Peak Requirement

A cell must distinguish:

```text
candidate peak
stable peak
rejected spike
```

A high-energy spike is not enough. The cell must check energy, separation,
coherence, persistence, transition support, and veto.

### 7. Propagation Requirement

A cell must emit a compact wave message to the surrounding medium.

The message is not an answer. It is a contribution:

```text
cell -> peak phase + energy + coherence + role
```

This output lets other cells resonate, reflect, or damp the signal.

### 8. Feedback Requirement

A cell must accept feedback from the wider wave field.

```text
global carrier -> local phase / thresholds / damping
```

Without feedback, cells cannot form an organism-level stable state. They remain
independent recognizers.

### 9. Reflection Requirement

A cell must be able to reject incompatible incoming waves without ignoring them.

```text
incoming mismatch -> reflected component
incoming alignment -> absorbed component
```

Reflection is required for routing: incompatible waves should not simply vanish;
they should return information to the medium.

### 10. Calibration Requirement

A cell must track trust and error.

At minimum:

```text
seen
accepted
reverted
false_positive
```

This prevents every resonance from becoming memory.

### 11. Ablation Requirement

A cell must not depend on one exact lane.

Removing a direct projection lane should degrade the peak, not erase all
recognition if modal, transition, and carrier support remain.

```text
ablate(one lane) -> weaker but still related peak
```

This is the practical test that the cell is distributed.

### 12. Bounded Memory Requirement

A cell must remain a fixed-size atom.

For `SymbolCell32`:

```text
cell size = 32 768 bytes
mode bank = 16 384 bytes
mode count = 2048
```

The cell may change its internal state, but it must not grow external lookup
tables to pass tests.

## Minimal Valid Cell

A minimal valid NANDA cell must therefore have:

```text
projection
mode bank
transition bank
interference state
decay
stable peak detector
propagation output
feedback input
reflection path
calibration/trust counters
fixed memory size
```

If any of these are missing, the cell can still be useful, but it is not yet a
full wave associative memory cell.

## Non-Peaks

The following are not stable peaks:

- a one-tick energy spike;
- a direct hash bucket hit;
- a peak that disappears after decay;
- a peak that flips under tiny carrier noise;
- a peak with high energy but low coherence;
- a peak that exists only when one exact lookup lane is present;
- a peak that wins only because all competitors are zero.

## Wave Field Definition

For one cell `C`, one symbol `x`, and time `t`:

```text
projection = P(x)
excited_modes = E(C, projection)
transition = T(previous_state, excited_modes)
carrier = G(global_bus)
field(t) = decay * field(t - 1)
         + interference(excited_modes, transition, carrier)
```

Each interference slot has:

```text
real
imag
energy
coherence
```

Energy is the magnitude of the accumulated wave:

```text
energy = sqrt(real^2 + imag^2)
```

Coherence is the alignment of the current wave with recent local and global
phase:

```text
coherence = alignment(local_phase, previous_phase, carrier_phase)
```

## Triad Composite Modes

NANDA must not store every pattern as a separate global trace. A pattern can be
split into local blocks. Each block may produce its own mode, and the relation
between blocks may produce an additional shared mode.

This is the key distinction:

```text
blocks:       B0, B1, B2
local modes:  m0, m1, m2
shape mode:   M = relation(m0, m1, m2)
```

The three local modes are information by themselves. For example:

```text
2, 3, 5
```

can be treated as three different modal facts. But their joint contour is also
information. For example:

```text
1, 2, 1
```

is not only three values. It is a symmetric rise-and-fall contour. In wave terms
it may behave like a half-wave:

```text
low -> high -> low
```

That contour should be able to excite a shared composite mode:

```text
triad_shape(1, 2, 1) -> half_wave_mode
```

This means storage is not:

```text
whole pattern -> whole clique
```

The intended storage rule is:

```text
pattern -> routed blocks -> local modes + composite triad mode
```

The composite mode is what lets many patterns share structure without occupying
a full independent trace. It is closer to Fourier composition than to a lookup
table: local modes act like components, and the triad contour acts like a
second-order harmonic constraint.

### Triad Acceptance Rule

A triad is accepted only when both levels agree:

```text
local support:    m0, m1, m2 are individually plausible
composite support: relation(m0, m1, m2) excites a stable shared mode
```

So a false pattern cannot win merely because one block is strong. The triad must
also produce the expected group wave.

### Why This Matters For Capacity

Global storage wastes cells:

```text
pattern -> all cells
```

Triad storage should be sparse:

```text
pattern -> 3 routed blocks
        -> 3 local modes
        -> 1 shared composite mode
```

This gives NANDA a way to store many more patterns in the same memory budget:
patterns can reuse local block modes and only need distinct composite constraints
when their block relation differs.

## Peak Candidate

A peak candidate is a slot or small slot region whose energy is locally maximal:

```text
candidate p is peak-like if:
energy(p) > energy(neighbor_left)
energy(p) > energy(neighbor_right)
energy(p) > average_field_energy
```

This only creates a candidate. It does not yet create a stable peak.

## Stability Score

The stable peak score combines independent supports:

```text
stable_score =
    energy_score
  * separation_score
  * coherence_score
  * persistence_score
  * transition_score
  * second_order_score
  * vigilance_score
```

A peak is accepted when:

```text
stable_score >= accept_threshold
veto_score < veto_threshold
```

This matters because a high-energy spike can still be rejected if it lacks
coherence or persistence.

The accepted state must also be classified by outcome:

```text
Accepted
Supported
Reflected
Vetoed
Spurious
LimitCycle
Unstable
NoPeak
```

This prevents the system from confusing:

```text
stable and valid
stable but wrong
unstable but high-energy
oscillating without settling
```

## Reflection

Reflection happens when an incoming wave reaches a cell but does not align with
that cell's local modes.

```text
reflected = incoming * mismatch * reflection_gain
absorbed = incoming * alignment
damped = incoming * damping
```

Reflection is not an error. It is how the system says:

```text
this wave arrived, but this cell cannot accept it in this phase
```

The reflected component can return to the bus and influence other cells.

## Decay

Decay prevents the system from treating old excitation as fresh evidence.

```text
field(t + 1) = field(t) * decay + new_excitation
```

Without decay, every old peak becomes permanent memory. With too much decay,
no peak can persist. The useful regime is between those extremes:

```text
noise decays quickly
structure decays slowly because it is re-excited
```

## L1 to L3 and Back

The cache language is an engineering analogy for wave scope:

```text
L1: one SymbolCell32 local field
L2: small cluster of cells sharing short transitions
L3: larger WaveBus / organism-level carrier
```

Forward propagation:

```text
cell peak -> bus contribution -> cluster/global center
```

Backward propagation:

```text
global center -> carrier feedback -> cell thresholds and phase alignment
```

The important claim is bidirectional:

```text
local cells create the global wave
global wave changes which local peaks can stabilize
```

## Main Theorem Draft

If:

1. symbols are projected into a modal wave space;
2. modes interfere as complex waves;
3. previous state contributes through transition memory;
4. old energy decays;
5. global carrier feedback returns from the bus to cells;
6. accept/veto thresholds require energy, separation, coherence, persistence,
   and transition support;

then:

```text
stable peaks can emerge without a direct input -> output table
```

## What Must Be Measured

The implementation must prove the theorem with evals, not with naming.

Required probes:

1. Determinism

   Same input, same seed, same state must produce the same peak.

2. Separation

   Real symbols must produce distinguishable peak regions.

3. Persistence

   A true peak must survive several ticks with decay.

4. Perturbation

   Small noise must not destroy the selected region.

5. Ablation

   Removing direct lanes must hurt but not erase supported peaks.

6. Transition dependence

   The same symbol after different previous symbols may stabilize different
   peaks.

7. Carrier dependence

   L3 feedback must improve stability or reject false peaks.

8. Spurious attractors

   Stable but wrong peaks must be detected and counted separately from accepted
   peaks.

9. Limit cycles

   Alternating peak histories must be detected instead of reported as unstable
   noise or accepted understanding.

10. Vigilance

    Mismatch must raise the strictness of acceptance; repeated coherent matches
    may lower it slowly.

11. Second-order support

    First-order phase alignment must not be enough to accept a peak.

## Implementation Boundary

Do not add more runtime layers until this definition is represented in tests.

The next implementation step should be the smallest possible eval:

```text
SymbolCell32 sequence
-> candidate peaks
-> stability score
-> stable / unstable decision
```

Only after that should the project grow `SymbolWaveBus` or L1/L2/L3 feedback.
