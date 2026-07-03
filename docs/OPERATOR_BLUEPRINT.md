# Чертёж операторов Nando Wave

Этот документ фиксирует компактную операторную грамматику для следующих v5/v6
рубежей. Цель не в том, чтобы раздуть список до сотен ручных правил, а в том,
чтобы собрать маленький алфавит действий, из которого строятся переносимые
workflow / reasoning transitions.

## Главная формула

```text
Operator = SELECT + TRANSFORM + WRITE + CONDITION + CHECK
```

Любой оператор должен отвечать на пять вопросов:

```text
что взять?
что сделать?
куда записать?
при каком условии?
как проверить?
```

Цель Nando Wave:

```text
state_t + action_tree -> state_t+1
```

А следующий уровень мышления:

```text
state_t + goal -> выбрать action_tree -> применить -> проверить -> repair/reject
```

## Почему не список из тысячи операторов

Неправильный путь:

```text
1000 отдельных named operators
```

Правильный путь:

```text
маленький набор атомов
+ параметры
+ композиция
+ proof gates
```

Модель должна учить структуру действия:

```text
какой объект взять
что с ним сделать
куда положить
по какому условию выбрать ветку
как проверить результат
```

а не запоминать готовую slot-map.

## Запрещённый формат оператора

Нельзя давать модели готовый ответ:

```text
out0 = src7
out1 = src2
out2 = src5
```

Это превращает action в target leak.

Правильный формат:

```text
move marked span after beta block
reverse selected window
if status is pending, normalize priority field
extract amount field and compare with limit
```

Action должен описывать смысл действия, а не готовую таблицу ответа.

Rust gate for the next clean corpus:

```text
cargo run -p nando-cli --release -- phase-action-contract-v1
```

This gate accepts only `action_tree = SELECT + TRANSFORM + WRITE + CONDITION +
CHECK`. It rejects slot maps (`out0 = src7` / `src0` style), proof-rule labels,
target leaks, `local_out_t`, lookup authority, and arrow demos. It is a corpus
contract gate only, not a runtime proof.

First Rust runtime smoke for a clean contract:

```text
cargo run -p nando-cli --release -- phase-action-runtime-v1
```

This command validates the contract, compiles train rows into
`nando_core::PhaseCenterFlatRuntime`, scores heldout correct-vs-wrong
transitions, and checks an action-ablation control. It is still a smoke rung,
not a broad action-router proof.

First saved package rung:

```text
cargo run -p nando-cli --release -- phase-action-corpus-v1
cargo run -p nando-cli --release -- phase-action-domain-corpus-v1
cargo run -p nando-cli --release -- phase-action-coverage-corpus-v1
cargo run -p nando-cli --release -- phase-action-shortcut-v1
cargo run -p nando-cli --release -- phase-action-operator-coverage-v1
cargo run -p nando-cli --release -- phase-action-package-v1
cargo run -p nando-cli --release -- phase-action-package-inspect-v1
cargo run -p nando-cli --release -- phase-action-source-verify-v1
cargo run -p nando-cli --release -- phase-action-package-score-v1
cargo run -p nando-cli --release -- phase-action-package-verify-v1
```

The corpus command generates a deterministic clean `action_contract_v1` JSONL
corpus in Rust. It is a corpus factory, not a proof by itself; the proof path is
the separate contract/package/inspect gate sequence over the written JSONL.
The domain corpus command generates a deterministic workflow-shaped
`action_contract_v1` corpus in Rust. It is deliberately bounded: it proves the
same package path on operationally named spans, not broad domain reasoning.
The shortcut command rejects exact lookup, token overlap, heldout length reuse,
bag-of-tokens separation, and source-bigram wins before package proof is trusted.
Its shortcut report is a required release-suite artifact, not a side note:
each package artifact must carry `shortcut_report_gate_pass: true`,
`shortcut_report_matches_corpus: true`, and the suite must carry
`all_shortcut_reports_pass: true`.
The operator coverage command is a separate audit over the same clean
`action_tree` contract. It does not use labels as authority and does not score
the runtime. It counts diversity in all five operator dimensions:
`SELECT`, `TRANSFORM`, `WRITE`, `CONDITION`, and `CHECK`.

The package command writes a `.nwpc` runtime plus manifest, reloads the package
through `PhaseCenterFlatRuntime::from_bytes`, and scores heldout through the
loaded runtime. The inspect command verifies package/manifest fingerprint,
record count, operator keys, forbidden flags, and saved gate metrics.
The source-verify command rebuilds the package from `manifest.corpus_path`
through the Rust `PhaseCenterCompiler` and requires exact `.nwpc` byte equality.
The score command evaluates an already-saved action package through
`PhaseCenterFlatRuntime` without recompiling the package. The verify command
checks package, manifest, and score report as one product proof artifact.

## Компактное дерево классов

```text
STATE TRANSITION
├── SELECT
├── MOVE / COPY
├── EDIT
├── ORDER
├── FIELD
├── FILTER / GROUP
├── CONDITION / ROUTE
├── COMPOSE
└── VERIFY / REPAIR
```

## 1. SELECT

Задача:

```text
найти нужный кусок состояния
```

