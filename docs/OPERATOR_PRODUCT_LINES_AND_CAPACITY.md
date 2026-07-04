# Operator Product Lines And Capacity

Дата среза: 2026-07-03

Назначение: зафиксировать, какие классы операторов нужны для продуктовой
линейки Nando Wave, сколько примерно operator families нужно на каждом уровне,
и какие текущие матрицы/L3-layout могут это вместить.

Этот файл не вводит новую архитектуру. Это карта требований и пределов.

## Короткий Ответ

```text
R&D proof:              24-32 operator families
CPU offload core:       30-50 operator families
Enterprise workflow:    64-80 operator families
Agent/Codex offload:    96-128 operator families
Platform catalog:       128-256+ operator families, only through shards/packages
```

Техническая граница:

```text
Phase-center runtime:
  может держать сотни и малые тысячи compact action_tree records на CPU.

Strict L3 slot-binding:
  16 slots доказаны и остаются frozen baseline / demo fallback;
  32 slots являются следующим product-facing scaling target;
  32 slots уже имеют 64-page layout smoke, 3-seed smoke, latency smoke и
  order/mixed/conditional multi-seed rungs плюс cache/offload benchmark и
  serialized role-binding `.nwrb` package rung плюс public Rust SDK smoke и
  all-seed compact binary `.nwreb` eval-pack suite плюс release-suite bundle,
  плюс OPERATOR_BLUEPRINT gap audit, плюс EDIT release-suite integration,
  плюс serving-only `.nwrb` profile runtime, worker shards, local/deployed
  LB replay, packed score parity и bounded `/score` throughput gate,
  но deployed cheap-VPS individual POST /score throughput ещё красный;
  64 slots требуют отдельный capacity rung;
  128+ slots нельзя считать одной монолитной L3-матрицей.
```

Ответ на главный вопрос:

```text
Нет, L3 не бесконечен.
CenterId u32 убирает старую адресную стену, но не отменяет:
  edge growth,
  cache budget,
  collision/superposition pressure,
  latency p99,
  false_local_accepts,
  proof maintenance.
```

## Текущая Продуктовая Линейка

Сейчас проектовая линия сместилась от research-only gates к serving-only
profile runtime:

```text
exact cache
  -> route / profile_id
  -> маленький L2-sized `.nwrb` profile shard
  -> local score / margin
  -> local operator accept или fallback_to_llm
```

Главная идея продукта:

```text
не заменить LLM целиком,
а снять с неё повторяемые переносимые действия,
которые exact cache не ловит как точное совпадение.
```

Текущий подтверждённый serving smoke:

```text
verdict: ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS
profiles: 7
endpoints: /health /profiles /score /replay /metrics
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
p99_latency_ns: 21436
runtime_bytes_estimate: 790020
rss_bytes: 10805248
compiler_used: false
eval_packs_loaded_in_serving: false
corpus_jsonl_loaded: false
python_demo_used: false
```

Текущий replay-suite поверх serving runtime:

```text
verdict: ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS
unique_sequences_replayed: 896
http_replay_batches: 224
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
missed_expected_local: 0
p99_latency_ns: 213509
runtime_bytes_estimate: 790020
rss_bytes: 8101888
serving_eval_packs_loaded: false
replay_client_eval_packs_used: true
```

Важная граница replay:

```text
PASS теперь доказан живым rerun без ручного batch override.

Текущие default limits:
  DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE = 128
  DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES = 4

Причина:
  batch=32 падал 413 HTTP request body too large;
  max_unique=256 проходил по reduction/false_accepts, но падал по p99.
```

Текущий fallback smoke:

```text
verdict: ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
local_accept_pass: true
bad_route_fallback_pass: true
low_margin_fallback_pass: true
bad_route_fallback_reason: profile_not_found
low_margin_fallback_reason: margin_below_threshold
local_energy_margin: 4194304
low_margin_energy_margin: 1024
low_margin_threshold: 1000000
local_operator_calls: 1
fallback_to_llm_calls: 2
false_local_accepts: 0
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
```

Это не финальная продуктовая метрика по реальному Codex/API трафику. Это
правильный новый маршрут:

```text
serving worker грузит только `.nwrb` packages;
`.nwreb` eval packs не живут в serving worker;
replay/eval остаются снаружи как проверка;
основной benchmark считается против exact-cache baseline.
```

Текущий product-serving envelope:

```text
Local LB replay:
  verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
  unique_sequences_replayed: 896
  exact_cache_llm_calls: 896
  exact_cache_plus_nando_llm_calls: 448
  incremental_reduction_vs_exact_cache: 500 milli
  false_local_accepts: 0
  load_balancer_p99_latency_ns: 736030
  core_score_p99_latency_ns: 78902
  worker_score_p99_latency_ns: 167663
  packed_score_parity_mismatches: 0 / 647928

Deployed HostWorld LB replay:
  verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
  exact_cache_llm_calls: 896
  exact_cache_plus_nando_llm_calls: 448
  incremental_reduction_vs_exact_cache: 500 milli
  false_local_accepts: 0
  load_balancer_p99_latency_ns: 2993688
  core_score_p99_latency_ns: 187721
  worker_score_p99_latency_ns: 545095
  packed_score_parity_mismatches: 0 / 647928

Local bounded individual POST /score throughput:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 1409548
  core_score_p99_latency_ns: 119908
  worker_score_p99_latency_ns: 305953

Deployed HostWorld bounded individual POST /score throughput:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  core_score_p99_latency_ns: 221243
```

Текущая real-traffic route линия:

```text
Historical strongest verified snapshot:
  feedback_report: cpu-route-feedback-loop-v3.mixed-v2-agent-control-planning-v3.report.json
  verified_cpu_accepts: 17 / 1000
  operator_candidate_calls: 457
  false_accepts: 0

Read-inspect route/profile rung:
  registry: profile-registry-read-inspect-v1.json
  read_inspect_routed_candidates: 33 / 1000
  read_inspect_scoreable_payloads: 12
  read_inspect_profile_edges: 8
  read_inspect_runtime_bytes_estimate: 33000
  output_evidence_report: read-inspect-output-evidence-v1.report.json
  output_evidence_matched_events: 9
  verifier_true_events: 1
  verifier_false_events: 8
  verification_hook_ready_events: 9
  local_accept_calibration_report: read-inspect-local-accept-calibration-v1.report.json
  safe_policy_found: false
  best_safe_true_accepts: 0
  minimum_true_support: 3
  support_qualified: false
  verified_cpu_accepts: 0
  blocker: local_accept_calibration_failed; no safe readout policy

Metrics-report route/profile rung:
  registry: profile-registry-metrics-report-v1.json
  metrics_report_routed_candidates: 55 / 1000
  metrics_report_scoreable_payloads: 42
  metrics_report_profile_edges: 8
  metrics_report_runtime_bytes_estimate: 33000
  output_evidence_report: metrics-report-output-evidence-v1.report.json
  output_evidence_matched_events: 32
  verifier_true_events: 18
  verifier_false_events: 14
  verification_hook_ready_events: 32
  local_accept_calibration_report: metrics-report-local-accept-calibration-v1.report.json
  safe_policy_found: true
  best_safe_true_accepts: 2
  minimum_true_support: 3
  support_qualified: false
  verified_cpu_accepts: 0
  blocker: local_accept_support_insufficient

Metrics-report 5000-row safe-policy soak:
  artifact_root: target/nando-wave/real-traffic-shadow/metrics-report-soak-v1
  trace_rows_written: 5000
  metrics_report_candidate_events: 98
  metrics_report_scoreable_payloads: 63
  output_evidence_matched_events: 51
  verifier_true_events: 31
  verifier_false_events: 20
  selected_policy_name: market_safe_metric_slot_margin_threshold_with_active_fringe_min
  selected_acceptance_policy: first_slot_threshold_active_fringe_min_114
  request_side_policy_name: metrics_report_active_fringe_min_114
  selected_policy_threshold: 393216
  request_side_policy_evaluated_rows: 63
  request_side_policy_accept_rows: 11
  request_side_policy_reject_rows: 52
  nando_shadow_accepts: 3
  verified_safe_accepts: 3
  unverified_shadow_accepts: 0
  false_accepts: 0
  p99_shadow_score_latency_ns: 272859
  market_claim_allowed: true
  interpretation: narrow route PASS for the separate 5000-row soak; do not add to default 1000-row CPU Routability total until the overall feedback window is regenerated with the same denominator

Feedback-loop window guard:
  default_feedback_report: target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
  default_total_llm_calls: 1000
  default_verified_cpu_accepts: 12
  default_audit_window_mismatches: []
  negative_test_report: target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-metrics-soak-window-guard-v1.report.json
  negative_test_input: 1000-row forecast + 5000-row metrics-report audit
  negative_test_result: metrics_report_readout audit excluded_from_feedback=true
  interpretation: route-specific audits with mismatched total_llm_calls cannot inflate CPU Routability

Git-control route/profile/evidence rung:
  base_registry: profile-registry-git-control-v1.json
  promoted_registry: profile-registry-git-control-safe-policy-v1.json
  git_control_candidates: 18 / 1000
  git_control_scoreable_payloads: 12
  git_control_profile_edges: 8
  git_control_runtime_bytes_estimate: 33000
  median_energy_margin: 1240064
  p10_energy_margin: 906240
  output_evidence_report: git-control-output-evidence-v1.report.json
  output_evidence_matched_events: 10
  verifier_true_events: 6
  verifier_false_events: 4
  tool_call_fingerprint_events: 5
  verification_hook_ready_events: 10
  local_accept_calibration_report: git-control-local-accept-calibration-v1.report.json
  safe_policy_found: true
  best_safe_true_accepts: 5
  minimum_true_support: 3
  support_qualified: true
  promoted_trace: git-control-safe-policy-v1.trace.jsonl
  selected_policy_threshold: 1505280
  nando_shadow_accepts: 1
  verified_safe_accepts: 1
  unverified_shadow_accepts: 0
  workspace_mutation_enabled: false
  local_accepts_enabled_in_live_daemon: false
  verified_cpu_accepts: 1
  p99_shadow_score_latency_ns: 273719
  market_claim_boundary: narrow git_control shadow PASS; tool-output fingerprints only; no git execution or workspace mutation; not CPU Routability 80

Serving-ops promoted safe-policy rung:
  registry: profile-registry-serving-ops-safe-policy-v1.json
  promoted_trace: serving-ops-safe-policy-v1.trace.jsonl
  selected_policy: market_safe_energy_margin_threshold
  selected_policy_threshold: 1392640
  candidate_events: 25 / 1000
  scoreable_payloads: 8
  verification_hook_ready_events: 7
  nando_shadow_accepts: 3
  verified_safe_accepts: 3
  false_accepts: 0
  unverified_shadow_accepts: 0
  incremental_savings_over_exact_cache: 3
  p99_shadow_score_latency_ns: 156653
  server_mutation_enabled: false
  market_claim_boundary: narrow route shadow PASS, not CPU Routability 80

Fresh default feedback after read_inspect + metrics_report calibration + git_control safe-policy + serving_ops safe-policy:
  operator_candidate_calls: 427 / 1000
  scoreable_candidate_calls: 167 / 1000
  verification_hook_ready_events: 130
  verified_cpu_accepts: 12 / 1000
  verified_gap_to_80_calls: 788
  market_claim_allowed: false
  git_control_stage: verified_cpu_accept_eligible
  serving_ops_stage: verified_cpu_accept_eligible

Route-gap after serving_ops registry:
  existing_route_candidate_events: 520 / 1000
  no_candidate_events: 480 / 1000
  payload_ready_events: 10
  top_payload_ready_family: uncatalogued
```

Интерпретация:

```text
Wave score-loop и worker score уже sub-ms.
Краснеет не модель, а deployed per-score HTTP/LB/upstream envelope
на дешёвом VPS.
```

Следующая взрослая проверка для этой линейки:

```text
real Codex/API traffic shadow/replay
persistent or binary LB -> worker upstream for deployed POST /score
production-like daemon/metrics/watchdog on server
>=20% incremental LLM-call reduction vs exact cache on real route mix
false_local_accepts = 0
p99 score latency <= 1-3 ms on cheap VPS
```

Real-traffic shadow V1 is now implemented as the measurement path:

```text
role-binding-real-traffic-record-v1:
  append one validated JSONL trace row without changing the live LLM flow.

role-binding-real-traffic-record-serve-v1:
  expose the recorder over local HTTP /health /trace /metrics;
  support request-limit for bounded smoke/watchdog runs.

role-binding-real-traffic-ingest-events-v1:
  convert externally collected agent/API event JSONL into the trace contract;
  keep synthetic/no-candidate event batches in REVIEW.

role-binding-real-traffic-codex-history-ingest-v1:
  convert local Codex prompt history into privacy-safe event fingerprints;
  write no raw prompt text;
  expose the current real baseline where exact cache can be measured but Nando
  cannot accept without a route/candidate adapter.

role-binding-real-traffic-codex-history-route-candidates-v1:
  select route/profile candidates from request-side prompt text only;
  emit empty score payloads so every candidate must fallback;
  expose the current next debt: build active_fringe/slot payload safely.

role-binding-real-traffic-shadow-v1:
  load serving-only `.nwrb` profile registry;
  compute exact-cache baseline;
  compute Nando shadow accepts/fallbacks;
  count verified_safe_accepts and false_local_accepts;
  report latency/RSS;
  rank operator keys by traffic share, verified accept rate, saved cost,
  safety, and p99 runtime.
  PASS requires verified_safe_accepts > 0 and positive incremental savings.

role-binding-real-traffic-shadow-smoke-v1:
  build synthetic smoke traces only;
  synthetic traces force REVIEW and cannot be sold as market savings.
```

Commercial operator selection must now use:

```text
operator_value =
  frequency_in_real_traces
  * local_accept_rate
  * saved_llm_cost
  * safety_score
  / runtime_cost
```

Claim boundary:

```text
Synthetic/release-suite replay доказывает механизм и serving envelope.
Рыночный claim по экономии можно делать только по real traffic:
  total_llm_calls
  exact_cache_hits
  nando_routable_calls
  nando_local_accepts
  fallbacks
  false_local_accepts
  incremental_reduction_vs_exact_cache
```

Текущий worker-shard acceptance:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
worker_count: 2
total_profile_count: 7
profile_split: 4 / 3
total_local_operator_calls: 7
wrong_worker_route_fallbacks: 2
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 6557696
max_worker_p99_latency_ns: 6286
serving_only: true
```

Текущий sharded replay:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
worker_count: 2
total_profile_count: 7
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
incremental_reduction_vs_exact_cache: 500 milli
false_local_accepts: 0
missed_expected_local: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 7135232
max_worker_p99_latency_ns: 265277
serving_only: true
```

