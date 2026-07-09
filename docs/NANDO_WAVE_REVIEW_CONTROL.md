# Nando Wave Review Control

Дата среза: 2026-07-06, обновление docs-sync

Назначение: короткий контрольный пульт проверяющего. Этот файл ведется в трех
слоях:

```text
TREE
  механизм / путь сигнала / что именно проверяется

SCOREBOARD
  цифры / PASS-WATCH-FAIL / артефакт

DEBT QUEUE
  что мешает продукту / следующий правильный удар
```

Журнал измеримых улучшений ведется отдельно:

```text
docs/NANDO_WAVE_IMPROVEMENT_LEDGER.md
```

Правило: не засчитывать mining-цифры как продуктовую экономию без compatible
denominator. Не называть batch loop потоковым miner-ом.

## Current Authoritative Snapshot

Единый контракт текущей архитектуры:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Короткая правда на этот срез:

```text
best compatible shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  unique_cpu_accepts_over_exact_cache: 6_644 / 29_770 rows
  false_accepts: 0
  wrong_wins: 0
  p99_shadow_latency_ns: 15_938
  local_accept_enabled: false
  product_promotion_allowed: false
  market_money_claim_allowed: false

hot runtime proof:
  PhaseCenterHotRuntime direct benchmark PASS
  hot_bytes_estimate: 592
  warm_metadata_bytes_estimate: 39_136
  false_accepts: 0
  unique_cpu_accepts_over_exact_cache: 8

live-store adapter smoke:
  WATCH because false_accepts: 2
  this is wiring evidence, not promotion evidence
```

Interpretation:

```text
The project crossed CPU20 in shadow on the compatible local trace frontier.
It has not crossed product local_accept or market-money proof.

Next work is automatic streaming selection:
  L4 opportunity board
  marginal-denominator selector
  bounded hot-set admission/eviction
  future-window shadow proof
```

Ключевая архитектурная фиксация по streaming/operator memory:

```text
docs/DRIFTING_PHASE_CENTER_OPERATOR_MEMORY.md
```

Главное правило оттуда: product mechanism is L2-resident mutable phase-center
state; `.nwpc` is snapshot/export only, not the streaming mechanism.

Operator memory is also bounded:

```text
HOT  -> 1-4 active shards per worker
WARM -> bounded process-memory registry
COLD -> disk snapshots only

No unbounded operator store.
No scanning all operators per request.
Route index selects bounded top-K operators.
Admission/eviction budgets are required.
```

## Architecture Audit

Срез: 2026-07-06.

```text
PASS:
  core PhaseCenterOnlineMiner exists and scores before update.
  It uses numeric bucket_id and rejects a bucket on false_accept.

WATCH:
  phase-stream-online-miner-daemon-v1 is a proof/miner daemon:
    JSONL input/report
    String/BTreeMap buckets
    periodic quarantine .nwpc checkpoint compile
    local_accept=false
    auto_promote=false

P0:
  do not mistake that daemon for product hot path.
  Product hot path must be mutable hot phase-center state + bounded operator
  memory + route_id -> top-K numeric profiles.
```

## Hot Path Numeric Atom Boundary

Срез: 2026-07-06.

```text
PASS:
  PhaseCenterAtomEncoder can encode numeric atom ids:
    encode_atom_ids([u64])

  PhaseCenterOnlineMiner can observe numeric atom ids:
    observe_atom_ids(bucket_id, [u64], verifier result, exact-cache flag, tokens, cost)

  Numeric atom ids are source-neutral.
  Codex/agent/log-specific parsing must stay in adapters.

WATCH:
  This is not a product savings claim.
  This only removes raw string/source authority from the core ingestion boundary.

P0:
  Hot/product path must keep using numeric route/profile/bucket/atom ids.
  No JSONL, String bucket_key, BTreeMap, report aggregation, verifier replay, or
  checkpoint compile in PhaseCenterHotRuntime scoring.
```

## Hot Route Plan Boundary