Families:

```text
select_slot
select_span
select_field
select_by_marker
select_by_predicate
select_window
```

Проверка:

```text
новые fillers/surfaces
heldout markers
ablation marker/predicate channel collapses
no exact lookup
```

## 2. MOVE / COPY

Задача:

```text
перенести или скопировать кусок состояния
```

Families:

```text
move_slot
move_span
copy_slot
copy_span
swap_slots
swap_spans
```

Проверка:

```text
same-bag negatives
strict slot readout
sequence energy correct > wrong
copy/move ablation collapses
```

## 3. EDIT

Задача:

```text
изменить состояние
```

Families:

```text
insert
delete
replace
clear
append
prepend
```

Проверка:

```text
wrong edit target
wrong edit position
wrong inserted filler
heldout edit surfaces
no target_id
```

## 4. ORDER

Задача:

```text
изменить порядок без потери элементов
```

Families:

```text
reverse
rotate
block_swap
window_reverse
interleave
stable_reorder
```

Проверка:

```text
correct/wrong same token bag
strict ordered decoder
sequence energy
mirror/symmetry consistency
multi-seed
```

## 5. FIELD

Задача:

```text
работать не только с буквами/слотами, а с полями и структурами
```

Families:

```text
extract_field
merge_fields
split_field
normalize_field
compare_fields
```

Проверка:

```text
new field names
new field values
wrong normalized form
wrong compared pair
field-channel ablation
```

## 6. FILTER / GROUP

Задача:

```text
выбрать, сгруппировать или отсортировать элементы
```

Families:

```text
filter_by_predicate
partition
group_by_key
stable_sort_by_key
deduplicate
```

Проверка:

```text
predicate heldout
same input bag with wrong kept/removed subset
stable order preservation
group-key ablation
```

## 7. CONDITION / ROUTE

Задача:

```text
выбрать действие по условию
```

Families:

```text
if_then_else
route_by_marker
route_by_field
route_by_compare
route_by_state
```

Проверка:

```text
then/else both present in action text
selected branch must come from state/action conjunction
wrong branch traps
condition ablation collapses
no proof_rule_id authority
```

## 8. COMPOSE

Задача:

```text
собрать цепочку действий
```

Families:

```text
A_then_B
A_then_if_B_else_C
repeat_n
verify_then_repair
```

Проверка:

```text
depth 2 first
then depth 3
intermediate-state traps
wrong-order composition traps
composed-demo/action ablation
```

## 9. VERIFY / REPAIR

Задача:

```text
не просто сделать переход, а проверить и стабилизировать результат
```

Families:

```text
check_same_bag
check_field_constraint
check_order_constraint
check_no_conflict
repair_unset_slot
reject_unsettled
```

Проверка:

```text
wrong answer rejected
low-gap answer marked unsettled
repair improves strict slot without target leak
repair/cleanup ablation collapses
```

## Минимальный атомный набор

Ядро можно свести примерно к 12 атомам:

```text
1. select_slot
2. select_span
3. select_field
4. select_by_predicate
5. move
6. copy
7. delete
8. insert
9. replace
10. normalize
11. compare/route
12. compose
```

Остальные operator families должны быть параметризацией или композицией этих
атомов, а не отдельными ручными сущностями.

## V5 proof battery

Ближайшая сильная батарея:

```text
9 classes
24-32 operator families
depth 1 and depth 2
lengths 9-16
3 seeds minimum
strict slot + sequence energy + cleanup + ablations
```

Примерное распределение:

```text
SELECT:       4 families
MOVE/COPY:    4 families
EDIT:         4 families
ORDER:        6 families
FIELD:        4 families
FILTER:       3 families
CONDITION:    3 families
COMPOSE:      4 families
VERIFY:       2 families
```

Итого:

```text
30 families
```

## Operator Dimension Coverage Audit

Action-tree key coverage is not enough by itself. A corpus can contain many
distinct full `action_tree` keys while still varying only one or two dimensions.
That proves packaging provenance, but not full operator-class coverage.

Rust audit:

```text
cargo run -p nando-cli --release -- phase-action-operator-coverage-v1 \
  [contract-jsonl] [report-json]
```

The gate is intentionally narrower than a runtime proof. It must prove only that
the corpus gives the compiler more than a one-dimensional operator surface:

```text
action_tree_key_count >= 6
select_value_count >= 2
transform_value_count >= 2
write_value_count >= 2
condition_value_count >= 2
check_value_count >= 2
train_dimension_coverage_pass: true
heldout_dimension_coverage_pass: true
full_operator_dimension_coverage_pass: true
label_authority_used: false
python_demo_used: false
```

Current audit result:

```text
generated_action:
  verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_WATCH
  rows: 200
  action_tree_key_count: 10
  select_value_count: 1
  transform_value_count: 10
  write_value_count: 1
  condition_value_count: 1
  check_value_count: 8
  min_dimension_value_count: 1
  wide_dimension_count: 2
  full_operator_dimension_coverage_pass: false

domain_action:
  verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_WATCH
  rows: 120
  action_tree_key_count: 6
  select_value_count: 6
  transform_value_count: 6
  write_value_count: 1
  condition_value_count: 1
  check_value_count: 6
  min_dimension_value_count: 1
  wide_dimension_count: 3
  full_operator_dimension_coverage_pass: false

coverage_action:
  verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_PASS
  rows: 360
  action_tree_key_count: 30
  select_value_count: 6
  transform_value_count: 10
  write_value_count: 5
  condition_value_count: 5
  check_value_count: 10
  min_dimension_value_count: 5
  wide_dimension_count: 5
  train_dimension_coverage_pass: true
  heldout_dimension_coverage_pass: true
  full_operator_dimension_coverage_pass: true
  label_authority_used: false
  python_demo_used: false

release suite:
  artifact_count: 3
  all_operator_coverage_reports_match_sources: true
  operator_dimension_coverage_artifact_count: 1
  release_operator_dimension_coverage_pass: true
  max_operator_coverage_min_dimension_value_count: 5
  max_operator_coverage_wide_dimension_count: 5
```

Claim boundary:

```text
The old generated_action/domain_action audits remain WATCH because those
corpora are intentionally bounded. They do not invalidate the release suite:
their coverage reports match sources, while coverage_action supplies the full
SELECT + TRANSFORM + WRITE + CONDITION + CHECK coverage artifact.

This closes operator-dimension coverage for the packaged flat action scorer
release chain. It is not strict ordered decoder, 32-slot operator transfer,
autonomous raw action parsing, text generation, broad workflow reasoning,
every production cache policy, or commercial licensing.
```

## V6 enterprise/workflow battery

Enterprise workflow почти всегда раскладывается в эти действия:

```text
select field
normalize
compare
route
update status
verify
repair
```

Пример:

```text
document received
-> extract_field
-> normalize_field
-> compare_fields
-> route_by_condition
-> replace_status
-> verify_required_fields
```

Это переводит Nando Wave от synthetic sequence gates к реальным
state-transition workflows.

## Proof invariants

Каждый новый operator family должен сохранять границы claim:

```text
no exact lookup
no target_id
no proof_rule_id authority
no concrete_x_lookup
no fixed answer template
no fixed frame_id
no manual local_out_t
no hand-coded bind(X)
```

Обязательные gates:

```text
shortcut gates clean
same-bag or structurally equivalent traps where applicable
heldout fillers/surfaces/seeds
sequence energy correct > wrong
strict slot readout green
flat/runtime parity exact
all_package_report_parity_pass: true
ablation of required channel collapses
all_action_ablation_collapses: true
cleanup ablation collapses when cleanup is used
failure breakdown recorded
```

## Product meaning

Этот чертёж нужен не ради красивой классификации. Он нужен, чтобы сделать:

```text
compact transferable operator grammar
```

для локального CPU operator runtime.

Если v5/v6 докажут эту грамматику, продуктовый claim становится:

```text
Nando Wave compiles repeated LLM/agent workflows into local proof-gated
state-transition operators.
```

По-русски:

```text
Nando Wave компилирует повторяемые workflow AI-агентов в локальные проверяемые
операторы перехода состояния.
```

## Runtime Package Contract

Операторный runtime считается движущимся к продукту только если он проходит
упакованный Rust-путь:

```text
contract JSONL
-> .nwpc flat runtime package
-> manifest
-> binary eval-pack
-> score-pack report
-> verify bundle
```

Python demos are no longer proof artifacts.

Accepted proof path:

```text
Rust corpus/contract gate
-> Rust compiler/package
-> binary eval-pack
-> flat CPU score-pack
-> shortcut report aggregation
-> benchmark-pack report
-> product/release/license/offload/regression reports
```

Score-pack report must prove:

```text
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
manifest_matches_package: true
score_report_matches_package: true
```

Benchmark-pack report must prove:

```text
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used_in_bench_loop: false
manifest_matches_package: true
eval_pack_matches_package: true
bench_report_matches_package: true
```

Product-proof bundle must prove:

```text
manifest_matches_package: true
eval_pack_matches_package: true
score_report_matches_package: true
bench_report_matches_package: true
product_report_matches_package: true
source_rebuild_matches_package: true
score_report_verdict: PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS
bench_report_verdict: PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS
compiler_used: false
forbidden_used: false
```

Release-suite report must prove:

```text
release_suite_kind: phase_action_release_suite_v1
artifact_count >= 2
distinct_package_fingerprints: true
each artifact product_verify_pass: true
all score/bench/product proof gates: true
each artifact source_verify_report_gate_pass: true
each artifact source_verify_report_matches_package: true
all_source_verify_reports_pass: true
all_manifest_package_parity_pass: true
all_eval_pack_package_parity_pass: true
all_score_report_package_parity_pass: true
all_bench_report_package_parity_pass: true
all_product_report_package_parity_pass: true
all_source_rebuild_package_parity_pass: true
all_source_verify_report_package_parity_pass: true
all_package_report_parity_pass: true
all_action_ablation_collapses: true
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
forbidden_used: false
commercial_license_closed: false
report_matches_sources: true during verify
```

Non-commercial license package must prove:

```text
license_package_kind: phase_action_noncommercial_license_package_v1
license_file: LICENSE-NONCOMMERCIAL.md
license_name: Nando Wave Non-Commercial Source License v1.0
license_file_contains_noncommercial_grant: true
license_file_contains_commercial_restriction: true
license_file_contains_no_warranty: true
cargo_workspace_license_file_declared: true
cargo_workspace_mit_license_declared: false
cargo_crate_license_file_workspace_declared: true
cargo_crate_license_workspace_declared: false
release_suite_gate_pass: true
release_suite_matches_sources: true
release_suite_license_boundary_mentions_mit: false
commercial_use_allowed: false
noncommercial_use_allowed: true
commercial_license_closed: false
non_commercial_license_closed: true
```

Daemon deployment package must prove:

```text
phase-action-daemon-live-proof-suite-v1: PASS
phase-action-daemon-systemd-smoke-v1: PASS
phase-action-daemon-deployment-package-v1: PASS
phase-action-daemon-deployment-verify-v1: PASS
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
registry_config_package_count_matches: true
deployment_artifacts_present: true
report_matches_sources: true during deployment verify
```

Daemon deployment boundary:

```text
This is a local deployment package proof over the HTTP daemon, generated
systemd service unit, env file, and registry config.

It is not an installed/running systemd service, TLS setup, dynamic reload,
external pilot workflow, or commercial license closure.
```

Current action contract rung:

```text
generated action package:
  phase-action-package-score-pack-v1 PASS
  phase-action-source-verify-v1 PASS
  phase-action-package-bench-pack-v1 PASS
  phase-action-product-proof-v1 PASS
  phase-action-release-suite-v1 PASS
  phase-action-license-package-v1 PASS
  source_contract_fingerprint64: 6845017756676715377
  source_contract_bytes: 122500
  source_rebuild_matches_package: true
  source_rebuild_package_fingerprint64: 14869999570221545448
  source_rebuild_package_bytes: 10256
  source_rebuild_operator_keys_match: true
  accuracy_milli: 1000
  bench_pack_p99_latency_ns: 86
  action_ablation_accuracy_milli: 450

domain action package:
  phase-action-package-score-pack-v1 PASS
  phase-action-source-verify-v1 PASS
  phase-action-package-bench-pack-v1 PASS
  phase-action-product-proof-v1 PASS
  phase-action-release-suite-v1 PASS
  phase-action-license-package-v1 PASS
  source_contract_fingerprint64: 9195974547787197795
  source_contract_bytes: 101612
  source_rebuild_matches_package: true
  source_rebuild_package_fingerprint64: 5367415087033800111
  source_rebuild_package_bytes: 6160
  source_rebuild_operator_keys_match: true
  accuracy_milli: 1000
  bench_pack_p99_latency_ns: 83
  action_ablation_accuracy_milli: 567

release-suite:
  artifact_count: 2
  total_runtime_bytes_estimate: 16896
  total_bench_samples: 128000
  total_source_verify_report_bytes: 3370
  total_shortcut_report_bytes: 2037
  all_source_verify_reports_pass: true
  all_shortcut_reports_pass: true
  all_manifest_package_parity_pass: true
  all_eval_pack_package_parity_pass: true
  all_score_report_package_parity_pass: true
  all_bench_report_package_parity_pass: true
  all_product_report_package_parity_pass: true
  all_source_rebuild_package_parity_pass: true
  all_source_verify_report_package_parity_pass: true
  all_package_report_parity_pass: true
  all_action_ablation_collapses: true
  max_score_action_ablation_accuracy_milli: 567
  max_bench_action_ablation_accuracy_milli: 567
  total_score_action_ablation_wrong_wins: 500
  total_bench_action_ablation_wrong_wins: 500000
  max_bench_p99_latency_ns: 86
  commercial_license_closed: false
  generated_action_source_verify_report_fingerprint64: 1146913633358320141
  generated_action_source_verify_report_bytes: 1678
  domain_action_source_verify_report_fingerprint64: 4273906322392560821
  domain_action_source_verify_report_bytes: 1692

license-package:
  license_file_fingerprint64: 17377756494518932165
  cargo_workspace_mit_license_declared: false
  non_commercial_license_closed: true
  commercial_license_closed: false

offload-audit:
  phase-action-offload-audit-v1 PASS
  phase-action-offload-verify-v1 PASS
  margin_threshold_micro: 300000
  simulated_calls: 1000
  local_operator_calls: 904
  fallback_to_llm_calls: 96
  offload_rate_milli: 904
  local_accuracy_milli: 1000
  false_local_accepts: 0
  compiler_used: false
  corpus_jsonl_used: false
  python_demo_used: false
  forbidden_used: false
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

regression:
  phase-action-regression-v1 PASS
  phase-action-regression-verify-v1 PASS
  release_verify_pass: true
  license_verify_pass: true
  offload_verify_pass: true
  release_suite_report_fingerprint64: 12340473504052004295
  release_suite_report_bytes: 10810
  license_package_report_fingerprint64: 17769124418356895286
  license_package_report_bytes: 2053
  offload_audit_report_fingerprint64: 6122761890799637522
  offload_audit_report_bytes: 6281
  total_source_verify_report_bytes: 3370
  total_shortcut_report_bytes: 2037
  all_source_verify_reports_pass: true
  all_shortcut_reports_pass: true
  all_manifest_package_parity_pass: true
  all_eval_pack_package_parity_pass: true
  all_score_report_package_parity_pass: true
  all_bench_report_package_parity_pass: true
  all_product_report_package_parity_pass: true
  all_source_rebuild_package_parity_pass: true
  all_source_verify_report_package_parity_pass: true
  all_package_report_parity_pass: true
  all_action_ablation_collapses: true
  max_score_action_ablation_accuracy_milli: 567
  max_bench_action_ablation_accuracy_milli: 567
  total_score_action_ablation_wrong_wins: 500
  total_bench_action_ablation_wrong_wins: 500000
  release_suite_report_fingerprint64 != 0
  release_suite_report_bytes > 0
  license_package_report_fingerprint64 != 0
  license_package_report_bytes > 0
  offload_audit_report_fingerprint64 != 0
  offload_audit_report_bytes > 0
  operator_blueprint_path: docs/OPERATOR_BLUEPRINT.md
  operator_blueprint_formula_present: true
  operator_blueprint_runtime_package_contract_present: true
  operator_blueprint_source_verify_contract_present: true
  operator_blueprint_shortcut_report_contract_present: true
  operator_blueprint_rust_proof_path_present: true
  operator_blueprint_proof_invariants_present: true
  operator_blueprint_forbidden_invariants_present: true
  offload_sdk_api: nando_core::PhaseCenterOffloadRuntime
  offload_sdk_inspect_api: nando_core::PhaseCenterOffloadRuntime::inspect_package_bytes
  state_transition_formula: state_t + action_tree -> state_t+1
  python_demo_used: false
  forbidden_used: false
```

