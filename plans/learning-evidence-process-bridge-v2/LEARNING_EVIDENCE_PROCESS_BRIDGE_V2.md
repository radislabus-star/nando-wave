# Learning Evidence Process Bridge V2

## Goal

Make the hot-to-cold learning boundary measurable and restart-safe without
joining three different truths into one counter.

```text
ordinary request
|-- OpportunityBridge          durable denominator and outcomes
|-- LearningStructureBridgeV2 durable compact structural evidence
`-- RawReplayBridge            optional bounded GenerationShadow input
```

The routes share no domain schema and grant no execution authority. A generic
opaque spool implementation may be shared below them.

## Required Accounting

```text
opportunity published = opportunity applied + pending + rejected
structure submitted    = structure published + censored before publish
structure published    = structure applied + pending + rejected
lookup attempts        = lookup hits + lookup misses + identity mismatches
```

Lifetime counters from different process starts are never subtracted. Delivery
parity requires the same bridge epoch and comparable sequence watermarks.

## Budgets

```text
hot submit p99             <= 250 us
hard ceiling               <= 2 ms
compact status response    <= 4 KiB
compact status p99         <= 10 ms
structural record          <= 16 KiB
hot queue                  <= 48
pending spool              <= 64 MiB
request-learning checkpoint <= 16 MiB
raw payload persistence    = 0
false accepts              = 0
parity mismatches          = 0
```

## Rollout Order

1. Compact read-only status and truthful dashboard.
2. Versioned structural epoch and sequence identity.
3. Durable compact structural spool.
4. Bounded RequestLearningIndex checkpoint.
5. Join accounting through RelationFrame.
6. Cold-first dual-read rollout, then hot V2 writer.
7. V1 retirement only after restart parity.

Operator discovery thresholds, Wave, admission and CPU authority are outside
this change.

## Final Status

R0-R7 are implemented and live in SHADOW. Delivery and join accounting are
working; operator discovery and authority remain separate downstream work.
See `STOP_R7_LIVE_ROLLOUT.md`.