Срез: 2026-07-06.

```text
PASS:
  PhaseCenterOperatorMemory can build a hot route plan:
    route_id -> bounded top-K profile_ids -> PhaseCenterHotRoutePlan

  PhaseCenterHotRoutePlan stores only profile indexes.
  PhaseCenterHotRuntime::score_route_plan_into() scores those indexes into a
  caller-provided decision buffer.

  Missing profile_id in loaded hot runtime is a hard error:
    ProgramIndexOutOfBounds

WATCH:
  This is setup/hot-boundary plumbing, not product savings.
  It does not enable local_accept.

P0:
  request scoring must use prebuilt plan indexes, not scan all operators,
  not resolve String keys, and not read the warm registry per score.
```

## Hot Route Table Boundary

Срез: 2026-07-06.

```text
PASS:
  PhaseCenterOperatorMemory can build a hot route table:
    hot_route_table(&PhaseCenterHotRuntime) -> PhaseCenterHotRouteTable

  PhaseCenterHotRouteTable is sorted by numeric route_id.
  Setup can resolve:
    route_id -> route_index

  Request scoring can use:
    route_index + phase vector + caller buffer
    -> PhaseCenterHotRuntime::score_route_index_into()

  Duplicate route plans are rejected.

WATCH:
  This is still core hot-boundary plumbing.
  It is not real-traffic product savings and does not enable local_accept.

P0:
  serving must cache route_index/plan for hot requests and avoid warm-registry
  reads, String keys, JSON, report aggregation, verifier replay, and .nwpc
  compilation inside request scoring.
```

## Hot Atom-Id Request Scoring Boundary

Срез: 2026-07-06.

```text
PASS:
  PhaseCenterHotRuntime can score a hot request shaped as:
    route_index + u64 atom ids + reusable encoder + caller decision buffer

  API:
    score_route_atom_ids_into(routes, route_index, encoder, atom_ids, out)

  It reuses PhaseCenterAtomEncoder scratch and caller decision Vec.
  It matches score_route_index_into() over the equivalent phase vector.

WATCH:
  Source-specific extraction is still outside core.
  This is not local_accept and not market savings.

P0:
  Hot request boundary must remain numeric:
    no raw text
    no JSONL
    no String bucket_key
    no warm-registry read
    no .nwpc compile
    no verifier replay inside scoring
```

## Hot Candidate Decision Boundary

Срез: 2026-07-06.

```text
PASS:
  Product-facing hot scoring can return:
    PhaseCenterHotCandidateDecision

  API:
    score_route_atom_id_candidates_into(routes, route_index, encoder, atom_ids, out)

  A positive score becomes:
    score_candidate=true
    verifier_required=true
    local_accept=false

  A negative score becomes:
    score_candidate=false
    verifier_required=false
    local_accept=false

WATCH:
  This still does not promote or accept locally.
  It only prevents raw score from being mistaken for accept authority.

P0:
  local_accept may only happen after the separate verifier/promotion/denominator
  gates; hot score alone is never accept authority.
```

## Prepared Hot Request Boundary

Срез: 2026-07-06.

```text
PASS:
  Product-hot request scoring now has a source-neutral prepared request:
    PhaseCenterPreparedHotRequest
    route_index + prepared phase vector
    PhaseCenterHotScratch
    PhaseCenterHotRuntime::score_prepared_hot_request_candidates()

  Local core latency smoke:
    prepared_hot_request_candidate_path_p99_budget
    p99_ns: 448

WATCH:
  Atom-id request scoring is still useful lean adapter plumbing, but not the
  final product-hot latency claim:
    hot_atom_request_candidate_path_adapter_cost_smoke
    p99_ns: 4043

P0:
  The product hot loop must consume prepared phase vectors.
  Raw text, JSONL, String keys, BTreeMap, .nwpc compile, verifier replay, and
  report aggregation remain outside request scoring.
```

## Serving Shadow Replay Prepared-Hot Bridge