## Главная Граница

```text
Nando Wave не должен расти как список из тысячи ручных правил.

Правильная единица:
  compact operator family
  + параметры
  + композиция
  + proof gates

Главная формула:
  state_t + action_tree -> state_t+1
```

Успех считается честным только если сохраняются запреты:

```text
no lookup
no target_id
no proof_rule_id authority
no concrete_x_lookup
no fixed answer template
no fixed frame_id
no manual local_out_t
no hand-coded bind(X)
```

## Два Разных Счётчика

Важно не смешивать:

```text
operator family
  класс переносимого действия:
  reverse_window, extract_field, route_by_compare, normalize_field

concrete operator / action_tree key
  конкретная параметризованная форма, которую компилятор кладёт в runtime:
  select=field.amount + transform=normalize_money + write=field.amount_norm
```

Продуктовые планы ниже считают `operator families`.
Runtime capacity считает `action_tree keys / flat records`.

## Базовая Грамматика

Все операторы должны раскладываться в 5 измерений:

```text
Operator = SELECT + TRANSFORM + WRITE + CONDITION + CHECK
```

Текущий V5 coverage_action уже доказывает разнообразие по этим измерениям:

```text
rows: 360
train / heldout: 180 / 180
action_tree_key_count: 30
select / transform / write / condition / check: 6 / 10 / 5 / 5 / 10
min_dimension_value_count: 5
full_operator_dimension_coverage_pass: true
```

## Все Нужные Классы Операторов

```text
STATE TRANSITION
|
+-- 1. SELECT
|   +-- select_slot
|   +-- select_span
|   +-- select_field
|   +-- select_by_marker
|   +-- select_by_predicate
|   +-- select_window
|
+-- 2. MOVE / COPY
|   +-- move_slot
|   +-- move_span
|   +-- copy_slot
|   +-- copy_span
|   +-- swap_slots
|   +-- swap_spans
|
+-- 3. EDIT
|   +-- insert
|   +-- delete
|   +-- replace
|   +-- clear
|   +-- append
|   +-- prepend
|
+-- 4. ORDER
|   +-- reverse
|   +-- rotate
|   +-- block_swap
|   +-- window_reverse
|   +-- interleave
|   +-- stable_reorder
|
+-- 5. FIELD
|   +-- extract_field
|   +-- merge_fields
|   +-- split_field
|   +-- normalize_field
|   +-- compare_fields
|
+-- 6. FILTER / GROUP
|   +-- filter_by_predicate
|   +-- partition
|   +-- group_by_key
|   +-- stable_sort_by_key
|   +-- deduplicate
|
+-- 7. CONDITION / ROUTE
|   +-- if_then_else
|   +-- route_by_marker
|   +-- route_by_field
|   +-- route_by_compare
|   +-- route_by_state
|
+-- 8. COMPOSE
|   +-- A_then_B
|   +-- A_then_if_B_else_C
|   +-- repeat_n
|   +-- verify_then_repair
|
+-- 9. VERIFY / REPAIR
    +-- check_same_bag
    +-- check_field_constraint
    +-- check_order_constraint
    +-- check_no_conflict
    +-- repair_unset_slot
    +-- reject_unsettled
```

## Продуктовые Линейки

### Line 0: R&D Proof Kernel

Цель: доказать, что маленькая операторная грамматика переносится без lookup.

```text
target families: 24-32
current target: 30 families
current V5 coverage_action: 30 action_tree keys
```

Распределение:

```text
SELECT:             4
MOVE / COPY:        4
EDIT:               4
ORDER:              6
FIELD:              4
FILTER / GROUP:     3
CONDITION / ROUTE:  3
COMPOSE:            4
VERIFY / REPAIR:    2
total:             34 if all maxed, practical target 30
```

Что должно быть закрыто:

```text
dimension coverage
shortcut gates
strict slot / sequence energy where applicable
flat runtime parity
ablations collapse
multi-seed minimum 3 seeds
```

### Line 1: CPU Offload Core

Цель: первый продаваемый локальный CPU layer для повторяемых action workflows.

```text
target families: 30-50
recommended target: 45
```

Распределение:

```text
SELECT:             6
MOVE / COPY:        6
EDIT:               6
ORDER:              8
FIELD:              6
FILTER / GROUP:     4
CONDITION / ROUTE:  5
COMPOSE:            4
VERIFY / REPAIR:    4
total:             49
```

Продуктовый смысл:

```text
локально закрывать простые state transitions;
снижать calls/tokens поверх cache-enabled baseline;
не принимать low-margin переходы локально.
```

### Line 2: Enterprise Workflow Pack

Цель: документы, заявки, статусы, compliance, ops workflows.

```text
target families: 64-80
recommended target: 72
```

Распределение:

```text
SELECT:             8
MOVE / COPY:        8
EDIT:               8
ORDER:              8
FIELD:             10
FILTER / GROUP:     8
CONDITION / ROUTE:  8
COMPOSE:            6
VERIFY / REPAIR:    8
total:             72
```

Продуктовый смысл:

```text
extract -> normalize -> compare -> route -> update -> verify -> repair/reject
```

### Line 3: Agent / Codex Local-Offload Pack

Цель: разгрузить повторяемые агентские шаги: classify action, route, update
state, verify constraints, reject unsafe local action.

```text
target families: 96-128
recommended target: 112
```

Распределение:

```text
SELECT:            12
MOVE / COPY:       10
EDIT:              14
ORDER:             10
FIELD:             14
FILTER / GROUP:    12
CONDITION / ROUTE: 16
COMPOSE:           12
VERIFY / REPAIR:   12
total:            112
```

Продуктовый смысл:

```text
часть agent workflow исполняется локально на CPU;
LLM вызывается только когда operator layer не уверен или задача выходит
за доказанный class boundary.
```

### Line 4: Platform Catalog

Цель: большая библиотека доменных операторов.

```text
target families: 128-256+
```

Правило:

```text
не класть всё в одну монолитную L3-матрицу;
делать registry / shards / packages по доменам и классам.
```

Примеры shards:

```text
document_ops
workflow_status_ops
code_agent_ops
compliance_ops
network_ops
legal_text_ops
medical_admin_ops
```

## Какие Матрицы Это Вмещают

### Matrix A: Phase-Center Runtime

Это продуктовый flat scorer:

```text
records = action_tree keys
cells = phase center width
current compact point: C32
```

Память примерно:

```text
bytes ~= records * 2 * cells * sizeof(PhaseCenterCell)
PhaseCenterCell = f64 re + f64 im = 16 bytes
C32 ~= 1024 bytes per record + small record overhead
C64 ~= 2048 bytes per record + small record overhead
```

Текущие факты:

```text
coverage_action:
  records: 30
  cells: 32
  runtime_bytes_estimate: 31 680
  package_bytes: 30 736
  bench_p99_latency_ns: 106
  score_accuracy_milli: 1000
  score_wrong_wins: 0

release suite:
  artifacts: 3
  total action_tree_key_count: 46
  total_runtime_bytes_estimate: 48 576
  max_bench_p99_latency_ns: 117
  false_local_accepts: 0

offload SDK proof:
  public Rust SDK surface: PASS
  offload audit verify: PASS, report_matches_sources = true
  regression freeze verify: PASS, report_matches_sources = true
  offload_rate_milli: 880
  local_accuracy_milli: 1000
  false_local_accepts: 0
  loopback HTTP service smoke: PASS
  loopback HTTP service smoke report:
    target/nando-wave/action-runtime-v1-daemon-smoke.product-proof.json
  single-package HTTP service smoke: PASS
  single-package HTTP service command:
    phase-action-daemon-serve-v1
  single-package HTTP service report:
    target/nando-wave/action-runtime-v1-daemon-package-smoke.product-proof.json
  HTTP package smoke package_fingerprint64: 11103824464258352074
  HTTP package smoke local/fallback margins: 791009 / -791009
  HTTP hardening smoke: PASS
  HTTP hardening smoke report:
    target/nando-wave/action-runtime-v1-daemon-hardening-smoke.product-proof.json
  HTTP hardening smoke endpoints: /health /stats /score
  HTTP hardening smoke bad-route status: 404
  HTTP hardening smoke request limit bytes: 65536
  HTTP hardening smoke false_local_accepts: 0
  HTTP bearer-auth smoke: PASS
  HTTP bearer-auth smoke report:
    target/nando-wave/action-runtime-v1-daemon-auth-smoke.product-proof.json
  HTTP bearer-auth protected endpoints: /score /stats
  HTTP bearer-auth public endpoint: /health
  HTTP bearer-auth unauthorized /score status: 401
  HTTP bearer-auth false_local_accepts: 0
  HTTP multi-package registry smoke: PASS
  HTTP multi-package registry report:
    target/nando-wave/action-runtime-v1-daemon-registry-smoke.product-proof.json
  HTTP registry aliases: generated_action, domain_action, coverage_action
  HTTP registry package_count: 3
  HTTP registry missing alias status: 404
  HTTP registry false_local_accepts: 0
  HTTP registry config-file smoke: PASS
  HTTP registry config-file:
    target/nando-wave/action-runtime-v1-daemon-registry.config.json
  HTTP registry config-file report:
    target/nando-wave/action-runtime-v1-daemon-registry-config-smoke.product-proof.json
  HTTP registry config-file package_count: 3
  HTTP registry config-file missing alias status: 404
  HTTP registry config-file false_local_accepts: 0
  HTTP registry config-file server_runtime_config_used: true
  HTTP registry config-file server_runtime_compiler_used: false
  HTTP registry config-file server_runtime_corpus_jsonl_used: false
  HTTP registry config validation smoke: PASS
  HTTP registry config validation report:
    target/nando-wave/action-runtime-v1-daemon-config-validation-smoke.product-proof.json
  HTTP registry config validation invalid cases: 5
  HTTP registry config validation invalid rejects: 5
  HTTP registry config validation server_started_for_invalid_configs: false
  HTTP score rate-limit smoke: PASS
  HTTP score rate-limit report:
    target/nando-wave/action-runtime-v1-daemon-rate-limit-smoke.product-proof.json
  HTTP score rate-limit max_score_requests: 1
  HTTP score rate-limit over-limit status: 429
  HTTP score rate-limit scorer calls: 1
  HTTP score rate-limit rate_limited_requests: 1
  HTTP score rate-limit false_local_accepts: 0
  HTTP structured observability smoke: PASS
  HTTP structured observability report:
    target/nando-wave/action-runtime-v1-daemon-observability-smoke.product-proof.json
  HTTP structured observability package_count: 3
  HTTP structured observability aliases:
    generated_action, domain_action, coverage_action
  HTTP structured observability stats counters:
    score_requests=1, bad_requests=2, rate_limited_requests=1
  HTTP structured observability runtime provenance:
    config_used=true, compiler_used=false, corpus_jsonl_used=false, python_demo_used=false
  HTTP structured audit-log smoke: PASS
  HTTP structured audit-log event log:
    target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.events.jsonl
  HTTP structured audit-log report:
    target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.product-proof.json
  HTTP structured audit-log event_count: 6
  HTTP structured audit-log statuses: 200, 200, 404, 200, 429, 200
  HTTP structured audit-log flags:
    config_used=true, compiler_used=false, corpus_jsonl_used=false, python_demo_used=false
  HTTP error-taxonomy smoke: PASS
  HTTP error-taxonomy report:
    target/nando-wave/action-runtime-v1-daemon-error-taxonomy-smoke.product-proof.json
  HTTP error-taxonomy statuses: 400, 404, 413, 413, 400, 405, 413
  HTTP error-taxonomy score_requests: 0
  HTTP error-taxonomy false_local_accepts: 0
  HTTP error-taxonomy flags:
    config_used=true, compiler_used=false, corpus_jsonl_used=false, python_demo_used=false
  HTTP daemon proof suite: PASS
  HTTP daemon proof suite report:
    target/nando-wave/action-runtime-v1-daemon-proof-suite.product-proof.json
  HTTP daemon proof suite artifacts: 12
  HTTP daemon proof suite pass_count: 12
  HTTP daemon proof suite:
    reports_pass=true, forbidden_flags_false=true, hot_path_clean=true, false_local_accepts_zero=true
  HTTP daemon live proof suite: PASS
  HTTP daemon live proof suite report:
    target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json
  HTTP daemon live proof suite:
    live_rerun_performed=true, live_rerun_step_count=12, reports_pass=true, false_local_accepts_zero=true
  HTTP daemon systemd packaging smoke: PASS
  HTTP daemon systemd service:
    target/nando-wave/nando-wave-action-daemon.service
  HTTP daemon systemd env:
    target/nando-wave/nando-wave-action-daemon.env
  HTTP daemon systemd report:
    target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json
  HTTP daemon systemd smoke:
    artifacts_written=true, hardening_pass=true, systemctl_invoked=false, installed_to_systemd=false
  HTTP daemon deployment package: PASS
  HTTP daemon deployment package report:
    target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json
  HTTP daemon deployment package:
    live_suite_artifact_count=12, live_suite_step_count=12, registry_config_package_count=3
  HTTP daemon deployment package flags:
    service_unit_exec_matches=true, service_unit_env_matches=true, env_file_config_matches=true
  HTTP daemon deployment verify: PASS
  HTTP daemon deployment verify:
    report_gate_pass=true, rebuilt_gate_pass=true, report_matches_sources=true
  HTTP daemon deployment verify tamper:
    live_suite_step_count=11 -> WATCH, report_matches_sources=false
  HTTP daemon deployment package boundary:
    local deployable package proof only; no systemd install/start, TLS, dynamic reload, or pilot traffic
  workflow replay product gate: PASS
  workflow replay report:
    target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json
  workflow replay:
    sessions=256, steps_per_session=12, trace_calls=3072
  workflow replay packages:
    generated_action, domain_action, coverage_action
  workflow replay coverage:
    total_unique_eval_rows=308, replay_unique_rows=308
  workflow replay offload:
    exact_cache_llm_calls=308, exact_cache_plus_nando_llm_calls=36,
    incremental_llm_calls_removed_vs_cache=272
  workflow replay safety:
    local_accuracy_milli=1000, false_local_accepts=0
  workflow replay verify: PASS, report_matches_sources=true
  workflow replay regression/freeze anchor:
    regression=PASS, freeze=PASS, report_matches_sources=true
  workflow replay regression fingerprint:
    2002304595771295125
  workflow replay tamper:
    replay_unique_rows=307 -> WATCH, report_matches_sources=false
  workflow replay boundary:
    deterministic frozen-package replay only; not raw action parsing, text generation, or real pilot traffic
  production HTTP daemon hardening: OPEN
```

