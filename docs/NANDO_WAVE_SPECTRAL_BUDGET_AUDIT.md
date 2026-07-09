# Nando Wave Spectral Budget Audit

Date: 2026-07-08

Scope: code/docs/data size audit for the phase-center miner/runtime path.

Rule: split by signal route, not by cosmetic filenames. Do not mix move-only
refactors with scoring, threshold, miner, verifier, or compression changes.

Latest full budget scan:

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24226 lines
crates/nando-cli/src/phase_package_cmd.rs                           16408 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      10802 lines
crates/nando-core/src/wave/phase_center_runtime.rs                   7656 lines
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      7190 lines
crates/nando-cli/src/phase_daemon_cmd.rs                             5320 lines
```

Latest all-file budget control, archived history excluded from P0 queue:

```text
docs/archive/EXECUTOR_REVIEW_NOTES_2026-07-08_precompact.md         67360 lines (archive, not active P0)
crates/nando-cli/src/phase_streaming_cmd.rs                         24226 lines
crates/nando-cli/src/phase_package_cmd.rs                           16408 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      10802 lines
crates/nando-core/src/wave/phase_center_runtime.rs                   7656 lines
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      7190 lines
crates/nando-cli/src/phase_daemon_cmd.rs                             5320 lines
docs/WAVE_LLM_LAYERS_LIVE_PLAN.md                                   4561 lines
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904 lines
crates/nando-eval/src/byte_context.rs                                3755 lines
```

Latest top Rust file scan:

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24226 lines
crates/nando-cli/src/phase_package_cmd.rs                           16408 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      10802 lines
crates/nando-core/src/wave/phase_center_runtime.rs                   7656 lines
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      7190 lines
crates/nando-cli/src/phase_daemon_cmd.rs                             5320 lines
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs  3904 lines
crates/nando-eval/src/byte_context.rs                                3755 lines
crates/nando-cli/src/organ128_cmd.rs                                 2922 lines
crates/nando-eval/src/phase.rs                                       2549 lines
crates/nando-eval/src/modadd.rs                                      2368 lines
crates/nando-cli/src/main.rs                                         2332 lines
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_np_rescue.rs  2097 lines
crates/nando-eval/src/symbol_retrieval.rs                            1696 lines
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_selector.rs  1671 lines
crates/nando-cli/src/live.rs                                         1604 lines
crates/nando-core/src/wave/l3_self_induced_grokking.rs               1472 lines
crates/nando-core/tests/wavepredictor_trainer_10k.rs                 1448 lines
crates/nando-cli/src/phase_streaming_cmd/agent_continue.rs           1437 lines
crates/nando-core/src/wave/symbol_cell.rs                            1434 lines
```

Cut order stays P0-first:

```text
1. live_store_adapter.rs      active source/miner/report split already started
2. phase_streaming_cmd.rs     root router split
3. phase_package_cmd.rs       deploy/package split
4. phase_center_runtime.rs    source-neutral core split, only after CLI cuts
5. online_miner_daemon.rs     cold miner split
6. phase_daemon_cmd.rs        serving/control split
```

## P0 Cut Targets

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24226 lines
crates/nando-cli/src/phase_package_cmd.rs                           16408 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      10802 lines
crates/nando-core/src/wave/phase_center_runtime.rs                   7656 lines
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      7190 lines
crates/nando-cli/src/phase_daemon_cmd.rs                             5320 lines
```

### live_store_adapter.rs

Verdict: mixed-frequency monolith.

User note: this file is explicitly marked as too fat and must be refactored
with the spectral-budget skill. Treat it as a first-class P0 split target, not
as incidental cleanup.

First move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/defaults.rs  61 lines
```

Scope: report/default paths and numeric defaults only. No scoring, threshold,
miner, verifier, promotion, local_accept, or compression-claim behavior changed.

Second move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/architecture.rs  47 lines
```

Scope: architecture-version report struct and key builder only. No scoring,
threshold, miner, verifier, promotion, local_accept, or compression-claim
behavior changed.

Third move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  1582 lines
```