Срез: 2026-07-06.

```text
PASS:
  Serving shadow replay commands now load admitted .nwpc packages into
  PhaseCenterHotRuntime and score prepared delta vectors through:
    PhaseCenterPreparedHotRequest
    score_prepared_hot_request_candidates()

  Covered commands:
    phase-stream-phase-atom-serving-shadow-replay-v1
    phase-stream-phase-atom-serving-future-shadow-replay-v1
    phase-stream-phase-atom-serving-append-shadow-replay-v1

  Compile check:
    cargo check -p nando-cli
    PASS

WATCH:
  The replay command still parses JSONL and builds vectors in the adapter/cold
  part. That is acceptable for shadow replay, not final serving.

P0:
  The timed scoring loop must not go back to PhaseCenterOffloadRuntime task
  scoring. Product local_accept remains disabled.
```

## Local Accept Gate Boundary

Срез: 2026-07-06.

```text
PASS:
  Core exposes the only typed local-accept gate:
    PhaseCenterLocalAcceptEvidence::evaluate()

  Required evidence:
    hot score candidate
    verifier_passed=true
    PhaseCenterPromotionEvidence::evaluate() green

  Blockers are typed:
    CandidateAlreadyClaimsLocalAccept
    ScoreNotCandidate
    VerifierRequired
    PromotionBlocked(...)

WATCH:
  This is a safety gate, not a product rollout.
  It does not create market savings and does not auto-promote profiles.

P0:
  PhaseCenterHotCandidateDecision.local_accept must remain false in score output.
  Any attempt to pre-mark a score candidate as local_accept is blocked.
```

## Runtime Budget Snapshot Boundary

Срез: 2026-07-06.

```text
PASS:
  Core now exposes:
    PhaseCenterRuntimeBudgetSnapshot
    PhaseCenterOperatorMemory::runtime_budget_snapshot()

  The snapshot reports:
    HOT profile count
    HOT route count
    HOT route-profile edges
    HOT runtime bytes
    HOT route-table bytes
    HOT total bytes
    WARM route count
    WARM profile count
    WARM metadata bytes
    WARM runtime bytes
    WARM total bytes

  It also reports explicit budget booleans:
    hot_profile_budget_passed
    hot_byte_budget_passed
    warm_profile_budget_passed
    product_runtime_budget_passed()

  PhaseCenterOperatorMemoryConfig now carries:
    max_hot_bytes_per_worker

WATCH:
  This is core accounting, not a live product residency claim.
  Product daemon must emit the snapshot from the real loaded hot runtime and
  warm operator memory.

Check:
  cargo test -p nando-core runtime_budget_snapshot --lib -- --nocapture
    PASS
  cargo check -p nando-core
    PASS
  cargo check -p nando-cli
    PASS
```

## Serving Shadow Runtime Budget Report

Срез: 2026-07-06.

```text
PASS:
  The prepared-hot shadow replay report now includes:
    runtime_budget
    profiles[].runtime_budget

  Aggregate runtime_budget describes the loaded shadow registry:
    snapshot_kind: shadow_loaded_hot_runtime_registry
    hot_profile_count
    hot_route_count
    hot_route_profile_edges
    hot_runtime_bytes_estimate
    hot_route_table_bytes_estimate
    hot_bytes_estimate
    warm_route_count
    warm_profile_count
    warm_metadata_bytes_estimate
    hot_budget_passed
    product_runtime_budget_passed

  stdout also prints:
    hot_bytes_estimate
    hot_budget_passed

WATCH:
  product_residency_claim_allowed remains false.
  This is shadow loaded-runtime accounting, not live daemon residency.

Check:
  cargo check -p nando-cli
    PASS
  cargo fmt --check -p nando-cli
    PASS
```

## Online Miner Memory Budget Report

Срез: 2026-07-06.

