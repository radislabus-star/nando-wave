# L3 Cell8K Budget

## Boundary

The active NANDA wave organism should fit inside L3 with working headroom.

```text
RAM is cold/archive memory.
L3 is the active thinking medium.
```

The fast default is intentionally smaller than the physical L3 ceiling:

```text
physical L3 budget = 8 MiB
turbo active budget = 2 MiB
fast active budget = 4 MiB
max stress budget  = 8 MiB
cell size          = 8 KiB
```

## Top-Level Count

| Параметр                 |   Turbo | Fast default | Max stress |
|--------------------------|--------:|-------------:|-----------:|
| Active budget            |   2 MiB |        4 MiB |      8 MiB |
| Cell size                |   8 KiB |        8 KiB |      8 KiB |
| Cells in hot organism    |     256 |          512 |       1024 |
| Clusters                 |      16 |           32 |         64 |
| Total mode count         | 131 072 |      262 144 |    524 288 |
| Total transition entries |  32 768 |       65 536 |    131 072 |
| Total interference slots |  32 768 |       65 536 |    131 072 |
| Total projection lanes   |  16 384 |       32 768 |     65 536 |

## One Cell Layout

| Слой              | Размер | Что держит               |
|-------------------|-------:|--------------------------|
| Header            |  128 B | id/version/role/checksum |
| Projection        | 1024 B | 64 lanes                 |
| Mode bank         | 4096 B | 512 modes                |
| Transition bank   | 1024 B | 128 links                |
| Interference      | 1024 B | 128 wave slots           |
| Calibration/stats |  512 B | thresholds/trust         |
| Scratch           |  384 B | tick-local temp          |

## Organ Capacity

| На клетку              | Turbo 256 cells | Fast 512 cells | Stress 1024 cells |
|------------------------|----------------:|---------------:|------------------:|
| 512 modes              |   131 072 modes |  262 144 modes |     524 288 modes |
| 128 transitions        |    32 768 links |   65 536 links |     131 072 links |
| 128 interference slots |    32 768 slots |   65 536 slots |     131 072 slots |
| 64 projection lanes    |    16 384 lanes |   32 768 lanes |      65 536 lanes |
| 4 active modes / tick  |        1024 ops |       2048 ops |          4096 ops |
| 8 active modes / tick  |        2048 ops |       4096 ops |          8192 ops |

## Propagation Budget

The hot organism must not flood L3 with bus traffic. Each cell should emit a
small number of compact wave messages per tick.

Suggested message:

```text
peak_slot: u16
phase: i8
energy: u16
coherence: u16
role: u8
```

This fits in 8 bytes. A 16 byte message can add source cell, target role, or
debug fields, but 8 bytes should be the hot target.

| Fanout | Messages / tick | If 8 B each | If 16 B each |
|-------:|----------------:|------------:|-------------:|
|      2 |            1024 |       8 KiB |       16 KiB |
|      4 |            2048 |      16 KiB |       32 KiB |
|      8 |            4096 |      32 KiB |       64 KiB |
|     16 |            8192 |      64 KiB |      128 KiB |

Recommended first target:

```text
fanout = 4
message = 8 B
bus traffic = 16 KiB / tick
```

Aggressive but still plausible:

```text
fanout = 8
message = 16 B
bus traffic = 64 KiB / tick
```

Avoid at the start:

```text
fanout >= 16
```

Reason: the organism may remain inside L3 as data, but the per-tick message
traffic can start behaving like a second working set.

## Reflection Budget

Reflection should be bounded. Every rejected wave must not create a full new
wave cascade.

Recommended rule:

```text
one incoming message may create at most one reflected message
only if mismatch_energy >= reflection_threshold
```

With fanout 4:

```text
forward messages <= 2048 / tick
reflected messages <= 2048 / tick
total messages <= 4096 / tick
8 B total traffic <= 32 KiB / tick
16 B total traffic <= 64 KiB / tick
```

This keeps reflection useful without turning it into uncontrolled branching.

## Peak Budget

Each cell has 128 interference slots.

Recommended detector:

```text
top_k candidates = 4
stable candidates exported = 1
optional reflected candidate = 1
```

So one cell should not export all local peaks. It should compress local
interference into one forward message and, only when needed, one reflected
message.

## Hot Tick Window

The full Cell8 field keeps 128 interference slots, but the tick path should not
decay and scan all 128 slots each time. The fast profile keeps a bounded active
window per cell:

```text
active_slots = 8
```

Per cell, decay and top-2 peak selection operate on these 8 active slots. Newly
excited slots enter the active set; when the set is full, the weakest active
slot is evicted and cleared.

For the fast 512-cell organism this changes the hot scan budget from:

```text
512 cells x 128 slots = 65 536 slots / tick
```

to:

```text
512 cells x 8 slots = 4096 slots / tick
```

The full 128-slot field remains the capacity boundary; the 8-slot window is the
current live wave frontier.

## What Fits

Turbo profile inside 2 MiB:

```text
256 active cells
131 072 compact modes
32 768 short transitions
32 768 interference slots
16 384 projection lanes
1024-2048 active mode ops per tick
8-32 KiB normal bus traffic per tick
16-64 KiB with bounded reflection
```

Fast default inside 4 MiB:

```text
512 active cells
262 144 compact modes
65 536 short transitions
65 536 interference slots
32 768 projection lanes
2048-4096 active mode ops per tick
16-64 KiB normal bus traffic per tick
32-128 KiB with bounded reflection
```

Max stress inside 8 MiB:

```text
1024 active cells
524 288 compact modes
131 072 short transitions
131 072 interference slots
65 536 projection lanes
4096-8192 active mode ops per tick
32-128 KiB normal bus traffic per tick
64-256 KiB with bounded reflection
```

This is enough for a real first L3-bound wave organism.

## What Does Not Fit

Do not place these in the hot L3 organism:

```text
large text corpus
large answer table
unbounded graph edges
full debug traces
large per-cell histories
long-range cold memory
```

Those belong in RAM/disk as cold memory. The hot organism may only warm selected
cold pages into cells before a run.

## Architecture Consequence

Cell8K makes sense only if the network does real work.

The L3 organism should not be:

```text
512 independent recognizers
```

It should be:

```text
512 weak resonators by default
connected by bounded propagation
stabilized by carrier feedback
and protected by decay/reflection/veto
```

## First Test Target

Before building larger runtime layers, test this exact budget:

```text
256 x SymbolCell8 for turbo speed proof
512 x SymbolCell8 for default quality proof
16 or 32 x SymbolWaveCluster
fanout 4
8 B wave messages
4 active modes per cell per tick
1 forward peak message per cell
optional 1 reflected message per cell
```

Keep `1024 x SymbolCell8` only as the max/stress profile.

Expected first proof:

```text
stable peaks survive decay
similar inputs converge nearby
different contexts shift peaks
small perturbations do not destroy peaks
ablation weakens but does not erase recognition
bounded reflection improves rejection of false peaks
```
