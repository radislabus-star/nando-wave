# Nando Wave Product Trajectory

Date: 2026-07-01

## Core Position

```text
Мы целимся не в "ещё одну LLM", а в operator layer для LLM-систем.
```

Самая сильная формула линейки:

```text
LLM discovers.
Nando Wave compiles.
CPU executes.
```

По-русски:

```text
LLM открывает новые решения.
Nando Wave компилирует повторяемые операторы.
CPU исполняет их дешево, быстро и проверяемо.
```

## Product Line

### 1. Operator Runtime

```text
CPU runtime
state_t + action -> state_t+1
flat tables
low latency
confidence / energy gap
fallback to LLM
```

Это ядро.

### 2. Operator Compiler

```text
JSONL/workflow logs -> corpus
shortcut gates
training
ablations
compiled runtime
reports
```

Это то, что превращает опыт LLM/agent workflow в локальные операторы.

### 3. Operator Battery

```text
order operators
edit operators
conditional operators
composed operators
domain operators
```

Это библиотека proof-gated задач и benchmark-ов.

### 4. Agent Offload SDK

```text
Python/Rust/HTTP API
подключается к agent runtime
решает: LLM call или local operator
пишет traces
считает экономию
```

Это продукт для интеграции.

### 5. Enterprise / Infra Package

```text
локальная установка
CPU-only
privacy
cost report
latency report
fallback policy
audit logs
```

Это то, что можно продавать.

## Market Meaning

Для рынка LLM это понятно:

```text
меньше вызовов к большой модели
меньше latency
меньше стоимость
локальная приватность
проверяемые переходы
```

## Roadmap

```text
1. Закрыть operator battery.
2. Закрыть 16/32-slot scaling.
3. Сделать release latency bench.
4. Сделать compiler.
5. Сделать local runtime API.
6. Показать LLM offload demo:
   было 1000 LLM calls
   стало 600 LLM calls + 400 CPU operators
   качество не упало
```

## Claim Boundary

Это продуктовая траектория, не замена proof-gates.

Нельзя продавать как доказанный runtime, пока не закрыты:

```text
operator battery
multi-seed
32-slot rung
release latency bench
runtime bytes report
LLM offload demo
quality/fallback audit
```

Current operator-battery artifact:

```text
data/rule_logic_operator_battery_v4/V4_OPERATOR_BATTERY_REPORT.md
```

Current concrete package proof:

```text
command: cargo run -p nando-cli --release -- phase-package-v4
inspect: cargo run -p nando-cli --release -- phase-package-inspect
score: cargo run -p nando-cli --release -- phase-package-score-v4
eval_pack: cargo run -p nando-cli --release -- phase-eval-pack-v4
score_pack: cargo run -p nando-cli --release -- phase-package-score-pack-v4
action_boundary: cargo run -p nando-cli --release -- phase-action-boundary-v4
package: target/nando-wave/phase-center-v4-c32.nwpc
manifest: target/nando-wave/phase-center-v4-c32.nwpc.manifest.json
eval_pack_path: target/nando-wave/phase-center-v4-c32.eval-pack
score_report: target/nando-wave/phase-center-v4-c32.score-report.json
score_pack_report: target/nando-wave/phase-center-v4-c32.score-pack-report.json
fingerprint64: 14549306353473335964
operator_key_count: 380
verdict: PHASE_PACKAGE_V4_PASS / PHASE_PACKAGE_INSPECT_PASS
score_verdict: PHASE_PACKAGE_SCORE_V4_PASS
eval_pack_verdict: PHASE_EVAL_PACK_V4_PASS
score_pack_verdict: PHASE_PACKAGE_SCORE_PACK_V4_PASS
score_report_verdict: PHASE_PACKAGE_SCORE_V4_PASS
verify_verdict: PHASE_PACKAGE_VERIFY_PASS
compiler_used_for_score: false
compiler_used_for_score_pack: false
eval_task_package_used_for_score_pack: true
corpus_jsonl_used_in_score_pack_loop: false
accuracy_milli: 1000
wrong_wins: 0
action_ablation_accuracy_milli: 443
action_ablation_wrong_wins: 2958
score_from_package_action_ablation_accuracy_milli: 443
score_from_package_action_ablation_wrong_wins: 2958
score_from_eval_pack_p99_latency_ns: 429
score_from_eval_pack_rows_per_second: 5614990.50
eval_pack_bytes: 10921516
action_boundary_verdict: PHASE_ACTION_BOUNDARY_V4_WATCH
autonomous_action_router_claim_allowed: false
explicit_operator_family_label_rows: 10624
explicit_order_slot_map_rows: 4096
explicit_branch_slot_map_rows: 1536
tampered_manifest_verdict: PHASE_PACKAGE_SCORE_V4_WATCH / non-zero exit
tampered_score_report_verdict: PHASE_PACKAGE_SCORE_V4_WATCH
tampered_score_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH / non-zero exit
tampered_eval_pack_score_verdict: WATCH before scoring / non-zero exit
tampered_score_pack_report_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH / non-zero exit
tampered_score_pack_no_eval_flag_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH / non-zero exit
tampered_score_pack_jsonl_loop_true_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH / non-zero exit
tampered_score_pack_jsonl_missing_verify_verdict: PHASE_PACKAGE_VERIFY_WATCH / non-zero exit
verify_prints_manifest_and_score_report_forbidden_flags: true
boundary: phase-center scorer package, not autonomous action-router, full strict decoder, or generator
```