Scope: budget/direct-hot/route/bucket/candidate/manifest/future-shadow,
prepared-hot, clean-manifest shadow, and worker report structs only. No
scoring, threshold, miner, verifier, promotion, local_accept, or
compression-claim behavior changed.

Fourth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/source_events.rs  447 lines
```

Scope: parsed atom event structs, event-to-hot-request conversion helpers, tail
cost estimate helper, safe-atom extraction, forbidden leak atom filter, adaptive
bucket policy, route key, bucket selector, refinement blocker, and hash-id
helpers. No atom list, scoring, threshold, miner, verifier, promotion,
local_accept, or compression-claim behavior changed.

Fifth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/source_events.rs  447 lines
```

Scope: full row-to-parsed-atom-event adapter moved out of the monolith. The
parser still uses the same safe atoms, route key, bucket refinement, exact-cache
key handling, token/cost extraction, and auto-subcenter bucket IDs. No atom
list, scoring, threshold, miner, verifier, promotion, local_accept, or
compression-claim behavior changed.

Sixth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hidden_state.rs  347 lines
```

Scope: hidden-state and auto-subcenter atom construction moved out of the
monolith. The same blockers, bucket keys, pair/combo atoms, hidden-state
fingerprints, and bucket IDs are used. No atom semantics, scoring, threshold,
miner, verifier, promotion, local_accept, or compression-claim behavior changed.

Seventh move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/profile_attribution.rs  175 lines
```

Scope: L4 profile-attribution helpers moved out of the monolith. The same
observable/hidden/unknown profile classification and call/token/cost attribution
counters are used. No scoring, threshold, miner, verifier, promotion,
local_accept, or compression-claim behavior changed.

Eighth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/bucket_decisions.rs  50 lines
```

Scope: bucket decision selector helpers moved out of the monolith. The same
exact-bucket, union-score-candidate, relevant-bucket-id, and relevant-decision
filters are used. No scoring, threshold, miner, verifier, promotion,
local_accept, or compression-claim behavior changed.

Ninth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/diagnostics.rs  162 lines
```

Scope: route/bucket diagnostics accumulators and report builders moved out of
the monolith. The same route/bucket counters, sorting, safety flags, selected
bucket atom cap, and diagnostic report rows are used. No scoring, threshold,
miner, verifier, promotion, local_accept, or compression-claim behavior changed.

Tenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/candidate_packages.rs  72 lines
```

Scope: verifier binding and verifier-bound `.nwpc` candidate package writers
moved out of the monolith. The same verifier IDs, package filenames, report
rows, route lookup, and `promotion_allowed: false` boundary are used. No
scoring, threshold, miner, verifier semantics, promotion, local_accept, or
compression-claim behavior changed.

Eleventh move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hot_path_gates.rs  78 lines
```

Scope: prepared-pack and worker hot-path blocker helpers moved out of the
monolith. The same false-accept, verifier-required, local-accept, parity, and
latency blocker strings are used. No scoring, threshold, miner, verifier
semantics, promotion, local_accept, or compression-claim behavior changed.

Twelfth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/frozen_candidates.rs  138 lines
```

Scope: verifier-bound `.nwpc` frozen candidate structs and freeze helpers moved
out of the monolith. The same package bytes, threshold_micro, hot runtime,
route table, scratch, and future-shadow counters/events are used. No scoring,
threshold, miner, verifier semantics, promotion, local_accept, or
compression-claim behavior changed.

Thirteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  838 lines
```

Scope: the top-level live-store adapter smoke report schema moved into the
report module with the other live-store report schemas. The same fields,
forbidden flags, runtime budget fields, future-shadow fields, and product
claim blockers are used. No scoring, threshold, miner, verifier semantics,
promotion, local_accept, or compression-claim behavior changed.

Fourteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/worker_path.rs  34 lines
```

Scope: prepared-memory row alias, prepared-hot eval alias, and worker
thread/batch message and metrics structs moved out of the adapter. The same
hot worker envelope, queue timestamps, eval counters, and latency vectors are
used. No scoring, threshold, miner, verifier semantics, promotion,
local_accept, or compression-claim behavior changed.

Fifteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  1098 lines
```