Практическая ёмкость при C32:

```text
1 MiB:  ~970 records
4 MiB:  ~3900 records
8 MiB:  ~7800 records
```

Вывод:

```text
для продуктовых линий 30 / 50 / 72 / 112 families phase-center runtime
не является главным лимитом, если concrete action_tree keys остаются
в сотнях или малых тысячах.
```

### Matrix B: L3 Role/Slot Center Pages

Это strict slot / role-filler binding layout.

Текущий sequence layout:

```text
CenterId: u32
lane_id: u16
output_slot_id / source_slot_id / sign_key: u8

PAGE_BITS = 12
PAGE_SIZE = 4096 centers
PAGE_COUNT = 32
SEQ_TOTAL_CENTER_COUNT = 131072

role pages: 0..15
action surface page: 16
operator-pair page: 17
state condition page: 18
condition/action page: 19
composed demo page: 20
reserve pages: 21..31
```

Текущий доказанный strict ordered rung:

```text
16 slots
lengths 13..16
strict ordered slot readout: 1000/1000
sequence energy: 1000/1000
flat parity mismatches: 0
```

Важно:

```text
PAGE_COUNT = 32 это 32 memory pages, не 32 output slots.
32-slot ordered decoder закрыт для order corpus rung lengths 17..32.
Full 32-slot operator battery ещё не закрыт.
```

### Matrix C: Operator-Pair Action Matrix

Это action motif вида:

```text
out_slot -> source_slot
```

Текущий код для sequence operator-pair:

```text
lane = (output_slot << 4) | source_slot
```

Это 4 bits + 4 bits:

```text
16 x 16 = 256 pair centers
```

Что будет дальше:

```text
32 slots:
  need 5 bits + 5 bits
  32 x 32 = 1024 centers
  fits in one 4096-center page

64 slots:
  need 6 bits + 6 bits
  64 x 64 = 4096 centers
  exactly fills one page

128 slots:
  need 7 bits + 7 bits
  128 x 128 = 16384 centers
  requires 4 pages
```

Вывод:

```text
operator-pair page itself can handle 32 and even 64 slots if packing is
changed deliberately.

But role pages and edge growth become the real limit.
```

### Matrix D: Flat Role-Binding Edge Table

Это strict L3 binding runtime.

Текущая оценка из roadmap:

```text
FlatRoleBindingEdge ~= 12 bytes
action_offsets = 8193 * 8 = 65 544 bytes
base_mass = 131072 * 2 = 262 144 bytes

flat_bytes =
  role_binding_edges * 12
  + action_offsets
```

Текущие факты:

```text
base v3 operator-pair:
  role_binding_edges: 22 460
  flat table: ~335 KiB
  hot field with base_mass: ~597 KiB

length 9..12 operator-pair:
  role_binding_edges: 60 638
  flat table: ~775 KiB
  hot field with base_mass: ~1.05 MiB

v4 strict multi-seed audit:
  report: target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json
  verdict: STRICT_MULTI_SEED_RUST_AUDIT_PASS
  verify: STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS
  logs_fingerprint64: 2847134219208477714
  observed_logs: 12
  strict_runtime_issues: 0
  freshness: PASS after 2026-07-02 23:05 phase_package_cmd.rs CLI edit
  fresh_log_window: 2026-07-02 23:24:45 .. 2026-07-03 00:08:10
  stale_logs_vs_latest_source: 0

v4 order logs:
  role_binding_edges by seed: 87952 / 87867 / 88441

v4 edit logs:
  role_binding_edges by seed: 136 / 136 / 136

v4 conditional logs:
  role_binding_edges by seed: 40813 / 40813 / 40858

v4 composed logs:
  role_binding_edges by seed: 366 / 366 / 366
```

Current claim caveat:

```text
The v4 16-slot audit proves full-channel strict runtime behavior under the
current gates. It does not prove every diagnostic subchannel independently:
edit marker_role and conditional condition_action ablations remain boundary
notes for future channel cleanup.
```

CPU cache budget on current reference machine:

```text
Intel i7-8650U
L1d: 32 KiB per core
L2: 256 KiB per core
L3: 8 MiB shared

safe working budget: ~4 MiB
safe role_binding_edges budget: ~320k
hard-ish role_binding_edges budget: ~670k
```

Вывод:

```text
16-slot strict L3 fits.
32-slot strict L3 is plausible only with explicit packing/cache benchmark.
64-slot strict L3 is not impossible, but it must be treated as a separate
capacity rung, not assumed free.
```

## Можем Ли Бесконечно Вмещать Операторы В L3?

Короткий ответ:

```text
нет.
```

Почему:

```text
1. CenterId u32 убрал старую u16 адресную стену, но не убрал память.
2. base_mass растёт как center_count * 2 bytes.
3. role_binding_edges растут с числом action centers, slots, signs и активных lanes.
4. cache misses растут, когда hot table выходит за L2/L3.
5. collision/superposition pressure растёт раньше, чем закончится u32.
6. strict proof требует flat parity, ablations, multi-seed, false accepts = 0.
```

Технический максимум `u32 CenterId` огромный, но продуктовый максимум задаётся
не `u32`, а:

```text
L3 cache budget
edge count
operator collision rate
latency p99
multi-seed robustness
false_local_accepts
maintenance of proof reports
```

## Slot Scaling Plan

### 16 Slots

```text
status: frozen baseline / demo fallback
proof: proven for current strict ordered rung; current-source freshness tracked in progress tree
layout: current 32 pages enough
target: keep hot runtime < 2 MiB
product role: safe first demo/offload baseline, not the final Codex-width target
```

### 32 Slots

```text
status: NEXT PRODUCT-FACING SCALING TARGET
current evidence: 64-page layout smoke passed, 3-seed smoke passed, latency smoke passed, order corpus multi-seed rung passed, mixed map corpus rung passed, conditional branch rung passed, mixed+conditional multi-seed combined rung passed, mixed+conditional cache/offload benchmark passed, serialized role-binding `.nwrb` package rung passed, public role-binding `.nwrb` SDK smoke passed, public SDK-loaded `.nwrb` package runtime rung passed, CLI inspect/verify for `.nwrb` package artifacts passed, CLI score/verify over explicit `.nwrb` eval-pack interface passed, independent corpus-emitted `.nwrb` CLI sequence scoring passed for representative 32-slot conditional package, compact binary `.nwreb` eval-pack score/verify passed for representative 32-slot conditional package, all-seed compact binary `.nwreb` eval-pack suite passed for current 32-slot role-binding package set, role-binding release-suite product-proof bundle passed for current `.nwrb/.nwreb` set, EDIT package integrated into release suite as PARTIAL blueprint coverage, serving-only `.nwrb` profile runtime smoke passed
remaining debt: full 32-slot product proof not closed
latest report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_CONDITIONAL_MULTI_SEED_RUNG.md
latest cache/offload report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH.md
latest role-binding package report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PACKAGE_RUNG.md
latest role-binding SDK report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE.md
latest role-binding SDK package runtime report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG.md
latest role-binding CLI inspect report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_INSPECT_RUNG.md
latest role-binding CLI score report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_SCORE_RUNG.md
latest role-binding CLI corpus score report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_CORPUS_SCORE_RUNG.md
latest role-binding binary eval-pack report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_RUNG.md
latest role-binding binary eval-pack suite report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_SUITE.md
latest role-binding release-suite report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_RELEASE_SUITE.md
latest role-binding profile runtime report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_RUNTIME_SMOKE.md
latest role-binding profile replay suite report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json
latest role-binding profile fallback smoke report: target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json
recommended layout: 64 pages * 4096 centers = 262144 centers
role pages: 0..31
action pages: 32+
operator-pair packing: (out << 5) | src
operator-pair centers: 1024
target: keep hot runtime < 4 MiB
required gate: real 32-slot corpus + strict slot + parity + ablations + cache/offload benchmark
product role: realistic Codex-like local offload window
```

