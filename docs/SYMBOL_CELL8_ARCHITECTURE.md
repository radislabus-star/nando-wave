# SymbolCell8 Architecture

## Goal

`SymbolCell8` is the first L3-bound NANDA wave cell.

It must satisfy the NANDA cell requirements while staying exactly:

```text
8192 bytes
```

The cell is not an answer store. It is a small wave resonator that can:

```text
receive excitation
interfere local modes
detect candidate peaks
emit wave advice
receive carrier feedback
decay unsupported state
reflect incompatible waves
```

## Role

`SymbolCell8` is a propagation-first cell.

It should not try to understand a whole word or phrase alone. Its job is to
produce a compact local judgment and share it with the surrounding wave medium.

In other words:

```text
Cell8 does not answer.
Cell8 advises the organism where the wave wants to stabilize.
```

## Fixed Layout

| Блок              | Байт | Назначение              |
|-------------------|-----:|-------------------------|
| Header            |  128 | id/schema/role/checksum |
| Projection        | 1024 | 64 входных lanes        |
| Mode bank         | 4096 | 512 компактных мод      |
| Transition bank   | 1024 | 128 локальных связей    |
| Interference      | 1024 | 128 wave slots          |
| Calibration/stats |  512 | пороги и доверие        |
| Scratch           |  384 | тик и top-k candidates  |

This layout is a development cell format. It is useful because one `SymbolCell8`
is self-contained, but it repeats service metadata in every cell.

The dense runtime format must split class metadata from cell data:

```text
SymbolCliqueClass:
  schema / projection / calibration / role layout

SymbolCellDense2K:
  packed modes
  packed transitions
  packed interference
  tiny local counters
```

The dense cell should not carry its own projection table, schema block, or large
calibration section. Those belong to the clique class.

Target dense layout:

```text
SymbolCliqueClass = 4096 B once per clique
SymbolCellDense2K = 2048 B per cell
```

In a 2 MB clique this gives:

```text
(2 MB - 4 KB class) / 2 KB = 1022 dense cells
```

The architectural point is not only smaller cells. It is that patterns should be
routed sparsely and decomposed into block modes plus composite modes, not written
as global traces through every cell.

```text
bad:  pattern -> whole clique
good: pattern -> 3 blocks -> local modes + shared triad mode
```

## Header

The header identifies the cell and its role in the organism.

Required fields:

```text
magic
id
version
schema
role
flags
checksum
```

The role is important. Not all cells should behave identically.

Initial roles:

```text
input
transition
carrier
guard
memory
reflection
```

## Projection Bank

Projection is the entry point from symbol space into wave space.

Size:

```text
1024 B / 16 B = 64 projection lanes
```

One projection lane should contain:

```text
utf8_len
byte_mix
lane
frequency_hint
amplitude
phase
damping
role
```

Rule:

```text
projection is not memory
```

Projection may select where a symbol enters, but it must not directly decide
the result.

## Mode Bank

The mode bank is the main body of the cell.

Size:

```text
4096 B / 8 B = 512 modes
```

One mode:

```text
frequency_id: u16
sin_weight: i8
cos_weight: i8
amplitude: i8
phase: i8
damping: u8
role: u8
```

Per tick, the cell should activate only a small subset:

```text
normal: 4 active modes
rich:   8 active modes
```

The rest remain latent.

## Transition Bank

Transition memory gives the cell sequence sensitivity.

Size:

```text
1024 B / 8 B = 128 transitions
```

One transition:

```text
previous_peak: u16
current_mode: u16
coupling: i8
phase_shift: i8
damping: u8
role: u8
```

The same symbol may lead to a different peak depending on previous wave state:

```text
symbol + previous_peak_a -> peak_x
symbol + previous_peak_b -> peak_y
```

This is mandatory. Without it, the cell is only a symbol recognizer.

## Interference State

Interference is where the local wave exists.

Size:

```text
1024 B / 8 B = 128 slots
```

One slot:

```text
real: i16
imag: i16
energy: u16
coherence: u16
```

A local peak can only be selected from interference state. It cannot be selected
directly from projection or from one mode.

The full field has 128 slots, but the hot tick path uses a bounded active
window:

```text
full field capacity = 128 slots
hot tick window     = 8 slots
```

Only active slots are decayed and considered for top-2 peak selection. Newly
excited slots enter the active window. If the window is full, the weakest active
slot is evicted and cleared. This keeps decay and peak detection proportional
to the live wave frontier instead of the full field size.

## Calibration And Trust

Calibration controls accept/veto behavior.

Required thresholds:

```text
excite
accept
veto
decay
temperature
vigilance
coherence_min
margin_min
reflection_min
second_order_min
```

Required counters:

```text
seen
accepted
reverted
false_positive
reflected
decayed
spurious
limit_cycle
active_slot_count
active_slots[8]
```

This prevents every spike from becoming memory.

`vigilance` is borrowed from Adaptive Resonance Theory. It controls how strict
the cell is about match quality:

```text
low vigilance  -> broad category / tolerate variation
high vigilance -> fine category / reset on mismatch
```

Vigilance rises after mismatch, reflection, false positive, or weak separation.
It falls slowly after repeated coherent stable peaks.

## Scratch

Scratch is tick-local. It must not become hidden long-term memory.

