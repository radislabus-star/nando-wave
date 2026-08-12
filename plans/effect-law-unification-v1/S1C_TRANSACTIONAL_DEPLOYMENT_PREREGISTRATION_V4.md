# S1C Transactional Deployment Preregistration V4

Status: `DESIGN FROZEN / NO DEPLOYMENT / CANDIDATE IDENTITY PENDING`

Date: `2026-08-12 Europe/Tallinn`

## 1. Exact Blocker

V3 repaired oracle ownership and reached an uncontaminated measurement window.
It then rejected the unchanged candidate because the old three-ledger benchmark
summed two production stages that are separated by response execution:

```text
pre-action request stage
  append precommit -> durable ACK -> response execution may begin

post-action settlement stage
  append selected action -> append verified satisfaction -> return
```

The V3 benchmark timed all three fsyncs as one synthetic contiguous latency:

```text
observed aggregate p99        5,767,585 ns
frozen aggregate p99 limit    5,000,000 ns
observed aggregate hard max   6,125,924 ns
```

No production request traverses that aggregate interval. V4 corrects the
measurement ownership; it does not weaken durability or change runtime code.

## 2. Frozen Candidate Boundary

The V4 candidate may change only the ignored resource test in
`grounded_decision_capture.rs` so it records the two actual synchronous stages
separately. Runtime types, append order, ledger format, fsync calls, failure
censors, configuration, and release code are immutable.

The final V4 freeze must prove:

```text
diff outside cfg(test)                         0
release binary SHA-256
  bff56756ee310344aa759b357e64dee2b8a8a75202d427dd3c3d54add78f8614
release binary bytes vs V3 candidate           IDENTICAL
Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1
candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

Candidate commit and tree are frozen only after that bounded test-only change
is committed. No remote attempt is authorized by this draft alone.

## 3. Stage-Correct Durability Gate

Each remote release run creates 256 joined records without warm-up and records:

```text
precommit_p99_ns
precommit_hard_max_ns
settlement_p99_ns
settlement_hard_max_ns
episode_p99_ns                  diagnostic only
episode_hard_max_ns             retained aggregate ceiling
records                         256
```

Frozen authority thresholds:

```text
pre-action precommit p99         <= 5,000,000 ns PASS 3/3
pre-action precommit hard max    <= 20,000,000 ns PASS 3/3
post-action settlement p99       <= 5,000,000 ns PASS 3/3
post-action settlement hard max  <= 20,000,000 ns PASS 3/3
aggregate episode hard max       <= 20,000,000 ns PASS 3/3
```

`episode_p99_ns` remains in the receipt so total write cost cannot disappear,
but it is not labelled request latency and does not own the 5 ms authority
bound. No batching, delayed fsync, async write, skipped ledger, retry, discarded
sample, warm-up exclusion, percentile change, or threshold change is allowed.

The existing single-ledger durability gate remains unchanged at p99 5 ms and
hard max 20 ms, PASS 3/3.

## 4. Inherited V3 Gates

V4 inherits the V3 ownership receipt, fresh build boundary, 30-second
quiescence gate, contamination monitor, direct-exec policy, parity oracle,
RSS/idle/hot limits, rollback chronology, connector identity, false-accept
gate, and service-survival checks byte-for-byte except for schema version and
the stage-correct durability fields.

No V2 or V3 checkout, target, oracle, harness, receipt, or metric may be reused.

## 5. Attempt Boundary

After final paper verification, V4 authorizes exactly one remote transaction.
Any build, ownership, quiescence, contamination, resource, parity, stale
baseline, deployment, or rollback result is terminal for V4.

Only a complete preflight may stop `nando-transition-serving.service`. Every
other service and the connector must preserve PID, restart count, and route
receipt authority.

## 6. Claim Boundary

```text
operational grounded-decision capture installed   only allowed new claim
natural decision episode                          not proved
grounded meaning                                   not proved
S1C-4                                              blocked until deployment PASS
K2                                                 blocked
model training                                     false
phase mutation                                     false
dashboard scientific claim                         forbidden
```

V4 changes the denominator of a resource benchmark to match production
chronology. It does not turn benchmark traffic into scientific evidence.
