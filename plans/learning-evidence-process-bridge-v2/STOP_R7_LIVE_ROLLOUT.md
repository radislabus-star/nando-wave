# STOP-R7 Live Rollout

Date: `2026-07-23`

## Result

```text
ordinary request
|-- OpportunityBridge                 producer / consumer parity, pending 0
|-- LearningStructureBridgeV2         23 / 23, pending 0, gaps 0
|   `-- RequestLearningIndex          checkpoint restored
`-- RawReplayBridge                   1 evaluated, 0 verified, 1 abstain

false accepts                         0
parity mismatches                     0
execution authority                   false
```

The shared structural epoch is
`9e61ff0a60ed92d517cea233e9c0a9e6fba28d93bc2195e02ecafbaef7b1e639`.
Cold restart restored sequence `22`; subsequent live traffic advanced both
producer and consumer to sequence `23` without gaps.

## Performance

```text
compact status bytes                  1988 / 1994
compact status latency                0.374 / 0.460 ms
hot structural publication            160-329 us observed
producer group durability              10 ms interval
pending consumer work                 asynchronous
```

The first implementation performed synchronous producer fsync and measured
`78.888 ms`. It was rejected before finalization. The live version publishes
atomically on the hot path, performs bounded group durability every 10 ms, and
uses the independently persisted cold checkpoint as the typed ACK boundary.

## Accounting

The dashboard no longer subtracts lifetime counters from unrelated process
starts. It reports:

```text
opportunity producer / consumer sequence
structural producer / consumer sequence under one epoch
pending structural records and sequence gaps
RequestLearning lookup attempts / hits / misses
raw replay evaluated / verified / abstained
```

At the final recorded snapshot, join accounting was `99 hits / 1255 attempts /
1156 misses`. This is not a bridge loss. It is downstream role-identity coverage and
remains visible as the next product bottleneck.

## Gates

- Focused bridge restart test: PASS.
- Gateway tests: 30/30 PASS.
- Affected-package Clippy with `-D warnings`: PASS.
- Structural gate: PASS.
- Wave causal gate: PASS.
- Composite gate: VETO, fail-closed.
- Local CPU eligibility: false.

The composite VETO is retained because deployment authority remains disabled;
this rollout changes evidence delivery and observation only.

The broad `nando-operator-learning` baseline remains dirty outside this route:
`201 PASS / 30 FAIL` in pre-existing B1B frozen-fixture checks. No failure
references the V2 bridge, request-learning checkpoint, gateway, or serving
route.