Allowed scratch:

```text
top_k candidates
second_best energy
incoming carrier summary
outgoing advice packet
temporary score components
previous peak ring
second-order phase score
```

Forbidden scratch:

```text
answer cache
unbounded history
large debug trace
external lookup key
```

## Inputs

A `SymbolCell8` tick receives:

```text
current symbol projection
previous local peak
incoming neighbor messages
cluster carrier
global carrier
temperature/noise seed
```

The cell may also receive one warmed cold-memory hint, but only before the active
wave tick. During the active tick, RAM must not participate in interference.

## Outputs

The cell emits advice, not answers.

Forward wave advice:

```text
source_cell: u16
peak_slot: u16
phase: i8
energy: u16
coherence: u16
role: u8
```

Hot target size:

```text
8 bytes if source is implicit
16 bytes if source/target/debug fields are explicit
```

Optional reflected advice:

```text
reflected_phase
reflected_energy
mismatch_role
```

Rule:

```text
one cell emits at most one forward advice message
and at most one reflected message per tick
```

## Tick Phases

### 1. Project

Map the external symbol into a projection lane.

```text
symbol -> projection_lane
```

### 2. Excite

Select 4-8 modes from projection, previous peak, and carrier phase.

```text
projection + previous_peak + carrier -> active_modes
```

### 3. Interfere

Apply modes into the 128 interference slots.

```text
slot.real += mode.cos * amplitude
slot.imag += mode.sin * amplitude
```

Transition coupling may shift phase or amplify/damp the contribution.

### 4. Decay

Decay unsupported old state.

```text
old_field *= decay
```

Noise should disappear quickly. Re-excited structure may persist.

### 5. Detect Candidate Peaks

Find local maxima.

Required candidate checks:

```text
energy above excite
greater than neighbors
greater than average field
top-k rank
```

### 6. Score Stability

Compute:

```text
energy_score
separation_score
coherence_score
transition_score
persistence_score
second_order_score
vigilance_score
veto_score
```

The cell marks a stable local peak only if:

```text
stable_score >= accept
veto_score < veto
```

The second-order score is a stabilizer for phase memory. Oscillatory associative
memory literature shows that first-order phase locking alone can create unstable
or wrong retrieval states. `SymbolCell8` therefore checks whether a candidate is
supported not only by phase alignment, but also by a harmonic/second-order
agreement term.

### 7. Emit Advice

If stable or nearly stable, emit a forward wave message.

If incoming energy mismatched local modes, emit a bounded reflection.

### 8. Receive Feedback

Carrier feedback from L2/L3 updates:

```text
phase bias
threshold bias
damping bias
role bias
```

Feedback must not overwrite local state. It should only tilt the next tick.

## Advice Semantics

The cell advises one of four meanings:

```text
accept: this local peak is stable
support: this peak is not stable alone but supports a cluster peak
reflect: this incoming wave mismatched local modes
veto: this peak looks like a false positive
```

This gives the organism more nuance than yes/no activation.

Internally the peak outcome must be more detailed than the advice message:

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

This distinction is required because a wrong stable state is more dangerous than
no peak. A cell must be able to say:

```text
this is stable, but it is probably not valid
```

## Required Invariants

`SymbolCell8` must obey:

```text
size_of(SymbolCell8) == 8192
projection alone cannot produce accept
one tick cannot export more than one forward peak
one tick cannot export more than one reflection
active-slot decay is applied before stable persistence is counted
stable peak requires energy + separation + coherence + transition support
stable peak also requires second-order support
high energy with low coherence becomes Spurious, not Accepted
alternating peak history becomes LimitCycle, not Accepted
active_slot_count <= 8
scratch is cleared or overwritten every tick
```

## L1/L2/L3 Mapping

```text
L1: one cell tick
L2: 16-cell cluster resonance
L3 fast: 32 clusters / 512 cells / global carrier
L3 max: 64 clusters / 1024 cells / stress carrier
RAM: cold archive only
```

Recommended first organism:

```text
32 clusters x 16 SymbolCell8
512 SymbolCell8 total
4 MiB active cell body
fanout 4
8 B messages
4 active modes per cell
```

Keep `64 clusters / 1024 cells / 8 MiB` as a max/stress profile, not as the
default working organism. The fast profile leaves L3 headroom for code, stack,
cluster centers, reflection traffic, and adjacent runtime state.

## Why This Satisfies NANDA

The cell satisfies the requirements because:

```text
projection creates excitation
modes create wave basis
transitions create sequence sensitivity
interference creates local field
decay removes unsupported noise
peak detector separates structure from spike
advice message enables propagation
feedback input enables global/local coupling
reflection rejects incompatible waves
calibration prevents false memory
fixed 8 KB size prevents hidden lookup growth
```

## First Implementation Target

Do not start with training.

Start with a deterministic proof cell:

```text
SymbolCell8::new(seed, id, role)
SymbolCell8::tick(input, previous_peak, carrier, messages)
SymbolCell8Advice
StablePeakScore
```

First tests:

```text
layout is exactly 8192 bytes
same input produces same peak
different previous peak can shift result
decay removes unsupported spike
coherent repeated input stabilizes
ablation weakens but does not erase
reflection appears on mismatch
```