```text
PASS:
  phase-stream-online-miner-daemon-v1 now emits:
    memory_budget
    checkpoints[].active_runtime_bytes_estimate
    buckets[].active_runtime_bytes_estimate
    buckets[].reservoir_events

  memory_budget reports:
    snapshot_kind: online_miner_daemon_mutable_bucket_memory
    hot_profile_count
    hot_runtime_bytes_estimate
    hot_bytes_estimate
    warm_profile_count
    warm_metadata_bytes_estimate
    warm_bytes_estimate
    reservoir_events
    reservoir_bytes_estimate
    checkpoint_package_bytes_estimate
    decision_buffer_events
    hot_budget_passed
    product_runtime_budget_passed

WATCH:
  product_residency_claim_allowed remains false.
  This daemon still has JSONL/BTreeMap/decision-log/checkpoint/report machinery.
  It reports memory pressure; it is not the final product hot path.

Check:
  cargo check -p nando-cli
    PASS
```

## Live Operator Store Boundary

Срез: 2026-07-06.

```text
PASS:
  PhaseCenterLiveOperatorStore exists in nando-core.
  It wraps mutable PhaseCenterOnlineMiner state behind route_id/bucket_id
  numeric event APIs.
  It reports runtime_budget_snapshot directly from mutable bucket state.
  It exports verifier-bound candidate packages only after safe shadow accepts.

BOUNDARY:
  no JSONL
  no String bucket_key
  no report aggregation
  no request-path .nwpc compilation
  no local_accept

WATCH:
  Product hot score is still the separate prepared-vector hot runtime path.
  route_id is L4 accounting/routing metadata, not answer authority.
  The store is a live mutable operator boundary, not a market savings claim.

Check:
  cargo test -p nando-core live_operator_store --lib -- --nocapture
```

## Live Store Adapter Smoke

Срез: 2026-07-06.

```text
PASS:
  phase-stream-live-store-adapter-smoke-v1 is wired.
  Cold phase-atom JSONL is converted to numeric route_id/bucket_id/atom_ids.
  PhaseCenterLiveOperatorStore receives observe_atom_event().

SMOKE:
  total_rows: 17000
  parsed_rows: 374
  route_count: 5
  route_bucket_count: 5
  candidate_bucket_count: 1
  unique_cpu_accepts_over_exact_cache: 5
  false_accepts: 23
  promotion_allowed: false
  verdict: LIVE_STORE_ADAPTER_SMOKE_WATCH_FALSE_ACCEPTS

BOUNDARY:
  no local_accept
  no promotion
  no market claim
  no .nwrb
  verifier label supervises shadow; it is not a score atom

WATCH:
  Safety is red for promotion.
  Next action: split/calibrate route buckets before any candidate promotion.
```

## TREE

```text
real agent traces
|
+-- V26 auto-subcenter discovery
|   |
|   +-- input: V24 active-turn + V25 command-result traces
|   +-- automatic split atoms
|   +-- automatic background negatives
|   +-- output: generated candidate JSONL
|   +-- status: batch bridge, not streaming miner
|
+-- V26 quarantine mining
|   |
|   +-- candidate JSONL
|   +-- compile quarantine .nwpc candidates
|   +-- shadow score
|   +-- local_accept: false
|   +-- market_money_claim_allowed: false
|
+-- V26 compatible denominator replay
|   |
|   +-- replay accepted quarantine .nwpc profiles
|   +-- dedupe against compatible denominator
|   +-- count unique CPU accepts over exact cache
|   +-- false_accepts gate
|
+-- required next rung
    |
    +-- phase-stream-online-miner-daemon-v1
        |
        +-- append-only stream
        +-- online bucket stats
        +-- online positive reservoirs
        +-- online background/negative reservoirs
        +-- periodic quarantine .nwpc emission
        +-- learn on past stream
        +-- score future events only
        +-- compatible denominator delta
        +-- status: implemented as shadow-only daemon, low coverage
```

## SCOREBOARD