License boundary:

```text
workspace Cargo metadata now declares license-file = LICENSE-NONCOMMERCIAL.md;
phase-action-license-package-v1 closes the non-commercial source/proof package;
separate commercial license is not closed by the current scorer gate.
```

Current offload audit:

```text
command: cargo run -p nando-cli --release -- phase-action-offload-audit-v1
verify: cargo run -p nando-cli --release -- phase-action-offload-verify-v1
verdict: PHASE_ACTION_OFFLOAD_AUDIT_V1_PASS
verify_verdict: PHASE_ACTION_OFFLOAD_VERIFY_V1_PASS
margin_threshold_micro: 300000
simulated_calls: 1000
local_operator_calls: 880
fallback_to_llm_calls: 120
offload_rate_milli: 880
local_accuracy_milli: 1000
false_local_accepts: 0
total_unique_eval_rows: 308
unique_local_operator_rows: 272
unique_fallback_rows: 36
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
source_rebuild_matches_package: true for generated_action, domain_action, and coverage_action
source_rebuild_path: action_contract JSONL -> Rust PhaseCenterCompiler -> exact .nwpc bytes
source_rebuild_command: cargo run -p nando-cli --release -- phase-action-source-verify-v1
runtime_path: nando_core::PhaseCenterFlatRuntime
offload_sdk_api: nando_core::PhaseCenterOffloadRuntime
offload_sdk_inspect_api: nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes
offload_policy_api: nando_core::PhaseCenterOffloadPolicy
offload_batch_api: nando_core::PhaseCenterFlatRuntime::offload_decisions
offload_summary_api: nando_core::PhaseCenterOffloadSummary
offload_buffer_api: nando_core::PhaseCenterFlatRuntime::offload_decisions_into
offload_summary_buffer_api: nando_core::PhaseCenterOffloadSummary::from_repeated_decision_fn_into
offload_runtime_summary_api: nando_core::PhaseCenterOffloadRuntime::offload_summary_into
each artifact sdk_inspected_fingerprint64 == package_fingerprint64
each artifact sdk_inspected_serialized_len == package_bytes
each artifact sdk_inspect_matches_package: true
each artifact sdk_inspect_matches_eval_pack: true
```

This is the first concrete `LLM discovers -> Nando Wave compiles -> CPU
executes` package audit: confident calls go to local CPU operator runtime;
low-margin calls fall back to the LLM. The decision policy now lives in
`nando_core::PhaseCenterOffloadPolicy`. The lower-level scorer remains
`PhaseCenterFlatRuntime`, but the product-facing packaged-runtime summary path
uses `PhaseCenterOffloadRuntime::offload_summary_into`, not private CLI
loop/summary logic and not a stale direct FlatRuntime summary claim. It is still
a packaged flat-scorer offload proof, not full text generation or autonomous raw
action parsing.

Current green regression is frozen by:

```text
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
release_verify_pass: true
license_verify_pass: true
offload_verify_pass: true
release_suite_report_fingerprint64: 9827723825761118426
release_suite_report_bytes: 21282
license_package_report_fingerprint64: 9589570789353175064
license_package_report_bytes: 2054
offload_audit_report_fingerprint64: 16396006654765989741
offload_audit_report_bytes: 7951
artifact_count: 3
total_runtime_bytes_estimate: 48576
total_bench_samples: 308000
total_source_verify_report_bytes: 7373
total_shortcut_report_bytes: 3065
total_source_rebuild_accepted_action_tree_rows: 680
total_source_rebuild_action_tree_key_count: 46
operator_blueprint_path: docs/OPERATOR_BLUEPRINT.md
operator_blueprint_fingerprint64: 9874423192353457577
operator_blueprint_formula_present: true
operator_blueprint_runtime_package_contract_present: true
operator_blueprint_forbidden_invariants_present: true
state_transition_formula: state_t + action_tree -> state_t+1
python_demo_used: false
forbidden_used: false
offload_sdk_api: nando_core::PhaseCenterOffloadRuntime
offload_sdk_inspect_api: nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes
```