Scope: hot-path benchmark, promotion review, daemon admission policy, daemon
admission policy smoke, and smoke guard report schemas moved into the report
module. The same fields, forbidden flags, admission blockers, runtime parity
fields, future-shadow fields, and product/local-accept boundaries are used. No
scoring, threshold, miner, verifier semantics, promotion, local_accept, or
compression-claim behavior changed.

Sixteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  1226 lines
```

Scope: numeric admission portfolio gate, accepted/rejected rows, runtime replay
report, and runtime replay item report schemas moved into the report module.
The same report fields, `.nwpc` package paths, false-accept counters,
runtime-parity counters, token/cost fields, and product/local-accept boundaries
are used. No scoring, threshold, miner, verifier semantics, promotion,
local_accept, or compression-claim behavior changed.

Seventeenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  1402 lines
```

Scope: numeric future portfolio audit, daemon shadow gate, and append shadow
gate report schemas moved into the report module. The same watermark/append
paths, future-shadow split fields, route/profile IDs, token/cost denominators,
runtime-parity counters, forbidden flags, and product/local-accept boundaries
are used. No scoring, threshold, miner, verifier semantics, promotion,
local_accept, or compression-claim behavior changed.

Eighteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs  1582 lines
```

Scope: live-loop budget smoke and append live-loop smoke report schemas moved
into the report module. The same architecture-version payload, append
watermark fields, profile-attribution counters, token/cost accounting,
promotion evidence fields, admission counters, forbidden flags, and
product/local-accept boundaries are used. No scoring, threshold, miner,
verifier semantics, promotion, local_accept, or compression-claim behavior
changed.

Nineteenth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/survivor_runtime.rs  204 lines
```

Scope: clean candidate frontier, candidate value reports, clean survivor hot
runtime construction, hidden-state/observable-subcenter priority selection, and
quarantined/observable-primary exclusion helpers moved out of the monolith. The
same candidate ordering, quarantine filters, subcenter preference, route table,
runtime byte estimate, and `LiveStoreProductHotRegistryRuntimeBundle` fields are
used. No scoring, threshold, miner, verifier semantics, promotion,
local_accept, or compression-claim behavior changed.

Twentieth move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/quarantine.rs  182 lines
```

Scope: stable decision-log architecture filters, non-exact false profile-id
extraction, stable decision-log window aggregation, and score-candidate /
local-accept counters that exclude quarantined profiles moved out of the
monolith. The same decision-log JSON fields, architecture key compatibility,
profile-id extraction, false-accept accounting, token/cost aggregation, and
`LiveStoreStableDecisionLogWindow` fields are used. No scoring, threshold,
miner, verifier semantics, promotion, local_accept, or compression-claim
behavior changed.

Stable clean-suffix proof-accounting addition:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/quarantine.rs  250 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs     2148 lines
```

Scope: append-live-tail reports now include a stable decision-log clean suffix
window after the latest non-exact false accept. This exposes whether current
post-quarantine traffic is clean without erasing the all-time stable window.
The same claim gate is reused: min rows, false_accepts = 0, no local_accept,
tokens/cost denominator, and final hot runtime required. No scoring, threshold,
miner, verifier semantics, promotion, local_accept, or market-money claim
behavior changed.

Smoke evidence:

```text
target/nando-wave/streaming/stable-clean-suffix-smoke.report.json
stable_decision_log_false_accepts: 5
stable_decision_log_clean_suffix_rows: 44
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 23
stable_decision_log_clean_suffix_tokens_saved: 26342
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_claim_blocker: append_no_final_hot_runtime

target/nando-wave/streaming/stable-clean-suffix-promoted.json
product_hot_score_only_runtime_source: call_token_promotion_manifest
product_hot_budget_passed: true
final_hot_runtime_available: true
stable_decision_log_clean_suffix_rows: 44
stable_decision_log_clean_suffix_min_rows: 100
stable_decision_log_clean_suffix_rows_to_min: 56
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_claim_blocker: append_window_below_min_rows

target/nando-wave/streaming/stable-clean-suffix-gap-smoke.json
stable_decision_log_clean_suffix_rows: 44
stable_decision_log_clean_suffix_min_rows: 100
stable_decision_log_clean_suffix_rows_to_min: 56
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 23
stable_decision_log_clean_suffix_tokens_saved: 26342
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_claim_blocker: append_window_below_min_rows
```