Public Rust SDK consumer proof:

```text
cargo test -p nando-core --test phase_center_offload_sdk_public
  PASS

This proves an external Rust consumer can load packaged .nwpc bytes through
nando_core::PhaseCenterOffloadRuntime, inspect package header/fingerprint
through PhaseCenterOffloadRuntime::inspect_package_bytes before loading, and
route local_operator vs fallback_to_llm without private CLI glue.
```

This does not close strict ordered decoder or autonomous raw action parsing. It
only closes the packaged flat scorer path for the clean action_tree contract,
now anchored to this blueprint by fingerprint and required contract strings.

## 2026-07-02 Clean ActionTree Source Rebuild Release Contract

Python demos are not proof artifacts. The release/regression chain must use the
Rust CLI and packaged flat runtime reports only. For the action contract path,
the source rebuild must prove that the package is rebuildable from clean
`action_tree` rows and that forbidden authority did not enter the contract.

Required product/source fields:

```text
source_rebuild_contract_gate_pass: true
source_rebuild_accepted_action_tree_rows > 0
source_rebuild_rejected_action_tree_rows: 0
source_rebuild_forbidden_operator_label_rows: 0
source_rebuild_forbidden_slot_map_rows: 0
source_rebuild_forbidden_target_leak_rows: 0
source_rebuild_forbidden_lookup_authority_rows: 0
source_rebuild_forbidden_local_out_t_rows: 0
source_rebuild_forbidden_arrow_demo_rows: 0
source_rebuild_concrete_output_token_leak_rows: 0
source_rebuild_action_tree_key_count >= 6
source_rebuild_train_action_tree_key_count == source_rebuild_action_tree_key_count
source_rebuild_heldout_action_tree_key_count == source_rebuild_action_tree_key_count
source_rebuild_min_train_rows_per_action_tree > 0
source_rebuild_min_heldout_rows_per_action_tree > 0
```

Required release/regression aggregate:

```text
all_action_contract_source_rebuild_clean: true
total_source_rebuild_accepted_action_tree_rows > 0
total_source_rebuild_rejected_action_tree_rows: 0
total_source_rebuild_forbidden_contract_rows: 0
total_source_rebuild_action_tree_key_count >= artifact_count * 6
min_source_rebuild_action_tree_key_count >= 6
all_action_tree_key_coverage_pass: true
all_operator_coverage_reports_match_sources: true
release_operator_dimension_coverage_pass: true
max_operator_coverage_min_dimension_value_count >= 2
max_operator_coverage_wide_dimension_count == 5
all_package_report_parity_pass: true
all_action_ablation_collapses: true
all_optimized_build_reports_pass: true
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Current green runtime package evidence:

```text
release_suite_report_fingerprint64: 9827723825761118426
artifact_count: 3
total_runtime_bytes_estimate: 48576
total_bench_samples: 308000
all_action_contract_source_rebuild_clean: true
total_source_rebuild_accepted_action_tree_rows: 680
total_source_rebuild_rejected_action_tree_rows: 0
total_source_rebuild_forbidden_contract_rows: 0
total_source_rebuild_action_tree_key_count: 46
min_source_rebuild_action_tree_key_count: 6
all_action_tree_key_coverage_pass: true
all_operator_coverage_reports_match_sources: true
operator_dimension_coverage_artifact_count: 1
release_operator_dimension_coverage_pass: true
max_operator_coverage_min_dimension_value_count: 5
max_operator_coverage_wide_dimension_count: 5
all_package_report_parity_pass: true
max_score_p99_latency_ns: 542
max_bench_p99_latency_ns: 117
offload_rate_milli: 880
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks required for this contract:

```text
tamper product source_rebuild_forbidden_operator_label_rows=1 -> product verify WATCH
tamper release all_action_contract_source_rebuild_clean=false -> release verify WATCH
tamper regression all_action_contract_source_rebuild_clean=false -> regression verify WATCH
tamper source source_rebuild_action_tree_key_count=1 -> release verify WATCH
tamper release all_action_tree_key_coverage_pass=false -> regression verify WATCH
tamper regression all_action_tree_key_coverage_pass=false -> freeze verify WATCH
tamper daemon deployment live_suite_step_count=11 -> deployment verify WATCH
tamper workflow replay replay_unique_rows=307 -> workflow replay verify WATCH
```

Claim boundary:

```text
This closes clean action_tree source rebuild provenance for the packaged flat
action scorer. It is not autonomous raw action parsing, strict ordered decoder,
text generation, commercial license closure, or broad workflow reasoning.
```

## 2026-07-02 Optimized Build Proof Contract

Python demos are not proof artifacts. Debug builds are not benchmark proof
artifacts either. The packaged flat action scorer proof must be produced by
the Rust release/optimized path and must expose that fact in machine-readable
reports.

Required score/bench/product fields:

```text
optimized_build: true
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
corpus_jsonl_used_in_bench_loop: false
python_demo_used: false
forbidden_used: false
```

Required release/regression aggregate:

```text
all_optimized_build_reports_pass: true
all_package_report_parity_pass: true
all_action_contract_source_rebuild_clean: true
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks required for this contract:

```text
debug score-pack report optimized_build=false -> score-pack WATCH
debug bench-pack report optimized_build=false -> bench-pack WATCH
tamper product optimized_build=false -> product verify WATCH
tamper release all_optimized_build_reports_pass=false -> release verify WATCH
tamper regression all_optimized_build_reports_pass=false -> regression verify WATCH
```

Claim boundary:

```text
This proves optimized release-build packaging evidence for the packaged flat
action scorer. It does not close strict ordered decoder, text generation,
autonomous raw action parsing, broad workflow reasoning, or commercial license.
```

## 2026-07-02 Green Regression Freeze Contract

`phase-action-regression-v1` builds the green regression proof. The freeze rung
is a separate checkpoint over that proof, so the current green state can be
verified later without trusting prose, terminal history, or Python demos.

Commands:

```text
cargo run --release -p nando-cli -- phase-action-regression-freeze-v1
cargo run --release -p nando-cli -- phase-action-regression-freeze-verify-v1
```

Required freeze fields:

```text
regression_verdict: PHASE_ACTION_REGRESSION_V1_PASS
regression_gate_pass: true
regression_matches_sources: true
regression_report_fingerprint64 != 0
regression_report_bytes > 0
cache_offload_bench_report_fingerprint64 != 0
cache_offload_bench_report_bytes > 0
cache_bench_verify_pass: true
cache_bench_report_matches_sources: true
cache_incremental_llm_calls_removed_vs_cache > 0
all_package_report_parity_pass: true
all_action_contract_source_rebuild_clean: true
total_source_rebuild_action_tree_key_count >= artifact_count * 6
min_source_rebuild_action_tree_key_count >= 6
all_action_tree_key_coverage_pass: true
all_optimized_build_reports_pass: true
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
state_transition_formula: state_t + action_tree -> state_t+1
```

Negative checks required for this contract:

```text
tamper freeze all_optimized_build_reports_pass=false -> freeze verify WATCH
tamper regression all_optimized_build_reports_pass=false -> freeze build WATCH
tamper freeze cache_bench_verify_pass=false -> freeze verify WATCH
tamper regression cache_offload_bench_report_fingerprint64=1 -> regression verify WATCH
tamper regression all_action_tree_key_coverage_pass=false -> freeze verify WATCH
```

Claim boundary:

```text
This freezes a green packaged flat action scorer regression checkpoint with
release/license/offload/cache-benchmark source anchors. It is not strict ordered
decoder, text generation, autonomous raw action parsing, broad workflow
reasoning, or commercial license closure.
```

## 2026-07-02 Cache-Enabled Offload Benchmark Contract

Nando Wave offload must be measured against a cache-enabled agent baseline, not
only against a naive no-cache LLM baseline.

Commands:

```text
cargo run --release -p nando-cli -- phase-action-cache-offload-bench-v1
cargo run --release -p nando-cli -- phase-action-cache-offload-bench-verify-v1
```

The benchmark compares three paths:

```text
A. no-cache LLM/agent:
   every simulated call reaches the LLM

B. exact-cache LLM/agent:
   first unique eval row reaches the LLM,
   repeated exact row is a cache hit

C. exact-cache + Nando Wave CPU operator:
   high-margin local operator rows do not reach the LLM,
   low-margin rows fall back to the exact-cache path