```text
V26 auto-subcenter discovery                       [PASS as batch bridge]
  report:
    target/nando-wave/streaming/auto-subcenter-discovery-v26.report.json
  rows_seen:                                      23_063
  eligible_rows:                                  18_063
  enumerated_split_atoms:                            196
  selected_candidates:                                48
  rejected_candidates:                                90
  generated_candidate_rows:                       109_300
  positive_rows:                                   54_650
  background_rows:                                 54_650
  local_accept_enabled:                             false
  market_money_claim_allowed:                       false

V26 quarantine mining                              [PASS as shadow batch]
  report:
    target/nando-wave/streaming/phase-atom-live-self-mining-loop-v26-auto-subcenter-top128.report.json
  compiled_quarantine_candidates:                      32
  shadow_accepted_candidates:                          25
  aggregate_unique_accepts_before_denominator:      1_643
  aggregate_tokens_saved_before_denominator:    2_639_909
  local_accept_enabled:                             false
  market_money_claim_allowed:                       false

V26 compatible denominator                          [PASS shadow-only / low product delta]
  report:
    target/nando-wave/streaming/phase-atom-compatible-denominator-shadow-v26-auto-subcenter.report.json
  denominator_total_rows:                        109_300
  routed_events:                                  60_000
  heldout_routed_events:                          12_000
  profile_count:                                      25
  local_operator_shadow_decisions:                 1_787
  denominator_deduped_unique_accepts:                328
  calls_saved_milli:                                   3
  calls_saved_pct:                                  ~0.3%
  nando_cpu_tokens_saved:                        750_413
  false_accepts:                                       0
  wrong_wins:                                        145
  p99_latency_ns:                                 17_567
  local_accept_enabled:                             false
  market_money_claim_allowed:                       false

Online miner daemon v1                             [PASS shadow-only / low coverage]
  required command/artifact family:
    phase-stream-online-miner-daemon-v1
  report:
    target/nando-wave/streaming/phase-stream-online-miner-daemon-v1-realtrace-safe.report.json
  mode:
    bounded_append_only_online_phase_center_miner_shadow_only
  stream contract:
    append_only_input:                            true
    score_before_train:                           true
    future_only_shadow_scoring:                   true
    incremental_bucket_updates:                   true
    positive_negative_reservoirs:                 true
    periodic_quarantine_nwpc_compile:             true
  total_rows:                                     17_000
  parsed_events:                                     374
  skipped_no_verifier_label:                      16_626
  bucket_count:                                        5
  compiled_checkpoint_count:                          76
  active_profile_count:                                5
  future_shadow_events:                              313
  local_operator_shadow_decisions:                     2
  fallback_shadow_decisions:                         311
  unique_cpu_accepts_over_exact_cache:                 2
  tokens_saved:                                      642
  cost_saved_microusd:                             1_926
  false_accepts:                                      0
  wrong_wins:                                        97
  runtime_margin_parity_mismatches:                   0
  p99_latency_ns:                                15_494
  local_accept_enabled:                            false
  market_money_claim_allowed:                      false
  verdict:
    PHASE_STREAM_ONLINE_MINER_DAEMON_V1_PASS_SHADOW_ONLY

Online miner unsafe comparison                     [WATCH / useful negative evidence]
  report:
    target/nando-wave/streaming/phase-stream-online-miner-daemon-v1-realtrace-smoke.report.json
  local_operator_shadow_decisions:                    18
  unique_cpu_accepts_over_exact_cache:                18
  false_accepts:                                       8
  verdict:
    PHASE_STREAM_ONLINE_MINER_DAEMON_V1_WATCH_FALSE_ACCEPTS
  meaning:
    lower threshold gives more accepts but unsafe; safe threshold is required

Online miner tool_status safe155                   [RUNNING / WATCH manual-threshold probe]
  command:
    phase-stream-online-miner-daemon-v1 ... toolstatus-safe155 ... 32 20 155000 500 32 400 ...
  input:
    target/nando-wave/streaming/codex-session-tool-status-verifier-trace-v4-l4-packer.jsonl
  meaning:
    this is an online-daemon threshold/safety probe for tool_status, not V26 batch topN
  current concern:
    threshold 155000 is manually chosen
  allowed only if:
    it is reported as calibration evidence
    false_accepts remains 0
    result is compared against previous toolstatus-safe run
    next step turns threshold choice into automatic calibration/policy
  forbidden:
    do not present hand-picked safe155 as product CPU compression claim

Online threshold policy                            [BLOCKING]
  problem:
    streaming miner goal is automatic online discovery, but safe155 still uses
    a manually supplied numeric threshold
  hard rule:
    manual threshold sweeps are diagnostics only
  PASS requires:
    threshold_source: automatic_train_false_margin_calibration
    calibration_window_before_shadow: true
    shadow_window_after_calibration: true
    per_bucket_thresholds_reported: true
    accepted_delta_reported_against_fixed_policy: true
    false_accepts: 0
  WATCH if:
    threshold is chosen from CLI number or post-hoc trial result
  FAIL if:
    manual threshold result is presented as product compression

Repo integration hygiene                            [WATCH]
  issue:
    phase_streaming_cmd.rs and phase_streaming_cmd modules are currently untracked
  impact:
    compile can pass, but final integration PASS needs tracked/committed boundary

Core online phase-center miner API                 [PASS core smoke / not product complete]
  files:
    crates/nando-core/src/wave/phase_center_runtime.rs
    crates/nando-core/src/wave.rs
  mechanism:
    in-memory PhaseCenterOnlineMiner
    observe(event) scores before adding the current event
    positive/negative phase sums update incrementally
    threshold is raised from calibration false margins
    bucket rejects itself on verified false accept after calibration
    candidate_runtime(bucket_id) emits PhaseCenterFlatRuntime only for safe
    buckets with unique accepts over exact cache
  forbidden path status:
    no .nwrb
    no role-binding backend
    no target/proof authority
    no concrete lookup
    no manual local_out_t
    no product local_accept
  checks:
    cargo test -p nando-core online_miner --lib -- --nocapture
    cargo check -p nando-core
  result:
    online_miner_learns_then_scores_future_events: ok
    online_miner_rejects_bucket_after_verified_false_accept: ok
    nando-core check: ok
  boundary:
    This is the small core learn-then-shadow mechanism, not the old heavy
    daemon path and not a market savings claim.

Lean event adapter and hot runtime split           [PASS core smoke / WATCH latency proof]
  files:
    crates/nando-core/src/wave/phase_center_runtime.rs
    crates/nando-core/src/wave.rs
  added:
    PhaseCenterAtomEncoder
    PhaseCenterOnlineEvent
    PhaseCenterOnlineCandidatePackage
    PhaseCenterOnlineMiner::observe_atoms()
    PhaseCenterOnlineMiner::observe_event()
    PhaseCenterOnlineMiner::observe_events_into()
    PhaseCenterOnlineMiner::candidate_hot_runtime()
    PhaseCenterOnlineMiner::candidate_package_bytes()
    PhaseCenterOnlineMiner::candidate_package_bytes_with_verifier()
    PhaseCenterOnlineMiner::candidate_packages_into()
    PhaseCenterOnlineMiner::candidate_packages_into_with_verifier()
    PhaseCenterHotRuntime::resolve_profile_index()
    PhaseCenterHotRuntime::profile_id_at()
    PhaseCenterVerifierBinding
    PhaseCenterPromotionEvidence
    PhaseCenterPromotionDecision
    PhaseCenterPromotionBlocker
    PhaseCenterThresholdPolicyEvidence
    PhaseCenterOperatorMemory
    PhaseCenterOperatorMemoryConfig
    PhaseCenterOperatorAdmission
    PhaseCenterSavingsEvidence
    PhaseCenterSavingsDenominator
    PhaseCenterSavingsReport
  hot path contract:
    route/profile ids are numeric
    phase vector is already prepared
    route/profile id is resolved to a usize index outside the score loop
    atom encoding lives in adapter/cold boundary, not hot score loop
    source adapters can feed safe atoms through observe_atoms()
    PhaseCenterHotRuntime::score_profile() does no JSON/file/report/verifier work
    cold path emits package bytes outside the event loop
    safe online buckets can be exported to PhaseCenterHotRuntime without IO
  promotion contract:
    eligible only if future shadow exists, unique CPU accepts over exact cache
    are positive, tokens/cost savings are positive, false_accepts=0,
    runtime parity mismatches=0, a concrete verifier binding exists,
    exact-cache overlap was excluded, token/cost denominator exists, threshold
    policy is backed by automatic calibration false-margin evidence, and
    product local_accept is still disabled.
  verifier package contract:
    plain candidate_package_bytes() is diagnostic/quarantine export
    promotion path must use candidate_package_bytes_with_verifier()
    verifier binding is numeric/source-neutral and requires
    false_accept_threshold=0
  threshold contract:
    manual threshold numbers are diagnostic only
    product promotion requires per-bucket calibration false-margin evidence
    threshold policy evidence is typed in core, not inferred from CLI text
  bounded memory contract:
    promoted operators enter a bounded source-neutral memory, not an unbounded
    store
    route_id selects bounded top-K profiles before scoring
    admission requires promotion evidence, min token value, accept rate, and
    false_accepts=0
  economics contract:
    raw unique accepts are not a market claim
    calls/tokens/money savings require total denominator, exact-cache baseline,
    non-synthetic trace, provider billing evidence, and false_accepts=0
  checks:
    cargo test -p nando-core atom_encoder --lib -- --nocapture
    cargo test -p nando-core online_atom_adapter --lib -- --nocapture
    cargo test -p nando-core online_miner_exports_only_safe_buckets_to_hot_runtime --lib -- --nocapture
    cargo test -p nando-core promotion_gate --lib -- --nocapture
    cargo test -p nando-core online_event_adapter --lib -- --nocapture
    cargo test -p nando-core online_stream_api_reuses_caller_buffers --lib -- --nocapture
    cargo test -p nando-core hot_runtime_scores_numeric_profile_without_cold_path --lib -- --nocapture
  result:
    atom_encoder_matches_allocating_phase_vector_and_reuses_scratch: ok
    online_atom_adapter_learns_then_emits_candidate_package: ok
    online_miner_exports_only_safe_buckets_to_hot_runtime: ok
    promotion_gate_allows_only_verified_future_shadow_savings: ok
    promotion_gate_blocks_unsafe_or_unproven_candidates: ok
    online_miner_reports_threshold_policy_evidence: ok
    operator_memory_admits_only_promoted_profiles: ok
    operator_memory_bounds_route_top_k_and_warm_profiles: ok
    savings_report_requires_real_denominator_and_provider_costs: ok
    savings_report_blocks_synthetic_or_unsafe_claims: ok
    online_event_adapter_emits_verifier_bound_nwpc_package: ok
    online_stream_api_reuses_caller_buffers: ok
    hot_runtime_scores_numeric_profile_without_cold_path: ok
  latency gate:
    hot_runtime_numeric_score_path_p99_budget is present as ignored release
    budget check; it must be run separately for product latency evidence.
  local core latency smoke:
    command:
      cargo test -p nando-core hot_runtime_numeric_score_path_p99_budget --lib -- --ignored --nocapture
    p99_score_path_ns: 291
    status: PASS local core smoke, not serving/product claim
```