Twenty-first move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/policy_json.rs      45 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/runtime_metrics.rs 124 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/claim_gates.rs     126 lines
```

Scope: JSON policy parsing helpers, runtime metric/budget/latency helpers, and
claim/blocker-name gates moved out of the monolith. The same forbidden flag
defaults, hot route/profile IDs, latency percentile, provider/estimated cost
checks, budget report mapping, promotion/admission blocker names, and append
compression claim blocker are used. No scoring, threshold, miner, verifier
semantics, promotion, local_accept, or compression-claim behavior changed.

Twenty-second move-only cut completed:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/source_readers.rs 649 lines
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hot_path_eval.rs  300 lines
```

Scope: JSONL/source readers, worker queue send/flush helpers, direct-store
event collection, append-shadow event collection, live-loop budget observation,
direct-hot snapshot selection/eval, hot-path prepared row/parity/denominator
helpers, and candidate-decision eval counters moved out of the monolith. The
same source rows, safe atom conversion, exact-cache tracking, auto-subcenter
observation, queue timings, hot runtime scoring, parity checks, token/cost
denominators, and false-accept accounting are used. No scoring, threshold,
miner, verifier semantics, promotion, local_accept, or compression-claim
behavior changed.

Routes currently tangled:

```text
source adapter parsing
live online miner tail
hidden-state / auto-subcenter observation
product-hot survivor selection
quarantine and promotion accounting
future-shadow billing/provider evidence
server snapshot/report schemas
runtime budget reporting
```

Required split:

```text
live_store_adapter/
  mod.rs                       thin command/router surface
  source_events.rs             source-specific event parsing
  live_tail.rs                 append/live-tail loop only
  miner_observation.rs         online observation and throttling
  survivor_runtime.rs          product-hot survivor selection
  quarantine.rs                false-accept/quarantine accounting
  billing_evidence.rs          provider/billing joins
  reports.rs                   report structs and schema writers
```

No scoring change allowed during this split.

### phase_streaming_cmd.rs

Verdict: root command file is too large for a router.

Required split:

```text
phase_streaming_cmd/
  mod.rs                       public command dispatch and shared exports only
  shared.rs                    stable primitives used by subcommands
  trace_io.rs                  JSONL / phase atom trace IO
  report_io.rs                 report writing helpers
```

Target: root module should become a thin router, not a second runtime.

### phase_package_cmd.rs

Verdict: package/deploy path is too broad.

Required split:

```text
phase_package_cmd/
  mod.rs
  package_manifest.rs
  deploy_bundle.rs
  systemd_assets.rs
  verification.rs
```

Target: package build stays separate from miner/runtime authority.

### phase_center_runtime.rs

Verdict: source-neutral core is valuable but too wide.

Allowed split only after CLI route cuts are stable:

```text
phase_center_runtime/
  cells.rs
  compiler.rs
  package.rs
  hot_runtime.rs
  online_miner.rs
  live_store.rs
  budget.rs
```

Rule: keep core source-neutral. No Codex/server/provider assumptions here.

### online_miner_daemon.rs

Verdict: proof/miner daemon and product hot path must stay separated.

Required split:

```text
online_miner_daemon/
  control.rs
  discovery.rs
  calibration.rs
  shadow.rs
  promotion_registry.rs
  reports.rs
```

Rule: cold path can be smart and heavier; hot path must remain tiny.

### phase_daemon_cmd.rs

Verdict: serving/control surface is too wide for one file.

Required split:

```text
phase_daemon_cmd/
  mod.rs                       public command surface
  service_config.rs            env/config parsing
  health.rs                    health/readiness handlers
  metrics.rs                   metrics snapshots
  workers.rs                   worker lifecycle/control
  reports.rs                   daemon report schema
```

Rule: no miner/proof accounting in serving hot handlers.

## P1 Watch Targets

```text
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904
crates/nando-cli/src/organ128_cmd.rs                                        2922
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_np_rescue.rs      2097
crates/nando-cli/src/main.rs                                                2332
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_selector.rs        1671
crates/nando-cli/src/live.rs                                                1604
crates/nando-cli/src/phase_streaming_cmd/agent_continue.rs                  1437
crates/nando-cli/src/phase_streaming_cmd/online_miner_promotion_billing_request.rs 1392
crates/nando-cli/src/phase_streaming_cmd/auto_subcenter.rs                  1364
crates/nando-cli/src/phase_streaming_cmd/provider_boundary_billing_capture_contract.rs 1139
crates/nando-cli/src/phase_streaming_cmd/live_store_clean_manifest_admission_gate.rs 1094
crates/nando-cli/src/help.rs                                                1092
crates/nando-cli/src/phase_streaming_cmd/automatic_continuation_split.rs    1081
crates/nando-cli/src/phase_streaming_cmd/selected_split_nwpc.rs             1033
```

Core watch list:

```text
crates/nando-core/src/wave/l3_self_induced_grokking.rs                      1472
crates/nando-core/src/wave/symbol_cell.rs                                   1434
crates/nando-core/src/wave/learn.rs                                         1280
```

Docs over budget:

```text
docs/WAVE_LLM_LAYERS_LIVE_PLAN.md                                          4561
docs/NANDO_WAVE_DEVELOPMENT_ROADMAP.md                                     3483
docs/OPERATOR_PRODUCT_LINES_AND_CAPACITY.md                                 2226
docs/CPU_CALL_CATALOG.md                                                    1683
docs/EXECUTOR_REVIEW_NOTES.md                                               1952
docs/OPERATOR_BLUEPRINT.md                                                  1573
docs/NANDO_WAVE_PRODUCT_RUNTIME_TASK.md                                     1503
docs/DETAILED_ROADMAP.md                                                    1459
docs/GOAL.md                                                                1366
docs/NANDO_WAVE_PROJECT_PROGRESS_TREE.md                                    1128
docs/PRODUCT_TRAJECTORY.md                                                  1070
```

Docs over 3000 lines should be archived/summarized into current-state docs.
`docs/EXECUTOR_REVIEW_NOTES.md` should stay short; if it grows again, archive
detail blocks and keep only active state, blocker, and next task.
Data JSONL/corpus files are not refactor targets; they are dataset artifacts.

## Execution Order

```text
1. Freeze current checks and report schemas.
2. Cut live_store_adapter.rs reports/types first.
3. Cut live_store_adapter.rs source-event parsing.
4. Cut survivor runtime selection and quarantine accounting.
5. Cut live-tail/miner observation loop last.
6. Cut phase_streaming_cmd.rs root into thin router/shared IO.
7. Cut phase_package_cmd.rs deploy/package surface.
8. Cut phase_daemon_cmd.rs serving/control surface.
9. Run fmt/check after each cut.
10. Only after CLI stabilizes, split phase_center_runtime.rs.
```

## Hard Gates

```text
public command names unchanged
report schemas unchanged or explicitly versioned
.nwpc phase-center path only
no .nwrb revival
no local_accept enablement
no provider billing logic in hot runtime
no source-agent hardcode in generic core
no scoring/threshold changes during move-only refactor
```

## 2026-07-08 - Budget Scan After Append Live Tail Cut

Command:

```text
rg --files -g '*.rs' -g '*.md' | xargs wc -l | sort -nr | head -n 45
```