```

Required fields:

```text
no_cache_llm_calls > exact_cache_llm_calls
exact_cache_llm_calls > exact_cache_plus_nando_llm_calls
incremental_llm_calls_removed_vs_cache > 0
exact_cache_hits > 0
nando_local_operator_calls > 0
nando_fallback_events > 0
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Current release benchmark:

```text
simulated_calls: 1000
no_cache_llm_calls: 1000
exact_cache_llm_calls: 128
exact_cache_hits: 872
exact_cache_hit_rate_milli: 872
exact_cache_plus_nando_llm_calls: 4
nando_local_operator_calls: 968
nando_fallback_events: 32
nando_operator_hit_rate_milli: 968
incremental_llm_calls_removed_vs_cache: 124
incremental_llm_call_reduction_vs_cache_milli: 969
token_units_removed_vs_cache: 124
cost_units_removed_vs_cache: 124
local_accuracy_milli: 1000
false_local_accepts: 0
max_bench_p99_latency_ns: 117
```

Negative checks required:

```text
tamper incremental_llm_calls_removed_vs_cache=0 -> cache bench verify WATCH
tamper python_demo_used=true -> cache bench verify WATCH
margin_threshold_micro too low, no fallback events -> cache bench WATCH
```

Regression/freeze promotion:

```text
phase-action-regression-v1 must read the cache-offload benchmark report,
rebuild it from release/license/package sources, and record:
  cache_offload_bench_report_fingerprint64 != 0
  cache_offload_bench_report_bytes > 0
  cache_bench_verify_pass: true
  cache_bench_report_matches_sources: true
  cache_bench_verdict: PHASE_ACTION_CACHE_OFFLOAD_BENCH_V1_PASS
  cache_exact_cache_llm_calls > cache_exact_cache_plus_nando_llm_calls
  cache_incremental_llm_calls_removed_vs_cache > 0
  cache_local_accuracy_milli: 1000
  cache_false_local_accepts: 0

phase-action-regression-freeze-v1 must carry the same cache-bench anchors.
```

Claim boundary:

```text
This proves incremental local CPU operator offload over an exact-cache
baseline for the packaged flat action scorer. It is not strict ordered decoder,
text generation, autonomous raw action parsing, broad workflow reasoning, or
commercial license closure.
```

## 2026-07-02 Workflow Replay Contract

The old `phase-action-workflow-bench-v1` remains a narrow domain_action smoke.
The product-facing replay gate must exercise all frozen release packages as a
deterministic workflow trace, without recompiling from JSONL:

```text
cargo run --release -p nando-cli -- phase-action-workflow-replay-v1
cargo run --release -p nando-cli -- phase-action-workflow-replay-verify-v1
```

Workflow replay report must prove:

```text
workflow_replay_kind: phase_action_workflow_replay_v1
release_suite_gate_pass: true
release_suite_matches_sources: true
license_package_gate_pass: true
license_report_matches_sources: true
workflow_sessions >= 128
steps_per_session >= 6
workflow_trace_calls == workflow_sessions * steps_per_session
package_count >= 3
all_packages_observed: true
sessions_cover_all_packages: true
total_unique_eval_rows >= 300
replay_unique_rows == total_unique_eval_rows
exact_cache_llm_calls == replay_unique_rows
exact_cache_plus_nando_llm_calls < exact_cache_llm_calls
incremental_llm_calls_removed_vs_cache > 0
nando_local_operator_calls > 0
nando_fallback_events > 0
local_accuracy_milli: 1000
false_local_accepts: 0
max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
python_demo_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
forbidden_used: false
commercial_license_closed: false
```

Current workflow replay:

```text
verdict: PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS
verify_verdict: PHASE_ACTION_WORKFLOW_REPLAY_VERIFY_V1_PASS
workflow_trace_calls: 3072
package_aliases: generated_action, domain_action, coverage_action
total_unique_eval_rows: 308
replay_unique_rows: 308
exact_cache_llm_calls: 308
exact_cache_plus_nando_llm_calls: 36
incremental_llm_calls_removed_vs_cache: 272
incremental_llm_call_reduction_vs_cache_milli: 883
local_accuracy_milli: 1000
false_local_accepts: 0
```

Negative checks required:

```text
tamper workflow replay replay_unique_rows=307 -> workflow replay verify WATCH
```

Claim boundary:

```text
This proves deterministic multi-package workflow replay over frozen `.nwpc`
packages and binary eval-packs. It is not raw action parsing, text generation,
dynamic real pilot traffic, broad workflow reasoning, or commercial license
closure.
```

## 2026-07-02 Workflow Replay Regression/Freeze Anchor

`phase-action-regression-v1` and `phase-action-regression-freeze-v1` must carry
the workflow replay report, not just the small `phase-action-workflow-bench-v1`
smoke:

```text
phase-action-regression-v1 \
  [release-suite-report-json] \
  [license-file] \
  [license-package-report-json] \
  [offload-audit-report-json] \
  [regression-report-json] \
  [cache-offload-bench-report-json] \
  [workflow-bench-report-json] \
  [workflow-replay-report-json]

phase-action-regression-freeze-v1 \
  [release-suite-report-json] \
  [license-file] \
  [license-package-report-json] \
  [offload-audit-report-json] \
  [regression-report-json] \
  [regression-freeze-report-json] \
  [cache-offload-bench-report-json] \
  [workflow-bench-report-json] \
  [workflow-replay-report-json]
```

