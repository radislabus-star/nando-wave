# Phase-Center Online Miner Daemon V27

Date: 2026-07-06

Current architecture contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current status relative to the main line:

```text
V27 is useful proof/cold-path evidence.
It is not the product hot runtime.

Best current compression frontier is now:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

Product next step:
  automatic L4 opportunity board and marginal-denominator selector,
  not manual batch bucket picking.
```

This step turns the V26 batch bridge into the first bounded online miner daemon
rung. It scans an append-only phase-atom stream in order, updates phase-center
buckets incrementally, writes quarantine `.nwpc` checkpoints, and shadow-scores
only future events. It still does not enable local accept, auto-promotion,
serving promotion, or any market money claim.

## Command

```bash
cargo run -q -p nando-cli -- phase-stream-online-miner-daemon-v1 \
  target/nando-wave/streaming/phase-stream-online-miner-daemon-v1-realtrace-safe.report.json \
  target/nando-wave/streaming/online-miner-daemon-v1-realtrace-safe \
  target/nando-wave/streaming/phase-stream-online-miner-daemon-v1-realtrace-safe.decisions.jsonl \
  32 5 114000 500 32 200 \
  target/nando-wave/streaming/real-traffic-phase-atom-trace-v1.jsonl
```

Arguments after paths:

```text
cells: 32
min_bucket_events: 5
base_margin_threshold_micro: 114000
compile_every_rows: 500
max_active_buckets: 32
reservoir_per_label: 200
```

## Result

```text
verdict: PHASE_STREAM_ONLINE_MINER_DAEMON_V1_PASS_SHADOW_ONLY
total_rows: 17_000
parsed_events: 374
skipped_no_verifier_label: 16_626
bucket_count: 5
compile_ticks: 35
compiled_checkpoint_count: 76
active_profile_count: 5
future_shadow_events: 313
local_operator_shadow_decisions: 2
fallback_shadow_decisions: 311
unique_cpu_accepts_over_exact_cache: 2
nando_cpu_tokens_saved: 642
nando_cpu_cost_saved_microusd: 1_926
runtime_margin_parity_mismatches: 0
false_accepts: 0
wrong_wins: 97
latency_p99_ns: 15_494
local_accept_enabled: false
auto_promote_enabled: false
market_money_claim_allowed: false
```

The two accepted future events were both `action_family:metrics_report`, had
real token/cost evidence, exceeded the 114000 micro-margin threshold, and were
not exact-cache hits.

## Stream Contract

```text
append_only_input: true
score_before_train: true
future_only_shadow_scoring: true
incremental_bucket_updates: true
positive_negative_reservoirs: true
periodic_quarantine_nwpc_compile: true
compatible_denominator_delta_in_same_pass: true
```

Forbidden flags stayed closed:

```text
nwrb_used: false
role_binding_backend_used: false
lookup_used: false
target_id_or_proof_rule_id_authority_used: false
concrete_x_lookup_used: false
manual_local_out_t_used: false
local_accept_without_verifier_used: false
```

## Bug Found And Fixed

The first real-trace daemon smoke compiled checkpoints only at the final row.
Root cause: periodic compile was placed after row parsing, and skipped rows
used `continue`, bypassing the compile tick. Sparse real traces therefore
missed scheduled compiles when the interval row had no verifier label.

Fix:

```text
compile tick now runs before processing the current row;
compiled_after_row = total_rows - 1;
the current row can only be scored by checkpoints built from earlier rows.
```

This preserves `score_before_train` while making sparse verifier streams work.

## Negative Diagnostics

V26 candidate trace smoke:

```text
trace: target/nando-wave/streaming/auto-subcenter-discovery-v26.candidates.jsonl
total_rows: 109_300
parsed_events: 109_300
bucket_count: 48
compiled_checkpoint_count: 88
future_shadow_events: 1_586
unique_cpu_accepts_over_exact_cache: 0
false_accepts: 0
verdict: PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_NO_UNIQUE_ACCEPTS
```

Interpretation: the V26 candidate file is mostly grouped by candidate class.
Future-shadow rows after the first checkpoints were background negatives only,
so the daemon could not produce unique safe accepts. This is useful as a
diagnostic, but the real daemon proof must use an append-like real trace.

Low-threshold realtrace smoke:

```text
base_margin_threshold_micro: 1000
unique_cpu_accepts_over_exact_cache: 18
false_accepts: 8
verdict: PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_FALSE_ACCEPTS
```

Interpretation: the mechanism was live, but threshold was unsafe. The safe
114000 threshold keeps the rung green with `false_accepts=0`.

## Boundary

This closes only the first online daemon proof:

```text
real trace -> phase atoms -> online buckets -> periodic quarantine .nwpc
checkpoints -> future-only shadow decisions -> zero false accepts
```

It does not close:

```text
CPU10 or CPU20 compression;
online promotion;
local accept;
market claim;
large verifier-rich trace coverage;
external customer pilot traffic;
raw-language action parsing.
```

## Next Work

The bottleneck is not daemon mechanics now. It is verifier coverage:

```text
17_000 rows in the current trace
374 rows with verifier labels
313 future-shadow decisions
2 unique safe accepts
16_626 rows skipped because verifier label is missing
```

Next best work:

```text
increase verifier-rich rows for metrics_report / planning / serving_ops;
feed the daemon a real append stream with repeated positive and negative events;
keep local_accept disabled until verifier + false_accepts=0 + promotion gate;
measure calls/tokens/money only on compatible real denominator.
```