Latest cache/offload bench:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_conditional_cache_offload_bench_release.log
verdict: SLOT32_MIXED_CONDITIONAL_CACHE_OFFLOAD_BENCH_PASS
total_simulated_calls: 36864
total_exact_cache_llm_calls: 12288
total_exact_cache_plus_nando_llm_calls: 0
total_false_local_accepts: 0
max_p99_latency_ns: 611686
max_hot_bytes_estimate: 681792
```

Latest serialized role-binding package rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PACKAGE_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_package_rung_release.log
verdict: SLOT32_ROLE_BINDING_PACKAGE_RUNG_PASS
package_magic: NWRB0001
package_count: 6
seeds: 3
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_p99_latency_ns: 623242
boundary: closes `.nwrb` role-binding package proof, not phase-center `.nwpc` or raw-language action parsing
```

Latest role-binding public SDK smoke:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE.md
verdict: SLOT32_ROLE_BINDING_PUBLIC_SDK_SMOKE_PASS
public runtime: nando_core::WavePredictorRoleBindingOffloadRuntime
public package path: inspect_package_bytes -> from_package_bytes -> offload_summary_into
test: cargo test -p nando-core --test wavepredictor_role_binding_sdk_public -- --nocapture
boundary: public Rust `.nwrb` SDK smoke, not `.nwpc`, CLI/daemon registry, or raw-language action parsing
```

Latest role-binding public SDK package runtime rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_public_sdk_package_rung_release.log
verdict: SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
package_magic: NWRB0001
seeds: 3
labels: sdk_conditional_branch, sdk_mixed_map
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_sdk_gap_parity_mismatches: 0
total_sdk_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 718891
boundary: public SDK-loaded `.nwrb` role-binding package proof, not `.nwpc`, CLI/daemon registry, or raw-language action parsing
```

Latest role-binding CLI inspect/verify rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_INSPECT_RUNG.md
product_report: target/nando-wave/slot32-role-binding/role-binding-package-inspect-v1.product-proof.json
verdict: ROLE_BINDING_PACKAGE_INSPECT_V1_PASS
verify_verdict: ROLE_BINDING_PACKAGE_VERIFY_V1_PASS
package_magic: NWRB0001
edge_count: 2202
package_bytes: 26468
package_fingerprint64: 365065097387925697
sdk_load_matches_inspect: true
report_matches_package: true
boundary: `.nwrb` CLI inspect/verify only, not `.nwrb` CLI scoring or daemon registry
```

Latest role-binding CLI score/verify rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_SCORE_RUNG.md
eval_pack: target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json
score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json
verdict: ROLE_BINDING_PACKAGE_SCORE_V1_PASS
verify_verdict: ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14619240648419331465
task_count: 128
local_operator_calls: 64
fallback_to_llm_calls: 64
false_local_accepts: 0
missed_expected_local: 0
report_matches_sources: true
boundary: `.nwrb` CLI scoring/verify interface over explicit eval-pack; the current generated eval-pack is package-derived smoke, not independent corpus proof
```

Latest role-binding CLI corpus score/verify rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_CLI_CORPUS_SCORE_RUNG.md
corpus_eval_pack: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json
score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-corpus-v1.product-proof.json
verdict: ROLE_BINDING_PACKAGE_SCORE_V1_PASS
verify_verdict: ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14754950188000667967
margin_threshold: 1000000
sequence_count: 4096
expected_local_sequences: 2048
expected_fallback_sequences: 2048
sequence_local_operator_calls: 2048
sequence_fallback_to_llm_calls: 2048
sequence_false_local_accepts: 0
sequence_missed_expected_local: 0
sequence_strict_ordered_accuracy_milli: 1000
sequence_median_energy_margin: 2449664
report_matches_sources: true
current_source_package_rerun_max_p99_latency_ns: 689788
boundary: independent corpus-emitted `.nwrb` CLI sequence scoring for one representative 32-slot conditional package; this JSON proof exposed the size pressure later addressed by the binary `.nwreb` rung below
```

Latest role-binding binary eval-pack rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_RUNG.md
binary_eval_pack: target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb
binary_pack_report: target/nando-wave/slot32-role-binding/role-binding-eval-pack-binary-corpus-v1.product-proof.json
binary_score_report: target/nando-wave/slot32-role-binding/role-binding-package-score-binary-corpus-v1.product-proof.json
verdict: ROLE_BINDING_EVAL_PACK_BINARY_V1_PASS
score_verdict: ROLE_BINDING_PACKAGE_SCORE_V1_PASS
verify_verdict: ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
binary_magic_text: NWRE0001
source_eval_pack_bytes: 455828420
binary_eval_pack_bytes: 60587229
size_reduction_milli: 867
roundtrip_exact: true
eval_pack_format: binary
sequence_count: 4096
sequence_strict_ordered_accuracy_milli: 1000
sequence_false_local_accepts: 0
sequence_missed_expected_local: 0
report_matches_sources: true
boundary: compact binary `.nwreb` eval-pack packaging/scoring for one representative 32-slot conditional package; superseded by the all-seed suite below for bundle coverage
```

Latest role-binding binary eval-pack suite:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_BINARY_EVAL_PACK_SUITE.md
suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
verdict: ROLE_BINDING_BINARY_EVAL_PACK_SUITE_V1_PASS
verify_verdict: ROLE_BINDING_BINARY_EVAL_PACK_SUITE_VERIFY_V1_PASS
suite_items: 6
total_source_eval_pack_bytes: 2712682190
total_binary_eval_pack_bytes: 359696838
suite_size_reduction_milli: 867
total_sequence_count: 24576
total_expected_local_sequences: 12288
total_expected_fallback_sequences: 12288
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 2330624
all_binary_gate_pass: true
all_binary_reports_match_sources: true
all_score_gate_pass: true
all_score_reports_match_sources: true
all_eval_pack_format_binary: true
all_package_fingerprints_match: true
boundary: all-seed compact binary `.nwreb` eval-pack packaging/scoring for the current 32-slot role-binding package set; full 32-slot operator battery remains open
```

Latest role-binding release suite:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_RELEASE_SUITE.md
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
verdict: ROLE_BINDING_RELEASE_SUITE_V1_PASS
verify_verdict: ROLE_BINDING_RELEASE_SUITE_VERIFY_V1_PASS
package_count: 7
binary_eval_pack_count: 7
score_report_count: 7
total_package_bytes: 134912
total_binary_eval_pack_bytes: 359696838
total_sequence_count: 27648
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 2330624
all_packages_magic_match: true
all_package_fingerprints_match_suite: true
all_eval_pack_fingerprints_match_suite: true
all_binary_reports_match_suite_rows: true
all_score_reports_match_suite_rows: true
all_forbidden_flags_false: true
boundary: product-proof release bundle for the current strict 32-slot role-binding `.nwrb/.nwreb` set; full 32-slot operator battery remains open
```

