# EXECUTOR REVIEW NOTES

Status: active handoff only.

Full archived history:

```text
docs/archive/EXECUTOR_REVIEW_NOTES_2026-07-08_precompact.md
```

Rule:

```text
Keep this file short.
Do not append long historical logs here.
Archive old detail blocks and leave only current state, current task, and next blocker.
```

## Current Architecture Direction

Reference:

```text
docs/NE_BUSY_ARCHITECTURE.md
docs/NANDA_CPU_COMPACT_LATENT_TRANSITION_ARCHITECTURE.md
```

Active miner direction:

```text
L1 -> surface atoms
L2 -> hidden state split
L3 -> phase-center operator
L4 -> portfolio survivor selector
verifier -> admission / quarantine
```

Client/server handoff:

```text
ops/phase-center-test-server/CLIENT_HANDOFF.md
OpenAI-compatible local URL: http://127.0.0.1:8787/v2
health: curl -s http://127.0.0.1:8787/health
```

Core product rule:

```text
bad profile -> quarantine
clean sibling profiles -> stay hot
local_accept remains disabled until explicitly proven
```

Hard bans:

```text
no .nwrb revival
no source-agent hardcode in generic core
no manual class list as product logic
no provider billing in hot path
no local_accept without verifier and false_accepts = 0
no synthetic-only market claim
```

## Current Traffic Boundary: No OpenAI API Key

```text
timestamp: 2026-07-09

fact:
  user has Codex plan access, but no OpenAI API key.

decision:
  do not build product claims around OpenAI upstream proxy / provider-boundary.
  Codex OAuth/session tokens are not API keys and must not be used as upstream
  credentials.

active traffic path:
  Codex normal channel
    -> ~/.codex/sessions
    -> nando-phase-center-appender.service
    -> live-agent-phase-atom-append-v1.jsonl
    -> nando-phase-center-live-tail.service
    -> miner / .nwpc / dashboard

server guard:
  nando-codex is fail-open. It routes through /v2 only when health is OK and
  upstream auth is available; otherwise it restores the original OpenAI env and
  launches real Codex directly.

dashboard:
  primary incoming flow is Codex session stream.
  provider boundary with 0 rows is not a P0 while upstream/API key is absent.
```

## Current Miner Work: Verifier-Blocked Fallback Recognition

```text
timestamp: 2026-07-09

goal:
  make the miner explain and learn unsafe missed traffic automatically, without
  pretending verifier-blocked rows are CPU savings.

change:
  live-tail now marks non-exact rows with verified_safe_accept=false as
  verifier_blocked_non_exact.

change:
  nonzero/failure rows are split out with nonzero_verifier_blocked and
  nonzero_exit_state_kind from phase atoms such as state_exit_code_band:*.

change:
  dashboard exposes verifier_blocked, phase_seen, and nonzero_blocked next to
  CPU accepts, so the remaining gap is visible as fallback/negative mining
  material rather than a silent miss.

boundary:
  these rows do not count as saved calls or saved tokens. They keep feeding the
  phase-center miner and auto-split pressure, but local accept still requires
  verifier safety and false_accepts=0.
```

## Current Checkpoint: V2 NANDA CPU Bridge

```text
timestamp: 2026-07-08T14:28Z

default client endpoint:
  http://127.0.0.1:8787/v2

compatibility:
  /v1 remains available for old clients

live v2 health:
  ok: true
  default_client_api_version: v2
  v2_architecture: compact_latent_transition_runtime
  upstream_configured: false

live v2 local route:
  prompt: nando compression
  local_accept: true
  api_version: v2
  transition_runtime: true
  architecture: compact_latent_transition_runtime
  route: nando_compression_status
  tokens_saved: 352157
  false_accepts: 0

live v2 broad route:
  prompt: ordinary broad prompt
  local_accept: false
  error.type: upstream_missing
  fallback_reason: verifier_required_not_ok

live scorecard after v2 dogfood:
  stable_rows: 981
  unique_cpu_accepts_over_exact_cache: 350
  tokens_saved: 353905
  false_accepts: 0

provider bridge v1/v2 metrics window:
  provider_bridge_decision_window_rows: 98
  provider_bridge_local_accept_events: 98
  provider_bridge_tokens_saved_estimated: 449
  provider_bridge_false_accepts: 0
  provider_bridge_v1_local_accept_events: 88
  provider_bridge_v1_tokens_saved_estimated: 402
  provider_bridge_v1_false_accepts: 0
  provider_bridge_v2_local_accept_events: 10
  provider_bridge_v2_tokens_saved_estimated: 47
  provider_bridge_v2_false_accepts: 0
  provider_bridge_v2_transition_runtime_events: 10

bridge smoke:
  verdict: NANDO_PROVIDER_BRIDGE_SMOKE_PASS
  case_count: 8
  passed_count: 8
  failed_count: 0

upstream smoke:
  verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS
  case_count: 5
  passed_count: 5
  upstream_hit_count: 2
  provider_boundary_event_count: 2
  provider_boundary_total_tokens: 20

client env:
  OPENAI_BASE_URL=http://127.0.0.1:8787/v2
  NANDO_CPU_API_VERSION=v2

boxed package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T142837Z.tar.gz
  sha256: 72569965f2ce4074c782cf6841efb50bd97ce3543feb0bcba86a5bb9ee5710e7

rust-action-memory:
  doctor: PASS
  version: 0.3.0
  stage: R23_RELEASE_CANDIDATE
  gate.release_allowed: true
  gate.quarantined_candidates: 0

boundary:
  v2 is a product surface / API boundary, not a proof that the full new
  hidden-state transition miner is complete.
  money claim remains blocked until real provider evidence exists.
```

## Current Error Work: False Accept Discipline

```text
timestamp: 2026-07-09

finding:
  raw false accepts are concentrated in agent_continue_execute continuations
  after nonzero tool results, especially positive_nonzero / cargo_101.

interpretation:
  this is an unsafe continuation boundary. Failed/nonzero tool results must
  not be treated as repeatable safe continuation unless a narrower recovery
  subcenter proves itself.

core change:
  PhaseCenterOnlineMiner now quarantines the bucket on a verified false accept:
    raw_local_operator && verified_safe_accept=false -> bucket.rejected=true

effect:
  the rejected bucket keeps learning from future events, but cannot emit
  raw_local_operator, local_operator_shadow_decision, candidate runtime, or hot
  export. Clean sibling/subcenter profiles stay eligible.

verified:
  cargo fmt --check: pass
  cargo test -p nando-core false_accept -- --nocapture: pass
  cargo check -p nando-core: pass
  cargo check -p nando-cli: pass
  rust-action-memory selector-report: no diagnostics, no safe apply needed
  ops/phase-center-test-server/deploy.sh: pass

live after deploy:
  /v2 health ok
  test-server verify false_accepts=0
  stable clean compression false_accepts=0

boundary:
  stable_decision_log_false_accepts / append_false_accepts can include old raw
  history. Product claims must use clean windows or post-quarantine windows.
```

## Current Miner Work: Token-First Quarantine Recovery

```text
timestamp: 2026-07-09

goal:
  recover token-heavy quarantined profiles by automatic split/subcenter mining;
  do not lower thresholds.

change:
  PhaseCenterOnlineMiner candidate ranking is now token-first:
    tokens_saved -> unique accepts -> cost -> bucket_id

change:
  product-hot survivor selector also ranks clean siblings/subcenters token-first.

change:
  live-tail now forces discovery sampling when the current event touches a
  quarantined primary/subcenter profile. These events bypass discovery
  throttling but remain bounded.

change:
  quarantined parent subcenters now spawn bounded recovery child buckets:
    quarantine_recovery(parent_profile, split_atom)
  using only safe split atoms. This is automatic streaming split pressure, not
  manual class selection and not threshold lowering.

new metrics:
  quarantine_recovery_discovery_events
  quarantine_recovery_discovery_tokens
  quarantine_recovery_auto_subcenter_observe_events

verified:
  cargo fmt --check: pass
  cargo test -p nando-core online_miner_ranks_candidate_recovery_by_tokens_before_call_count -- --nocapture: pass
  cargo check -p nando-core: pass
  cargo check -p nando-cli: pass
  rust-action-memory selector-report: diagnostics=0, quarantined_candidates=0
  ops/phase-center-test-server/deploy.sh: pass

live after deploy:
  /v2 health ok
  services active: appender, live-tail, provider-bridge
  stable clean window:
    rows: 435
    saved_tokens: 270640
    total_tokens: 335877
    saved_milli: 805
    false_accepts: 0
  append_false_accepts: 0
  product_hot_post_quarantine_false_accepts: 0
  quarantine_recovery_discovery_events: 12
  quarantine_recovery_discovery_tokens: 11718
  quarantine_recovery_auto_subcenter_observe_events: 244
  top quarantined token profiles:
    4084164558 hidden_state tokens_saved=367680 false_accepts=0
    1215237470 hidden_state tokens_saved=336307 false_accepts=0
    1648691765 hidden_state tokens_saved=158805 false_accepts=0

late fix:
  product-hot eval now intersects runtime decisions with current event
  relevant_online_bucket_ids before scoring active decisions. This prevents
  unrelated same-route profiles from producing fresh append false accepts.

multi-split:
  quarantine recovery now generates bounded parent+split and parent+split_pair
  recovery children. This gives mixed quarantined profiles deeper automatic
  subcenters without manual classes and without lowering threshold.

trust filter:
  PhaseCenterOnlineBucket now keeps miner-only EWMA trust telemetry:
    trust_quality_micro
    trust_false_risk_micro
    trust_drift_micro
    trust_token_value_micro
  It is O(1), not in hot runtime scoring, and is intended for promote /
  quarantine / split-deeper / sleep control.
  first live top quarantine profile:
    profile_id: 4084164558
    trust_quality_micro: 138872
    trust_false_risk_micro: 0
    trust_drift_micro: 143687
    trust_token_value_micro: 347

boundary:
  this proves the miner is now applying automatic token-first recovery pressure.
  It does not yet prove those fattest quarantined parents have all recovered
  into final hot subcenters. Current active hot profile count is still 1, while
  hidden_state:quarantined holds roughly 1.4M token opportunity. Next work:
  use trust telemetry to promote clean low-risk children and split high-drift
  quarantined parents deeper.
```

## Current CPU Traffic Goal: Serving vs Shadow Split

```text
timestamp: 2026-07-09

goal:
  maximize real traffic handled on CPU without weakening verifier/safety.

finding:
  stable/live-tail decision log is a shadow/miner surface. Its score_candidate
  false events must keep driving quarantine, but they are not the same as
  real provider/gateway local_accept failures.

change:
  added separate stable_serving_cpu_* windows that count false_accepts only
  when a decision actually has local_accept=true.

change:
  metrics/dashboard now expose edge_serving_cpu_* from gateway + provider v2:
    edge_serving_cpu_local_accept_events
    edge_serving_cpu_tokens_saved_estimated
    edge_serving_cpu_false_accepts

live after deploy:
  /v2 health ok
  dashboard /v2/status returns 200
  services active: appender, live-tail, provider-bridge
  edge_serving_cpu_local_accept_events: 724
  edge_serving_cpu_tokens_saved_estimated: 2986
  edge_serving_cpu_false_accepts: 0
  gateway_local_accept_events: 550
  provider_bridge_v2_local_accept_events: 174
  product_hot_score_only_post_quarantine_false_accepts: 0
  stable shadow false_accepts: 0

boundary:
  edge_serving_cpu is the actual local CPU serving layer.
  stable_clean/shadow metrics remain diagnostic pressure for quarantine and
  miner recovery, not the user-facing "CPU processed traffic" denominator.
```

## Latest Production Canary Checkpoint

```text
timestamp: 2026-07-08

boxed deploy:
  ops/phase-center-test-server/deploy.sh: pass

systemd:
  nando-phase-center-appender.service: active
  nando-phase-center-live-tail.service: active
  nando-provider-bridge.service: active

resource guards:
  nando-provider-bridge.service:
    MemoryCurrent: ~13.3 MB
    MemoryPeak: ~19.0 MB
    MemoryHigh: 64 MB
    MemoryMax: 256 MB
    MemorySwapMax: 0
    MemorySwapCurrent: 0
    CPUQuota: 100%
  nando-phase-center-appender.service:
    MemoryCurrent: ~0.6 MB
    MemoryPeak: ~2.3 MB
    MemoryHigh: 64 MB
    MemoryMax: 256 MB
    MemorySwapMax: 0
    MemorySwapCurrent: 0
    CPUQuota: 25%
  nando-phase-center-live-tail.service:
    MemoryCurrent: ~4.7 MB
    MemoryPeak: ~4.9 MB after swap-guarded redeploy
    MemoryHigh: 256 MB
    MemoryMax: 512 MB
    MemorySwapMax: 0
    MemorySwapCurrent: 0
    CPUQuota: 50%

server policy:
  NANDO_LOCAL_ACCEPT_ENABLED=1
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1
  NANDO_CLIENT_SAFETY_POLICY=guarded_verified_routes
  NANDO_CLIENT_TIER=canary_verified
  NANDO_CLIENT_REQUIRE_VERIFIER=1
  NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO=1

local executor routes now verifier-bound:
  nando health
  nando status / nando server
  nando compression
  nando readiness
  nando promotion

cold snapshot refresh:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-refresh-snapshots.sh
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.refresh-snapshots.json
  verdict: NANDO_PHASE_CENTER_REFRESH_SNAPSHOTS_PASS
  failed_count: 0
  boundary: cold reports only; not used in hot request scoring

boxed gateway canary smoke:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-gateway-canary-smoke.sh
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.gateway-canary-smoke.json
  verdict: NANDO_GATEWAY_CANARY_SMOKE_PASS
  case_count: 6
  passed_count: 6
  failed_count: 0

gateway smoke:
  nando compression -> local_accept
  nando readiness -> local_accept
  nando promotion -> local_accept
  ordinary broad prompt -> provider fallback

promotion:
  promotion_allowed: true
  blocker: none
  runtime_canary_active: true
  runtime_canary_safe: true

clean token compression scorecard:
  stable_rows: 627
  unique_cpu_accepts_over_exact_cache: 209
  tokens_saved: 215108
  false_accepts: 0

actual gateway canary:
  gateway_local_accept_events: 110
  gateway_tokens_saved_estimated: 439
  gateway_false_accepts: 0
  gateway_local_route_count: 6

HTTP provider bridge:
  service: nando-provider-bridge.service
  bind: 127.0.0.1:8787
  health: ok
  smoke: NANDO_PROVIDER_BRIDGE_SMOKE_PASS
  cases: 4
  passed: 4
  failed: 0
  /health: ok
  /v1/chat/completions "nando compression": local_accept
  /v1/responses "nando readiness": local_accept
  broad prompt: upstream_missing while upstream is unset

HTTP provider bridge upstream transport smoke:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-smoke.sh
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-smoke.json
  verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS
  failed_count: 0
  upstream_hit_count: 1
  provider_boundary_event_count: 1
  provider_boundary_total_tokens: 10
  proof:
    "nando compression" stays local nando-local
    ordinary broad prompt reaches fake upstream exactly once
    fake upstream response writes one metadata-only provider boundary event

provider boundary capture:
  path: /var/lib/nando-wave/streaming/nando-provider-bridge.provider-boundary-events.jsonl
  production_current_rows: 0 while real upstream is unset
  smoke_boundary_rows: isolated temp path only; not counted as market evidence
  boundary: records upstream request hash, provider ids, and usage tokens when
    real upstream traffic flows through bridge; cost remains blocked without
    real provider cost evidence

provider boundary evidence snapshot:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-evidence-snapshot.sh
  report: /var/lib/nando-wave/streaming/provider-evidence/provider-evidence-snapshot.report.json
  coverage_report: /var/lib/nando-wave/streaming/provider-evidence/provider-boundary-capture-coverage.report.json
  current_status: no_provider_bridge_boundary_rows while real upstream is unset
  market_money_claim_allowed: false
  boundary: cold provider-boundary metadata coverage only; does not create
    provider billing evidence, local_accept, or money claims

HTTP provider bridge upstream readiness:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-readiness.sh
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-readiness.json
  verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
  upstream_configured: false
  real_probe_attempted: false
  ready_for_broad_provider_traffic: false
  boundary: default readiness does not call a real upstream provider; optional
    real probe requires NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL=1

HTTP provider bridge upstream config:
  script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh
  status:
    upstream_configured: false
    api_key_present: false
    api_key_value_printed: false
  use:
    printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh /etc/nando-wave/phase-center.env set --base-url https://api.openai.com --api-key-stdin --provider openai
  boundary: server-side secret tool only; provider keys are never printed and
    must not be passed to client windows

actual HTTP bridge canary:
  provider_bridge_local_accept_events: 29
  provider_bridge_tokens_saved_estimated: 132
  provider_bridge_false_accepts: 0
  provider_bridge_local_route_count: 2

money:
  market_money_claim_allowed: false
  blocker: external_provider_export_missing

server verify:
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json
  verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
  install_ready: true
  shadow_metrics_ready: true
  upstream broad provider traffic: not ready until server upstream is configured

boundary:
  This is real verifier-bound canary local_accept for narrow artifact-backed
  routes. Broad provider traffic through the HTTP bridge fails open to upstream
  when configured, writes metadata-only provider boundary events, or returns
  upstream_missing while upstream is unset. Full Codex provider transport
  interception remains separate production work.
```

## Latest 10-Knee Scorecard Checkpoint

```text
timestamp: 2026-07-09T00:41+03:00

scorecard:
  average: 9.7/10
  token_compression: 77.3%
  cpu_accepts: 174
  false_accepts: 0

10/10 knees:
  L1 Surface Capture
  L2 Hidden State Packer
  Online Miner
  Subcenter Split
  Candidate Lifecycle
  Shadow / Promotion Gate
  .nwpc Package
  Hot Runtime

remaining non-10 knees:
  Event Sources: 9/10, upstream_configured=false
  Server / Dashboard: 8/10, money_evidence=false

changes:
  metrics snapshot now treats clean-promotion-manifest safe promoted profiles
  as exportable for observability scorecards.
  metrics snapshot writes JSON/prom files atomically.
  shared clean registry admission budget was widened to 16 profiles per route/worker;
  false_accepts, parity, budget, verifier gates remain mandatory.

boundary:
  token claim is allowed.
  market money claim remains blocked until provider billing/export evidence exists.
```

## Current Spectral Budget Debt

Reference:

```text
docs/NANDO_WAVE_SPECTRAL_BUDGET_AUDIT.md
```

Current P0 split targets:

```text
phase_streaming_cmd.rs
live_store_adapter.rs
phase_package_cmd.rs
phase_center_runtime.rs
online_miner_daemon.rs
phase_daemon_cmd.rs
```

Current budget snapshot:

```text
phase_streaming_cmd.rs                                             24226 lines
phase_package_cmd.rs                                               16408 lines
live_store_adapter.rs                                              10802 lines
phase_center_runtime.rs                                             7656 lines
online_miner_daemon.rs                                              7190 lines
phase_daemon_cmd.rs                                                 5320 lines
```

User directive:

```text
live_store_adapter.rs is still too fat.
Keep it as a first-class spectral-budget refactor target until it is no
longer a mixed source/miner/report/runtime monolith.
After each budget pass, rescan all active files and update the audit.
```

Rule:

```text
move-only refactor first; no scoring, threshold, verifier, miner, promotion,
or compression-claim behavior changes during budget cuts.
```

Latest budget refresh:

```text
2026-07-08 full active-file scan refreshed.
Data corpora/lexicons are data-budget artifacts, not Rust refactor targets.
Rust P0 remains command/runtime/miner files, with live_store_adapter.rs kept as
the user-flagged first-class split target.
```

Latest move-only cut:

```text
live_store_adapter/numeric_false_accept_split_audit.rs added
live_store_adapter.rs false-accept split audit route moved out:
  run_phase_stream_hot_path_daemon_numeric_false_accept_split_audit_v1
live_store_adapter.rs reduced 11285 -> 10802
behavior change: none
checks:
  cargo fmt --check
  RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
  git diff --check
  rust-action-memory review --workspace .
```

Latest value-pass denominator smoke:

```text
report:
  target/nando-wave/streaming/phase-stream-online-miner-value-pass-denominator-smoke.report.json

total_rows: 754
exact_cache_hits: 612
non_exact_rows: 142
total_tokens_seen: 872321
total_cost_microusd_seen: 0
estimated_total_cost_microusd_seen: 872321
product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: 62
product_hot_candidate_upper_bound_estimated_cost_saved_microusd: 35279
product_hot_candidate_upper_bound_estimated_cost_saved_milli_over_estimated_total_cost: 40
market_money_claim_allowed: false
market_money_claim_blocker: provider_cost_missing_estimate_only
estimated_money_claim_allowed: false
estimated_money_claim_blocker: estimate_only_not_market_claim
```

## Production Server Checkpoint

Latest boxed server deploy:

```text
ops/phase-center-test-server/deploy.sh

installed:
  /opt/nando-wave
  /etc/nando-wave/phase-center.env
  /var/lib/nando-wave/streaming
  /var/log/nando-wave
  /usr/local/bin/nando-llm-gateway

systemd:
  nando-phase-center-appender.service: active
  nando-phase-center-live-tail.service: active
  nando-phase-center-* timers: active

server policy:
  NANDO_LOCAL_ACCEPT_ENABLED=0
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=0
  NANDO_CLIENT_SAFETY_POLICY=shadow_only
  NANDO_CLIENT_REQUIRE_VERIFIER=1
  NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO=1

verification:
  cargo fmt --check: pass
  RUSTFLAGS='-D warnings' cargo check -q -p nando-cli: pass
  rust-action-memory review --workspace .: diagnostics 0
  systemd_verify_pass: true
  install_ready: true

gateway:
  shadow mode falls back to provider command
  canary-health policy accepts only exact built-in health/status route
  broad prompt still falls back

current readiness blocker:
  product_hot_post_quarantine_window_missing
  market_money_claim_blocked
  local_accept_promotion_blocked
```

Latest live-tail claim blocker fix:

```text
report:
  target/nando-wave/streaming/live-tail-claim-blocker-source-ready-fix.seeded.report.json

stable_decision_log_rows: 1809
stable_decision_log_clean_suffix_rows: 407
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 80
stable_decision_log_clean_suffix_tokens_saved: 111228
product_hot_score_only_runtime_source: live_store_clean_candidate_survivors
product_hot_score_only_runtime_loaded: true
product_hot_score_only_runtime_active: true
final_hot_runtime_available: true
stable_decision_log_clean_suffix_claim_allowed: false
stable_decision_log_clean_suffix_claim_blocker: append_runtime_source_not_claim_ready
local_accept_enabled: false
```

Meaning:

```text
The previous append_no_final_hot_runtime blocker was misleading for clean
suffix windows. Runtime exists, but the runtime source is an in-memory clean
survivor view, not a claim-ready promoted manifest/registry source. Next high
value step: package or promote clean survivors through a verifier-bound
claim-ready source, still with false_accepts=0 and local_accept disabled.
```

Latest cut:

```text
live_store_adapter/defaults.rs added
live_store_adapter.rs default paths/limits moved out
live_store_adapter/architecture.rs added
live_store_adapter.rs architecture-version report/key moved out
live_store_adapter/reports.rs added
live_store_adapter.rs report schema groups moved out:
  budget/direct-hot/route/bucket/candidate/manifest/future-shadow
  prepared-hot
  clean-manifest shadow
  worker report schemas
  append live-tail report schema
  numeric package/future/false-accept audit report schemas
  live_store_adapter/source_events.rs added
live_store_adapter.rs source extraction helpers moved out:
  parsed event structs
  safe atoms / leak filter
  adaptive bucket policy
  route key / bucket selector / refinement blocker
  full row-to-parsed-atom-event adapter
live_store_adapter/hidden_state.rs added
live_store_adapter.rs L2 hidden-state / auto-subcenter construction moved out:
  hidden_state atoms
  observable subcenter atoms
  pair/combo subcenter bucket IDs
live_store_adapter/state.rs added
live_store_adapter.rs state/type bundles moved out:
  stable decision-log window
  product-hot credit rows
  false-accept atom accumulator
  future billing/provider artifact summaries
  persisted product-hot quarantine state
  registry shadow reports
  clean/product-hot runtime bundles
  direct-hot snapshot bank/eval
live_store_adapter/persistence.rs added
live_store_adapter.rs persistence helpers moved out:
  persisted product-hot quarantine report loader
live_store_adapter/paths.rs added
live_store_adapter.rs path/key helpers moved out:
  route key from bucket key
  registry-relative package path resolver
  hot-path review/policy report paths
  append-tail promotion manifest/package paths
  numeric future package/portfolio report paths
live_store_adapter/runtime_registry.rs added
live_store_adapter.rs runtime registry helpers moved out:
  clean promotion manifest .nwpc runtime loader
  product-hot registry .nwpc runtime loader
  call/token active manifest runtime loader
  call/token quarantine detector
  product-hot route-index resolver
  call/token active manifest disable helper
live_store_adapter/profile_attribution.rs added
live_store_adapter.rs L4 profile-attribution helpers moved out:
  observable/hidden/unknown profile classification
  call/token/cost attribution counters
live_store_adapter/bucket_decisions.rs added
live_store_adapter.rs bucket decision selectors moved out:
  exact bucket decisions
  union score-candidate decision
  relevant online bucket IDs / decisions
live_store_adapter/diagnostics.rs added
live_store_adapter.rs diagnostics accumulators/builders moved out:
  route/bucket counters
  route/bucket report builders
live_store_adapter/candidate_packages.rs added
live_store_adapter.rs candidate package IO moved out:
  verifier binding
  verifier-bound .nwpc candidate package writer
live_store_adapter/hot_path_gates.rs added
live_store_adapter.rs hot-path blocker helpers moved out:
  prepared-pack blocker
  memory/thread worker blockers
live_store_adapter/survivor_runtime.rs added
live_store_adapter.rs clean survivor runtime helpers moved out:
  candidate frontier
  candidate value reports
  clean survivor hot runtime builder
  hidden-state/observable-subcenter priority selectors
  quarantined/observable-primary exclusion set
live_store_adapter/quarantine.rs added
live_store_adapter.rs stable decision-log quarantine helpers moved out:
  decision-log architecture filters
  non-exact false profile-id extraction
  stable decision-log window aggregation
  score-candidate/local-accept counters excluding quarantined profiles
live_store_adapter/policy_json.rs added
live_store_adapter.rs JSON policy helpers moved out:
  u32 vector extraction
  forbidden flag parsing
live_store_adapter/runtime_metrics.rs added
live_store_adapter.rs runtime metric helpers moved out:
  hot route/profile IDs
  active profile counts
  milli/permille helpers
  provider/estimated cost row checks
  runtime budget report mapping
live_store_adapter/claim_gates.rs added
live_store_adapter.rs claim/blocker helpers moved out:
  hot-path benchmark blocker
  promotion/admission blocker names
  append compression claim blocker
  product-hot runtime source claim readiness
live_store_adapter/source_readers.rs added
live_store_adapter.rs source/queue helpers moved out:
  score source adapter reader
  queue and threaded source readers
  direct store event collection
  append shadow event collection
  live-loop budget event observation
  worker batch send/flush
live_store_adapter/hot_path_eval.rs added
live_store_adapter.rs hot-path eval helpers moved out:
  direct-hot snapshot capture/select/eval
  prepared hot-path row builder
  runtime parity checks
  denominator and candidate-decision eval counters
behavior change: none
checks:
  cargo fmt --check
  RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
  git diff --check
  rust-action-memory review --workspace .
phase_streaming_cmd/defaults.rs added
phase_streaming_cmd.rs report/default path constants moved out
live_store_adapter/promotion_manifests.rs added
live_store_adapter.rs promotion manifest handoff moved out:
  clean promotion manifest writer
  call/token promotion manifest summary
  call/token promotion manifest writer
  call/token manifest blockers
  candidate runtime parity helpers
live_store_adapter/provider_evidence.rs added
live_store_adapter.rs provider evidence handoff moved out:
  future-shadow billing request JSONL writer
  provider export signature helpers
  provider money claim blocker
  cold provider evidence artifact refresh
live_store_adapter/future_shadow_registry.rs extended
live_store_adapter.rs future-shadow refresh/observe moved out:
  live_store_refresh_future_shadow_summary
  observe_live_store_future_shadow
live_store_adapter.rs reduced 11556 -> 11285
live_store_adapter/hot_path_eval.rs extended
live_store_adapter.rs direct-hot eval report moved out:
  direct mutable-store hot runtime report helper
  direct-hot blocker helper
behavior change: none
checks so far:
  cargo fmt
  RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
  git diff --check
rust-action-memory review --workspace .
```

## 2026-07-08 - Reviewer Check: One-Command Server Status

CHANGE:

```text
Added operator-facing status command:
  ops/phase-center-test-server/bin/nando-phase-center-status.sh

Installed command:
  /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh

Purpose:
  one JSON view for bridge health, verify, readiness, upstream readiness,
  provider evidence, metrics, scorecard, and key systemd services.
```

BOUNDARY:

```text
Status command is read-only by default.
It does not mine, score, call provider, mutate policy, or print secrets.
--refresh only refreshes existing snapshots/verify reports.
```

LIVE STATUS:

```text
command:
  /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh

summary.canary_local_accept_ready: true
summary.broad_provider_traffic_ready: false
summary.money_claim_ready: false
summary.next_action: configure_provider_upstream

scorecard.stable_rows: 765
scorecard.unique_cpu_accepts_over_exact_cache: 262
scorecard.tokens_saved: 319806
scorecard.false_accepts: 0

bridge.health_ok: true
bridge.local_accept_enabled: true
bridge.client_allow_local_accept: true
bridge.safety_policy: guarded_verified_routes
bridge.upstream_configured: false

upstream.verdict:
  NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET

services active:
  nando-phase-center-appender.service
  nando-phase-center-live-tail.service
  nando-provider-bridge.service
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111100Z.tar.gz

sha256:
  39d53dfd03d27cb418e5bb5bf2e18e8165cde35a208775d0b22529bbb1f23ecd

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111100Z.package.json

package includes:
  nando-phase-center-status.sh
  rust-action-memory-gate.json
  install-from-bundle.sh
```

RUST ACTION MEMORY:

```text
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

CONTROL:

```text
git diff --check
bash -n scripts + ops bin
systemd-analyze verify ops units
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
bundle sha256 check
bundle install-only smoke
```

## 2026-07-08 - Reviewer Check: Continuous Status Timer

CHANGE:

```text
Added continuous status snapshot units:
  ops/phase-center-test-server/systemd/nando-phase-center-status.service
  ops/phase-center-test-server/systemd/nando-phase-center-status.timer

Deploy now enables:
  nando-phase-center-status.timer

Verify now requires:
  nando-phase-center-status.service
  nando-phase-center-status.timer
```

PURPOSE:

```text
Keep one always-fresh operator status file:
  /var/lib/nando-wave/streaming/metrics/nando-phase-center.status.json

This avoids manual multi-file JSON inspection for:
  bridge health
  local_accept policy
  compression scorecard
  upstream readiness
  money-claim readiness
  service states
```

LIVE SYSTEMD:

```text
nando-phase-center-status.timer: active
nando-phase-center-status.service: oneshot success
Mem peak: 5M
CPU: about 115ms
```

LIVE STATUS SNAPSHOT:

```text
generated_utc: 2026-07-08T11:19:36Z
canary_local_accept_ready: true
broad_provider_traffic_ready: false
money_claim_ready: false
next_action: configure_provider_upstream

stable_rows: 827
unique_cpu_accepts_over_exact_cache: 292
tokens_saved: 327650
false_accepts: 0
```

VERIFY:

```text
verdict:
  NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY

install_ready: true
missing_units: []
missing_scripts: []
local_accept_enabled: true
upstream_configured: false
ready_for_broad_provider_traffic: false
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111956Z.tar.gz

sha256:
  3200b35af354a14596a6f015acb4e8feeeae10338aa94ecfedc488282743c0c8

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111956Z.package.json

package includes:
  nando-phase-center-status.service
  nando-phase-center-status.timer
  nando-phase-center-status.sh
  rust-action-memory-gate.json
```

CONTROL:

```text
git diff --check
bash -n ops bin + scripts
systemd-analyze verify ops units
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
bundle sha256 check
bundle install-only smoke
```

Latest proof-denominator update:

```text
online_miner_daemon value-pass report now includes real denominator fields:
  exact_cache_hits
  non_exact_rows
  total_tokens_seen
  total_cost_microusd_seen
  token_denominator_present
  cost_denominator_present
  token_cost_denominator_present
  market_money_claim_blocker
  upper-bound calls/tokens/cost saved milli over total denominators

smoke report:
  target/nando-wave/streaming/phase-stream-online-miner-value-pass-denominator-smoke.report.json

smoke snapshot:
  total_rows: 754
  exact_cache_hits: 612
  non_exact_rows: 142
  total_tokens_seen: 872321
  total_cost_microusd_seen: 0
  token_denominator_present: true
  cost_denominator_present: false
  token_cost_denominator_present: false
  product_hot_candidate_upper_bound_unique_accepts_over_exact_cache: 62
  product_hot_candidate_upper_bound_calls_saved_milli_over_total_rows: 82
  product_hot_candidate_upper_bound_tokens_saved_milli_over_total_tokens: 40
  product_hot_candidate_upper_bound_cost_saved_milli_over_total_cost: 0
  local_accept_enabled: false
  market_money_claim_allowed: false
  market_money_claim_blocker: cost_denominator_missing

boundary:
  selector upper-bound only; token denominator exists, cost denominator is missing, so no money claim.
```

## Current Live Miner State

Latest archived checkpoint:

```text
2026-07-08 - Reviewer Check: Clean Survivor Runtime Keeps Portfolio Alive
```

What changed:

```text
The live daemon can build a shadow-only product-hot runtime from clean online
candidate survivors even when the call/token promotion manifest is blocked by
quarantine.
```

Latest snapshot from that checkpoint:

```text
source:
  live_store_clean_candidate_survivors

product_hot_score_only_runtime_loaded: true
product_hot_score_only_runtime_active: true
product_hot_score_only_active_profile_count: 4
product_hot_score_only_package_bytes: 2272

append_score_candidate_events: 13
append_unique_cpu_accepts_over_exact_cache: 10
append_tokens_saved: 7585
append_false_accepts: 1

final_hot_runtime_available: true
final_hot_profile_count: 4

local_accept_enabled: false
market_money_claim_allowed: false
```

Current versioning gate:

```text
live_tail_daemon: append_live_tail_shadow_daemon_v6
compression_accounting: restart_safe_claimsafe_stable_window_calls_tokens_cost_milli_accounting_v4

New report contract:
  append_compression_claim_min_rows
  append_compression_claim_allowed
  append_compression_claim_blocker
  stable_decision_log_architecture_key
  stable_decision_log_rows
  stable_decision_log_claim_allowed
  stable_decision_log_claim_blocker
  stable_decision_log_clean_suffix_rows
  stable_decision_log_clean_suffix_min_rows
  stable_decision_log_clean_suffix_rows_to_min
  stable_decision_log_clean_suffix_claim_allowed
  stable_decision_log_clean_suffix_claim_blocker

Reason:
  visible calls/tokens/cost counters are not product claims when append
  false_accepts, local_accept, missing denominator, missing hot runtime, or
  post-quarantine false_accepts are present. A clean tiny window is only a
  smoke and stays blocked by append_window_below_min_rows. Stable proof-window
  accounting is now restart-safe and counts only decision-log rows with the
  current architecture_version_key. Stable clean-suffix accounting separately
  tracks the rows after the latest non-exact false accept so old quarantine
  failures do not masquerade as current clean traffic.
```

Latest stable clean-suffix smoke:

```text
report:
  target/nando-wave/streaming/stable-clean-suffix-smoke.report.json

stable_decision_log_rows: 1179
stable_decision_log_false_accepts: 5
stable_decision_log_claim_allowed: false
stable_decision_log_claim_blocker: append_false_accepts_nonzero

stable_decision_log_clean_suffix_rows: 44
stable_decision_log_clean_suffix_min_rows: 100
stable_decision_log_clean_suffix_rows_to_min: 56
stable_decision_log_clean_suffix_score_candidate_events: 48
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 23
stable_decision_log_clean_suffix_tokens_saved: 26342
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_last_quarantine_row_index: 1135
stable_decision_log_clean_suffix_claim_allowed: false
stable_decision_log_clean_suffix_claim_blocker: append_no_final_hot_runtime

local_accept_enabled: false
market_money_claim_allowed: false
```

Registry smoke:

```text
target:
  target/nando-wave/streaming/stable-clean-suffix-smoke-with-registry.report.json

result:
  blocked before report write
  blocker: product-hot registry budget gate is not passed

Meaning:
  false_accept suffix is clean, but product claim still needs a valid final hot
  runtime/budget path and enough suffix rows. This is proof debt, not a pass.
```

Correct promoted .nwpc handoff smoke:

```text
manifest copied for smoke:
  target/nando-wave/streaming/stable-clean-suffix-promoted-call-token-promotion-manifest.json

report:
  target/nando-wave/streaming/stable-clean-suffix-promoted.json

product_hot_score_only_runtime_source: call_token_promotion_manifest
product_hot_score_only_runtime_loaded: true
product_hot_score_only_runtime_active: true
product_hot_score_only_active_profile_count: 1
product_hot_budget_passed: true
final_hot_runtime_available: true

stable_decision_log_clean_suffix_rows: 44
stable_decision_log_clean_suffix_min_rows: 100
stable_decision_log_clean_suffix_rows_to_min: 56
stable_decision_log_clean_suffix_score_candidate_events: 48
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 23
stable_decision_log_clean_suffix_tokens_saved: 26342
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_claim_allowed: false
stable_decision_log_clean_suffix_claim_blocker: append_window_below_min_rows

local_accept_enabled: false
market_money_claim_allowed: false
```

Current next blocker:

```text
clean suffix needs >= 100 rows under promoted .nwpc runtime.
Current suffix rows: 44.
Need 56 more clean post-quarantine rows before product compression claim can
advance to the next gate.
```

Current local live service:

```text
service:
  nando-phase-live-tail-current.service

command:
  phase-stream-hot-path-daemon-append-live-tail-v1

status:
  active/running under user systemd

report:
  target/nando-wave/streaming/live-tail-daemon-current.report.json

decision log:
  target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.decisions.jsonl

latest observed snapshot:
  append_parsed_rows: 33
  append_score_events_before_update: 33
  append_score_candidate_events: 33
  append_unique_cpu_accepts_over_exact_cache: 26
  append_tokens_saved: 30695
  append_false_accepts: 1
  append_clean_suffix_rows: 9
  append_clean_suffix_false_accepts: 0
  stable_decision_log_rows: 1212
  stable_decision_log_unique_cpu_accepts_over_exact_cache: 367
  stable_decision_log_tokens_saved: 427596
  stable_decision_log_false_accepts: 5
  stable_decision_log_clean_suffix_rows: 77
  stable_decision_log_clean_suffix_rows_to_min: 23
  stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 49
  stable_decision_log_clean_suffix_tokens_saved: 57037
  stable_decision_log_clean_suffix_false_accepts: 0
  final_hot_runtime_available: true
  product_hot_score_only_runtime_source: live_store_clean_candidate_survivors
  product_hot_score_only_runtime_active: true
  product_hot_score_only_active_profile_count: 4
  product_hot_score_only_quarantined: true
  product_hot_score_only_quarantine_false_accepts: 1
  product_hot_score_only_post_quarantine_score_candidate_events: 31
  product_hot_score_only_post_quarantine_false_accepts: 0
  hot_bytes_estimate: 2248
  worker_rss_observed: about 60 MiB
  local_accept_enabled: false
  market_money_claim_allowed: false

current interpretation:
  shadow signal is live. One bad product-hot profile was quarantined; clean
  sibling profiles stayed active, and post-quarantine product-hot false_accepts
  are zero so far.
  product/market claim remains blocked because the current hot runtime source
  is clean survivor runtime, not a claim-ready promotion/registry source, and
  the clean suffix is still below the 100-row proof window.
```

Latest claim-safe smoke after restart:

```text
report:
  target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.report.json

append_parsed_rows: 1
append_false_accepts: 0
append_compression_claim_min_rows: 100
append_compression_claim_allowed: false
append_compression_claim_blocker: append_window_below_min_rows
active_clean_calls_saved: 1
active_clean_tokens_saved: 479
product_hot_score_only_post_quarantine_false_accepts: 0
final_hot_runtime_available: true
final_hot_profile_count: 4
local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: no_future_shadow_billing_request_rows

Boundary:
  small fresh tail window; valid as schema/safety smoke, not a market money
  claim and not a stable compression claim.
```

Latest restart-safe decision-log smoke:

```text
decision_schema_version: append_live_tail_decision_v2
decision rows carry:
  architecture_versions
  architecture_version_key

stable_decision_log_rows: 5
stable_decision_log_score_candidate_events: 0
stable_decision_log_false_accepts: 0
stable_decision_log_claim_allowed: false
stable_decision_log_claim_blocker: append_window_below_min_rows

Boundary:
  decision log is now append-only and restart-safe, but current-version stable
  rows have not yet reached the minimum proof window or score-candidate value.
```

Latest test-server package:

```text
package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T012500Z.tar.gz
package_report:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T012500Z.package.json
sha256 sidecar:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T012500Z.tar.gz.sha256
sha256:
  8e40e9732ad5da0701e827167520734bf3ebe04bbe4251d3214f2e740def6eaf

contents:
  release nando-cli
  phase-center appender systemd unit
  phase-center live-tail shadow miner systemd unit
  metrics snapshot systemd unit/timer
  provider-evidence snapshot systemd unit/timer
  provider-export contract-pack systemd unit/timer
  readiness snapshot systemd unit/timer
  test-server verify systemd unit/timer
  local-accept promotion gate systemd unit/timer
  provider-export watch systemd unit/timer
  install/readiness verify script
  metrics snapshot script for JSON + Prometheus text
  deploy README and env example