Required replay anchor:

```text
workflow_replay_verify_pass: true
workflow_replay_report_matches_sources: true
workflow_replay_verdict: PHASE_ACTION_WORKFLOW_REPLAY_V1_PASS
workflow_replay_package_count >= artifact_count
workflow_replay_trace_calls > 0
workflow_replay_total_unique_eval_rows > 0
workflow_replay_unique_rows == workflow_replay_total_unique_eval_rows
workflow_replay_exact_cache_llm_calls > workflow_replay_exact_cache_plus_nando_llm_calls
workflow_replay_exact_cache_llm_calls ==
  workflow_replay_exact_cache_plus_nando_llm_calls
  + workflow_replay_incremental_llm_calls_removed_vs_cache
workflow_replay_incremental_llm_calls_removed_vs_cache > 0
workflow_replay_incremental_llm_call_reduction_vs_cache_milli > 0
workflow_replay_local_accuracy_milli: 1000
workflow_replay_false_local_accepts: 0
workflow_replay_max_bench_p99_latency_ns <= ACTION_BENCH_P99_NS_GATE
```

Current regression/freeze anchor:

```text
workflow_replay_report_fingerprint64: 16637049491119000274
workflow_replay_report_bytes: 5274
workflow_replay_verify_pass: true
workflow_replay_report_matches_sources: true
workflow_replay_trace_calls: 3072
workflow_replay_unique_rows: 308
workflow_replay_exact_cache_llm_calls: 308
workflow_replay_exact_cache_plus_nando_llm_calls: 36
workflow_replay_incremental_llm_calls_removed_vs_cache: 272
workflow_replay_local_accuracy_milli: 1000
workflow_replay_false_local_accepts: 0
regression_report_fingerprint64: see the current regression-freeze report
```

## Current V5 Coverage-Action Release Snapshot

This section supersedes older numeric cache/release anchors above when checking
the current packaged flat action scorer state.

Current release/regression/freeze anchors:

```text
artifact_count: 3
release_suite_report_fingerprint64: 9827723825761118426
total_runtime_bytes_estimate: 48576
total_bench_samples: 308000
total_source_rebuild_action_tree_key_count: 46
all_operator_coverage_reports_match_sources: true
operator_dimension_coverage_artifact_count: 1
release_operator_dimension_coverage_pass: true
max_operator_coverage_min_dimension_value_count: 5
max_operator_coverage_wide_dimension_count: 5
cache_incremental_llm_calls_removed_vs_cache: 272
cache_exact_cache_llm_calls: 308
cache_exact_cache_plus_nando_llm_calls: 36
offload_rate_milli: 880
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Artifact coverage boundary:

```text
generated_action and domain_action may keep operator coverage WATCH because
they are bounded corpora.

coverage_action supplies the full operator-dimension coverage:
  operator_coverage_report_verdict: PHASE_ACTION_OPERATOR_COVERAGE_V1_PASS
  full_operator_dimension_coverage_pass: true
  min_dimension_value_count: 5
  wide_dimension_count: 5
```

## Strict Multi-Seed Audit Command

Strict ordered robustness remains a separate gate. It must not be hidden inside
the flat action scorer release claim.

Current Rust audit commands:

```text
strict-multiseed-rust-audit-v1 [diagnostics-root] [audit-report-json]
strict-multiseed-rust-audit-verify-v1 [diagnostics-root] [audit-report-json]
```

Current artifact:

```text
target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json
```

Current result:

```text
verdict: STRICT_MULTI_SEED_RUST_AUDIT_PASS
verify: STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS
report_matches_sources: true
observed_logs: 12
missing_logs: 0
strict_runtime_issues: 0
logs_fingerprint64: 2824724535851559095
logs_total_bytes: 133299
evidence_warnings: 0
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_logs_used: true
```

Current strict runtime result:

```text
order/edit/conditional/composed, seeds 001/002/003:
  strict ordered slot readout: 1000
  flat strict slot readout: 1000
  sequence energy: 1000
  flat gap parity mismatches: 0
  flat sequence-energy parity mismatches: 0
  energy_pass_slot_fail: 0
  output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0

forbidden flags:
  target_center_id_training_used: false
  proof_rule_id_training_authority_used: false
  concrete_x_lookup_used: false
  local_out_t_runtime_extension_used: false
```

Boundary:

```text
This closes the current v4 16-slot strict multi-seed Rust runtime audit over
canonical release logs. It is not a 32-slot ordered decoder proof, not broad
workflow reasoning, not autonomous raw action parsing, not text generation, and
not Python demo authority.
```

Diagnostic subchannel caveat:

```text
Full-channel strict gates pass, and hard ablations for binding/action/role/
active_fringe collapse. Do not claim every diagnostic subchannel as a standalone
mechanism:
  edit marker_role ablation keeps energy high while strict accuracy falls to
  500 milli;
  conditional condition_action ablation keeps partial energy and seed_003 has
  3 milli strict accuracy.
```