Latest role-binding profile runtime smoke:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_RUNTIME_SMOKE.md
product_report: target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json
registry_config: target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json
verdict: ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS
profiles: 7
endpoints: /health /profiles /score /replay /metrics
exact_cache_llm_calls: 2
exact_cache_plus_nando_llm_calls: 1
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
p99_latency_ns: 21436
runtime_bytes_estimate: 790020
rss_bytes: 10805248
compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
boundary: serving-only profile runtime smoke; not real Codex traffic and not full product replay.
```

Latest role-binding profile replay suite:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json
verdict: ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS
unique_sequences_replayed: 896
http_replay_batches: 224
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
p99_latency_ns: 213509
runtime_bytes_estimate: 790020
rss_bytes: 8101888
compiler_used: false
eval_packs_loaded_in_serving_worker: false
eval_packs_used_by_replay_client: true
corpus_jsonl_loaded_in_serving_worker: false
python_demo_used: false
boundary: serving replay PASS through the default CLI path; default max_unique=128 and batch=4 are part of the current proof envelope.
```

Latest role-binding profile fallback smoke:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json
verdict: ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
local_accept_pass: true
bad_route_fallback_pass: true
low_margin_fallback_pass: true
local_action: local_operator
bad_route_fallback_reason: profile_not_found
low_margin_fallback_reason: margin_below_threshold
local_energy_margin: 4194304
low_margin_energy_margin: 1024
low_margin_threshold: 1000000
local_operator_calls: 1
fallback_to_llm_calls: 2
false_local_accepts: 0
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
compiler_used: false
eval_packs_loaded: false
corpus_jsonl_loaded: false
python_demo_used: false
boundary: local accept, missing-route fallback, and low-margin fallback over serving-only `.nwrb`; not worker scaling or real Codex traffic.
```

Latest role-binding profile worker scaling:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_SCALING.md
verdict: ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
worker_count: 2
total_profile_count: 7
total_local_operator_calls: 7
wrong_worker_route_fallbacks: 2
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_rss_bytes: 6557696
max_worker_p99_latency_ns: 6286
boundary: local profile-shard worker acceptance; not real Codex traffic, external load-balancer proof, throughput scaling proof, or cheap-VPS deployment.
```

Latest role-binding profile worker replay:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_WORKER_REPLAY.md
verdict: ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_p99_latency_ns: 265277
boundary: sharded HTTP replay over local serving-only `.nwrb` workers; not real Codex traffic, external load-balancer proof, throughput scaling proof, or cheap-VPS deployment.
```

Latest role-binding profile local load-balancer replay:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_LB_REPLAY.md
lb_config: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.lb.json
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p99_latency_ns: 736030
core_score_p99_latency_ns: 78902
worker_score_p99_latency_ns: 167663
lb_upstream_roundtrip_p99_latency_ns: 735692
replay_client_wall_p99_latency_ns: 5489536
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 6307840
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 167663
boundary: local external proxy/load-balancer replay over serving-only `.nwrb` workers; not real Codex/API traffic, concurrent throughput proof, or cheap-VPS deployment.
```

Latest role-binding profile deployed HostWorld replay:

```text
product_report: target/nando-wave/role-binding-profile-runtime/profile-lb-replay-hostworld-v1-clean.product-proof.json
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_DEPLOYED_HOSTWORLD_REPLAY.md
remote_host_alias: hostworld-ee
remote_root: /opt/nando-wave-profile-runtime-v1
binary: x86_64-unknown-linux-musl static nando-cli
verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
missed_expected_local: 0
load_balancer_p99_latency_ns: 2993688
core_score_p99_latency_ns: 187721
worker_score_p99_latency_ns: 545095
lb_upstream_roundtrip_p99_latency_ns: 2993349
replay_client_wall_p99_latency_ns: 22311949
packed_score_parity_checks: 647928
packed_score_parity_mismatches: 0
load_balancer_rss_bytes: 2838528
max_worker_runtime_bytes_estimate: 492792
max_worker_p99_latency_ns: 545095
boundary: deployed cheap-VPS packed hot-path replay is inside the 3 ms p99 envelope while safety, exact-cache reduction, and packed-score parity remain green; not real Codex/API production traffic or concurrent throughput proof.
```

Latest role-binding profile bounded throughput:

```text
local_report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json
hostworld_report: target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-hostworld-v1.product-proof.json
diagnostic_report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_PROFILE_LB_THROUGHPUT.md

local:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 1409548
  lb_upstream_roundtrip_p99_latency_ns: 1409175
  worker_score_p99_latency_ns: 305953
  core_score_p99_latency_ns: 119908

hostworld:
  verdict: ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL
  client_threads: 4
  score_requests: 896
  false_local_accepts: 0
  client_errors: 0
  load_balancer_p99_latency_ns: 3611864
  lb_upstream_roundtrip_p99_latency_ns: 3610931
  worker_score_p99_latency_ns: 577626
  core_score_p99_latency_ns: 221243

boundary: local bounded individual-score pressure is green, but deployed cheap-VPS POST /score throughput is red; concurrent throughput is not closed.
```

Latest role-binding OPERATOR_BLUEPRINT gap audit:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ROLE_BINDING_OPERATOR_BLUEPRINT_GAP.md
gap_report: target/nando-wave/slot32-role-binding/role-binding-operator-blueprint-gap-v1.product-proof.json
verdict: ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_V1_WATCH
verify_verdict: ROLE_BINDING_OPERATOR_BLUEPRINT_GAP_VERIFY_V1_PASS
release_suite_gate_pass: true
blueprint_required_class_count: 9
proven_classes: 0
partial_classes: 7
missing_classes: 2
coverage_gate_pass: false
full_32_slot_operator_battery_closed: false
PARTIAL: SELECT, MOVE_COPY, EDIT, ORDER, CONDITION_ROUTE, COMPOSE, VERIFY_REPAIR
MISSING: FIELD, FILTER_GROUP
boundary: source-verified claim-boundary audit; current role-binding release suite remains green, but full OPERATOR_BLUEPRINT coverage remains open
```

Fresh EDIT runtime gate:

```text
report: data/rule_logic_operator_battery_v4/edit/EDIT_RUNTIME_BOUNDARY_REPORT.md
runtime_log: data/rule_logic_operator_battery_v4/edit/edit_marker_length_runtime_gate_release.log
verdict: EDIT_CURRENT_SOURCE_RUNTIME_GATE_PASS
release_integration: PASS
blueprint_status: PARTIAL
train_rows / heldout_rows: 1536 / 1536
edit_output_slot_count / edit_role_slot_count: 17 / 17
slot / flat_slot / sequence_energy accuracy milli: 1000 / 1000 / 1000
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
role_binding_edges: 136
forbidden flags: false
boundary: EDIT is integrated into the `.nwrb/.nwreb` role-binding release-suite
  proof as `sdk_edit_marker_length`, but it is not full EDIT blueprint coverage.
```

Почему 32 является продуктовой целью:

```text
16 slots доказывают базовый transferable-action engine, но Codex-like запросы
быстро выходят за 16 активных ролей:
  file
  function
  symbol
  old_name
  new_name
  import
  callsite
  type
  error
  test
  condition
  patch_region
  replacement
  fallback_reason
  action_arg_1
  action_arg_2
  action_arg_3