## DEBT QUEUE

```text
P0 - Build the actual online miner daemon
|
+-- status: first shadow-only implementation exists
+-- keep: append-only scoring before update
+-- keep: periodic .nwpc checkpoints
+-- keep: local_accept=false
+-- correction:
|   +-- do not polish heavy JSONL/report daemon as product hot path
|   +-- keep it as proof/cold path only
+-- next:
    +-- wire core PhaseCenterOnlineMiner into a lean stream adapter
    +-- no JSON/BTreeMap/string bucket keys in product hot path
    +-- emit .nwpc outside the event loop only after false_accepts=0

P0 - Split proof/cold path from product/hot path
|
+-- rule:
|   +-- cold path can parse JSONL, write reports, calibrate, verify, and emit .nwpc
|   +-- hot path must be numeric-only score/accept/fallback
+-- implemented core pieces:
|   +-- PhaseCenterOnlineEvent for normalized stream events
|   +-- PhaseCenterOnlineCandidatePackage for quarantine .nwpc bytes
|   +-- PhaseCenterHotRuntime for profile_id + phase vector scoring
+-- next:
    +-- add a release latency artifact for PhaseCenterHotRuntime
    +-- target p99 <= 1us, preferred 200-800ns
    +-- keep product numbers separate from proof/cold diagnostics

P0 - Expand online daemon coverage without false accepts
|
+-- current safe accepts: 2
+-- current future events: 313
+-- current skipped_no_verifier_label: 16_626 / 17_000 rows
+-- main blocker:
|   +-- most rows still have no verifier label for online mining
+-- next:
    +-- feed richer verifier-bearing traces into daemon
    +-- add per-bucket marginal-denominator scoring
    +-- do not lower threshold if false_accepts appears

P0 - Remove manual safety-threshold dependence
|
+-- current live run uses manual base_margin_threshold_micro=155000
+-- acceptable as one calibration probe
+-- not acceptable as final mechanism
+-- next:
    +-- threshold must come from train false-margin calibration
    +-- report must show why chosen threshold is minimal-safe
    +-- compare accepted delta vs false_accepts across fixed policy, not hand tuning
    +-- final online miner command must not require hand-picked safety threshold

P0 - Stop counting batch mining as product compression
|
+-- 1_643 before denominator is useful diagnostics
+-- 328 after denominator is the honest V26 product-facing number
+-- calls_saved_milli=3 is not CPU20

P0 - Reduce denominator collapse
|
+-- problem: 1_643 mining accepts -> 328 denominator accepts
+-- likely causes:
|   +-- duplicate/overlapping profiles
|   +-- candidates with high internal mining score but low denominator reach
|   +-- profile selection not denominator-aware enough
+-- next:
    +-- select by marginal denominator delta, not only class-local score

P0 - Clarify denominator language in online daemon
|
+-- report says compatible_denominator_delta_in_same_pass: true
+-- current implementation counts future unique accepts over exact cache in the daemon
+-- missing:
    +-- separate compatible-denominator replay artifact comparable to V26 denominator replay
    +-- explicit calls_saved_milli in online report

P1 - Keep generic streaming architecture
|
+-- no source agent as decision authority
+-- no Codex-only hardcode in generic core
+-- adapters may parse Codex traces
+-- core must consume universal event/state/action/result atoms

P1 - Continue spectral split
|
+-- parent phase_streaming_cmd.rs is still very large
+-- move-only refactor only
+-- do not mix module split with scoring changes
+-- preserve reports/commands/schemas

P1 - Track integration state
|
+-- decide whether untracked Rust files are intentional R&D boundary or must be added
+-- do not call final PASS while key implementation files are untracked
```

## Current Reviewer Verdict

```text
V26 is useful and worth keeping.
It proves automatic batch discovery/mining with clean safety flags.

Online miner v1 now exists as a real shadow-only daemon rung.
It scores future events after past checkpoints and keeps false_accepts=0
in the safe run.

But:
  online safe coverage is tiny: 2 unique accepts
  skipped verifier labels dominate: 16_626 / 17_000 rows
  unsafe lower-threshold run produced 8 false accepts
  V26 product-facing denominator delta is small: 328 accepts / calls_saved_milli=3.
  online report still needs a separate denominator/money claim boundary.

Next correct move:
  keep online daemon path
  feed verifier-rich future traces
  optimize by marginal denominator delta
  keep threshold safety first
  no local_accept until product gate
```