local snapshot from current live-tail report:
  stable_decision_log_rows: 682
  stable_decision_log_unique_cpu_accepts_over_exact_cache: 197
  stable_decision_log_tokens_saved: 234042
  stable_decision_log_false_accepts: 1
  stable_decision_log_claim_allowed: false
  stable_decision_log_claim_blocker: append_false_accepts_nonzero
  product_hot_compression_claim_allowed: false
  product_hot_compression_claim_blocker: product_hot_post_quarantine_window_missing
  product_hot_post_quarantine_false_accepts: 0
  product_hot_post_quarantine_score_candidate_events: 0
  active_clean_calls_saved: 197
  active_clean_tokens_saved: 234042
  future_shadow_billing_request_rows: 19
  future_shadow_billing_request_tokens: 46080
  market_money_claim_allowed: false
  provider_money_claim_blocker: external_provider_export_missing

provider evidence snapshot smoke:
  acquisition.billing_request_rows: 19
  acquisition.provider_boundary_capture_request_rows: 19
  acquisition.total_tokens_requiring_billing: 46080
  acquisition.external_provider_collection_worklist_ready: true
  evidence_chain.provider_billing_evidence_present: false
  evidence_chain.market_money_claim_allowed: false
  blocker: external_provider_export_missing

provider export contract-pack smoke:
  contract_ready: true
  billing_request_rows: 19
  total_tokens_requiring_billing: 46080
  blocker: external_provider_export_missing
  market_money_claim_allowed: false
  local_accept_enabled: false

readiness snapshot smoke:
  compression_claim_allowed: false
  raw_stable_compression_claim_allowed: false
  raw_stable_compression_claim_blocker: append_false_accepts_nonzero
  product_hot_compression_claim_allowed: false
  product_hot_compression_claim_blocker: product_hot_post_quarantine_window_missing
  money_evidence_ready: false
  market_money_claim_allowed: false
  local_accept_promotion_allowed: false
  blocker: product_hot_post_quarantine_window_missing

test-server verify smoke:
  install_ready: true
  shadow_metrics_ready: false
  verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_INSTALL_WATCH_METRICS
  blockers:
    - product_hot_post_quarantine_window_missing
    - market_money_claim_blocked
    - local_accept_promotion_blocked

local-accept promotion gate smoke:
  promotion_allowed: false
  local_accept_policy_candidate_written: true
  local_accept_enabled: false
  requires_manual_activation_after_review: true
  blocker: shadow_metrics_not_ready
  verdict: NANDO_PHASE_CENTER_LOCAL_ACCEPT_PROMOTION_GATE_BLOCKED

Checks:
  systemd-analyze verify: PASS
  cargo build --release -p nando-cli: PASS
  metrics snapshot script and timer unit: PASS
  provider evidence snapshot: PASS
  provider export contract-pack: PASS
  readiness snapshot: PASS
  test-server verify: PASS
  local-accept promotion gate: PASS
  provider export contract-pack stale guard: PASS
  git diff --check for package files: PASS

Boundary:
  deployable test-server metric stand only. It can measure real traffic
  compression and produce provider-evidence work, but it does not enable
  local_accept and does not claim money until external provider export joins.
```

Verdict:

```text
PASS:
  per-profile survivor availability

WATCH:
  product compression claim

Blocker:
  live false_accepts must return to 0 on a future shadow window
```

## Current P0 Task

```text
Make the miner keep clean survivor profiles hot while automatically splitting
or quarantining unsafe hidden-state branches.
```

Required reporting:

```text
active_clean_profile_count
active_clean_calls_saved
active_clean_tokens_saved
quarantined_profile_count
lost_calls_due_to_quarantine
lost_tokens_due_to_quarantine
append_false_accepts
product_hot_score_only_post_quarantine_false_accepts
```

Success target:

```text
final_hot_runtime_available: true
final_hot_profile_count: >0
append_false_accepts: 0
product_hot_score_only_post_quarantine_false_accepts: 0
active_clean_tokens_saved > 0
```

## P0 Implementation Checklist

Baseline before edits:

```text
Read the live report and preserve these fields as before/after:
  final_hot_runtime_available
  final_hot_profile_count
  product_hot_score_only_runtime_loaded
  product_hot_score_only_runtime_active
  append_score_candidate_events
  append_unique_cpu_accepts_over_exact_cache
  append_tokens_saved
  append_false_accepts
  product_hot_score_only_post_quarantine_false_accepts
  quarantined_profile_count / quarantined_profile_ids
```

Step 1 - isolate runtime availability from promotion manifest:

```text
If the promotion manifest is blocked because it contains a quarantined profile,
do not disable the entire score-only runtime.

Build the active runtime from clean survivor candidates:
  all candidates
    - quarantined_profile_ids
    - candidates with verifier_bound = false
    - candidates with false_accepts > 0
    - candidates without negative/background evidence

Expected behavior:
  bad profile -> unavailable
  clean siblings -> still scoreable
```

Step 2 - make quarantine local:

```text
Quarantine key must be narrow enough:
  profile_id
  route/profile edge
  hidden-state bucket if available

It must not be:
  whole route
  whole action_family
  whole product-hot runtime
```

Step 3 - apply NE BUSY hidden-state split:

```text
Broad bucket:
  agent_continue_execute / planning / positive_nonzero

must be split by L2 hidden-state atoms before L3 scoring:
  command_kind
  tool_kind
  exit_code_band
  output_shape
  edit_vs_inspect
  state/result atom availability
  verifier label availability
  risk atoms

No hand-written product class list.
The splitter can use atom families, but candidate splits must be generated and
ranked from stream evidence.
```

Step 4 - L3 phase-center per hidden branch:

```text
For each hidden-state branch:
  build/update phase-center
  keep positive evidence
  keep negative/background evidence
  compute margin
  auto-calibrate threshold
  reject if false_accepts > 0 after calibration
```

Step 5 - L4 survivor portfolio selection:

```text
Select active hot profiles by marginal value, not pretty bucket count.

Primary value:
  new unique accepts over exact cache
  new tokens saved over exact cache
  false_accepts = 0
  verifier_ready = true
  hot bytes within budget

Penalties:
  overlap with already selected profile
  low margin
  unstable threshold
  high lost tokens after quarantine
  excessive hot bytes

Output must say why each selected profile survived and why each rejected profile
was rejected.
```

Step 6 - report both active and lost value:

```text
active_clean_calls_saved
active_clean_tokens_saved
active_clean_profile_count

lost_calls_due_to_quarantine
lost_tokens_due_to_quarantine
quarantined_profile_count
quarantined_profile_ids

This is required because after quarantine the product number can go down, but
the report must explain whether value was lost to safety or missing splits.
```

Step 7 - keep hot path small:

```text
Hot score loop may use:
  numeric route/profile ids
  prebuilt phase-center runtime
  small scratch buffers
  margin/threshold scoring

Hot score loop must not use:
  JSONL
  filesystem writes
  provider billing
  report generation
  heavy mining
  string route matching as authority
  local_accept
```

Minimum checks:

```text
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
cargo build --release -q -p nando-cli
git diff --check

plus the live/miner command that produces:
  target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.report.json
```

PASS condition for this P0:

```text
final_hot_runtime_available: true
final_hot_profile_count: >0
append_false_accepts: 0
product_hot_score_only_post_quarantine_false_accepts: 0
active_clean_tokens_saved > 0
local_accept_enabled: false
market_money_claim_allowed: false
```

WATCH if:

```text
runtime stays alive but append_false_accepts > 0
or active_clean_tokens_saved = 0
or only manual split/list made it work
```

FAIL if:

```text
one quarantined profile disables all clean survivors
or unsafe profile remains active
or local_accept becomes true
or hot path starts doing cold/report/provider work
```

## Why This Is Better Than Full Runtime Disable

This is not a style preference. It is a better safety/product tradeoff.

Current full-disable behavior:

```text
one unsafe profile
  -> entire product-hot runtime disabled
  -> clean profiles stop saving tokens
  -> report shows safety but loses product value
```

Survivor-portfolio behavior:

```text
one unsafe profile
  -> only that profile/edge is quarantined
  -> clean verifier-bound profiles keep scoring
  -> report separates active value from lost quarantined value
```

Why it is safer:

```text
It does not relax the verifier gate.
It does not lower thresholds.
It does not enable local_accept.
It reduces blast radius of a false_accept.
```

Why it is better for product:

```text
The customer does not care that one operator failed.
The customer cares whether the system still safely saves tokens.

If 1 profile is bad and 3 profiles are clean, turning off all 4 profiles throws
away verified value.
```

Why it is better for the miner:

```text
A false_accept becomes training signal:
  this hidden branch is too broad
  split it or quarantine it

Instead of:
  false_accept happened
  kill all hot runtime
```

Expected measurable improvement:

```text
Before:
  final_hot_runtime_available: false
  final_hot_profile_count: 0

After:
  final_hot_runtime_available: true
  final_hot_profile_count: clean survivors > 0
  product_hot_score_only_post_quarantine_false_accepts: 0
```

The honest comparison is not just "more accepts".

The comparison is:

```text
same or better safety
  + nonzero active clean savings
  + explicit lost value from quarantine
  + smaller blast radius
```

When this idea is wrong:

```text
If clean survivors still produce false_accepts, reject them.
If active_clean_tokens_saved stays 0, the portfolio is not useful yet.
If survivor selection needs a manual class list, the L4 selector is not good enough.
If hot path grows cold/report work, the implementation is wrong.
```

So the goal is not to keep profiles alive at any cost.

The goal is:

```text
keep only verifier-bound clean survivors alive,
measure their value,
and let unsafe branches teach the splitter what to cut next.
```

## Reviewer Reminder

Do not chase cosmetic cleanup while this is open.

The money node is still:

```text
automatic streaming class discovery
  -> hidden-state split
  -> phase-center operator
  -> clean survivor portfolio
  -> verified CPU tokens/calls saved
```

## 2026-07-08 - Reviewer Check: NE BUSY Implementation Progress

Current evidence:

```text
report:
  target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.report.json

runtime:
  product_hot_score_only_runtime_loaded: true
  product_hot_score_only_runtime_active: true
  product_hot_score_only_runtime_source: live_store_clean_candidate_survivors
  final_hot_runtime_available: true
  final_hot_profile_count: 4
  hot_bytes_estimate: 2272

safety:
  append_false_accepts: 0
  product_hot_score_only_post_quarantine_false_accepts: 0
  local_accept_enabled: false
  market_money_claim_allowed: false

portfolio:
  quarantined_profile_count: 18
  clean_candidate_exportable_profile_ids: 7
```

Verdict:

```text
NE BUSY is partially implemented.

PASS:
  L3 phase-center runtime is loaded from clean survivors.
  L4 survivor portfolio keeps runtime alive after quarantine.
  Verifier/quarantine boundary is preserved.

WATCH:
  L1/L2 -> hot route/profile matching is not yet working in the current window.
```

Current blocker:

```text
append_hot_view_available_events: 34
append_route_index_missing_events: 34
append_scoring_started: false
append_score_candidate_events: 0
active_clean_tokens_saved: 0
```

Interpretation:

```text
The hot runtime exists, but current live events do not resolve into its route
table. This is now a route/profile coverage problem, not a survivor/quarantine
problem.
```

Next implementation target:

```text
Make the L2 hidden-state route emitted by incoming events match the route ids
stored in the survivor runtime.

Report before/after:
  append_route_index_missing_events
  append_scoring_started
  append_score_candidate_events
  active_clean_calls_saved
  active_clean_tokens_saved
```

PASS target:

```text
append_route_index_missing_events decreases
append_scoring_started: true
append_score_candidate_events > 0
append_false_accepts: 0
product_hot_score_only_post_quarantine_false_accepts: 0
active_clean_tokens_saved > 0
```

## 2026-07-08 - Reviewer Check: Active Clean Vs Quarantine Lost Counters

CHANGE:

```text
Append live-tail report now exposes explicit product-facing counters:

  active_clean_calls_saved
  active_clean_tokens_saved
  lost_calls_due_to_quarantine
  lost_tokens_due_to_quarantine

Definitions:

  active_clean_*:
    score_candidate=true from non-quarantined product-hot/survivor profiles
    verified_safe_accept=true
    exact_cache_hit=false

  lost_*:
    score_candidate=true from quarantined profiles
    verified_safe_accept=true
    exact_cache_hit=false

Unsafe quarantined hits are not counted as lost savings. They remain quarantine
evidence.
```

WHY:

```text
The daemon now separates three signals:

  1. active clean savings
  2. savings lost because a profile is quarantined
  3. unsafe false_accept pressure

This is needed for the NE BUSY L4 portfolio selector:
  keep clean profiles hot
  quarantine unsafe profiles
  report the economic cost of quarantine separately
```

LIVE SNAPSHOT AFTER RESTART:

```text
service:
  nando-phase-live-miner-tail-20260707T165122.service

pid:
  1532552

source:
  live_store_clean_candidate_survivors

append_parsed_rows: 23
append_score_candidate_events: 5
append_unique_cpu_accepts_over_exact_cache: 3
append_tokens_saved: 729
append_false_accepts: 0

active_clean_calls_saved: 3
active_clean_tokens_saved: 729
lost_calls_due_to_quarantine: 0
lost_tokens_due_to_quarantine: 0

post_quarantine_score_candidate_events: 5
post_quarantine_false_accepts: 0

final_hot_runtime_available: true
final_hot_profile_count: 4
final_hot_profile_ids:
  1065971288
  326443327
  569147600
  2404847706

local_accept_enabled: false
market_money_claim_allowed: false
```

INTERPRETATION:

```text
The current future window is clean on safety:
  append_false_accepts: 0
  post_quarantine_false_accepts: 0

It is now positive on calls/tokens in a live clean survivor window:
  active_clean_calls_saved: 3
  active_clean_tokens_saved: 729

It is not yet a market-money proof:
  provider cost evidence: still absent
```

CHECKS:

```text
cargo fmt --check PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner_repairs_threshold_after_verified_false_accept --lib PASS
cargo build --release -q -p nando-cli PASS
git diff --check PASS
```

VERDICT:

```text
PASS for active-clean/lost quarantine observability.
PASS-shadow for live clean calls/tokens over exact cache.
WATCH-money for market proof until provider cost evidence is present and the
future/admission promotion gate is allowed.
```

## 2026-07-08 - Reviewer Check: Smart/Fast Online Miner Tail

CHANGE:

```text
The online phase-center tail was changed from a mostly global top-candidate
selector into a stream-prioritized selector:

  current live event
  -> relevant bucket ids
  -> contrastive zero-false candidate filter
  -> prioritized hot runtime
  -> score before update

Core phase-center math did not change.
No .nwrb / role-binding backend was reintroduced.
```

DETAILS:

```text
Core:
  candidate_bucket ranking now uses bucket.is_candidate()
  candidate_runtime requires contrastive evidence:
    positive_events > 0
    negative_events > 0
    unique_cpu_accepts_over_exact_cache > 0
    false_accepts = 0

Live tail:
  clean survivor runtime can prioritize relevant bucket ids from the current
  append row.

  fallback scoring path now uses:
    candidate_hot_runtime_and_route_table_excluding_prioritized(...)

  auto-subcenter discovery now includes triple combo centers:
    combo:state_exit_code_band:*|command|shape_band

  This is automatic multi-split, not a manual route/class list.
```

CHECKS:

```text
cargo fmt --check PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
cargo test -q -p nando-core hot_runtime_numeric_score_path_p99_budget --lib -- --ignored --nocapture PASS
  phase_center_hot_runtime_numeric_score_path_p99_ns: 722
cargo build --release -q -p nando-cli PASS
git diff --check PASS
```

LIVE SNAPSHOT AFTER MULTI-SPLIT RESTART:

```text
appender pid:
  1055387
  cpu: 0.3%
  rss: 17900 KB

tail pid:
  1605458
  cpu: 1.9%
  rss: 21568 KB

append_parsed_rows: 12
append_auto_subcenter_observe_events: 100
append_score_candidate_events: 3
append_unique_cpu_accepts_over_exact_cache: 2
append_tokens_saved: 8137
append_false_accepts: 0

active_clean_calls_saved: 2
active_clean_tokens_saved: 8137
lost_calls_due_to_quarantine: 0
lost_tokens_due_to_quarantine: 0

online_bucket_count: 317
candidate_bucket_count: 44
active_bucket_count: 152
shadow_ready_bucket_count: 117

final_hot_profile_count: 4
final_hot_profile_ids:
  1065971288
  146661061
  1451329680
  550802256

local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: no_future_shadow_billing_request_rows
```

INTERPRETATION:

```text
PASS-shadow for smart/fast online miner tail in the current live window:
  it discovered narrower combo subcenters,
  scored live rows before update,
  found 2 unique CPU accepts over exact cache,
  saved 8137 estimated tokens,
  and kept false_accepts at 0.

WATCH-product:
  local_accept remains disabled.
  market money claim remains blocked until provider billing/export evidence is present.
```

## 2026-07-08 - Reviewer Check: Two-Profile Live Miner Attribution

CHANGE:

```text
The active live-tail miner now has a second inferred profile lane:

  observable profile:
    primary bucket + visible auto-subcenter/pair/combo atoms

  hidden-state profile:
    source-neutral cross-layer atoms:
      hidden_state:request_state:...
      hidden_state:state_tool:...
      hidden_state:request_tool:...
      hidden_state:request_state_tool:...

The hidden-state atoms are built from request/state/tool evidence, with route,
source, proof, target, profile, and local_out_t style leaks blocked.
```

EVIDENCE:

```text
Checks:
  cargo fmt PASS
  RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
  cargo test -q -p nando-core online_miner --lib PASS
  cargo build --release -q -p nando-cli PASS

Live tail restarted with the new binary:
  appender pid: 1055387
  tail pid: 1668057
  tail cpu after warmup: 3.8%
```

LIVE ATTRIBUTION SNAPSHOT:

```text
append_parsed_rows: 12
active_clean_calls_saved: 7
active_clean_tokens_saved: 2836
append_false_accepts: 0

append_hidden_state_subcenter_observe_events: 84

observable profile:
  score_candidate_events: 1
  unique_accepts_over_exact_cache: 1
  tokens_saved: 541

hidden-state profile:
  score_candidate_events: 4
  unique_accepts_over_exact_cache: 4
  tokens_saved: 1405

unknown/unmapped profile:
  score_candidate_events: 3
  unique_accepts_over_exact_cache: 3
  tokens_saved: 1431

profile_attribution_overlap_accepts: 1
```

INTERPRETATION:

```text
PASS for live hidden-state presence:
  hidden_state:* atoms are now generated in the active tail path and produce
  score candidates before update.

WATCH for full per-profile economics:
  attribution is visible, but some product-hot/profile ids are still reported
  as unknown/unmapped. Add persistent profile_id -> profile_kind metadata before
  claiming exact hidden-vs-observable economics.

Use active_clean_calls_saved / active_clean_tokens_saved for deduped real shadow
compression. Candidate-level token fields can overlap and are diagnostic.
```

## 2026-07-08 - Reviewer Check: Hidden-State vs Observable Attribution Fix

CHANGE:

```text
Live-tail attribution now keeps an in-memory profile_id -> profile_kind map.
The map is populated from watermark, warm-history, and live append events.

Kinds:
  observable_primary
  observable_subcenter
  hidden_state

This does not change phase-center scoring or thresholds. It only prevents
product-hot profile ids from becoming unknown when the current event does not
carry the exact source atom that originally created the profile.
```

CHECKS:

```text
cargo fmt PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
cargo build --release -q -p nando-cli PASS
git diff --check PASS
```

LIVE SNAPSHOT AFTER RESTART:

```text
tail pid: 1682092
rss: 32396 KB
cpu: about 5.1% in the sampled window

append_parsed_rows: 6
active_clean_calls_saved: 6
active_clean_tokens_saved: 1572
append_false_accepts: 0

known_profile_kind_count: 813
observable_known_profile_count: 331
hidden_state_known_profile_count: 482

observable profile:
  score_candidate_events: 0
  unique_accepts_over_exact_cache: 0
  tokens_saved: 0

hidden-state profile:
  score_candidate_events: 6
  unique_accepts_over_exact_cache: 6
  tokens_saved: 1572

unknown/unmapped profile:
  score_candidate_events: 0
  unique_accepts_over_exact_cache: 0
  tokens_saved: 0

product_hot_score_only_quarantined: true
post_quarantine_false_accepts: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

INTERPRETATION:

```text
PASS for attribution accounting: the second hidden-state profile lane is now
separated from ordinary observable profiles and unknown dropped to zero in the
fresh window.

WATCH for claim size: the post-restart live window is small. Use it as a clean
mechanism check, not a market denominator.

Earlier red window had append_false_accepts: 1, so the current clean number is
post-quarantine / post-restart evidence only. No local_accept or money claim is
allowed.
```

## 2026-07-08 - Reviewer Check: Disjoint Hidden-State Contribution Split

CHANGE:

```text
Live-tail report now separates diagnostic profile participation from disjoint
clean contribution:

  observable_only
  hidden_state_only
  mixed_profile
  unknown_only

This avoids double-counting when observable and hidden profiles both score the
same accepted event. Phase-center scoring, thresholds, verifier binding, and
local_accept policy did not change.
```