Public Rust SDK consumer proof:

```text
cargo test -p nando-core --test phase_center_offload_sdk_public: PASS
PhaseCenterOffloadRuntime::inspect_package_bytes: PASS
```

Current refreshed SDK/offload product proof:

```text
phase-action-offload-audit-v1: PASS
phase-action-offload-verify-v1: PASS, report_matches_sources = true
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS, report_matches_sources = true
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS, report_matches_sources = true

artifact_count: 3
offload_audit_report_fingerprint64: 16396006654765989741
regression_report_fingerprint64: 2002304595771295125
release_suite_report_fingerprint64: 9827723825761118426
operator_blueprint_fingerprint64: 9874423192353457577
total_unique_eval_rows: 308
local_operator_calls: 880
fallback_to_llm_calls: 120
offload_rate_milli: 880
unique_offload_rate_milli: 883
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
offload_sdk_api: nando_core::PhaseCenterOffloadRuntime
offload_runtime_summary_api: nando_core::PhaseCenterOffloadRuntime::offload_summary_into
```

This closes the public Rust SDK/offload surface for packaged flat action
scorers.

Loopback HTTP service smoke:

```text
command: cargo run -p nando-cli --release -- phase-action-daemon-smoke-v1
report: target/nando-wave/action-runtime-v1-daemon-smoke.product-proof.json
verdict: PHASE_ACTION_DAEMON_SMOKE_V1_PASS
package_fingerprint64: 12305377795745418155
package_cells: 32
package_record_count: 1
package_serialized_len: 1040
runtime_bytes_estimate: 1056
http_requests: 2
http_requests_handled: 2
http_bad_requests: 0
local_action: local_operator
fallback_action: fallback_to_llm
local_margin_micro: 1869387
fallback_margin_micro: -1869387
false_local_accepts: 0
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only the loopback HTTP service boundary smoke over
`PhaseCenterOffloadRuntime` package bytes. It does not close a production
daemon, auth/TLS, service manager integration, multi-package registry, or a real
external workflow pilot.

Existing package HTTP service smoke:

```text
serve command:
  cargo run -p nando-cli --release -- phase-action-daemon-serve-v1

bounded proof command:
  cargo run -p nando-cli --release -- phase-action-daemon-package-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-package-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_PACKAGE_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_cells: 32
package_record_count: 30
package_serialized_len: 30736
runtime_bytes_estimate: 31680
fixture_task_id: generated_coverage_contract_v1_heldout_len5_select_span_reverse_replace_always_bag_0
fixture_center_index: 9
http_requests: 2
http_requests_handled: 2
http_bad_requests: 0
local_action: local_operator
fallback_action: fallback_to_llm
local_margin_micro: 791009
fallback_margin_micro: -791009
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes a single-package HTTP service surface for existing `.nwpc` action
packages. The corpus is used only to build a request fixture in the bounded
proof command; `phase-action-daemon-serve-v1` itself loads package bytes and
scores HTTP requests through `PhaseCenterOffloadRuntime`. It still does not
close production hardening: auth/TLS, service-manager integration, multi-package
registry, rate limits, observability, or real pilot traffic.

HTTP hardening smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-hardening-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-hardening-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_HARDENING_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_cells: 32
package_record_count: 30
runtime_bytes_estimate: 31680
http_max_request_bytes: 65536
max_score_atoms: 1024
max_score_atom_bytes: 256
health_status_code: 200
stats_status_code: 200
bad_route_status_code: 404
local_status_code: 200
fallback_status_code: 200
local_action: local_operator
fallback_action: fallback_to_llm
local_margin_micro: 791009
fallback_margin_micro: -791009
http_requests: 5
http_requests_handled: 4
http_score_requests: 2
http_health_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 1
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only the first HTTP hardening smoke: `/health`, `/stats`, bounded
request size, unsupported-route errors, and local/fallback counters. Production
daemon hardening remains open beyond this smoke.