Top active Rust/docs debt:

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24398
crates/nando-cli/src/phase_package_cmd.rs                           16408
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      14812
crates/nando-core/src/wave/phase_center_runtime.rs                   7656
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      6996
crates/nando-cli/src/phase_daemon_cmd.rs                             5320
docs/WAVE_LLM_LAYERS_LIVE_PLAN.md                                   4561
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904
crates/nando-eval/src/byte_context.rs                                3755
docs/NANDO_WAVE_DEVELOPMENT_ROADMAP.md                              3483
```

Current phase-streaming submodule debt:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs       14812
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs       6996
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_np_rescue.rs 2097
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/reports.rs 2148
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/runtime_registry.rs 479
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/source_events.rs 447
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hidden_state.rs 347
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/state.rs 206
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/survivor_runtime.rs 204
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/quarantine.rs 250
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/profile_attribution.rs 175
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/diagnostics.rs 162
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/paths.rs 146
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/frozen_candidates.rs 138
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/hot_path_gates.rs 78
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/candidate_packages.rs 72
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/defaults.rs 61
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/bucket_decisions.rs 48
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/architecture.rs 47
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/persistence.rs 45
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/worker_path.rs 34
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_selector.rs 1671
crates/nando-cli/src/phase_streaming_cmd/agent_continue.rs           1437
crates/nando-cli/src/phase_streaming_cmd/online_miner_promotion_billing_request.rs 1392
crates/nando-cli/src/phase_streaming_cmd/auto_subcenter.rs           1364
```

Next budget rule:

```text
Continue live_store_adapter.rs report/type cuts first.
Do not touch scoring, thresholds, miner behavior, verifier, promotion,
local_accept, or compression claims while this file is being split.
```

## 2026-07-08 - Budget Scan After source_readers/hot_path_eval/defaults Cuts

Current active P0 scan:

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24226
crates/nando-cli/src/phase_package_cmd.rs                           16408
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      13639
crates/nando-core/src/wave/phase_center_runtime.rs                   7656
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      6996
crates/nando-cli/src/phase_daemon_cmd.rs                             5320
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904
crates/nando-eval/src/byte_context.rs                                3755
```

Latest move-only cuts:

```text
live_store_adapter/source_readers.rs added                           649
live_store_adapter/hot_path_eval.rs added                            300
phase_streaming_cmd/defaults.rs added                                174
```

Scope:

```text
source readers / queues / append-shadow collection moved out
hot-path eval helpers moved out
phase_streaming_cmd report/default path constants moved out
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
phase_streaming_cmd.rs reduced 24398 -> 24226 lines
live_store_adapter.rs reduced 14812 -> 13639 lines since the previous scan block
```

## 2026-07-08 - Full Budget Control After User live_store_adapter Flag

User directive:

```text
live_store_adapter.rs is still too fat.
Keep it in the spectral-budget queue and continue cutting it by signal route.
Then rescan all active files with the same budget discipline.
```

All-file scan command:

```text
find . -path './target' -prune -o -path './crates/nando-cli/target' -prune -o -path './.git' -prune -o -type f -print0 | xargs -0 wc -l | sort -nr | head -120
```

Active Rust/docs scan command:

```text
rg --files docs crates ops scripts | xargs wc -l | sort -nr | head -120
```

Largest whole-repo files:

```text
data/corpus/russian_words_danakt_cp1251.txt                       1532628
data/corpus/russian_words_danakt_full.txt                         1528731
data/lexicon_foundation_v1/ru_cold_300k.txt                        300000
data/corpus/russian_words_300k.txt                                 300000
data/corpus/russian_words_full.txt                                 185269
data/lexicon_foundation_v1/ru_hot_100k.txt                         100000
data/corpus/russian_words_100k.txt                                 100000
data/corpus/russian_words_frequency_raw.txt                         99996
data/corpus/english_words_system_full.txt                           75119
data/lexicon_foundation_v1/en_hot.txt                               74960
docs/archive/EXECUTOR_REVIEW_NOTES_2026-07-08_precompact.md         67360
```

Budget verdict:

```text
lexicon/corpus files are data artifacts, not refactor targets
docs/archive is historical storage, not active P0 context
active P0 remains Rust command/runtime/miner files
```

Current active Rust/docs debt:

```text
crates/nando-cli/src/phase_streaming_cmd.rs                         24226
crates/nando-cli/src/phase_package_cmd.rs                           16408
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs      10802
crates/nando-core/src/wave/phase_center_runtime.rs                   7656
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs      7190
crates/nando-cli/src/phase_daemon_cmd.rs                             5320
docs/WAVE_LLM_LAYERS_LIVE_PLAN.md                                   4561
crates/nando-cli/src/phase_streaming_cmd/online_portfolio_runtime_replay.rs 3904
crates/nando-eval/src/byte_context.rs                                3755
docs/NANDO_WAVE_DEVELOPMENT_ROADMAP.md                              3483
```

Current active P0 order:

```text
1. live_store_adapter.rs
   reason: user-flagged, still 10802 lines, source/miner/report/runtime mixed