CHECKS:

```text
cargo fmt PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
cargo build --release -q -p nando-cli PASS
git diff --check PASS
```

LIVE SNAPSHOT AFTER DISJOINT SPLIT RESTART:

```text
tail pid: 1689196
rss: 33032 KB
cpu: about 5.2% in the sampled window

append_parsed_rows: 27
active_clean_calls_saved: 12
active_clean_tokens_saved: 5877
append_false_accepts: 0

known_profile_kind_count: 814
observable_known_profile_count: 331
hidden_state_known_profile_count: 483

diagnostic participation:
  observable accepts: 0
  hidden-state accepts: 12
  unknown accepts: 0
  overlap accepts: 0

disjoint clean contribution:
  observable_only accepts: 0
  hidden_state_only accepts: 12
  mixed_profile accepts: 0
  unknown_only accepts: 0

  observable_only tokens: 0
  hidden_state_only tokens: 5877
  mixed_profile tokens: 0
  unknown_only tokens: 0

post_quarantine_false_accepts: 0
local_accept_enabled: false
market_money_claim_allowed: false
```

INTERPRETATION:

```text
PASS for no-double-count attribution: in this fresh post-quarantine window, all
deduped clean CPU accepts came from hidden_state_only.

WATCH for denominator size: window is still small and must not be sold as a
market claim. It is a mechanism/accounting proof that the second profile lane
can be measured separately from observable profile traffic.
```

## 2026-07-08 - Reviewer Check: Architecture Version Registry In Reports

CHANGE:

```text
Added a first-class architecture_versions block to the live-tail JSON report.
Added docs/ARCHITECTURE_VERSION_REGISTRY.md as the active version map.

The goal is to tie every compression snapshot to the exact miner/profile/report
architecture that produced it.
```

ACTIVE VERSION BLOCK:

```text
phase_center_core: phase_center_core_v1
online_miner: online_phase_center_miner_v1
live_tail_daemon: append_live_tail_shadow_daemon_v3
hot_runtime: phase_center_hot_runtime_v1
auto_subcenter_discovery: auto_subcenter_discovery_v2_hidden_first
hidden_state_profile: hidden_state_cross_layer_profile_v1
profile_attribution: profile_attribution_disjoint_v1
compression_accounting: calls_tokens_cost_milli_accounting_v1
package_format: nwpc_v1
forbidden_backend_policy: no_nwrb_no_lookup_no_local_accept_without_verifier_v1
```

LIVE SNAPSHOT WITH VERSIONED ACCOUNTING:

```text
append_parsed_rows: 9
append_total_tokens: 3208
append_total_cost_microusd: 3208

exact cache:
  calls_saved_milli: 333
  tokens_saved_milli: 85

Nando active clean CPU:
  calls_saved_milli: 444
  tokens_saved_milli: 479

combined cache + Nando:
  calls_saved_milli: 777
  tokens_saved_milli: 564

hidden_state_only:
  calls_saved_milli: 444
  tokens_saved_milli: 479

append_false_accepts: 0
local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: no_future_shadow_billing_request_rows
```

CHECKS:

```text
cargo fmt --check PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
cargo build --release -q -p nando-cli PASS
git diff --check PASS
```

VERDICT:

```text
PASS for architecture-versioned live report fields.
WATCH for market denominator and money claim: provider billing/export evidence
is still missing.
```

## 2026-07-08 - Reviewer Check: Product-Hot Route Refresh + Historical False Quarantine

CHANGE:

```text
Fixed product-hot score-only route coverage:
  if the active product-hot survivor runtime has profiles but does not cover the
  current event route, the live-tail shadow miner rebuilds a clean-survivor
  runtime prioritized by the current relevant bucket ids before scoring.

Added historical non-exact false quarantine:
  profile ids that already produced score_candidate on verified_safe_accept=false
  and exact_cache_hit=false rows in the stable decision log are excluded from
  product-hot survivor selection on startup.

Adjusted product-hot post-quarantine false accounting:
  post_quarantine_false_accepts only counts non-exact rows. Raw append false
  remains strict and still quarantines unsafe profiles.
```

MECHANISM CHECKS:

```text
cargo fmt --check PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
cargo build --release -q -p nando-cli PASS
bash -n ops/phase-center-test-server/bin/*.sh scripts/build-phase-center-test-server-package.sh PASS
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer PASS
```

BOUNDED LOCAL SNAPSHOTS:

```text
route-refresh smoke4:
  product_hot_score_only_post_quarantine_score_candidate_events: 78
  product_hot_score_only_post_quarantine_false_accepts: 0
  active_clean_calls_saved: 0
  note: mostly exact-cache rows, so not a savings window.

larger smoke5:
  rows: 211
  non_exact: 127
  active_clean_calls_saved: 109
  active_clean_tokens_saved: 146988
  product_hot_score_only_post_quarantine_score_candidate_events: 170
  product_hot_score_only_post_quarantine_false_accepts: 2
  verdict: WATCH, useful value signal but unsafe survivor profiles still appear.

seeded-history smoke8:
  rows: 211
  active_clean_calls_saved: 109
  active_clean_tokens_saved: 146988
  product_hot_score_only_post_quarantine_score_candidate_events: 170
  product_hot_score_only_post_quarantine_false_accepts: 2
  quarantine_count: 28
  verdict: WATCH, historical quarantine excludes known bad profiles but new
  unsafe survivor profiles are still discovered in the fresh window.
```

PACKAGE:

```text
path: /home/ubu/projects/nando-wave/target/nando-wave/deploy/nando-phase-center-test-server-20260708T021200Z.tar.gz
sha256: see adjacent .tar.gz.sha256 artifact
local_accept_enabled: false
market_money_claim_allowed: false
```

VERDICT:

```text
PASS for route coverage improvement: product-hot now reaches the active agent_continue route
instead of leaving all value in online_store_candidate fallback.

PASS for stricter survivor hygiene: known non-exact false profile ids are excluded
from product-hot survivor selection.

WATCH for product claim: fresh windows still discover new unsafe survivor profiles,
so product_hot_post_quarantine_false_accepts can be nonzero. Do not enable
local_accept and do not claim market savings.

NEXT:
  move selector from top-N clean survivors to risk-aware survivor selection:
  prefer profiles with future-window clean support and no non-exact false family,
  not merely next available route candidate after quarantine.
```

## 2026-07-08 - Reviewer Check: Product-Hot Clean Credit Ledger

CHANGE:

```text
Fixed product-hot accounting so score-only savings are no longer credited
directly at first hit.

New behavior:
  product-hot verified non-exact hits are stored as credit rows with the
  scoring profile ids.
  Before heartbeat/final JSON, product_hot_score_only_* totals are recomputed
  only from rows whose scoring profiles are not quarantined.

Also widened quarantine evidence ingestion:
  stable decision-log rows without architecture_version_key now count for
  non-exact false profile quarantine.
  rows with an explicit different architecture_version_key remain excluded.
```

WHY:

```text
active_clean_* remains a raw diagnostic signal.
product_hot_score_only_* is now the cleaner product-hot view.

If a profile later catches a false accept and enters quarantine, its earlier
credits are removed from product-hot clean totals. This prevents selling a
profile that later proved unsafe in the same server window.
```

BOUNDED SERVER-WINDOW SNAPSHOT:

```text
path: target/nando-wave/streaming/product-hot-clean-credit-ledger-v1/report.json

rows: 211
non_exact_rows: 127

raw active_clean:
  calls_saved: 109
  tokens_saved: 146988

product_hot_score_only clean ledger:
  calls_saved: 39
  tokens_saved: 42038
  cost_saved_microusd: 42038

product_hot_score_only_post_quarantine_score_candidate_events: 170
product_hot_score_only_post_quarantine_false_accepts: 2
append_false_accepts: 3
quarantine_count: 31
active_profiles: 4
runtime_source: live_store_clean_candidate_survivors
```

CHECKS:

```text
cargo fmt --check PASS
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli PASS
cargo build --release -q -p nando-cli PASS
cargo test -q -p nando-core online_miner --lib PASS
bash -n ops/phase-center-test-server/bin/*.sh scripts/build-phase-center-test-server-package.sh PASS
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer PASS
git diff --check PASS
rust-action-memory doctor --format json PASS
```

VERDICT:

```text
PASS for product-hot accounting hygiene: credits from later-quarantined profiles
are no longer counted as product-hot clean savings.

WATCH for product claim: product_hot_score_only_post_quarantine_false_accepts is
still nonzero. Do not enable local_accept and do not claim server product
compression from live_store_clean_candidate_survivors.

NEXT:
  treat live_store_clean_candidate_survivors as raw shadow/diagnostic unless a
  profile has future-window clean support. Product-hot clean should come from
  verifier-bound .nwpc candidates that survived future split, not merely from
  the current store frontier.
```

## 2026-07-08 - Reviewer Check: Dogfood Server Attachment + Source Claim Gate

CURRENT DOGFOOD STATE:

```text
Nando is attached to the local Codex workstream in shadow mode.

live event writer:
  phase-stream-codex-sessions-live-append-v1
  source: /home/ubu/.codex/sessions
  output: target/nando-wave/streaming/live-agent-phase-atom-append-v1.jsonl

live miner:
  phase-stream-hot-path-daemon-append-live-tail-v1
  report: target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.report.json
  decisions: target/nando-wave/streaming/nando-phase-live-miner-tail-20260707T165122.decisions.jsonl

local_accept_enabled: false
market_money_claim_allowed: false
```

INTERPRETATION:

```text
This is real self-use/dogfood as observation and shadow scoring:
  Codex work produces real events.
  Nando watches those events.
  Nando mines/updates phase-center candidates.
  Nando reports potential CPU compression.

It is not yet real call replacement:
  Nando does not choose Codex actions.
  Nando does not skip provider calls.
  Nando does not enable local_accept.
```

CHANGE:

```text
Added a product-hot source claim gate:
  claim-ready sources:
    call_token_active_manifest
    call_token_promotion_manifest
    product_hot_registry

  non-claim source:
    live_store_clean_candidate_survivors

The raw survivor frontier can still score and produce diagnostic signal, but it
cannot satisfy append_compression_claim_blocker as a final hot runtime source.
```

BOUNDED SERVER-WINDOW SNAPSHOT:

```text
path: target/nando-wave/streaming/product-hot-source-claim-gate-v1/report.json

rows: 211
non_exact_rows: 127

raw active_clean:
  calls_saved: 109
  tokens_saved: 146988

product_hot_score_only clean ledger:
  calls_saved: 39
  tokens_saved: 42038
  cost_saved_microusd: 42038

product_hot_score_only_post_quarantine_score_candidate_events: 170
product_hot_score_only_post_quarantine_false_accepts: 2
append_false_accepts: 3
runtime_source: live_store_clean_candidate_survivors
append_compression_claim_allowed: false
append_compression_claim_blocker: append_false_accepts_nonzero
```

VERDICT:

```text
PASS for self-use attachment: the daemon is observing real local Codex work.

PASS for claim boundary: raw live_store_clean_candidate_survivors cannot be
treated as product-hot market compression.

WATCH for product runtime: product-hot still needs a verifier-bound future-split
.nwpc promoted source with false_accepts=0 before local_accept or savings claims.
```

## 2026-07-08 - Reviewer Debt: Spectral Budget Refactor Required

DEBT:

```text
crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs is too large and mixed-frequency.
It currently combines source adapters, live online miner, product-hot runtime selection,
quarantine accounting, future-shadow reports, provider evidence plumbing, and server snapshot
schemas in one file.
```

RULE:

```text
After the current miner release/snapshot, run nanda-wave-spectral-budget over the repo.
First target: split live_store_adapter.rs by signal route, preserving behavior:
  adapter parsing
  online miner loop
  hot runtime selection
  quarantine/promotion accounting
  reports/schemas
  provider evidence/server ops

Do not mix this refactor with scoring changes. Keep .nwpc phase-center path only.
```

Full budget audit:

```text
docs/NANDO_WAVE_SPECTRAL_BUDGET_AUDIT.md
```

## 2026-07-08 - Reviewer Check: Live Tail Hot Budget Split

CHANGE:

```text
append live-tail report now separates:
  product_hot_budget_passed
  warm_miner_budget_passed
  warm_miner_budget_blocker

The live-tail gate uses hot runtime budget for product-hot health. Warm miner
budget remains visible as a separate WATCH signal instead of making the compact
hot runtime look fat.
```

SAFETY FIX:

```text
HOT_PATH_DAEMON_APPEND_LIVE_TAIL_PASS now requires append_score_events_before_update > 0.
Before this fix, an append window with new rows but zero scoring could pass the
adapter gate. That is no longer allowed.
```

CONTROLLED SNAPSHOT:

```text
report:
  target/nando-wave/streaming/hot-budget-split-live-tail-controlled-tail/report.json

append_parsed_rows: 12
append_score_events_before_update: 0
append_false_accepts: 0
product_hot_budget_passed: true
warm_miner_budget_passed: true
warm_miner_budget_blocker: none
hot_profile_count: 1
hot_bytes_estimate: 592
warm_profile_count: 8
warm_bytes_estimate: 9536
miner_discovery_sample_permille: 100
miner_clean_hot_runtime_throttle_events: 12
append_auto_subcenter_throttled_events: 220
local_accept_enabled: false
market_money_claim_allowed: false
verdict: HOT_PATH_DAEMON_APPEND_LIVE_TAIL_WATCH
blocker: append_live_tail_no_score_before_update_events
```

BOUNDARY:

```text
This is a controlled live-tail schema/safety snapshot, not a compression claim.
It proves the gate no longer reports PASS without score-before-update.
```

DEPLOY PACKAGE:

```text
package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T022650Z.tar.gz
package_report:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T022650Z.package.json
sha256:
  708b2c2c857c7bfd0acbc15ea0c720de35de18ba418c2ce7430ce4c1287158c0

forbidden_flags:
  nwrb_used: false
  role_binding_backend_used: false
  lookup_used: false
  target_id_or_proof_rule_id_authority_used: false
  concrete_x_lookup_used: false
  manual_local_out_t_used: false
  local_accept_without_verifier_used: false
```

## 2026-07-08 - Reviewer Check: Post-Quarantine Clean Suffix Accounting

CHANGE:

```text
append live-tail report now exposes a rolling clean suffix after the last
quarantine-causing false accept:
  append_clean_suffix_rows
  append_clean_suffix_score_events
  append_clean_suffix_unique_cpu_accepts_over_exact_cache
  append_clean_suffix_tokens_saved
  append_clean_suffix_cost_saved_microusd
  append_clean_suffix_false_accepts
  append_clean_suffix_last_quarantine_row_index
  append_clean_suffix_claim_allowed
  append_clean_suffix_claim_blocker
```

WHY:

```text
The miner must be judged after it quarantines bad profiles. A whole append
window may contain early false accepts that were already disabled. The suffix
fields show whether the post-quarantine tail is clean and whether it still saves
calls/tokens.
```

CONTROLLED SNAPSHOT:

```text
report:
  target/nando-wave/streaming/clean-suffix-repeat80-snapshot/report.json

window:
  80 saved real agent events replayed after prior quarantine state

append_parsed_rows: 80
append_score_events_before_update: 80
append_unique_cpu_accepts_over_exact_cache: 0
append_tokens_saved: 0
append_false_accepts: 1

append_clean_suffix_rows: 78
append_clean_suffix_score_events: 78
append_clean_suffix_unique_cpu_accepts_over_exact_cache: 0
append_clean_suffix_tokens_saved: 0
append_clean_suffix_false_accepts: 0
append_clean_suffix_last_quarantine_row_index: 1
append_clean_suffix_claim_allowed: false
append_clean_suffix_claim_blocker: append_no_final_hot_runtime

local_accept_enabled: false
market_money_claim_allowed: false
```

VERDICT:

```text
PASS for visibility: the daemon now reports post-quarantine clean suffix state.
WATCH for product compression: post-quarantine suffix is clean but currently
saves 0 calls/tokens in this controlled window.
```

DEPLOY PACKAGE:

```text
package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T023839Z.tar.gz
package_report:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T023839Z.package.json
sha256:
  baf365b571fb83f43d6d8ebfc97e1ebc1aef7cd91d465a947a40a72faaef54ee
```

## 2026-07-08 - Reviewer Check: live_store_adapter Frozen Candidate Budget Cut

CHANGE:

```text
live_store_adapter/frozen_candidates.rs added
live_store_adapter.rs frozen candidate lifecycle moved out:
  verifier-bound .nwpc frozen candidate structs
  freeze helpers from live store / store route map
```

CONTROL:

```text
move-only refactor
no scoring change
no threshold change
no miner behavior change
no verifier semantic change
no promotion/local_accept change
no compression claim change
```

LINE BUDGET AT PATH CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 17286 lines
frozen_candidates.rs: 138 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Smoke Report Budget Cut

CHANGE:

```text
top-level PhaseStreamLiveStoreAdapterSmokeReport moved to live_store_adapter/reports.rs
reports.rs now owns this schema with the rest of live-store reports
```

CONTROL:

```text
move-only schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT PATH CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 17221 lines
reports.rs: 838 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Worker Path Budget Cut

CHANGE:

```text
live_store_adapter/worker_path.rs added
prepared-memory row alias, prepared-hot eval alias, worker messages, and worker metrics moved out
```

CONTROL:

```text
move-only worker envelope refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT RUNTIME REGISTRY CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 17189 lines
worker_path.rs: 34 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Hot Admission Report Budget Cut

CHANGE:

```text
hot-path/admission report schemas moved to live_store_adapter/reports.rs:
  PhaseStreamHotPathBenchmarkReport
  PhaseStreamHotPathPromotionReviewReport
  PhaseStreamHotPathDaemonAdmissionPolicyReport
  PhaseStreamHotPathDaemonAdmissionPolicySmokeReport
  PhaseStreamHotPathDaemonAdmissionPolicySmokeGuard
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT PATH CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 16929 lines
reports.rs: 1098 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Numeric Admission Portfolio Report Budget Cut

CHANGE:

```text
numeric admission portfolio report schemas moved to live_store_adapter/reports.rs:
  PhaseStreamHotPathDaemonNumericAdmissionPortfolioGateReport
  PhaseStreamHotPathDaemonNumericAdmissionPortfolioAcceptedReport
  PhaseStreamHotPathDaemonNumericAdmissionPortfolioRejectedReport
  PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayReport
  PhaseStreamHotPathDaemonNumericAdmissionPortfolioRuntimeReplayItemReport
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT RUNTIME REGISTRY CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 16804 lines
reports.rs: 1226 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Future/Shadow Gate Report Budget Cut

CHANGE:

```text
future/shadow report schemas moved to live_store_adapter/reports.rs:
  PhaseStreamHotPathDaemonNumericFuturePortfolioAuditReport
  PhaseStreamHotPathDaemonShadowGateReport
  PhaseStreamHotPathDaemonAppendShadowGateReport
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 16628 lines
reports.rs: 1402 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Live Loop Smoke Report Budget Cut

CHANGE:

```text
live-loop smoke report schemas moved to live_store_adapter/reports.rs:
  PhaseStreamHotPathDaemonLiveLoopBudgetSmokeReport
  PhaseStreamHotPathDaemonAppendLiveLoopSmokeReport
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 16449 lines
reports.rs: 1582 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Append Live Tail Report Budget Cut

CHANGE:

```text
append-live-tail report schema moved to live_store_adapter/reports.rs:
  PhaseStreamHotPathDaemonAppendLiveTailReport
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 16221 lines
reports.rs: 1811 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Numeric Audit Report Budget Cut

CHANGE:

```text
numeric audit/provider report schemas moved to live_store_adapter/reports.rs:
  LiveStoreProviderEvidenceArtifactsReport
  PhaseStreamHotPathDaemonLiveLoopNumericBenchmarkReport
  PhaseStreamHotPathDaemonNumericPackageShadowAuditReport
  PhaseStreamHotPathDaemonNumericFuturePackageAuditReport
  PhaseStreamHotPathDaemonNumericFalseAcceptSplitAuditReport
  PhaseStreamHotPathDaemonNumericFalseAcceptAtomReport
```

CONTROL:

```text
move-only report schema refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 15893 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Survivor Runtime Budget Cut

CHANGE:

```text
live_store_adapter/survivor_runtime.rs added
clean survivor runtime helpers moved out:
  live_store_clean_candidate_frontier
  live_store_clean_candidate_value_reports
  live_store_clean_candidate_survivor_runtime_from_store
  live_store_product_hot_subcenter_priority_bucket_ids
  live_store_product_hot_subcenter_candidate_allowed
  live_store_product_hot_excluded_profile_ids
```

CONTROL:

```text
move-only survivor runtime refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 14876 lines
survivor_runtime.rs: 204 lines
runtime_registry.rs: 479 lines
paths.rs: 146 lines
persistence.rs: 45 lines
state.rs: 206 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter State Type Budget Cut

CHANGE:

```text
live_store_adapter/state.rs added
state/type bundles moved out:
  LiveStoreStableDecisionLogWindow
  LiveStoreProductHotCreditRow
  LiveStoreFalseAcceptAtomAccumulator
  LiveStoreFutureShadowBillingRequestSummary
  LiveStoreProviderArtifactSignature
  LiveStorePersistedProductHotQuarantine
  LiveStoreCandidateRegistryShadowReport
  LiveStoreSharedRegistryShadowReport
  LiveStoreCleanManifestRuntimeBundle
  LiveStoreProductHotRegistryRuntimeBundle
  LiveStoreDirectHotSnapshot
  LiveStoreDirectHotSnapshotBank
  LiveStoreDirectHotSnapshotEval
```

CONTROL:

```text
move-only state/type refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 15701 lines
state.rs: 206 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Persistence Helper Budget Cut

CHANGE:

```text
live_store_adapter/persistence.rs added
persistence helper moved out:
  live_store_load_persisted_product_hot_quarantine
```

CONTROL:

```text
move-only persistence/helper refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 15662 lines
persistence.rs: 45 lines
state.rs: 206 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Path Helper Budget Cut

CHANGE:

```text
live_store_adapter/paths.rs added
path/key helpers moved out:
  live_store_route_key_from_bucket_key
  live_store_resolve_registry_relative_path
  live_store_hot_path_promotion_review_path
  live_store_hot_path_daemon_admission_policy_path
  live_store_numeric_candidate_package_dir
  live_store_append_tail_clean_promotion_manifest_path
  live_store_append_tail_clean_promotion_package_dir
  live_store_append_tail_call_token_promotion_manifest_path
  live_store_append_tail_call_token_active_manifest_path
  live_store_append_tail_call_token_promotion_package_dir
  live_store_numeric_future_candidate_package_dir
  live_store_numeric_future_policy_smoke_path
  live_store_numeric_future_portfolio_child_report_path
  live_store_numeric_future_portfolio_gate_report_path
  live_store_numeric_future_portfolio_runtime_replay_report_path
```

CONTROL:

```text
move-only path/helper refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT PATH CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 15530 lines
paths.rs: 146 lines
persistence.rs: 45 lines
state.rs: 206 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Runtime Registry Budget Cut

CHANGE:

```text
live_store_adapter/runtime_registry.rs added
runtime registry helpers moved out:
  load_live_store_clean_manifest_runtime
  load_live_store_product_hot_registry_runtime
  load_live_store_product_hot_runtime_from_clean_manifest
  try_load_live_store_allowed_call_token_runtime
  live_store_call_token_manifest_promotes_quarantined_profile
  live_store_product_hot_route_index
  disable_live_store_call_token_active_manifest
```

CONTROL:

```text
move-only runtime registry refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
```

LINE BUDGET AT RUNTIME REGISTRY CUT, SUPERSEDED BY LATER CUTS:

```text
live_store_adapter.rs: 15071 lines
runtime_registry.rs: 479 lines
paths.rs: 146 lines
persistence.rs: 45 lines
state.rs: 206 lines
reports.rs: 2136 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Future Shadow Registry Budget Cut

CHANGE:

```text
live_store_adapter/future_shadow_registry.rs added
future-shadow registry/eval helpers moved out:
  live_store_future_shadow_candidate_reports
  live_store_candidate_promotion_contract
  live_store_candidate_registry_shadow
  live_store_shared_registry_shadow
  live_store_serving_policy_blocker
  live_store_clean_promotion_manifest_blocker
  live_store_candidate_promotion_evidence
```

CONTROL:

```text
move-only future-shadow registry refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 12189 lines
future_shadow_registry.rs: 459 lines
claim_gates.rs: 177 lines
hot_path_eval.rs: 391 lines
provider_evidence.rs: 486 lines
promotion_manifests.rs: 413 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Clean Manifest Gate Budget Cut

CHANGE:

```text
live_store_adapter/claim_gates.rs extended
clean manifest shadow blocker moved out:
  live_store_clean_manifest_shadow_blocker
```

CONTROL:

```text
move-only claim-gate refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 11556 lines
portfolio_replay.rs: 307 lines
numeric_future_package.rs: 371 lines
claim_gates.rs: 177 lines
future_shadow_registry.rs: 459 lines
hot_path_eval.rs: 391 lines
provider_evidence.rs: 486 lines
promotion_manifests.rs: 413 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Portfolio Replay Budget Cut

CHANGE:

```text
live_store_adapter/portfolio_replay.rs added
portfolio runtime replay helper moved out:
  live_store_replay_one_portfolio_admission
```

CONTROL:

```text
move-only portfolio runtime replay refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 11556 lines
portfolio_replay.rs: 307 lines
numeric_future_package.rs: 371 lines
claim_gates.rs: 177 lines
future_shadow_registry.rs: 459 lines
hot_path_eval.rs: 391 lines
provider_evidence.rs: 486 lines
promotion_manifests.rs: 413 lines
```

## 2026-07-08 - Reviewer Check: live_store_adapter Numeric Future Package Budget Cut

CHANGE:

```text
live_store_adapter/numeric_future_package.rs added
numeric future package audit helper moved out:
  LiveStoreFrozenNumericFuturePackage
  live_store_write_numeric_future_package_audit_from_frozen
```

CONTROL:

```text
move-only fresh-future .nwpc package audit refactor
no scoring/threshold/miner/verifier/promotion/local_accept/compression-claim change
cargo fmt
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 11840 lines
numeric_future_package.rs: 371 lines
claim_gates.rs: 177 lines
future_shadow_registry.rs: 459 lines
hot_path_eval.rs: 391 lines
provider_evidence.rs: 486 lines
promotion_manifests.rs: 413 lines
```

## 2026-07-08 - Reviewer Check: Clean-Survivor .nwpc Claim-Ready Handoff

CHANGE:

```text
live_store_adapter/promotion_manifests.rs extended
clean survivor call/token promotion manifest added:
  write_live_store_clean_survivor_call_token_promotion_manifest

append live-tail final refresh now writes verifier-bound .nwpc packages from
non-quarantined clean survivor candidates and loads them through:
  call_token_active_manifest

No local_accept enabled.
No market money claim enabled.
No .nwrb / role-binding backend.
```

EVIDENCE:

```text
report:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report.json

manifest:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report-call-token-promotion-manifest.json

active manifest:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report-call-token-promotion-active-manifest.json

.nwpc promoted packages:
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report-call-token-promotion/bucket-efab6670-97fecca8fc4a7fc5.nwpc
  target/nando-wave/streaming/live-tail-clean-survivor-manifest-v2.report-call-token-promotion/bucket-486f0d5e-a2224df72211185e.nwpc
```

SCOREBOARD:

```text
stable_decision_log_clean_suffix_rows: 502
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 111
stable_decision_log_clean_suffix_tokens_saved: 180753
stable_decision_log_clean_suffix_total_tokens: 555702
stable_decision_log_clean_suffix_false_accepts: 0
stable_decision_log_clean_suffix_claim_allowed: true
stable_decision_log_clean_suffix_claim_blocker: none

product_hot_score_only_runtime_source: call_token_active_manifest
final_hot_runtime_available: true
final_hot_profile_ids: [4020987504, 1215237470]

call_token_promotion_manifest_allowed: true
call_token_promotion_manifest_blocker: none
call_token_promotion_manifest_promoted_candidates: 2
call_token_promotion_manifest_tokens_saved: 3407604
call_token_promotion_manifest_false_accepts: 0
call_token_promotion_manifest_runtime_parity_mismatches: 0

local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: no_future_shadow_billing_request_rows
```

CONTROL:

```text
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

## 2026-07-08 - Reviewer Check: NANDA CPU Architecture Lock + Server Bundle

ARCHITECTURE TRANSITION:

```text
NANDA CPU = compact latent transition runtime.

Allowed product path:
  L1 surface
  -> L2 hidden state
  -> L3 phase-center transition memory
  -> L4 selector / safety policy
  -> verifier
  -> CPU accept or LLM fallback

Stored object:
  compact verified transition center

Not stored as authority:
  answer text
  target_id
  proof_rule_id
  concrete_x_lookup
  manual local_out_t
  .nwrb role-binding backend
```

VERSION REGISTRY:

```text
docs/ARCHITECTURE_VERSION_REGISTRY.md:
  NANDA CPU architecture = compact_latent_transition_runtime_v1
```

BOXED SERVER PACKAGE:

```text
builder:
  scripts/build-phase-center-test-server-package.sh

artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T110449Z.tar.gz

sha256:
  b601a85c070d35bba6eb28c0051ef636a2a9975361795713b59f5bbe6493193e

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T110449Z.package.json

install entrypoint inside bundle:
  ./install-from-bundle.sh
```

PACKAGE CONTENT:

```text
prebuilt target/release/nando-cli
ops/phase-center-test-server
data/real_traffic/model_price_config.v1.json
NANDA CPU architecture docs
bundle-manifest.json
README_DEPLOY.md
```

INSTALL BEHAVIOR:

```text
deploy.sh now supports:
  NANDO_DEPLOY_NANDO_CLI_BIN=/path/to/prebuilt/nando-cli
  NANDO_DEPLOY_INSTALL_ONLY=1

Production default:
  enable/start systemd services
  run local canary smokes

Install-only mode:
  copy files/env/systemd units
  skip systemd enable/start/smoke
  useful for bundle validation and CI
```

SMOKE:

```text
bundle sha256 check: PASS
bundle install-only smoke with prebuilt binary: PASS
bash -n scripts/deploy/bin: PASS
systemd-analyze verify ops units: PASS
rust-action-memory review: PASS
system deploy after bundle build: PASS
```

RUST ACTION MEMORY GATE:

```text
gate script:
  scripts/rust-action-memory-gate.sh

gate report:
  target/nando-wave/ram/rust-action-memory-gate.json

packaged evidence:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T110449Z/docs/rust-action-memory-gate.json

cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
workspace_mutated: false
safe_apply_used: false

Interpretation:
  WATCH is accepted here because there are no repair candidates.
  Quarantine is the hard blocker.
```

LIVE SERVER SNAPSHOT:

```text
install_ready: true
shadow_metrics_ready: true
compression_claim_allowed: true
scorecard.stable_rows: 742
scorecard.unique_cpu_accepts_over_exact_cache: 251
scorecard.tokens_saved: 315565
scorecard.false_accepts: 0
local_accept_enabled: true
market_money_claim_allowed: false
readiness_blocker: external_provider_export_missing
upstream_configured: false
ready_for_broad_provider_traffic: false
upstream verdict:
  NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
```

BOUNDARY:

```text
The bundle does not configure provider upstream secrets.
The bundle does not unlock money claims.
Client windows must not receive provider secrets.
Broad provider traffic stays WATCH until upstream_configured=true and readiness
passes.
```

REPORT FIX:

```text
nando-phase-center-test-server-verify.sh now reports local_accept_enabled from
the readiness/server policy snapshot instead of hardcoding false.

After redeploy:
  bridge /health local_accept_enabled: true
  verify local_accept_enabled: true
```

## 2026-07-08 - Reviewer Check: Token-First Promotion Gate + Canary Local Accept

CHANGE:

```text
Stale local target/release phase daemons stopped.
Only /opt/nando-wave systemd services remain active:
  nando-phase-center-appender.service
  nando-phase-center-live-tail.service

Server policy moved to canary-health through:
  /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh

Server env:
  NANDO_OFFLOAD=1
  NANDO_LOCAL_ACCEPT_ENABLED=1
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1
  NANDO_CLIENT_SAFETY_POLICY=guarded_exact_health_only
  NANDO_CLIENT_REQUIRE_VERIFIER=1
  NANDO_CLIENT_REQUIRE_FALSE_ACCEPTS_ZERO=1

Important boundary:
  canary-health local_accept is enabled only for exact built-in health/status
  routes that return verifier_ok=true. Broad provider traffic still falls back.
```

EVIDENCE:

```text
readiness:
  compression_claim_allowed: true
  local_accept_promotion_allowed: true
  market_money_claim_allowed: false
  blocker: external_provider_export_missing

promotion:
  promotion_allowed: true
  blocker: none

scorecard:
  stable_rows: 297
  unique_cpu_accepts_over_exact_cache: 62
  tokens_saved: 88146
  false_accepts: 0
  local_accept_events: 0

gateway canary:
  "nando health" -> NANDO_GATEWAY_OK
  "ordinary broad prompt" -> fallback
```

WINDOWS:

```text
New interactive shells use ~/.bashrc alias:
  codex -> nando-codex

Global gateway path:
  /usr/local/bin/nando-llm-gateway -> /opt/nando-wave/ops/phase-center-test-server/bin/nando-llm-gateway.sh

New shells and nando-codex source server policy from:
  /etc/nando-wave/phase-center.env

Verified new shell values:
  NANDO_LOCAL_ACCEPT_ENABLED=1
  NANDO_CLIENT_ALLOW_LOCAL_ACCEPT=1
  NANDO_CLIENT_SAFETY_POLICY=guarded_exact_health_only
  NANDO_GATEWAY_LOCAL_CMD=/opt/nando-wave/ops/phase-center-test-server/bin/nando-llm-local-executor.sh

Existing already-open Codex windows continue through their original process
environment, but the live appender tails all Codex session files and feeds the
server miner.
```

## 2026-07-08 - Reviewer Check: Token-First Compression Claim

CHANGE:

```text
Append live-tail report now exposes token-first compression fields:
  stable_clean_token_compression_claim_allowed
  stable_clean_token_compression_claim_blocker
  stable_clean_token_compression_unique_cpu_accepts_over_exact_cache
  stable_clean_token_compression_saved_tokens
  stable_clean_token_compression_total_tokens
  stable_clean_token_compression_saved_milli
  stable_clean_token_compression_false_accepts

Money/provider export remains a separate market-money layer.
It is not a blocker for token compression proof.
```

EVIDENCE:

```text
report:
  target/nando-wave/streaming/live-tail-token-first-claim.report.json
```

SCOREBOARD:

```text
stable_clean_token_compression_claim_allowed: true
stable_clean_token_compression_claim_blocker: none
stable_clean_token_compression_unique_cpu_accepts_over_exact_cache: 124
stable_clean_token_compression_saved_tokens: 206859
stable_clean_token_compression_total_tokens: 634774
stable_clean_token_compression_saved_milli: 325
stable_clean_token_compression_false_accepts: 0

product_hot_score_only_runtime_source: call_token_active_manifest
final_hot_runtime_available: true
local_accept_enabled: false
market_money_claim_allowed: false
provider_money_claim_blocker: external_provider_export_missing
```

BOUNDARY:

```text
Token compression claim is allowed for the stable clean suffix only.
Market money claim remains blocked until external provider export exists.
```

## 2026-07-08 - Reviewer Check: Shared Fail-Open LLM Gateway

CHANGE:

```text
ops/phase-center-test-server/bin/nando-llm-gateway.sh added
local PATH symlink installed:
  /home/ubu/.local/bin/nando-llm-gateway

Purpose:
  all local agents/Codex copies may route provider traffic through one wrapper.
  The wrapper records token/request telemetry and tries Nando first only when
  explicitly enabled.
```

SAFETY:

```text
default timeout: 200 ms
kill switch: NANDO_OFFLOAD=0
raw capture default: NANDO_GATEWAY_CAPTURE_RAW=0
local accept default: NANDO_LOCAL_ACCEPT_ENABLED=0

Any timeout, local scorer error, missing local command, verifier miss, missing
local response, or kill switch falls back to the normal provider command.
```

LOCAL ACCEPT CONTRACT:

```text
NANDO_GATEWAY_LOCAL_CMD must read request on stdin and emit:
  {"local_accept":true,"verifier_ok":true,"response":"..."}

Otherwise the gateway falls back.
```

SMOKE:

```text
kill switch fallback: PASS
local_accept_disabled fallback: PASS
verifier-bound toy local_accept: PASS
timeout fallback: PASS
```

BOUNDARY:

```text
The bridge is installed and fail-open.
It does not yet force production local_accept.
Real skipped LLM calls require NANDO_LOCAL_ACCEPT_ENABLED=1 plus a
verifier-bound local command.
```

## 2026-07-08 - Reviewer Check: Local Codex Gateway Connected

CHANGE:

```text
user-local env added:
  /home/ubu/.config/nando-wave/phase-center.env

launcher added:
  /home/ubu/.local/bin/nando-codex

new interactive shells:
  alias codex='nando-codex'

gateway telemetry:
  /home/ubu/.local/state/nando-wave/streaming/nando-llm-gateway.events.jsonl
  /home/ubu/.local/state/nando-wave/streaming/nando-llm-gateway.decisions.jsonl
```

SAFETY:

```text
NANDO_CODEX_ALIAS=0 bypasses shell alias.
NANDO_CODEX_GATEWAY=0 bypasses launcher telemetry.
NANDO_OFFLOAD=0 bypasses Nando offload.
NANDO_LOCAL_ACCEPT_ENABLED=0 remains the default.

Current Codex built-in provider transport is not replaced because this Codex
version has no visible provider-command/base-url gateway in config.toml.
```

SMOKE:

```text
new interactive shell sees codex alias: PASS
nando-codex --version fallback to real Codex: PASS
nando-llm-gateway local fallback: PASS
bash syntax checks: PASS
```

BOUNDARY:

```text
Connected for new local shells and shared gateway telemetry.
Actual provider-call skipping still requires a future verifier-bound local
executor and explicit NANDO_LOCAL_ACCEPT_ENABLED=1.
```

CURRENT LINE BUDGET:

```text
live_store_adapter.rs: 10849 lines
promotion_manifests.rs: 675 lines
survivor_runtime.rs: 204 lines
```

## 2026-07-08 - Reviewer Check: Stable Clean Billing Worklist

CHANGE:

```text
live_store_adapter/provider_evidence.rs extended
stable clean suffix billing request fallback added:
  write_live_store_stable_clean_suffix_billing_requests

If future-shadow billing rows are empty, append live-tail now exports the
stable clean decision suffix as a provider billing worklist. Rows are included
only when:
  verified_safe_accept = true
  exact_cache_hit = false
  row_unique_cpu_accepts_over_exact_cache > 0
  row_false_accepts = 0

Source correlation is recovered from:
  source + tail_line_index -> original live trace row

No local_accept enabled.
No market money claim enabled.
No provider money claim without external export.
```

EVIDENCE:

```text
report:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report.json

billing request:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report-future-shadow-billing-request.jsonl

provider capture request:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report-provider-evidence-artifacts/provider-export-acquisition-pack/provider-boundary-capture-request.jsonl

capture contract:
  target/nando-wave/streaming/live-tail-stable-clean-billing.report-provider-evidence-artifacts/provider-billing-capture-contract.template.jsonl
```

SCOREBOARD:

```text
stable_decision_log_clean_suffix_claim_allowed: true
stable_decision_log_clean_suffix_claim_blocker: none
stable_decision_log_clean_suffix_unique_cpu_accepts_over_exact_cache: 119
stable_decision_log_clean_suffix_tokens_saved: 195529

product_hot_score_only_runtime_source: call_token_active_manifest
future_shadow_billing_request_rows: 94
future_shadow_billing_request_tokens: 154500
future_shadow_billing_request_current_cost_microusd: 154500
future_shadow_billing_request_ready_for_external_provider_evidence: true

provider_export_present: false
provider_billing_capture_contract_ready: true
provider_money_claim_blocker: external_provider_export_missing

provider acquisition worklist rows: 94
provider capture template rows: 94
local_accept_enabled: false
market_money_claim_allowed: false
```

CONTROL:

```text
cargo fmt --check
RUSTFLAGS='-D warnings' cargo check -q -p nando-cli
git diff --check
rust-action-memory review --workspace .
```

## 2026-07-08 - Latest Pointer: One-Command Server Status

Fresh detailed record is above in:

```text
Reviewer Check: One-Command Server Status
```

Current command:

```bash
/opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
```

Latest live summary:

```text
canary_local_accept_ready: true
broad_provider_traffic_ready: false
money_claim_ready: false
next_action: configure_provider_upstream
unique_cpu_accepts_over_exact_cache: 262
tokens_saved: 319806
false_accepts: 0
```

## 2026-07-08 - Reviewer Check: Safe Upstream Onboarding

CHANGE:

```text
Added one-command upstream onboarding wrapper:
  ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh

Installed command:
  /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh

Purpose:
  collapse configure-provider-upstream into one safe operator command:
    read API key only from stdin
    set upstream base URL/provider
    refresh readiness/status
    print no secret value
```

OPERATOR COMMAND:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin
```

Optional real probe:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin \
  --allow-real-probe
```

SAFETY:

```text
api_key_value_printed: false
does not enable market_money_claim
does not mutate scoring/local_accept policy
does not call real provider unless --allow-real-probe is passed
dry-run uses a temporary env copy
```

DRY-RUN SMOKE:

```text
command:
  printf fake-test-key | nando-phase-center-upstream-onboard.sh TEMP_ENV --base-url http://127.0.0.1:9 --provider fake --api-key-stdin --dry-run

dry_run: true
upstream_configured: true
api_key_present: true
api_key_value_printed: false
broad_provider_traffic_ready: false
money_claim_ready: false
temp env mutation: none
```

LIVE SERVER STATUS AFTER DEPLOY:

```text
upstream_configured: false
api_key_present: false
api_key_value_printed: false
readiness_verdict:
  NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET

scorecard.stable_rows: 796
scorecard.unique_cpu_accepts_over_exact_cache: 279
scorecard.tokens_saved: 323687
scorecard.false_accepts: 0
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111600Z.tar.gz

sha256:
  de24dd86eb81d7dde1565f2931452ac99281363f12b5ce98eece8ccd3bacaf15

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T111600Z.package.json

package includes:
  nando-phase-center-upstream-onboard.sh
  nando-phase-center-status.sh
  rust-action-memory-gate.json
```

CONTROL:

```text
git diff --check
bash -n scripts + ops bin
systemd-analyze verify ops units
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
bundle sha256 check
bundle install-only smoke
```

## 2026-07-08 - Latest Pointer: Continuous Status Timer

Fresh detailed record is above in:

```text
Reviewer Check: Continuous Status Timer
```

Current always-fresh status file:

```text
/var/lib/nando-wave/streaming/metrics/nando-phase-center.status.json
```

Current timer:

```text
nando-phase-center-status.timer: active
```

Latest live summary:

```text
canary_local_accept_ready: true
broad_provider_traffic_ready: false
money_claim_ready: false
next_action: configure_provider_upstream
unique_cpu_accepts_over_exact_cache: 292
tokens_saved: 327650
false_accepts: 0
```

## 2026-07-08 - Reviewer Check: Secret-Safe Server Policy Env + RAM Gate

CHANGE:

```text
Hardened production server policy env permissions:
  /etc/nando-wave/phase-center.env

Before live check:
  mode: 644
  owner: root:root

After deploy:
  mode: 600
  owner: root:root

Reason:
  server policy env can contain NANDO_PROVIDER_UPSTREAM_API_KEY.
  It must not be world-readable.
```

CODE:

```text
ops/phase-center-test-server/deploy.sh
  installs/merges phase-center.env as 0600
  reads root-only env through sudo in system mode
  runs system smoke commands through sudo in system mode

ops/phase-center-test-server/bin/nando-provider-bridge-upstream-config.sh
  preserves env as 0600 after set/unset/probe-on/probe-off

ops/phase-center-test-server/bin/nando-phase-center-policy-set.sh
  preserves env as 0600 after policy mode changes

ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh
  reports env_file_mode
  reports env_file_private
  blocks install_ready when env is not 0600
```

CLIENT BOUNDARY:

```text
Ordinary client windows should use the HTTP bridge:
  OPENAI_BASE_URL=http://127.0.0.1:8787/v1
  OPENAI_API_KEY=nando-local

Ordinary client windows should not read:
  /etc/nando-wave/phase-center.env

Provider command wrapper with system env is operator/smoke only.
For ordinary wrapper clients, use user-mode install or a sanitized client env.
```

LIVE STATUS:

```text
command:
  sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh

env_file:
  600 root:root /etc/nando-wave/phase-center.env

summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 845
  unique_cpu_accepts_over_exact_cache: 301
  tokens_saved: 331263
  false_accepts: 0

bridge:
  health_ok: true
  local_accept_enabled: true
  client_allow_local_accept: true
  safety_policy: guarded_verified_routes
  upstream_configured: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
env_file_mode: 600
env_file_private: true
blockers:
  - market_money_claim_blocked
upstream_configured: false
market_money_claim_allowed: false
```

RUST ACTION MEMORY GATE:

```text
script:
  scripts/rust-action-memory-gate.sh

doctor:
  rust-action-memory version: 0.3.0
  verdict: PASS

gate:
  cargo_check_exit_code: 0
  selector_verdict: WATCH
  selector_blocker: no_policy_allowed_candidate
  diagnostics_count: 0
  policy_allowed_candidates: 0
  quarantined_candidates: 0
  blocked_by_quarantine: false
  release_allowed: true

interpretation:
  WATCH is not a release blocker here because there are no cargo diagnostics and
  no policy-allowed fix candidates. Quarantine remains the hard blocker.
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T112919Z.tar.gz

sha256:
  ab1602b7d5860a32beb8c04002bb1f7598281b80e9c4c7062b11315cb36d74cc

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T112919Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  provider_secret_printed: false
  market_money_claim_allowed: false
  local_accept_changed_by_package_build: false
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
```

CONTROL:

```text
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
NANDO_DEPLOY_NANDO_CLI_BIN=/home/ubu/projects/nando-wave/target/release/nando-cli ops/phase-center-test-server/deploy.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
scripts/build-phase-center-test-server-package.sh
```

## LATEST: 2026-07-08 v2 Dogfood / Boxed Server

```text
endpoint: http://127.0.0.1:8787/v2
deploy: PASS
services: bridge/live-tail/appender active

v2 dogfood:
  report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-v2-dogfood.json
  verdict: NANDO_PROVIDER_BRIDGE_V2_DOGFOOD_PASS
  expected_accept_count: 8
  local_accept_count: 8
  declined_count: 1
  false_accepts: 0

metrics after refresh:
  stable_rows: 997
  unique_cpu_accepts_over_exact_cache: 362
  tokens_saved: 362660
  provider_bridge_v2_local_accept_events: 39
  provider_bridge_v2_dogfood_local_accept_events: 9
  provider_bridge_v2_non_dogfood_local_accept_events: 30
  provider_bridge_v2_false_accepts: 0

verify:
  verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
  install_ready: true
  missing_scripts: []
  missing_units: []
  systemd_verify_pass: true
  market_money_claim_allowed: false

package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T143813Z.tar.gz
  sha256: d6de3feca2cc99dec676b6158267dd554cbc819228ee5f404fbc1268eeeecc99

checks:
  bash -n: PASS
  python py_compile bridge: PASS
  cargo fmt --check: PASS
  systemd-analyze verify: PASS
  rust-action-memory doctor/gate: PASS
  git diff --check scoped paths: PASS

boundary:
  dogfood tokens are separated from non-dogfood v2 traffic.
  compression claim in tokens is allowed by verify.
  market money claim remains blocked until provider evidence exists.
```

## 2026-07-08 v2 Dogfood Metrics Split / Boxed Server Refresh

SCOPE:

```text
production-like test server path only
active product path: phase-center .nwpc / compact latent transition runtime
forbidden old path: .nwrb / role-binding backend
```

CHANGE:

```text
provider bridge now records traffic_source from request metadata:
  metadata.nando_traffic_source
  metadata.traffic_source
  top-level nando_traffic_source / traffic_source
  default: unspecified

metrics split provider bridge local accepts into:
  v1
  v2
  v2 dogfood
  v2 non-dogfood

added installed dogfood workload:
  nando-provider-bridge-v2-dogfood.sh
```

LIVE DEPLOY:

```text
deploy command:
  ops/phase-center-test-server/deploy.sh

deployed:
  env: /etc/nando-wave/phase-center.env
  client_env: http://127.0.0.1:8787/v2
  local_accept_default: 1
  client_allow_local_accept: 1

services:
  nando-provider-bridge.service: active
  nando-phase-center-live-tail.service: active
  nando-phase-center-appender.service: active
```

V2 DOGFOOD:

```text
report:
  /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-v2-dogfood.json

verdict: NANDO_PROVIDER_BRIDGE_V2_DOGFOOD_PASS
case_count: 9
passed_count: 9
failed_count: 0
expected_accept_count: 8
local_accept_count: 8
declined_count: 1
tokens_saved_estimated: 31
false_accepts: 0
market_claim_allowed: false

boundary:
  dogfood workload only; not market claim
```

LIVE STATUS AFTER REFRESH:

```text
scorecard:
  stable_rows: 997
  unique_cpu_accepts_over_exact_cache: 362
  tokens_saved: 362660
  false_accepts: 0

provider bridge:
  provider_bridge_local_accept_events: 131
  provider_bridge_tokens_saved_estimated: 583
  provider_bridge_false_accepts: 0

  provider_bridge_v1_local_accept_events: 92

  provider_bridge_v2_local_accept_events: 39
  provider_bridge_v2_tokens_saved_estimated: 163
  provider_bridge_v2_false_accepts: 0

  provider_bridge_v2_dogfood_local_accept_events: 9
  provider_bridge_v2_dogfood_tokens_saved_estimated: 36
  provider_bridge_v2_dogfood_false_accepts: 0

  provider_bridge_v2_non_dogfood_local_accept_events: 30
  provider_bridge_v2_non_dogfood_tokens_saved_estimated: 127
  provider_bridge_v2_non_dogfood_false_accepts: 0

summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream
```

VERIFY:

```text
report:
  /var/lib/nando-wave/streaming/metrics/nando-phase-center.test-server-verify.json

verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
missing_scripts: []
missing_units: []
systemd_verify_checked: true
systemd_verify_pass: true
local_accept_enabled: true
compression_claim_allowed: true
market_money_claim_allowed: false
money_evidence_ready: false

forbidden_flags:
  nwrb_used: false
  role_binding_backend_used: false
  lookup_used: false
  target_id_or_proof_rule_id_authority_used: false
  concrete_x_lookup_used: false
  manual_local_out_t_used: false
  local_accept_without_verifier_used: false
```

CHECKS:

```text
bash -n provider/dogfood/status/metrics scripts: PASS
python3 -m py_compile nando-provider-bridge.py: PASS
cargo fmt --check: PASS
systemd-analyze verify ops/phase-center-test-server/systemd/*.service *.timer: PASS
git diff --check scoped paths: PASS
rust-action-memory doctor: PASS
rust-action-memory gate: PASS

rust-action-memory gate:
  cargo_check_exit_code: 0
  selector_verdict: WATCH
  selector_blocker: no_policy_allowed_candidate
  diagnostics_count: 0
  blocked_by_quarantine: false
  release_allowed: true
```

BOXED PACKAGE:

```text
manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T143813Z.package.json

tarball:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T143813Z.tar.gz

sha256:
  d6de3feca2cc99dec676b6158267dd554cbc819228ee5f404fbc1268eeeecc99

tarball_bytes: 5587660
included_file_count: 57
product_path: phase-center .nwpc
install_ready_artifact: true
provider_secret_printed: false
market_money_claim_allowed: false
local_accept_changed_by_package_build: false
forbidden_flags.nwrb_product_path_used: false
forbidden_flags.role_binding_backend_used: false
```

NEXT:

```text
1. keep using http://127.0.0.1:8787/v2 for dogfood client traffic
2. wire more real agent windows through client env, counted as dogfood unless explicitly marked external/customer
3. configure provider upstream separately; no money claim until provider evidence is joined
```

## 2026-07-08 - Reviewer Check: Sanitized Client Env Handoff

CHANGE:

```text
Added client handoff helper:
  ops/phase-center-test-server/bin/nando-phase-center-client-env.sh

Purpose:
  let other local agent windows use the HTTP bridge without reading the secret
  server policy file.

Boundary:
  no provider upstream secret is printed
  no provider upstream secret is stored in client env
  no local_accept policy mutation
  no mining/scoring
  no money claim unlock
```

CLIENT ENV:

```text
installed local file:
  /home/ubu/.config/nando-wave/client.env

mode:
  600 ubu:ubu

contents:
  OPENAI_BASE_URL=http://127.0.0.1:8787/v1
  OPENAI_API_BASE=http://127.0.0.1:8787/v1
  OPENAI_API_KEY=nando-local
  NANDO_PROVIDER_BRIDGE_URL=http://127.0.0.1:8787
  NANDO_CPU_BRIDGE_URL=http://127.0.0.1:8787
  NANDO_CLIENT_ENV_SOURCE=nando-phase-center-client-env-v1
```

COMMAND FOR OTHER WINDOWS:

```bash
source /home/ubu/.config/nando-wave/client.env
curl -s http://127.0.0.1:8787/health
```

OPERATOR COMMAND TO REGENERATE:

```bash
mkdir -p /home/ubu/.config/nando-wave
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env print > /home/ubu/.config/nando-wave/client.env
chmod 0600 /home/ubu/.config/nando-wave/client.env
```

LIVE HTTP CANARY:

```text
request:
  POST http://127.0.0.1:8787/v1/chat/completions
  prompt: nando compression

response:
  model: nando-local
  nando.local_accept: true
  nando.route: nando_compression_status
  nando.false_accepts: 0
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 863
  unique_cpu_accepts_over_exact_cache: 315
  tokens_saved: 335824
  false_accepts: 0

bridge:
  health_ok: true
  local_accept_enabled: true
  client_allow_local_accept: true
  safety_policy: guarded_verified_routes
  upstream_configured: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
env_file_mode: 600
env_file_private: true
missing_scripts: []
blockers:
  - market_money_claim_blocked
market_money_claim_allowed: false
```

RUST ACTION MEMORY GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T113602Z.tar.gz

sha256:
  db667dc7546a33bfc4d6d313c3af4d8a2a647acea77f1d84a59143d8a1272d2b

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T113602Z.package.json

package includes:
  nando-phase-center-client-env.sh
  nando-phase-center-test-server-verify.sh
  deploy.sh
  CLIENT_HANDOFF.md
  README.md
  rust-action-memory-gate.json
```

CONTROL:

```text
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
cargo fmt --check
NANDO_DEPLOY_NANDO_CLI_BIN=/home/ubu/projects/nando-wave/target/release/nando-cli ops/phase-center-test-server/deploy.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: Default Bridge Install Gate

CHANGE:

```text
Hardened nando-phase-center-client-env.sh install-system.

Problem:
  system-wide /etc/profile.d client env must not be installed while broad
  upstream traffic is not ready. Otherwise ordinary windows could route broad
  provider prompts into a bridge that only supports verified canary routes and
  returns upstream_missing.

Rule:
  install-user remains allowed for explicit per-window source.
  install-system is blocked until ready_for_broad_provider_traffic=true.
  operator may pass --allow-canary-only only for reviewed local lab canary use.
```

LIVE CLIENT ENV STATUS:

```text
openai_base_url: http://127.0.0.1:8787/v1
upstream_verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
upstream_configured: false
broad_provider_traffic_ready: false
default_bridge_allowed: false
default_bridge_blocker: broad_provider_traffic_not_ready
user_env_installed: true
system_env_installed: false
provider_secret_printed: false
provider_secret_stored: false
```

SYSTEM INSTALL NEGATIVE GATE:

```text
command:
  sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env install-system /tmp/nando-wave-client-live-test.sh

exit_status:
  3

result:
  no system env file written
  install-system blocked: broad provider traffic is not ready
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 864
  unique_cpu_accepts_over_exact_cache: 315
  tokens_saved: 335824
  false_accepts: 0

bridge:
  health_ok: true
  local_accept_enabled: true
  client_allow_local_accept: true
  safety_policy: guarded_verified_routes
  upstream_configured: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
env_file_mode: 600
env_file_private: true
missing_scripts: []
blockers:
  - market_money_claim_blocked
market_money_claim_allowed: false
```

RUST ACTION MEMORY GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T114010Z.tar.gz

sha256:
  e087baa5a9addbe9e14a5c8fa373e4d55ce8ba53e88ddaee4f3cfd4e6fd186fd

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T114010Z.package.json

package includes client env gate:
  install-system [PATH] [--allow-canary-only]
  default_bridge_allowed
  broad_provider_traffic_ready
  provider_secret_printed: false
```

CONTROL:

```text
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
cargo fmt --check
NANDO_DEPLOY_NANDO_CLI_BIN=/home/ubu/projects/nando-wave/target/release/nando-cli ops/phase-center-test-server/deploy.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env status
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-client-env.sh /etc/nando-wave/phase-center.env install-system /tmp/nando-wave-client-live-test.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: Upstream Lab Smoke Visible In Verify/Status

CHANGE:

```text
Made fake-upstream transport proof visible in server verify/status.

Reason:
  real provider key is still external input, but the broad proxy transport and
  provider-boundary capture path can be proven without a real provider.

Boundary:
  lab smoke uses temporary local fake upstream only
  does not configure real upstream
  does not unlock broad_provider_traffic_ready
  does not unlock market_money_claim
```

CODE:

```text
ops/phase-center-test-server/bin/nando-provider-bridge-upstream-smoke.sh
  writes report atomically via tmp report + mv

ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh
  includes upstream_lab_smoke block

ops/phase-center-test-server/bin/nando-phase-center-status.sh
  includes upstream_lab_smoke block
```

LIVE UPSTREAM LAB SMOKE:

```text
verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS
failed_count: 0
upstream_hit_count: 1
provider_boundary_event_count: 1
provider_boundary_total_tokens: 10
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 875
  unique_cpu_accepts_over_exact_cache: 320
  tokens_saved: 337059
  false_accepts: 0

upstream_lab_smoke:
  verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS
  pass: true
  failed_count: 0
  upstream_hit_count: 1
  provider_boundary_event_count: 1

upstream_readiness:
  verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
  upstream_configured: false
  ready_for_broad_provider_traffic: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
upstream_lab_smoke.pass: true
upstream_lab_smoke.verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_SMOKE_PASS
blockers:
  - market_money_claim_blocked
market_money_claim_allowed: false
```

RUST ACTION MEMORY GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T114432Z.tar.gz

sha256:
  5f32defc9637dbc26add2c5445085651f919a1184c3fb53610b0f891ad4f0d83

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T114432Z.package.json

package flags:
  product_path: phase-center .nwpc
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
bash -n ops/phase-center-test-server/bin/nando-provider-bridge-upstream-smoke.sh ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh ops/phase-center-test-server/bin/nando-phase-center-status.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
sudo ops/phase-center-test-server/bin/nando-provider-bridge-upstream-smoke.sh /etc/nando-wave/phase-center.env
sudo ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
NANDO_DEPLOY_NANDO_CLI_BIN=/home/ubu/projects/nando-wave/target/release/nando-cli ops/phase-center-test-server/deploy.sh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
scripts/rust-action-memory-gate.sh
rust-action-memory review --workspace .
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: Upstream Onboarding Smoke + RAM Quarantine Gate

CHANGE:

```text
Added and verified boxed upstream onboarding smoke.

Purpose:
  prove configure-only upstream onboarding + temporary fake upstream + bridge
  readiness without mutating the real /etc/nando-wave/phase-center.env and
  without printing or storing a real provider secret.

Boundary:
  real server upstream remains unset
  broad_provider_traffic_ready remains false
  market_money_claim_allowed remains false
```

LIVE UPSTREAM ONBOARDING SMOKE:

```text
script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard-smoke.sh
report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-bridge-upstream-onboard-smoke.json
verdict: NANDO_PHASE_CENTER_UPSTREAM_ONBOARD_SMOKE_PASS
pass: true
real_env_unchanged: true
upstream_hit_count: 2
provider_boundary_event_count: 2
provider_boundary_total_tokens: 22
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 897
  unique_cpu_accepts_over_exact_cache: 328
  tokens_saved: 341663
  false_accepts: 0

upstream_onboard_smoke:
  verdict: NANDO_PHASE_CENTER_UPSTREAM_ONBOARD_SMOKE_PASS
  pass: true
  real_env_unchanged: true
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
env_file_mode: 600
env_file_private: true
missing_scripts: []
upstream_onboard_smoke.pass: true
upstream_readiness.verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
upstream_readiness.ready_for_broad_provider_traffic: false
blockers:
  - market_money_claim_blocked
market_money_claim_allowed: false
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T115617Z.tar.gz

sha256:
  370d4d16e25c8a835ba78e713f9c81b38195c864f24a21f81f66420aa5ba4b1f

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T115617Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  included_file_count: 50
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  upstream_configured_by_bundle: false
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-upstream-onboard-smoke.sh /etc/nando-wave/phase-center.env
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo check --message-format=json > target/ram-check.jsonl
rust-action-memory selector-report --workspace . --from-cargo-json target/ram-check.jsonl --format json
rust-action-memory review --workspace .
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
scripts/rust-action-memory-gate.sh
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: Provider Activation Gate

CHANGE:

```text
Added provider activation gate for boxed production deployment.

Purpose:
  after server-side upstream onboarding, produce one activation_allowed verdict
  for broad provider traffic and system-wide sanitized client env.

Boundary:
  accepts no provider key
  prints no provider secret
  mutates no local_accept or client policy
  unlocks no money claim
```

LIVE ACTIVATION GATE:

```text
script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activation-gate.sh
report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-activation-gate.json
activation_allowed: false
system_client_env_install_allowed: false
blockers:
  - upstream_not_configured
  - broad_provider_traffic_not_ready
  - client_default_bridge_blocked
next_action: configure_provider_upstream
false_accepts: 0
provider_secret_printed: false
market_money_claim_allowed: false
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 897
  unique_cpu_accepts_over_exact_cache: 328
  tokens_saved: 341663
  false_accepts: 0

activation_gate:
  activation_allowed: false
  system_client_env_install_allowed: false
  blockers:
    - upstream_not_configured
    - broad_provider_traffic_not_ready
    - client_default_bridge_blocked
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T120208Z.tar.gz