HTTP bearer-auth smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-auth-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-auth-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_AUTH_SMOKE_V1_PASS
package_path: target/nando-wave/action-runtime-v1-generated-coverage-c32.nwpc
package_fingerprint64: 11103824464258352074
package_cells: 32
package_record_count: 30
runtime_bytes_estimate: 31680
auth_enabled: true
health_public_status_code: 200
unauthorized_score_status_code: 401
authorized_score_status_code: 200
authorized_fallback_status_code: 200
authorized_stats_status_code: 200
local_action: local_operator
fallback_action: fallback_to_llm
local_margin_micro: 791009
fallback_margin_micro: -791009
http_requests: 5
http_requests_handled: 4
http_score_requests: 2
http_health_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 1
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only a bearer-auth smoke for protected `/score` and `/stats`.
`/health` remains public for liveness. TLS, service-manager integration,
multi-package registry, rate limits, structured observability, and real pilot
traffic are still open.

HTTP multi-package registry smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-registry-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-registry-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_REGISTRY_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
generated_package_fingerprint64: 14869999570221545448
domain_package_fingerprint64: 5367415087033800111
coverage_package_fingerprint64: 11103824464258352074
generated_status_code: 200
domain_status_code: 200
coverage_status_code: 200
missing_alias_status_code: 404
packages_status_code: 200
stats_status_code: 200
health_status_code: 200
generated_action: local_operator
domain_action: local_operator
coverage_action: local_operator
generated_margin_micro: 675249
domain_margin_micro: 1526347
coverage_margin_micro: 791009
http_requests: 7
http_requests_handled: 6
http_score_requests: 3
http_health_requests: 1
http_packages_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 3
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only a static multi-package registry smoke over already built
`.nwpc` packages. It proves alias routing and `/packages` listing. Dynamic
package reload, registry config files, rate limits, TLS, service-manager
integration, structured observability, and real pilot traffic are still open.

HTTP registry config smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-registry-config-smoke-v1

config:
  target/nando-wave/action-runtime-v1-daemon-registry.config.json

report:
  target/nando-wave/action-runtime-v1-daemon-registry-config-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_REGISTRY_CONFIG_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
generated_status_code: 200
domain_status_code: 200
coverage_status_code: 200
missing_alias_status_code: 404
packages_status_code: 200
stats_status_code: 200
health_status_code: 200
generated_action: local_operator
domain_action: local_operator
coverage_action: local_operator
generated_margin_micro: 675249
domain_margin_micro: 1526347
coverage_margin_micro: 791009
http_requests: 7
http_requests_handled: 6
http_score_requests: 3
http_packages_requests: 1
http_stats_requests: 1
http_bad_requests: 1
local_operator_calls: 3
fallback_to_llm_calls: 0
false_local_accepts: 0
request_fixture_corpus_jsonl_used: true
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes config-file loading for a multi-package HTTP registry. The proof
command writes a JSON registry config, the service path loads existing `.nwpc`
package bytes and verifies manifest parity. It is not dynamic package reload,
rate limits, TLS, service-manager integration, structured observability, or
real pilot traffic.

HTTP score rate-limit smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-rate-limit-smoke-v1

config:
  target/nando-wave/action-runtime-v1-daemon-registry.config.json

report:
  target/nando-wave/action-runtime-v1-daemon-rate-limit-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_RATE_LIMIT_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
max_score_requests: 1
health_status_code: 200
packages_status_code: 200
allowed_score_status_code: 200
rate_limited_score_status_code: 429
stats_status_code: 200
allowed_action: local_operator
allowed_margin_micro: 791009
http_requests: 5
http_requests_handled: 4
http_score_requests: 1
http_bad_requests: 1
http_rate_limited_requests: 1
local_operator_calls: 1
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only a deterministic `/score` max-score guard over the JSON
registry service path. It proves a request beyond `max_score_requests` returns
429 and does not invoke the scorer. It is not time-window rate limiting, TLS,
dynamic reload, service-manager integration, structured observability, or real
pilot traffic.

HTTP structured observability smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-observability-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-observability-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_OBSERVABILITY_SMOKE_V1_PASS
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
max_score_requests: 1
missing_alias_status_code: 404
rate_limited_score_status_code: 429
requests_handled_observed_by_stats: 3
score_requests_observed_by_stats: 1
bad_requests_observed_by_stats: 2
rate_limited_requests_observed_by_stats: 1
local_operator_calls_observed_by_stats: 1
fallback_to_llm_calls_observed_by_stats: 0
false_local_accepts_observed_by_stats: 0
requests_handled_final: 4
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only `/stats` structured observability for aliases, counters,
rate-limit counters, and runtime provenance flags. It is not tracing,
persistent logs, TLS, dynamic reload, service-manager integration, or real
pilot traffic.

