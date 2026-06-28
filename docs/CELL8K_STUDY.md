# Cell8K Study

## Question

Can a NANDA wave associative memory cell fit into 8 KB?

Short answer:

```text
yes, but Cell8K must be a propagation-first cell, not a dense local memory cell
```

An 8 KB cell can satisfy the NANDA requirements if it gives up some local mode
density and relies more heavily on inter-cell propagation, carrier feedback,
and repeated stabilization across a larger network.

## Proposed Layout

```text
Header / metadata          128 B
Unicode projection        1024 B
Mode bank                 4096 B
Transition bank           1024 B
Interference state        1024 B
Calibration / stats        512 B
Scratch                    384 B
Total                     8192 B
```

The main payload is still the mode bank.

One compact mode remains 8 bytes:

```text
frequency_id: u16   2 B
sin_weight: i8      1 B
cos_weight: i8      1 B
amplitude: i8       1 B
phase: i8           1 B
damping: u8         1 B
role: u8            1 B
------------------------
total               8 B
```

Therefore:

```text
4096 B / 8 B = 512 modes
```

## Capacity

Compared with `SymbolCell32`:

```text
Cell32K = 2048 modes
Cell8K  =  512 modes
```

So one cell loses 75% of local modes.

But for the same memory budget:

```text
32 KB budget: 1 Cell32K or 4 Cell8K
```

The total number of modes is equal:

```text
1 * 2048 = 2048 modes
4 * 512  = 2048 modes
```

The difference is topology:

```text
Cell32K: modes concentrated inside one local medium
Cell8K:  modes distributed across four communicating media
```

For a wave associative memory, this may be better.

## Requirement Check

### Projection

Fits.

The projection bank becomes smaller:

```text
1024 B / 16 B = 64 projection lanes
```

This is not enough for rich symbol memory by itself, but that is acceptable.
Projection must not be the whole memory anyway.

Cell8K should use projection only as an entry point into modes.

### Modal Bank

Fits.

512 modes is enough for a small wave atom.

The cell should excite fewer active modes per symbol:

```text
Cell32K: 8-16 active modes
Cell8K:  4-8 active modes
```

### Transition Bank

Fits, but is tight.

```text
1024 B / 8 B = 128 transitions
```

This means Cell8K cannot hold rich long-range sequence memory alone. It should
hold short local transitions and rely on neighboring cells / bus memory for
larger context.

### Interference State

Fits.

```text
1024 B / 8 B = 128 interference slots
```

This is enough for local peak formation, but not enough for broad internal
spectral maps. The peak detector must work on compact regions.

### Decay

Fits.

Decay is cheap. It belongs in calibration and per-mode damping.

### Stable Peak Detector

Fits if the detector is simple.

Cell8K should not run a heavy internal search. It should detect:

```text
peak energy
second-best margin
local coherence
short persistence
transition support
```

Long persistence should be measured at the cluster or bus level.

### Propagation Output

Fits and becomes more important.

Cell8K should emit a compact wave message:

```text
phase
energy
coherence
role
peak_slot
```

The cell becomes useful because many small cells exchange these messages.

### Feedback Input

Fits.

Cell8K needs carrier feedback more than Cell32K. Without feedback it may be too
small and noisy.

### Reflection

Fits.

Reflection can be represented as a compact mismatch signal:

```text
reflected_phase
reflected_energy
mismatch_role
```

### Calibration / Trust

Fits.

512 B is enough for counters, thresholds, and a small amount of local history.

### Fixed Memory

Fits.

`Cell8K` should be exactly:

```text
8192 bytes
```

## What Cell8K Means Architecturally

Cell8K changes the center of gravity.

`Cell32K` says:

```text
one cell is a rich local resonator
```

`Cell8K` says:

```text
one cell is a small wave relay / resonator
the memory lives in the network dynamics
```

This is closer to the claim that:

```text
memory, search, and propagation are one process
```

Because the answer cannot fit inside one small cell. It must emerge across
multiple cells.

## Scale

For the same memory:

```text
4 MB:
128 Cell32K
512 Cell8K

128 MB:
4096 Cell32K
16384 Cell8K

2 GB:
65536 Cell32K
262144 Cell8K

32 GB:
1048576 Cell32K
4194304 Cell8K
```

The mode count is roughly the same if all memory goes to mode banks, but the
number of wave nodes is four times larger.

That gives:

```text
more propagation paths
more local peak regions
more reflection surfaces
more distributed redundancy
better ablation survival
```

The cost:

```text
weaker local resonance
more synchronization pressure
more bus traffic
more need for carrier feedback
more risk of noisy unstable peaks
```

## Best Future Use

Cell8K is likely best for:

```text
large distributed wave memory
sequence propagation
context routing
ablation-resistant recall
many weak signals becoming one stable peak
```

Cell32K is likely best for:

```text
dense local symbol memory
rich modal interference
small organisms
early proof of stable peaks
```

## Recommendation

Do not replace `SymbolCell32` immediately.

Define `SymbolCell8` as the propagation-first sibling:

```text
SymbolCell32 = dense local resonance cell
SymbolCell8  = distributed propagation cell
```

Then test equal-memory organisms:

```text
128 x Cell32K  = 4 MB
512 x Cell8K   = 4 MB
```

The winner is not the one with more raw modes. The winner is the one with better:

```text
stable peak rate
separation
coherence
persistence
transition sensitivity
ablation survival
propagation distance
carrier feedback gain
```

## Hypothesis

Cell8K may be the better long-term cell size if the NANDA theorem is true.

Reason:

```text
thinking is more likely to emerge from many interacting weak resonators
than from fewer isolated dense resonators
```

But Cell32K remains the better first proof target because it gives one cell more
room to form a clean stable peak before the network becomes complicated.