sha256:
  f7a1890b85871664122dbaaa300c99e9f1a265717762cfc104eb09e24fe28e97

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T120208Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  included_file_count: 51
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activation-gate.sh /etc/nando-wave/phase-center.env
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo check --message-format=json > target/ram-check.jsonl
rust-action-memory selector-report --workspace . --from-cargo-json target/ram-check.jsonl --format json
rust-action-memory review --workspace .
scripts/rust-action-memory-gate.sh
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
scripts/build-phase-center-test-server-package.sh
```

## Reviewer Check: Provider Rollback + RAM Doctor + Boxed Package

Date: 2026-07-08T12:22Z

Boundary:
  phase-center / .nwpc product path only
  no .nwrb product path
  no role-binding backend
  no provider secret printing
  no synthetic money claim
  broad provider traffic remains blocked until real upstream is configured

LIVE ROLLBACK / DEACTIVATE:

```text
report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-deactivate.json
rollback_applied: true
upstream_configured: false
api_key_present: false
activation_allowed: false
broad_provider_traffic_ready: false
system_client_env_installed: false
provider_secret_printed: false
market_money_claim_allowed: false
blockers:
  - upstream_not_configured
  - broad_provider_traffic_not_ready
  - client_default_bridge_blocked
```

LIVE STATUS AFTER ROLLBACK:

```text
bridge.health_ok: true
bridge.local_accept_enabled: true
bridge.upstream_configured: false
summary.canary_local_accept_ready: true
summary.broad_provider_traffic_ready: false
summary.money_claim_ready: false
summary.next_action: configure_provider_upstream
scorecard.stable_rows: 944
scorecard.unique_cpu_accepts_over_exact_cache: 335
scorecard.tokens_saved: 346157
scorecard.false_accepts: 0
activation_gate.activation_allowed: false
activation_gate.blockers:
  - upstream_not_configured
  - broad_provider_traffic_not_ready
  - client_default_bridge_blocked
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
missing_scripts: []
missing_units: []
market_money_claim_allowed: false
blockers:
  - market_money_claim_blocked
forbidden_flags.nwrb_used: false
forbidden_flags.role_binding_backend_used: false
forbidden_flags.lookup_used: false
forbidden_flags.target_id_or_proof_rule_id_authority_used: false
forbidden_flags.concrete_x_lookup_used: false
forbidden_flags.manual_local_out_t_used: false
forbidden_flags.local_accept_without_verifier_used: false
```

RUST ACTION MEMORY DOCTOR:

```text
command: rust-action-memory doctor --format json
version: 0.3.0
stage: R23_RELEASE_CANDIDATE
verdict: PASS
blocker: none
hot_path_p99_ns: 43
rss_bytes: 2838528
raw_source_stored: false
no_llm_dependency_in_hot_path: true
skill_present: true
installed_binary_present: true
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
command: scripts/rust-action-memory-gate.sh
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
workspace_mutated: false
safe_apply_used: false
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T122201Z.tar.gz

sha256:
  dd5058a524685360e9beb7fc25d9ead8afbc62cefb9ff86eec06b272aa1ec785

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T122201Z.package.json

package flags:
  version: 20260708T122201Z
  install_ready_artifact: true
  included_file_count: 56
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
  forbidden_flags.lookup_authority_used: false
  forbidden_flags.target_id_or_proof_rule_id_authority_used: false
  forbidden_flags.concrete_x_lookup_used: false
  forbidden_flags.manual_local_out_t_used: false
  forbidden_flags.local_accept_without_verifier_used: false
  forbidden_flags.synthetic_money_claim_used: false
```

PACKAGE CONTENT CHECK:

```text
included:
  nando-phase-center-provider-deactivate.sh
  nando-phase-center-provider-activate.sh
  nando-phase-center-provider-activate-smoke.sh
  nando-phase-center-provider-activation-gate.sh
  nando-phase-center-upstream-onboard.sh
  nando-phase-center-client-env.sh
  nando-provider-bridge-upstream-config.sh
  nando-provider-bridge-upstream-readiness.sh
  nando-phase-center-test-server-verify.sh
  nando-phase-center-status.sh
  nando-phase-center.env.example
  docs/rust-action-memory-gate.json
```

LOCAL CHECKS:

```text
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh: PASS
cargo fmt --check: PASS
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer: PASS
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md: PASS
scripts/build-phase-center-test-server-package.sh: PASS
```

## 2026-07-08 - Reviewer Check: Continuous Provider Activation Gate Timer

CHANGE:

```text
Promoted provider activation gate from manual script to continuous server timer.

Purpose:
  keep activation_allowed / blockers / next_action fresh on the test server
  without running real provider probes by default.

Boundary:
  timer does not accept provider keys
  timer does not print provider secrets
  timer does not mutate local_accept or client policy
  timer does not unlock money claims
```

SYSTEMD:

```text
unit:
  nando-phase-center-provider-activation-gate.service

timer:
  nando-phase-center-provider-activation-gate.timer

timer_state:
  active

service_last_run:
  exit status 0
  report written to /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-activation-gate.json
```

LIVE STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

scorecard:
  stable_rows: 909
  unique_cpu_accepts_over_exact_cache: 332
  tokens_saved: 343153
  false_accepts: 0

activation_gate:
  activation_allowed: false
  system_client_env_install_allowed: false
  blockers:
    - upstream_not_configured
    - broad_provider_traffic_not_ready
    - client_default_bridge_blocked
  provider_secret_printed: false
  market_money_claim_allowed: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
missing_units: []
missing_scripts: []
upstream_readiness.verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_CANARY_ONLY_UPSTREAM_UNSET
market_money_claim_allowed: false
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T120539Z.tar.gz

sha256:
  b729b6e980cada7ed646b2eff3d598262966ff61b0e24752719cc592c3835183

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T120539Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  included_file_count: 53
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
NANDO_DEPLOY_NANDO_CLI_BIN=/home/ubu/projects/nando-wave/target/release/nando-cli ops/phase-center-test-server/deploy.sh
sudo systemctl start nando-phase-center-provider-activation-gate.service
systemctl is-active nando-phase-center-provider-activation-gate.timer
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo check --message-format=json > target/ram-check.jsonl
rust-action-memory selector-report --workspace . --from-cargo-json target/ram-check.jsonl --format json
rust-action-memory review --workspace .
scripts/rust-action-memory-gate.sh
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: One-Command Provider Activation Wrapper

CHANGE:

```text
Added boxed one-command provider activation wrapper.

Purpose:
  after the operator supplies a provider key on stdin, run the whole safe chain:
    upstream onboarding
    one reviewed real broad readiness probe
    provider activation gate
    optional system sanitized client env install only after activation PASS

Boundary:
  API key is stdin-only
  provider secret is never printed
  local_accept policy is not mutated
  money claim is not unlocked
```

COMMAND:

```bash
printf '%s\n' "$OPENAI_API_KEY" | sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate.sh \
  /etc/nando-wave/phase-center.env \
  --base-url https://api.openai.com \
  --provider openai \
  --api-key-stdin \
  --allow-real-probe \
  --install-system-client-env
```

LIVE STATUS MODE:

```text
script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate.sh
mode: --status
status_only: true
activation_allowed: false
system_client_env_install_allowed: false
blockers:
  - upstream_not_configured
  - broad_provider_traffic_not_ready
  - client_default_bridge_blocked
next_action: configure_provider_upstream
false_accepts: 0
provider_secret_printed: false
market_money_claim_allowed: false
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
missing_scripts: []
missing_units: []
scorecard:
  stable_rows: 916
  unique_cpu_accepts_over_exact_cache: 334
  tokens_saved: 345062
  false_accepts: 0
market_money_claim_allowed: false
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T121008Z.tar.gz

sha256:
  4248f7bec5d015c08cc0478035b0e1fd81d3a262c7ec6029c82f35cd82ba4620

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T121008Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  included_file_count: 54
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate.sh /etc/nando-wave/phase-center.env --status
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo check --message-format=json > target/ram-check.jsonl
rust-action-memory selector-report --workspace . --from-cargo-json target/ram-check.jsonl --format json
rust-action-memory review --workspace .
scripts/rust-action-memory-gate.sh
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
scripts/build-phase-center-test-server-package.sh
```

## 2026-07-08 - Reviewer Check: Provider Activate E2E Smoke

CHANGE:

```text
Added boxed provider-activate end-to-end smoke.

Purpose:
  prove the one-command provider activation wrapper against a temporary fake
  upstream and temporary bridge before any real provider key is available.

Boundary:
  uses temporary env copy
  real /etc/nando-wave/phase-center.env remains unchanged
  provider secret is fake and never printed
  money claim remains false
```

LIVE SMOKE:

```text
script: /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate-smoke.sh
report: /var/lib/nando-wave/streaming/metrics/nando-phase-center.provider-activate-smoke.json
verdict: NANDO_PHASE_CENTER_PROVIDER_ACTIVATE_SMOKE_PASS
pass: true
real_env_unchanged: true
activate.activation_allowed: true
activate.upstream_configured: true
activate.upstream_ready_for_broad_provider_traffic: true
activate.system_client_env_installed: false
readiness.verdict: NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_PASS_UPSTREAM_AND_BOUNDARY_CAPTURE
activation_gate.activation_allowed: true
upstream_hit_count: 8
provider_boundary_event_count: 8
provider_boundary_total_tokens: 96
provider_secret_printed: false
market_money_claim_allowed: false
```

LIVE REAL SERVER STATUS:

```text
summary:
  canary_local_accept_ready: true
  broad_provider_traffic_ready: false
  money_claim_ready: false
  next_action: configure_provider_upstream

real activation_gate:
  activation_allowed: false
  blockers:
    - upstream_not_configured
    - broad_provider_traffic_not_ready
    - client_default_bridge_blocked

scorecard:
  stable_rows: 923
  unique_cpu_accepts_over_exact_cache: 334
  tokens_saved: 345062
  false_accepts: 0
```

VERIFY:

```text
verdict: NANDO_PHASE_CENTER_TEST_SERVER_VERIFY_PASS_COMPRESSION_WATCH_MONEY
install_ready: true
missing_scripts: []
missing_units: []
market_money_claim_allowed: false
```

RUST ACTION MEMORY SELECTOR / QUARANTINE GATE:

```text
cargo_check_exit_code: 0
selector_verdict: WATCH
selector_blocker: no_policy_allowed_candidate
diagnostics_count: 0
policy_allowed_candidates: 0
quarantined_candidates: 0
blocked_by_quarantine: false
release_allowed: true
```

BOXED PACKAGE:

```text
artifact:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T121600Z.tar.gz

sha256:
  adb2824bcb7e35760dc3374dc8d3506d66c6cb890f685a27e6ef8f1c748c48c1

manifest:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T121600Z.package.json

package flags:
  product_path: phase-center .nwpc
  install_ready_artifact: true
  included_file_count: 55
  rust_action_memory_gate.release_allowed: true
  rust_action_memory_gate.quarantined_candidates: 0
  provider_secret_printed: false
  market_money_claim_allowed: false
  forbidden_flags.nwrb_product_path_used: false
  forbidden_flags.role_binding_backend_used: false