HTTP structured audit-log smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-audit-log-smoke-v1

event log:
  target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.events.jsonl

report:
  target/nando-wave/action-runtime-v1-daemon-audit-log-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_AUDIT_LOG_SMOKE_V1_PASS
audit_event_count: 6
audit_status_codes: 200, 200, 404, 200, 429, 200
audit_request_kinds: health, packages, error, score, error, stats
audit_sequences_are_dense: true
audit_missing_alias_event_found: true
audit_rate_limit_event_found: true
audit_local_operator_event_found: true
audit_flags_pass: true
local_operator_calls: 1
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only server-side structured JSONL audit events for handled and
rejected requests. It is not distributed tracing, log rotation, TLS, dynamic
reload, service-manager integration, or real pilot traffic.

HTTP error-taxonomy smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-error-taxonomy-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-error-taxonomy-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_ERROR_TAXONOMY_SMOKE_V1_PASS
error_status_codes: 400, 404, 413, 413, 400, 405, 413
error_messages_pass: true
score_requests: 0
bad_requests: 7
local_operator_calls: 0
fallback_to_llm_calls: 0
false_local_accepts: 0
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only explicit HTTP rejection taxonomy and proves these rejects do
not invoke the scorer. It is not fuzzing, TLS, dynamic reload,
service-manager integration, or real pilot traffic.

HTTP registry config validation smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-config-validation-smoke-v1

report:
  target/nando-wave/action-runtime-v1-daemon-config-validation-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_CONFIG_VALIDATION_SMOKE_V1_PASS
valid_registry_load_pass: true
valid_package_count: 3
invalid_case_count: 5
invalid_reject_count: 5
invalid_error_messages_pass: true
server_started_for_invalid_configs: false
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

This closes only startup-time registry config validation for valid load and
five invalid reject-before-serve cases. It is not dynamic reload, TLS,
service-manager integration, or real pilot traffic.

HTTP daemon proof suite:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-proof-suite-v1

report:
  target/nando-wave/action-runtime-v1-daemon-proof-suite.product-proof.json

verdict: PHASE_ACTION_DAEMON_PROOF_SUITE_V1_PASS
artifact_count: 12
pass_count: 12
all_reports_pass: true
all_forbidden_flags_false: true
all_python_demo_false: true
all_server_runtime_hot_path_clean: true
all_false_local_accepts_zero: true
```

This closes only a saved-report daemon proof bundle over existing product-proof
JSON artifacts. It is not a live rerun, TLS, service-manager integration,
dynamic reload, or real pilot traffic.

HTTP daemon live proof suite:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-live-proof-suite-v1

report:
  target/nando-wave/action-runtime-v1-daemon-live-proof-suite.product-proof.json

verdict: PHASE_ACTION_DAEMON_LIVE_PROOF_SUITE_V1_PASS
live_rerun_performed: true
live_rerun_step_count: 12
artifact_count: 12
pass_count: 12
all_reports_pass: true
all_forbidden_flags_false: true
all_python_demo_false: true
all_server_runtime_hot_path_clean: true
all_false_local_accepts_zero: true
```

This freshly reruns the 12 local HTTP daemon and service-packaging smoke gates,
then verifies the updated product-proof JSON artifacts as one bundle. It is not
TLS, installed service, dynamic reload, or real pilot traffic.

HTTP daemon systemd packaging smoke:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-systemd-smoke-v1

service:
  target/nando-wave/nando-wave-action-daemon.service

env:
  target/nando-wave/nando-wave-action-daemon.env

report:
  target/nando-wave/action-runtime-v1-daemon-systemd-smoke.product-proof.json