32 slots должны уменьшить window_too_wide fallback и поднять долю безопасных
local accepts.
```

Current smoke:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_PAGED_LAYOUT_CAPACITY_SMOKE.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_paged_layout_capacity_smoke_release.log

lengths: 17..32
role_top_l1_lanes: 64
slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
ablations without binding/action/role/active: 0
role_binding_edges: 892
hot_bytes_estimate: 600536
flat_eval_avg_ns_per_row: 147415
multi_seed_smoke_pass: true
multi_seed_smoke_seeds: 3
multi_seed_min_slot_accuracy_milli: 1000
multi_seed_min_sequence_energy_accuracy_milli: 1000
multi_seed_total_flat_parity_mismatches: 0
multi_seed_max_flat_eval_avg_ns_per_row: 150392
flat_runtime_latency_smoke_pass: true
flat_runtime_latency_measured_rows: 16384
flat_runtime_latency_p50_ns: 135476
flat_runtime_latency_p99_ns: 245822
flat_runtime_latency_gate_ns: 1000000
forbidden flags: false
```

First real order corpus rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ORDER_CORPUS_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_rung_release.log
verdict: SLOT32_ORDER_CORPUS_RUNG_PASS

train_rows: 1024
heldout_rows: 1024
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows: 1024
max_train_state_reuse: 8
max_heldout_state_reuse: 8
train_tokens_overlap_heldout: 0

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
flat parity mismatches: 0
ablations without binding/action/role/active: 0 / 0 / 0 / 0
role_binding_edges: 1354
hot_bytes_estimate: 606080
flat_eval_avg_ns_per_row: 185511
forbidden flags: false
```

Order corpus multi-seed rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_multiseed_rung_release.log
verdict: SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG_PASS

seeds: 3
rows_per_seed_train / heldout: 1024 / 1024
unique_rules / surfaces / noise / lengths: 8 / 4 / 2 / 16
lengths: 17..32
train_tokens_overlap_heldout_per_seed: 0

min_slot_accuracy_milli: 1000
min_flat_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_energy_pass_slot_fail: 0
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
max_role_binding_edges: 1354
max_hot_bytes_estimate: 606080
max_flat_eval_avg_ns_per_row: 187982
forbidden flags: false
```

Mixed map corpus rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_MIXED_MAP_CORPUS_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_map_corpus_rung_release.log
verdict: SLOT32_MIXED_MAP_CORPUS_RUNG_PASS

seed: 0
train_rows / heldout_rows: 2048 / 2048
unique_operator_classes: 3
operator classes: order / edit-map / composed-map
unique_rules: 16
unique_surfaces / noise / lengths: 4 / 2 / 16
lengths: 17..32
same_bag_rows: 1536
edit_rows / edit_non_same_bag_rows: 512 / 512
train_tokens_overlap_heldout: 0

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
energy_pass_slot_fail: 0
flat parity mismatches: 0
ablations without binding/action/role/active: 0 / 0 / 0 / 0
state_delta_edges: 0
role_binding_edges: 1492
hot_bytes_estimate: 607736
flat_eval_avg_ns_per_row: 219009
forbidden flags: false
```

Conditional branch corpus rung:

```text
report: data/rule_logic_operator_battery_v4/diagnostics/slot32/SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG.md
log: data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_conditional_branch_corpus_rung_release.log
verdict: SLOT32_CONDITIONAL_BRANCH_CORPUS_RUNG_PASS

seed: 0
train_rows / heldout_rows: 2048 / 2048
unique_operator_classes: 1
unique_rules: 8
unique_surfaces / noise / lengths: 4 / 2 / 16
lengths: 17..32
same_bag_rows: 2048
condition_true_rows / condition_false_rows: 1024 / 1024
direct_operator_pair_active_centers: 0
condition_action_active_centers: 50176
state_condition_active_centers: 120832
train_tokens_overlap_heldout: 0

slot_accuracy_milli: 1000
flat_slot_accuracy_milli: 1000
sequence_energy_accuracy_milli: 1000
energy_pass_slot_fail: 0
flat parity mismatches: 0
ablations without binding/action/condition-action/role/active: 0 / 0 / 0 / 0 / 0
state_delta_edges: 0
role_binding_edges: 2202
hot_bytes_estimate: 681792
flat_eval_avg_ns_per_row: 174654
forbidden flags: false
```

Boundary:

```text
The layout smoke is synthetic. The order corpus multi-seed rung covers order
only. The mixed+conditional multi-seed rung now covers mixed order/edit-map/
composed-map transfer plus state/action conditional branch selection over
symbolic branch-map action inputs.
This is still not the full 32-slot product package proof, not raw-language
action parsing, not insert-new-constant edit operators, not autonomous
action_tree induction, not broad corpus-battery robustness, not a phase-center
`.nwpc` bridge/product package proof for this strict role-binding path, and not
final product p99 across the packaged daemon/API path.

Interpretation:
  p99 245822 ns = 0.246 ms for the current smoke row path.
  This is not a blocker for CPU offload.
  It is a green signal to build the real 32-slot product package benchmark.
```

### 64 Slots

```text
status: not product baseline
recommended layout: 128 pages * 4096 centers = 524288 centers
role pages: 0..63
action pages: 64+
operator-pair packing: (out << 6) | src
operator-pair centers: 4096
target: explicit L3-cache benchmark before any claim
```

### 128 Slots

```text
status: platform/research only
recommended shape: sharded packages, not one monolithic strict L3 table
operator-pair pages: at least 4
main risk: edge growth and collision pressure
```

## Product Capacity Recommendation

Для ближайшей продаваемой линии:

```text
operator families: 45-72
phase-center records: keep under 1000 first
phase cells: C32 default, C64 only if margin requires it
strict slot length: 16 frozen baseline, 32 next product-facing target
hot runtime budget: < 4 MiB
p99 latency target: sub-microsecond for scoring path
false_local_accepts: 0
```

Для сильной agent/Codex линии:

```text
operator families: 96-128
phase-center records: hundreds to low thousands
runtime shape: multiple packages/shards
strict slot length: 32 required for richer state windows
64 slots only after dedicated cache proof
```

Для платформы:

```text
operator families: 128-256+
do not use one matrix
use package registry:
  choose domain shard
  score local package
  fallback to LLM when margin is low
```

## Capacity Gates Для Каждой Линейки

Каждая линейка должна иметь отчёт:

```text
operator_family_count
action_tree_key_count
select/transform/write/condition/check counts
min rows per action_tree
runtime_bytes_estimate
package_bytes
p50/p99 latency
cache-enabled offload reduction
false_local_accepts
shortcut verdict
flat parity mismatches
ablation collapse
multi-seed result
strict_runtime_issues
```

Не засчитывать PASS, если:

```text
coverage есть только в одном измерении;
action_tree содержит slot-map answer;
operator count раздут concrete lookup-ами;
package не source-matching;
runtime path читает JSONL/corpus;
offload gain считается только против no-cache LLM baseline;
strict slot заменён одним sequence-energy judge.
```

## Итог

```text
Phase-center product runtime может вместить сотни и малые тысячи compact
action_tree records на CPU.

Strict L3 slot-binding не бесконечен:
  16 slots доказаны текущим v4 strict multi-seed Rust behavior audit и
  остаются frozen baseline;
  32 slots являются следующим product-facing scaling target и требуют real
  corpus + full product proof + cache/offload proof;
  64 slots требуют 128-page layout и отдельный capacity rung;
  128+ slots лучше делать через shards/packages.

Главный предел сейчас не CenterId, а edge growth, cache budget, collision
pressure и строгость proof-gates.
```