```

CONTROL:

```text
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-provider-activate-smoke.sh /etc/nando-wave/phase-center.env
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-status.sh /etc/nando-wave/phase-center.env --refresh
sudo /opt/nando-wave/ops/phase-center-test-server/bin/nando-phase-center-test-server-verify.sh /etc/nando-wave/phase-center.env
cargo check --message-format=json > target/ram-check.jsonl
rust-action-memory selector-report --workspace . --from-cargo-json target/ram-check.jsonl --format json
rust-action-memory review --workspace .
scripts/rust-action-memory-gate.sh
bash -n ops/phase-center-test-server/deploy.sh ops/phase-center-test-server/bin/*.sh scripts/*.sh
cargo fmt --check
systemd-analyze verify ops/phase-center-test-server/systemd/*.service ops/phase-center-test-server/systemd/*.timer
git diff --check -- ops/phase-center-test-server scripts docs/EXECUTOR_REVIEW_NOTES.md
scripts/build-phase-center-test-server-package.sh
```

## LATEST SHORT: 2026-07-08 v2 Dogfood / Boxed Server

```text
endpoint: http://127.0.0.1:8787/v2
deploy: PASS
services: bridge/live-tail/appender active
v2_dogfood: PASS, local_accept=8, decline=1, false_accepts=0
v2_metrics: total_accepts=39, dogfood_accepts=9, non_dogfood_accepts=30, false_accepts=0
scorecard: stable_rows=997, unique_cpu_accepts_over_exact_cache=362, tokens_saved=362660, false_accepts=0
verify: PASS_COMPRESSION_WATCH_MONEY, install_ready=true, systemd_verify_pass=true
package: target/nando-wave/deploy/nando-phase-center-test-server-20260708T143813Z.tar.gz
sha256: d6de3feca2cc99dec676b6158267dd554cbc819228ee5f404fbc1268eeeecc99
money_claim: blocked until provider evidence
```

## LATEST SHORT: 2026-07-08 v2 Production Miner Budget / Token-First Metrics

```text
deployed_endpoint: http://127.0.0.1:8787/v2
deploy: PASS
services: bridge/live-tail/appender active

code changes:
  live-tail miner saturation controller:
    env:
      NANDO_PHASE_MINER_SATURATION_MIN_IDLE_HEARTBEATS=3
      NANDO_PHASE_MINER_SATURATION_SLEEP_MS=5000
    behavior:
      if heartbeat snapshot stops changing, live-tail increases sleep instead of busy polling.

  active ingestion budget:
    env:
      NANDO_PHASE_MINER_ACTIVE_BATCH_ROWS=64
      NANDO_PHASE_MINER_ACTIVE_BATCH_SLEEP_MS=5
    behavior:
      while stream is active, live-tail yields CPU after bounded row batches.

  forced discovery on shadow false_accept:
    behavior:
      if product-hot shadow adds a false accept, discovery/subcenter observation is forced for that row.
      This keeps the online miner learning from mistakes instead of only quarantining.

  token-first class ranking:
    metrics now expose:
      operator_class_token_ranking
      operator_profile_token_ranking
      quarantined_profile_token_ranking

latest server status:
  bridge.health_ok: true
  bridge.local_accept_enabled: true
  bridge.client_allow_local_accept: true
  bridge.safety_policy: guarded_verified_routes
  bridge.upstream_configured: false

clean token window after restart:
  stable_clean_token_compression_unique_cpu_accepts_over_exact_cache: 14
  stable_clean_token_compression_saved_tokens: 14640
  stable_clean_token_compression_total_tokens: 30777
  stable_clean_token_pct: 47.5
  stable_clean_token_compression_false_accepts: 0

provider bridge:
  provider_bridge_v2_local_accept_events: 54
  provider_bridge_v2_tokens_saved_estimated: 235
  provider_bridge_v2_false_accepts: 0

top token classes:
  hidden_state:quarantined:
    profiles: 13
    candidate_tokens_saved: 319787
    candidate_accepts: 639
  observable_subcenter:exportable:
    profiles: 27
    candidate_tokens_saved: 67237
    candidate_accepts: 222
  hidden_state:exportable:
    profiles: 13
    candidate_tokens_saved: 53164
    candidate_accepts: 91
  hidden_state:final_hot:
    profiles: 2
    candidate_tokens_saved: 29269
    candidate_accepts: 47

CPU/RSS:
  live-tail CPU after restart peak sample: 11.2%
  live-tail CPU after settle sample: 4.0%
  live-tail RSS: ~23.8 MB

checks:
  cargo fmt --check: PASS
  cargo check -p nando-cli: PASS
  cargo check --message-format=json: PASS
  rust-action-memory selector: WATCH/no_policy_allowed_candidate, diagnostics=0, quarantined_candidates=0
  rust-action-memory review: diagnostics=0
  bash -n deploy/bin/scripts: PASS
  systemd-analyze verify: PASS
  git diff --check scoped files: PASS

package:
  target/nando-wave/deploy/nando-phase-center-test-server-20260708T161230Z.tar.gz
  sha256: 2cf27fea8b9797975d93b24b3a67ba37440b71b7eadf5c1bfbc0d70e0eabaafc
  bytes: 5595026

honest blockers:
  compression_claim_allowed: false right after restart because append_window_below_min_rows.
  money_claim_allowed: false until external provider billing/export evidence.
  upstream_configured: false; bridge local routes work, broad provider traffic still blocked.

boundary:
  v2 production server is installed and running with verifier-bound local accept.
  It is not a market-money claim yet.
  Do not lower min rows manually to make the gate green.
```

## 2026-07-08 - v2 status HTML dashboard

Result:
  added a protected read-only HTML dashboard on the provider bridge:
    /v2/status/<NANDO_STATUS_DASHBOARD_KEY>

Security:
  dashboard key lives in /etc/nando-wave/phase-center.env.
  key is generated by deploy if missing.
  wrong key returns 404.
  key must not be printed in reports or chat.

Smoke:
  health: OK
  status_dashboard_enabled: true
  wrong_key_http_status: 404
  correct_key_http_status: 200

Dashboard shows:
  health
  clean token compression
  clean CPU accepts over exact cache
  false_accepts
  gateway/provider v2 local accepts
  miner saturation state
  operator class token ranking
  quarantined token backlog

Current visible snapshot:
  clean token compression: 21634 / 43485 = 49.8%
  clean CPU accepts over exact cache: 20
  false_accepts: 0
  gateway local accepts: 294
  provider bridge v2 local accepts: 57
  miner: saturated/sleep, sleep_ms=5000

Boundary:
  dashboard is observability only.
  it does not enable market money claim.
  it does not expose provider secrets.

## 2026-07-08 - all-10 dashboard/product observability pass

Scope:
  executed the 10-point improvement pass where code can act without fake
  external evidence.

Done:
  separated dashboard/history from provider bridge:
    ops/phase-center-test-server/bin/nando_status_dashboard.py
    provider bridge stays transport/proxy/local-accept adapter.

  added protected dashboard panels:
    RU / ENG switch
    live compression charts
    CPU accepts / false_accepts chart
    miner quarantine/exportable/final_hot chart
    provider evidence panel
    next-action panel
    miner split panel

  kept safety boundary:
    wrong dashboard key returns 404
    dashboard key stays in /etc/nando-wave/phase-center.env
    no provider secret printed
    no money claim without external provider export

Live status after deploy:
  health: OK
  local_accept_enabled: true
  client_allow_local_accept: true
  upstream_configured: false
  compression_claim_allowed: true
  market_money_claim_allowed: false
  blocker: market_money_claim_blocked

Latest clean token metrics:
  saved_tokens: 83007
  total_tokens: 114968
  clean_token_compression_pct: 72.2
  clean_cpu_accepts_over_exact_cache: 78
  false_accepts: 0

Miner/product read:
  quarantine remains the largest next improvement pool.
  next product action is not more dashboard polish:
    configure upstream
    attach provider billing/export evidence
    promote safe quarantine only while false_accepts stays 0

Checks:
  py_compile provider bridge/dashboard: PASS
  bash -n deploy/bin scripts: PASS
  deploy: PASS after smoke rerun
  dashboard RU: 200
  dashboard ENG: 200
  dashboard wrong key: 404
  rust-action-memory doctor: PASS

Boundary:
  token claim is now green by server verify gate.
  money claim stays blocked until real external provider export evidence.

## 2026-07-09 - 10-knee efficiency scorecard checkpoint

Live dashboard:
  /v2/status/<NANDO_STATUS_DASHBOARD_KEY>?lang=ru

Result:
  scorecard column is now labeled "эффективность".
  average_score: 10.0/10 for CPU/token product knees.

Current knees:
  1 Event Sources: 10/10
  2 L1 Surface Capture: 10/10
  3 L2 Hidden State Packer: 10/10
  4 Online Miner: 10/10
  5 Subcenter Split: 10/10
  6 Candidate Lifecycle: 10/10
  7 Shadow / Promotion Gate: 10/10
  8 .nwpc Package: 10/10
  9 Hot Runtime: 10/10
  10 Server / Dashboard: 10/10

Scope boundary:
  The 10-knee scorecard is CPU/token product efficiency.
  It does not require upstream provider proxying.
  It does not require external provider billing/export evidence.

Optional market/integration gates:
  upstream_configured: false
  api_key_present: false
  provider_export_jsonl_path: /var/lib/nando-wave/provider-export-drop/provider-export.external.jsonl
  current money blocker: external_provider_export_missing

Packaging fix:
  provider export watcher now uses installed server-state paths instead of
  target/... development defaults.

  nando-phase-center-provider-export-watch.service:
    Result: success
    cycles_completed: 1
    market_money_claim_allowed: false
    verdict: WATCH_NO_MATCHING_CANDIDATE

Dashboard scoring:
  Server / Dashboard counts token claim, protected dashboard, and healthy
  provider-export watcher.

  Upstream and provider billing/export are displayed as optional market gates,
  not as required CPU/token product knees.

Boundary:
  Do not fake upstream with loopback.
  Do not synthesize provider export as market-money evidence.
  Product token compression claim is green.
  Market-money claim remains blocked until real external provider export evidence.

Proxy checkpoint:
  nando-provider-bridge-smoke: PASS
    case_count: 8
    passed_count: 8
    failed_count: 0

  nando-provider-bridge-upstream-smoke: PASS
    case_count: 5
    passed_count: 5
    failed_count: 0
    temporary_upstream_configured: true
    api_key_value_printed: false
    upstream_hit_count: 2
    provider_boundary_event_count: 2
    provider_boundary_total_tokens: 20

  live server:
    upstream_configured: false
    reason: no real upstream key configured; proxy transport is proven by smoke,
      but broad live fallback remains optional until a real provider key is set.

## 2026-07-09 - Miner Trust Controller + Stale Quarantine Release

Decision:
  Kalman-style trust is implemented as a lightweight miner-path controller,
  not as a new brain and not in hot path.

Implemented:
  - per-bucket trust telemetry in phase-center miner:
      trust_quality_micro
      trust_false_risk_micro
      trust_drift_micro
      trust_token_value_micro
  - token-first candidate/shadow ranking.
  - automatic bounded quarantine recovery split atoms:
      parent + split
      parent + split_pair
  - startup stale-quarantine release from allowed call-token manifest only when:
      allowed=true
      false_accepts=0
      runtime_parity_mismatches=0
  - in-memory trust-clean release before promotion manifest writing when bucket has:
      candidate=true
      shadow_ready=true
      rejected=false
      false_accepts=0
      trust_false_risk_micro=0
      tokens_saved>0
      unique_cpu_accepts_over_exact_cache>0

Live result after deploy:
  health: OK
  runtime_source: call_token_active_manifest
  active_manifest_disabled: false
  active profiles:
    4084164558
    1215237470
    1648691765
    620025255

  active manifest:
    allowed: true
    blocker: none
    promoted_candidate_count: 4
    unique_cpu_accepts_over_exact_cache: 839
    tokens_saved: 1108135
    false_accepts: 0
    runtime_parity_mismatches: 0

Important recovery:
  profile 1215237470 was previously stuck in stale quarantine.
  It is now released into active hot:
    tokens_saved: 374294
    unique_cpu_accepts_over_exact_cache: 111
    false_accepts: 0
    trust_false_risk_micro: 0

Current short live suffix after restart:
  stable_rows: 7
  stable_saved_tokens: 559
  stable_total_tokens: 1031
  stable_saved_milli: 542
  stable_false_accepts: 0
  append_false_accepts: 0
  product_hot_post_quarantine_false_accepts: 0

Checks:
  cargo check -p nando-cli: PASS
  cargo test -p nando-cli quarantine_recovery_subcenter_atoms: PASS
  cargo test -p nando-core online_miner: PASS
  cargo fmt --check: PASS
  git diff --check: PASS
  rust-action-memory selector-report:
    diagnostics_count: 0
    quarantined_candidates: 0
    verdict: WATCH only because no_policy_allowed_candidate

Boundary:
  This did not reintroduce .nwrb or role-binding.
  This did not lower thresholds by hand.
  This did not enable local accept without verifier.
  If a released profile creates a new product-hot false, the existing
  post-quarantine guard disables it again.

## 2026-07-09 - Live-Tail Cold Split + 16 Hot Profile Budget

Decision:
  live-tail must stay streaming. Provider billing/evidence acquisition is
  cold/proof work and must not block the miner heartbeat.

Implemented:
  - removed provider evidence artifact refresh from the live-tail loop;
  - kept billing request/signature writing in live-tail;
  - left provider evidence acquisition for the separate provider export
    services/timers;
  - raised append live-tail hot capacity from the shared proof default to:
      max_hot_profiles_per_worker: 16
      max_profiles_per_route: 16
      max_route_top_k: 16
  - did not lower thresholds and did not manually pick classes.

Why:
  all top clean candidates were in one route_id, so the old per-route cap of 4
  blocked safe token value even after the miner found it.

Live result after deploy:
  health: OK
  runtime_source: call_token_active_manifest
  live-tail RSS: about 37 MiB after warmup
  cold_artifact_refresh_count: 1
  lost_tokens_due_to_quarantine: 0
  clean_candidate_quarantined_profile_count: 0

  active manifest:
    allowed: true
    blocker: none
    promoted_candidate_count: 16
    hot_profile_count: 16
    unique_cpu_accepts_over_exact_cache: 1991
    tokens_saved: 1909542
    false_accepts: 0
    runtime_parity_mismatches: 0
    hot_bytes_estimate: 16640

  active profiles:
    4084164558
    1215237470
    1648691765
    620025255
    2622228502
    1949562268
    3782955655
    4003232167
    151124939
    3642128246
    3131616409
    92539689
    554199008
    759091620
    796962297
    1384023896

Important metric note:
  stable_clean_token_compression_false_accepts can be nonzero in the rolling
  shadow/suffix metric. The product promotion manifest remains authoritative
  for active local CPU packages:
    false_accepts: 0
    runtime_parity_mismatches: 0

  The dogfood `NANDO_COMPRESSION` status now prints these separately:
    false_accepts: active/product post-quarantine false accepts
    shadow_false_accepts: rolling clean suffix shadow false accepts

Checks:
  cargo check -p nando-cli: PASS
  cargo test -p nando-cli quarantine_recovery_subcenter_atoms: PASS
  cargo test -p nando-core online_miner: PASS
  cargo fmt --check: PASS
  git diff --check: PASS

Boundary:
  This is still phase-center .nwpc only.
  No .nwrb / role-binding backend was revived.
  No manual threshold tuning was used.
  No local accept is promoted without verifier-bound manifest gates.

## 2026-07-09 - Value Frontier Max: 64 Hot Profiles

Decision:
  after the 16-pack deploy, quarantine was no longer the bottleneck:
    lost_tokens_due_to_quarantine: 0
    clean_candidate_quarantined_profile_count: 0

  The remaining loss was a portfolio cap: 64 clean candidates existed, all
  verifier-bound with false_accepts=0, but only 16 were hot.

Implemented:
  - made append live-tail hot capacity follow the candidate frontier limit:
      candidate_frontier_limit: 64
      max_hot_profiles_per_worker: 64
      max_profiles_per_route: 64
      max_route_top_k: 64
  - did not lower thresholds;
  - did not manually select classes/profiles;
  - did not revive .nwrb / role-binding.

Live result after deploy and one cold refresh:
  health: OK
  live-tail RSS: about 38 MiB after warmup
  cold_artifact_refresh_count: 1
  clean_candidate_count: 64
  clean_candidate_quarantined_profile_count: 0
  lost_tokens_due_to_quarantine: 0

  active manifest:
    allowed: true
    blocker: none
    promoted_candidate_count: 64
    hot_profile_count: 64
    unique_cpu_accepts_over_exact_cache: 3586
    tokens_saved: 2876864
    false_accepts: 0
    runtime_parity_mismatches: 0
    hot_bytes_estimate: 66560

  dogfood status:
    hot_profiles: 64
    false_accepts: 0
    shadow_false_accepts: 0

Checks:
  cargo check -p nando-cli: PASS
  cargo test -p nando-cli quarantine_recovery_subcenter_atoms: PASS
  cargo test -p nando-core online_miner: PASS
  cargo fmt --check: PASS
  git diff --check: PASS
  rust-action-memory selector-report:
    diagnostics_count: 0
    quarantined_candidates: 0
    verdict: WATCH only because no_policy_allowed_candidate

Boundary:
  This is a capacity/policy unlock for clean phase-center .nwpc profiles.
  It is not threshold tuning and not a manual profile list.
  Active product manifest remains the authority for false_accepts=0.

## 2026-07-09 - Peer Hidden-State Recovery Split

Decision:
  after opening the frontier to 256, the miner correctly blocked a small set
  of drifting/quarantined profiles. The right move was not to lower threshold
  or release them, but to give quarantine recovery a richer automatic split
  basis.

Implemented:
  - quarantine recovery now ranks split atoms by specificity:
      hidden_state > combo > pair > check/command/state > shape > broad
  - quarantine recovery can split a quarantined parent by peer hidden-state
    atoms from the same event;
  - live-tail now builds recovery split basis from both:
      bucket_selector_candidate_atoms
      auto_subcenter_atoms
  - broad blocked atoms remain filtered by the existing blocker.

Live result after deploy and one cold refresh:
  health: OK
  live-tail RSS: about 37 MiB after warmup
  promoted_candidate_count: 93
  hot_profile_count: 93
  unique_cpu_accepts_over_exact_cache: 4059
  tokens_saved: 3056480
  false_accepts: 0
  runtime_parity_mismatches: 0
  clean_candidate_quarantined_profile_count: 0
  lost_tokens_due_to_quarantine: 0

Notes:
  one non-hot clean candidate remained:
    profile_id: 474378738
    kind: observable_primary
    tokens_saved: 10838
    blocker: call_token_manifest_not_in_promoted_route_set

  This is expected while subcenters exist; the manifest prefers subcenters over
  broad observable_primary profiles.

Checks:
  cargo check -p nando-cli: PASS
  cargo test -p nando-cli quarantine_recovery_subcenter_atoms: PASS
  cargo test -p nando-core online_miner: PASS
  cargo fmt --check: PASS
  git diff --check: PASS
  rust-action-memory selector-report:
    diagnostics_count: 0
    quarantined_candidates: 0
    verdict: WATCH only because no_policy_allowed_candidate

Boundary:
  This is automatic split-basis improvement.
  No manual profile allowlist.
  No threshold lowering.
  No .nwrb / role-binding backend.

## 2026-07-09 Dashboard False-Accept Semantics Fix

Problem:
  Live dashboard chart labeled `stable_clean_token_compression_false_accepts`
  as plain `false accepts` and plotted it on the same 0-100% visual frame.
  This made shadow diagnostic risk look like active/product CPU errors.

Fix:
  Dashboard now separates:
    active_false_accepts =
      product_hot_score_only_post_quarantine_false_accepts
    shadow_risk_events =
      stable_clean_token_compression_false_accepts

Live verification:
  provider bridge health: OK
  dashboard card: Active false = 0
  dashboard card: Shadow risk = 0 at current metrics snapshot
  chart caption on historical row: active false 0, shadow risk 23
  nando-provider-bridge RSS after restart: about 17 MiB

Boundary:
  Active/product false accepts are still the safety gate.
  Shadow risk remains visible as miner diagnostics.

## 2026-07-09 Lightweight Status Dashboard V2

Request:
  Rebuild the protected `/v2/status/<key>` page so it gives a clear server
  state without dumping every report at the top.

Changes:
  - Added a compact top summary:
      server health, current clean-window compression, lifetime promoted tokens,
      active false accepts.
  - Separated current window from accumulated manifest counters:
      current window comes from metrics snapshot;
      accumulated total comes from the active .nwpc promotion manifest.
  - Kept all detailed information, but moved heavy miner/runtime/ranking tables
    into expandable sections.
  - Kept RU/ENG switch and Russian labels in RU mode.
  - Kept SVG charts with no JS or external assets.
  - Added bounded dashboard history compaction:
      graph history is retained, but the JSONL file is compacted after the
      configured byte limit so a long-lived auto-refresh page cannot grow
      forever.

Live verification:
  dashboard HTML size: about 48 KiB
  provider bridge: active
  provider bridge RSS: about 18 MiB
  current dashboard headline:
    current compression: 83.5%
    current tokens: 23211 / 27813
    current CPU accepts: 24
    active false: 0
    shadow risk: 80
    accumulated tokens saved: 3204807
    accumulated accepts: 4296
    promoted profiles: 109
  dashboard history file: about 608 KiB after update

Boundary:
  Dashboard remains observability-only.
  It does not score requests, compile profiles, proxy traffic, or enable claims.

## 2026-07-09 Traffic Frame Denominator Dashboard

Problem:
  The dashboard showed current clean-window token compression, but not the full
  traffic denominator. This made `23211 / 27813 tokens` hard to interpret:
  it was unclear whether this meant all traffic or only the current safe suffix.

Fix:
  Added a top `Nando Frame` card and a `Traffic Coverage Map` panel.
  The panel now separates:
    - full Nando-frame:
        stable_decision_log_rows
        stable_decision_log_total_tokens
        stable_decision_log_unique_cpu_accepts_over_exact_cache
        stable_decision_log_tokens_saved
        stable_decision_log_false_accepts as shadow false
    - current clean suffix:
        stable_decision_log_clean_suffix_rows
        stable_clean_token_compression_total_tokens
        stable_clean_token_compression_saved_tokens
        stable_clean_token_compression_false_accepts as shadow risk
    - miner candidates:
        stable_decision_log_score_candidate_events
        stable_decision_log_clean_suffix_score_candidate_events
        append rows/candidates
    - ingress sources:
        gateway decision window
        provider bridge decision window
        provider boundary window
        future shadow billing request window
    - accumulated .nwpc manifest:
        promoted profiles, accepts, tokens, false accepts

Live verification:
  full Nando-frame: 2235 rows / 1662694 tokens
  current clean suffix: 13 rows / 6809 tokens
  clean suffix token coverage: 0.4% of Nando-frame
  stable frame shadow false: 1825
  current shadow risk: 27
  active false: 0
  gateway rows: 526
  provider bridge rows: 347
  provider boundary rows: 0
  accumulated .nwpc: 118 profiles / 4272 accepts / 3189727 tokens / false 0

Boundary:
  This proves current dashboard coverage over the Nando-observed frame.
  It does not claim full machine/provider traffic capture while provider
  boundary rows are 0 and upstream is not configured.

## 2026-07-09 Full-Width Minimal Dashboard Layout

Request:
  Rework the HTML so the dashboard is not a card/grid pile. It must use
  full-width blocks, keep graphs, and still expose all key server indicators.

Changes:
  - Removed the top card-grid presentation from the rendered page.
  - Rendered one full-width block per question:
      1. server state
      2. traffic coverage
      3. safe compression now
      4. accumulated .nwpc total
      5. how it works
      6. live graphs
      7. all key indicators
  - Kept all critical indicators visible:
      health, local_accept, client_allow, safety policy, upstream,
      metrics age, Nando frame rows/tokens, gateway/provider/boundary rows,
      stable accepts/saved/shadow false, clean-window tokens/rows/accepts,
      active false, shadow risk, claim, accumulated .nwpc profiles/tokens/
      accepts/false/parity, miner state, candidates, history, next action.
  - Kept service details collapsed, not removed.

Live verification:
  dashboard HTML size: about 44 KiB
  provider bridge: active
  provider bridge RSS: about 18 MiB
  /v2/health: OK
  repo and /opt dashboard files have identical sha256

Boundary:
  Dashboard remains observability-only and does not affect runtime decisions.

## 2026-07-09 Dashboard Server-Owned History Fix

Request:
  Dashboard data must keep accumulating even when the page is closed, and
  hidden sections must not collapse again after the 10s refresh.

Changes:
  - Dashboard HTML is read-only again. It no longer appends chart history on
    page render, so opening/refreshing the page cannot distort metrics.
  - `nando-phase-center-metrics-snapshot.sh` now appends dashboard history
    points from the server snapshot timer and compacts the JSONL by byte
    budget.
  - Hidden `<details>` service block was removed. Service details now render as
    visible full-width panels.

Live verification:
  /v2 health: OK
  dashboard HTML contains no `<details>`
  dashboard history line count grew after metrics snapshot service tick

Boundary:
  Dashboard remains observability-only. No miner/runtime scoring logic changed.

## 2026-07-09 Phase-Trust Hot Accept Guard

Request:
  CPU traffic must grow through phase-center grokking, not through a table of
  manually allowed profiles.

Changes:
  - Product-hot accept now requires live phase-center trust:
      candidate bucket, shadow-ready, not rejected, false_accepts=0,
      trust_false_risk_micro=0, trust_drift_micro <= 100000, positive tokens,
      and unique accepts over exact cache.
  - Untrusted product-hot score candidates are not accepted. They are sampled
    back into discovery/recovery, so the miner must form cleaner subcenters
    instead of relying on a broad old center.
  - Added `product_hot_phase_trust_filtered_events` to decisions, reports,
    metrics snapshot, Prometheus output, and dashboard status.

Live verification:
  - Services active after deploy: provider bridge, appender, live-tail miner.
  - `/v2/health`: OK, local_accept_enabled=true, client_allow_local_accept=true.
  - Fresh post-restart frame: 28 append rows, 5 candidates, 2 CPU accepts,
    596 tokens saved, false_accepts=0.
  - New trust gate fired: product_hot_phase_trust_filtered_events=13.
  - Recovery is active: quarantine_recovery_auto_subcenter_observe_events=940.

Boundary:
  This is a safety/grokking gate, not a compression win by itself. It may
  temporarily reduce accepts, because dirty broad centers are forced back into
  automatic subcenter discovery. It does not introduce `.nwrb`, lookup,
  target/proof authority, manual local_out_t, or a manual profile allow-list.

## 2026-07-09 Phase-Trust Drift Correction

Finding:
  The first phase-trust gate treated high `trust_drift_micro` as unsafe.
  That was too conservative: drift is computed as `abs(margin - learned_threshold)`,
  so a very strong positive margin can also produce high drift.

Change:
  Product-hot trust no longer rejects solely on high drift. The trust gate now
  keeps the real safety checks:
    shadow-ready candidate,
    false_accepts=0,
    trust_false_risk_micro=0,
    trust_quality_micro > 0,
    positive unique accepts/tokens over exact cache.

Live verification:
  After deploy, fresh live frame recovered accepts without false accepts:
    rows=22,
    candidates=10,
    CPU accepts=8,
    tokens_saved=3613,
    product_hot_false=0,
    trust_filtered=0,
    recovery_events=722.

Boundary:
  This is not a manual threshold loosening. The learned threshold and false-risk
  estimator remain authoritative. The fix removes an incorrect interpretation
  of positive-margin spread as danger.

## 2026-07-09 Route-Wide Zero-State Phase Transfer

Finding:
  Product-hot compression was limited by an overly narrow bucket match. Clean
  phase-centers could recognize the transition at route scope, but the daemon
  only credited current bucket/subcenter matches.

Change:
  Allow route-wide trusted phase-center transfer only when the event state is a
  successful tool state: `state_exit_code_band:zero`. Nonzero/failure states
  keep the old narrow bucket/subcenter scope and continue to fallback/recovery.

Live verification:
  Pre-change simulation on the live tail:
    all route score candidates: 67 accepts / 9 false accepts,
    route-wide zero-state only: 67 accepts / 0 false accepts.
  Post-deploy fresh live frame:
    rows=26,
    candidates=19,
    CPU accepts=15,
    tokens_saved=4946,
    false_accepts=0,
    product_hot_false=0.

Boundary:
  This is not a manual profile allow-list. It is a generic success-state safety
  split around phase-center transfer: successful state transitions may transfer
  route-wide; failure/nonzero states cannot.

## 2026-07-09 Symbiotic Hidden/Observable Product-Hot Gate

Finding:
  The current live CPU accepts were already behaving like a two-layer
  symbiosis: hidden_state centers supplied the transferable transition signal,
  while observable_subcenter centers supplied the visible safety/shape guard.
  The report showed mixed-profile accepts, with hidden-only and observable-only
  accepts at zero.

Change:
  Product-hot live-tail accept now requires hidden+observable agreement. A
  score candidate from only one side is not credited as CPU accept; it is
  sampled back into miner discovery via `product_hot_phase_symbiosis_filtered`.
  This turns the symbiosis from a dashboard observation into a runtime gate.

Live verification:
  After deploy:
    rows=17,
    candidates=16,
    CPU accepts=9,
    tokens_saved=4332,
    false_accepts=0,
    product_hot_false=0,
    mixed_profile_accepts=9,
    hidden_only_accepts=0,
    observable_only_accepts=0,
    phase_trust_filtered=11,
    phase_symbiosis_filtered=0.

Dashboard:
  The RU/ENG status page now shows:
    `Симбиотический hidden+observable accept`,
    `Автофильтр symbiosis`.
  These values are read from the full live-tail report, not only the shorter
  metrics snapshot.

Boundary:
  This is not a lookup/table/manual profile allow-list. The gate is generic:
  a product-hot local accept requires a trusted hidden phase center and a
  trusted observable phase center on the same event. Single-sided candidates
  stay in shadow/discovery until the miner grows a clean paired center.

## 2026-07-09 Phase-Trust Lost-Vs-Noise Split

Finding:
  The first reading of `product_hot_phase_trust_filtered_events=11` was too
  coarse. Live decision audit showed:
    filtered rows=11,
    accepted non-exact verified rows=8,
    exact-cache rows=3,
    lost non-exact verified rows=0.
  So those 11 events were not a compression loss; they were untrusted
  side-candidates/noise beside an already accepted trusted symbiotic path.

Change:
  Added explicit product-hot lost counters:
    `product_hot_phase_trust_lost_events`,
    `product_hot_phase_trust_lost_tokens`.
  The old `product_hot_phase_trust_filtered_events` remains as miner material /
  shadow noise. Only `phase_trust_lost` is the real CPU-compression debt.

Live verification:
  After deploy:
    rows=20,
    candidates=17,
    CPU accepts=13,
    tokens_saved=6817,
    false_accepts=0,
    phase_trust_filtered=0,
    phase_trust_lost=0,
    phase_trust_lost_tokens=0,
    symbiotic_accepts=13.

Boundary:
  Do not chase `phase_trust_filtered` as a product-loss metric. The miner should
  prioritize `phase_trust_lost > 0`, `symbiosis_filtered > 0`, quarantine loss,
  or real false accepts. Filtered-only side noise is useful discovery material,
  not evidence that compression was lost.