verdict: PHASE_ACTION_DAEMON_SYSTEMD_SMOKE_V1_PASS
package_count: 3
service_manager_artifacts_written: true
service_exec_serve_registry: true
service_environment_file_matches: true
service_restart_on_failure: true
service_hardening_pass: true
env_registry_config_matches: true
auth_token_placeholder_used: true
installed_to_systemd: false
systemctl_invoked: false
server_runtime_config_used: true
server_runtime_compiler_used: false
server_runtime_corpus_jsonl_used: false
python_demo_used: false
```

This writes and validates local systemd unit/env/registry artifacts under
`target` for `phase-action-daemon-serve-registry-v1`. It does not install or
start a service, configure TLS, dynamic reload, or real pilot traffic.

HTTP daemon deployment package:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-deployment-package-v1

report:
  target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json

verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_PACKAGE_V1_PASS
live_suite_artifact_count: 12
live_suite_step_count: 12
live_suite_contains_systemd: true
live_suite_hot_path_clean: true
live_suite_forbidden_flags_false: true
live_suite_python_demo_false: true
live_suite_false_local_accepts_zero: true
systemd_smoke_pass: true
systemd_artifacts_written: true
systemd_hardening_pass: true
systemd_auth_placeholder_used: true
systemd_not_installed: true
systemctl_not_invoked: true
systemd_hot_path_clean: true
systemd_forbidden_flags_false: true
service_unit_exec_matches: true
service_unit_env_matches: true
env_file_config_matches: true
registry_config_package_count: 3
registry_config_package_count_matches: true
deployment_artifacts_present: true
installed_to_systemd: false
```

This verifies the daemon live proof suite, generated systemd service, env file,
and registry config as one local deployment package. It does not install/start
systemd, configure TLS, dynamic reload, or prove real pilot traffic.

HTTP daemon deployment verify:

```text
command:
  cargo run -p nando-cli --release -- phase-action-daemon-deployment-verify-v1

report:
  target/nando-wave/action-runtime-v1-daemon-deployment-package.product-proof.json

verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_PASS
report_gate_pass: true
rebuilt_gate_pass: true
report_matches_sources: true
live_suite_artifact_count: 12
live_suite_step_count: 12
service_unit_exec_matches: true
registry_config_package_count: 3
deployment_artifacts_present: true
```

This verifies that the saved deployment report still matches the current live
proof suite, systemd smoke report, service unit, env file, and registry config.
It does not rerun the smoke gates and does not install/start the daemon.

Deployment verify tamper control:

```text
tamper: live_suite_step_count 12 -> 11
verdict: PHASE_ACTION_DAEMON_DEPLOYMENT_VERIFY_V1_WATCH
exit_code: 1
report_matches_sources: false
```

Workflow replay product gate:

```text
command:
  cargo run -p nando-cli --release -- phase-action-workflow-replay-v1

verify:
  cargo run -p nando-cli --release -- phase-action-workflow-replay-verify-v1

report:
  target/nando-wave/action-runtime-v1-workflow-replay.product-proof.json

verdict: PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS
verify_verdict: PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_PASS
workflow_sessions: 256
steps_per_session: 12
workflow_trace_calls: 3072
package_aliases: generated_action, domain_action, coverage_action
package_count: 3
all_packages_observed: true
sessions_cover_all_packages: true
total_unique_eval_rows: 308
replay_unique_rows: 308
exact_cache_llm_calls: 308
exact_cache_hits: 2764
exact_cache_plus_nando_llm_calls: 36
nando_local_operator_calls: 2780
nando_fallback_events: 292
incremental_llm_calls_removed_vs_cache: 272
incremental_llm_call_reduction_vs_cache_milli: 883
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

This closes a deterministic multi-package workflow replay over frozen `.nwpc`
packages and binary eval-packs. It is stronger than the old 48-row
domain_action workflow smoke, but it is not a real external pilot, raw action
parser, text generation, dynamic workflow traffic, or commercial license.

Workflow replay tamper control:

```text
tamper: replay_unique_rows 308 -> 307
verdict: PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_WATCH
exit_code: 1
report_gate_pass: false
report_matches_sources: false
```

Replay-anchored regression/freeze:

```text
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS

regression_report_fingerprint64: 2002304595771295125
workflow_replay_report_fingerprint64: 16637049491119000274
workflow_replay_verify_pass: true
workflow_replay_report_matches_sources: true
workflow_replay_trace_calls: 3072
workflow_replay_unique_rows: 308
workflow_replay_exact_cache_llm_calls: 308
workflow_replay_exact_cache_plus_nando_llm_calls: 36
workflow_replay_incremental_llm_calls_removed_vs_cache: 272
workflow_replay_local_accuracy_milli: 1000
workflow_replay_false_local_accepts: 0
```

Boundary:

```text
The workflow replay proof is now part of the regression/freeze chain. This is
still deterministic frozen-package replay, not raw action parsing, dynamic
pilot traffic, text generation, or commercial license closure.
```

Короткая фиксация:

```text
Nando Wave должен стать proof-gated operator layer:
LLM находит и объясняет новые решения,
Nando Wave компилирует повторяемые переходы,
CPU исполняет их локально, дешево и проверяемо.
```