2. phase_streaming_cmd.rs
   reason: 24226-line root command router, should become thin module entry
3. phase_package_cmd.rs
   reason: 16408-line package/deploy surface
4. online_miner_daemon.rs
   reason: 7190-line cold miner orchestration
5. phase_daemon_cmd.rs
   reason: 5320-line serving/control surface
6. phase_center_runtime.rs
   reason: 7656-line core, split only after CLI boundaries are cleaner
```

Move-only rules for the next cuts:

```text
do not change scoring
do not change threshold calibration
do not change miner decisions
do not change verifier/admission/promotion policy
do not enable local_accept
do not add provider billing logic to hot path
do not revive .nwrb or role-binding commercial path
```

Latest cut:

```text
live_store_adapter/promotion_manifests.rs added                       413
live_store_adapter.rs reduced 13639 -> 13239
```

Scope:

```text
promotion manifest handoff moved out:
  clean promotion manifest writer
  call/token promotion manifest summary
  call/token promotion manifest writer
  call/token manifest blockers
  candidate runtime parity helpers
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/numeric_future_package.rs added                   371
live_store_adapter.rs reduced 12189 -> 11840
```

Scope:

```text
fresh-future .nwpc package audit helper moved out:
  LiveStoreFrozenNumericFuturePackage
  live_store_write_numeric_future_package_audit_from_frozen
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/hot_path_eval.rs extended                         391
live_store_adapter.rs reduced 12771 -> 12681
```

Scope:

```text
direct-hot eval report moved out:
  direct mutable-store hot runtime report helper
  direct-hot blocker helper
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/provider_evidence.rs added                         486
live_store_adapter.rs reduced 13239 -> 12771
```

Scope:

```text
provider evidence handoff moved out:
  future-shadow billing request JSONL writer
  provider export signature helpers
  provider money claim blocker
  cold provider evidence artifact refresh
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/future_shadow_registry.rs added                    459
live_store_adapter.rs reduced 12681 -> 12239
```

Scope:

```text
future-shadow registry/eval helpers moved out:
  future-shadow candidate reports
  candidate promotion evidence
  candidate registry shadow
  shared registry shadow
  serving/clean promotion blockers
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/claim_gates.rs extended                            177
live_store_adapter.rs reduced 12239 -> 12189
```

Scope:

```text
clean manifest shadow gate moved out:
  live_store_clean_manifest_shadow_blocker
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/portfolio_replay.rs added                         307
live_store_adapter.rs reduced 11840 -> 11556
```

Scope:

```text
portfolio runtime replay helper moved out:
  live_store_replay_one_portfolio_admission
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/future_shadow_registry.rs extended                737
live_store_adapter.rs reduced 11556 -> 11285
```

Scope:

```text
future-shadow refresh/observe helpers moved out:
  live_store_refresh_future_shadow_summary
  observe_live_store_future_shadow
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

Latest cut:

```text
live_store_adapter/numeric_false_accept_split_audit.rs added         491
live_store_adapter.rs reduced 11285 -> 10802
```

Scope:

```text
false-accept split audit route moved out:
  run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1
```

Control:

```text
move-only refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

## P1 Proof-Denominator Update

Latest update:

```text
crates/nando-cli/src/phase_streaming_cmd/online_miner_daemon.rs
```

Scope:

```text
value-pass report now carries real denominator fields:
  exact_cache_hits
  non_exact_rows
  total_tokens_seen
  total_cost_microusd_seen
  estimated_total_cost_microusd_seen
  token_denominator_present
  cost_denominator_present
  estimated_cost_denominator_present
  token_cost_denominator_present
  market_money_claim_blocker
  estimated_money_claim_allowed
  estimated_money_claim_blocker
  product-hot upper-bound calls/tokens/cost saved milli over denominators
  product-hot upper-bound estimated cost saved milli over estimated denominator
```

Smoke evidence:

```text
command:
  cargo run -q -p nando-cli -- phase-stream-online-miner-value-pass-v1 \
    target/nando-wave/streaming/phase-stream-online-miner-value-pass-denominator-smoke.report.json \
    32 \
    --price-config data/real_traffic/model_price_config.v1.json \
    target/nando-wave/streaming/auto-subcenter-discovery-contract-fix-v1.candidates.jsonl

report:
  target/nando-wave/streaming/phase-stream-online-miner-value-pass-denominator-smoke.report.json

total_rows: 754
exact_cache_hits: 612
non_exact_rows: 142
total_tokens_seen: 872321
total_cost_microusd_seen: 0
estimated_total_cost_microusd_seen: 872321
token_denominator_present: true
cost_denominator_present: false
estimated_cost_denominator_present: true
token_cost_denominator_present: false
product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: 62
product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows: 82
product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens: 40
product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost: 0
product_hot_candidate_upper_bound_estimated_cost_saved_microusd: 35279
product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost: 40
local_accept_enabled: false
market_money_claim_allowed: false
market_money_claim_blocker: provider_cost_missing_estimate_only
estimated_money_claim_allowed: false
estimated_money_claim_blocker: estimate_only_not_market_claim
```

Boundary:

```text
selector upper-bound only
no scoring/threshold/miner/verifier/promotion/local_accept behavior change
token denominator exists, estimate-only cost exists, provider cost denominator
is missing, so no market money claim
```

## 2026-07-08 - Clean-Survivor .nwpc Promotion Handoff

Budget reason:

```text
P0/P1 proof boundary fix: clean survivor shadow evidence previously stayed
in-memory as live_store_clean_candidate_survivors, so compression claim was
blocked by runtime source. The fix writes the same clean survivor candidates as
verifier-bound .nwpc packages and loads them through call_token_active_manifest.
```

Evidence:

```text
report:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report.json

manifest:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report-call-token-promotion-manifest.json

stable_decision_log_clean_suffix_claim_allowed: true
stable_decision_log_clean_suffix_claim_blocker: none
product_hot_score_only_runtime_source: call_token_active_manifest
call_token_promotion_manifest_allowed: true
call_token_promotion_manifest_promoted_candidates: 2
call_token_promotion_manifest_false_accepts: 0
call_token_promotion_manifest_runtime_parity_mismatches: 0
local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: no_future_shadow_billing_request_rows
```

Control:

```text
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

## 2026-07-08 - Stable Clean Billing Worklist

Budget reason:

```text
P0/P1 money-claim boundary fix: after clean .nwpc promotion, provider money
evidence was still blocked at no_future_shadow_billing_request_rows when the
bounded report had no new append future events. The fix exports stable clean
decision-suffix CPU accepts as a provider billing worklist, with source
correlation recovered from source + tail_line_index.
```

Evidence:

```text
report:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report.json

billing request:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report-future-shadow-billing-request.jsonl

future_shadow_billing_request_rows: 94
future_shadow_billing_request_tokens: 154500
future_shadow_billing_request_current_cost_microusd: 154500
future_shadow_billing_request_ready_for_external_provider_evidence: true
provider_billing_capture_contract_ready: true
provider_money_claim_blocker: external_provider_export_missing
local_accept_enabled: false
market_money_claim_allowed: false
```

Control:

```text
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```
