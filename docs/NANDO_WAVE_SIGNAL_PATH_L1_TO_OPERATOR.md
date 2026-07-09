# Nando Wave Signal Path: From N-Grams To Transferable Action

Дата среза: 2026-07-06

Назначение: показать, как сигнал идёт от surface n-grams до оператора действия,
какие структуры он проходит, какие размерности используются, и как это живёт в
CPU/runtime.

## Current Authoritative Contract

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Текущий signal path теперь надо читать так:

```text
real event stream
|
+-- source adapter
|   |
|   +-- parses Codex/agent/app-specific raw event
|   +-- core never depends on the source name
|
+-- L1/L2 safe atoms
|   |
|   +-- request atoms
|   +-- state atoms
|   +-- result atoms
|   +-- numeric atom ids for hot path
|
+-- L4 streaming goal-state layer
|   |
|   +-- route_id / profile_id / bucket_id
|   +-- opportunity ranking
|   +-- admission / eviction / top-K
|
+-- L3 drifting phase-center operator memory
|   |
|   +-- score_before_update
|   +-- positive/background centers
|   +-- margin/threshold
|
+-- PhaseCenterHotRuntime
    |
    +-- route_index + fixed phase_vector + scratch -> margin
    +-- accept remains shadow unless verifier-bound promotion passes
```

Current best compression signal:

```text
shadow frontier:
  denominator_rows: 29_770
  unique_cpu_accepts_over_exact_cache: 6_644
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0
  local_accept_enabled: false
```

Current hot runtime shape:

```text
hot_bytes_estimate: 592
warm_metadata_bytes_estimate: 39_136
hot_runtime_bytes_estimate: 544
hot_route_table_bytes_estimate: 48
```

## Current Active Signal Path: L4 -> L3 Phase-Center

Текущий активный путь продукта:

```text
real agent-loop event
|
+-- L1 / surface atoms
|   |
|   +-- request atoms
|   +-- state atoms
|   +-- result atoms
|
+-- L2 / event family and route hints
|   |
|   +-- action_family
|   +-- route_hint
|   +-- tool/result shape
|
+-- L4 Streaming Operator Router/Packer
|   |
|   +-- choose operator family/profile
|   +-- build phase-center input / shadow_request
|   +-- filter forbidden authority atoms
|
+-- L3 Phase-Center Runtime
    |
    +-- compare event vector to positive/negative phase centers
    +-- accept when margin/verifier boundary is safe
    +-- fallback otherwise
```

Разделение ролей:

```text
L3 = operator scorer / compact operator memory.
L4 = streaming router/packer that makes events visible to L3.
```

Текущий доказанный L3-сдвиг:

```text
planning_update:
  verifier_true cache-miss ceiling: 5939
  phase-center accepts: 2990
  class coverage: 50.35%
  false_accepts: 0
  wrong_wins: 0

compatible denominator v17:
  rows: 35_829
  CPU accepts over exact cache: 4160
  calls saved: 11.6107%
  tokens saved: 11.5390%
```

Текущая незакрытая L4-дыра:

```text
planning_update:
  rows_with_shadow_request: 0
  L4 packing coverage: 0 / 5939 = 0%
```

То есть фазовый центр уже доказал полезный оператор для `planning_update`, но
поточный L4-пакер ещё должен научиться собирать этот вход в live/daemon path.

Forbidden filters on the phase-center path:

```text
no verifier_label as score authority
no verified_safe_accept as score authority
no output_hash64 lookup
no target/proof authority
no concrete_x_lookup
no manual local_out_t
no legacy .nwrb role-binding backend
```

## Current Implementation Check

Сверено с текущими файлами:

```text
crates/nando-core/src/wave/surface_wave.rs
crates/nando-core/src/wave/wavepredictor_hebbian.rs
crates/nando-core/src/wave/phase_center_runtime.rs
crates/nando-core/tests/wavepredictor_binding_pressure_l3.rs
```

Базовый путь ниже всё ещё совпадает с реализацией:

```text
SurfaceWave4096 -> active lanes -> paged centers -> active fringe
-> role-binding edges -> flat role-binding table -> slot/energy score
```

## Current Product Serving Line

Текущая продуктовая линия поверх этого сигнального пути:

```text
request
  -> exact cache check
  -> route / profile_id
  -> profile registry
  -> L2-sized `.nwrb` profile shard
  -> local role-binding score / margin
  -> local_operator_accept или fallback_to_llm
```

Serving worker должен быть лёгким:

```text
loads: `.nwrb` packages
does not load: `.nwreb` eval packs
does not use: compiler / corpus JSONL / Python demo / training
```

Replay/eval живёт снаружи:

```text
`.nwreb` eval packs -> replay client -> /replay endpoint
serving worker -> only scores loaded profile packages
```

Текущий smoke этой линии:

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
```

Текущий replay-suite этой линии:

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
```

Текущий fallback smoke этой линии:

```text
verdict: ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS
local_accept_pass: true
bad_route_fallback_pass: true
low_margin_fallback_pass: true
bad_route_fallback_reason: profile_not_found
low_margin_fallback_reason: margin_below_threshold
local_operator_calls: 1
fallback_to_llm_calls: 2
false_local_accepts: 0
p99_latency_ns: 24312
runtime_bytes_estimate: 790020
```

Текущий worker-shard слой этой линии:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS
worker_count: 2
profile_split: 4 / 3
total_local_operator_calls: 7
wrong_worker_route_fallbacks: 2
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_p99_latency_ns: 6286
```

Текущий sharded replay слой этой линии:

```text
verdict: ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS
worker_count: 2
unique_sequences_replayed: 896
exact_cache_llm_calls: 896
exact_cache_plus_nando_llm_calls: 448
exact_cache_incremental_reduction_milli: 500
false_local_accepts: 0
max_worker_runtime_bytes_estimate: 398456
max_worker_p99_latency_ns: 265277
```

Текущий LB / deployed serving envelope:

```text
Local LB replay:
  verdict: ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS
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

Смысл latency breakdown:

```text
L1/L2/L3 score path уже не является главным bottleneck.
Core score и worker score sub-ms.
Красный слой сейчас:
  external per-score HTTP/LB/upstream envelope на дешёвом VPS.
```

Граница replay:

```text
serving worker не грузит `.nwreb`;
`.nwreb` читает только replay client;
default replay path проходит;
текущие default limits: max_unique=128, batch=4.
Историческая причина фикса: batch=32 падал 413 HTTP request body too large,
а max_unique=256 выбивал p99 за текущий gate.
```

Граница claim:

```text
это уже правильная product-serving форма,
и теперь есть local external load-balancer replay через proxy -> worker shards,
плюс deployed cheap-VPS replay на hostworld-ee для sampled release-suite.
Но это ещё не реальный Codex/API traffic benchmark.
Следующий claim должен мерить real Codex/API traffic в shadow/replay режиме.
Synthetic/release-suite 50% reduction нельзя подавать как рыночный real-traffic
claim.
```

Что важно уточнить по текущей реализации:

```text
1. L1 не изменился:
   4096 lanes, i16, 4-gram atoms, boundary atoms, 3 trits per atom.

2. CenterId остаётся u32:
   lane_id остаётся u16;
   output_slot_id / source_slot_id / sign_key остаются u8.

3. 16-slot sequence layout всё ещё:
   32 pages * 4096 centers = 131072 centers.

4. 32-slot layout теперь является следующим product-facing scaling target:
   64 pages * 4096 centers = 262144 centers;
   role pages 0..31;
   action surface page 32;
   operator-pair page 33;
   operator_pair_source_bits = 5.

5. 32-slot уже прошёл несколько Rust rungs:
   layout/capacity smoke;
   3-seed capacity smoke;
   latency smoke;
   order corpus multi-seed rung lengths 17..32;
   mixed map rung;
   conditional branch rung;
   mixed+conditional multi-seed rung;
   mixed+conditional cache/offload benchmark;
   serialized role-binding `.nwrb` package rung;
   public Rust `.nwrb` SDK smoke.

   Но это ещё не full 32-slot product proof:
   остаются full operator battery, public SDK package-runtime final PASS,
   phase-center `.nwpc` bridge/product package и raw-language action parsing.

6. EDIT path теперь имеет отдельную форму:
   output/role slots = 17;
   marker role slot = 16;
   edit demo page = 18;
   edit role-binding edges в текущих v4 logs = 136 by seed.

7. Phase-center memory:
   serialized C32 record примерно 1024 bytes per record,
   in-memory bytes_estimate добавляет record overhead.
```

## Короткая Схема

```text
текст / токены
  -> L1 surface wave
  -> активные lanes
  -> role/action centers
  -> active fringe
  -> L3 role-binding edges
  -> slot score / sequence energy
  -> flat CPU runtime decision
```

## 1. L1: N-Gramm -> SurfaceWave4096

Входной текст режется на surface atoms:

```text
байтовые 4-граммы
boundary atoms
short-token identity atoms
service-word atoms
```

Каждый atom даёт 3 trits:

```text
lane: 0..4095
value: -1 / 0 / +1
```

И всё складывается в:

```text
SurfaceWave4096 = [i16; 4096]
размер = 4096 * 2 bytes = 8192 bytes
```

То есть L1 не "понимает правило". L1 делает стабильный поверхностный
отпечаток.

Код:

```text
crates/nando-core/src/wave/surface_wave.rs
```

```text
L1 vector:
  lanes[4096]: i16
  active lanes: sparse subset
```

## 2. L2-Ish: Lanes -> Centers

Дальше берутся самые сильные lanes и превращаются в центры:

```text
lane_id: u16
center_id: u32
strength: i16
```

Текущие важные числа:

```text
SURFACE_WAVE_DIM = 4096

TOP_ACTIVE_L1_LANES = 48
TOP_ACTION_L1_LANES = 64
TOP_ROLE_L1_LANES = 32

для 32-slot smoke:
SEQ32_TOP_ROLE_L1_LANES = 64
```

Тут уже появляется адресная раскладка:

```text
16-slot layout:
  PAGE_BITS = 12
  PAGE_SIZE = 4096
  PAGE_COUNT = 32
  TOTAL_CENTER_COUNT = 131072

role pages: 0..15
action surface page: 16
operator-pair page: 17
condition pages: 18..20
reserve: 21..31
```

Адрес центра:

```text
center = (page << 12) | lane
```

То есть страница = грубый тип признака, lane = конкретная L1-позиция внутри
страницы.

## 3. Role Pages: Slot + Lane

Для роли:

```text
role_center = role_base + slot_id * 4096 + lane_id
```

Пример:

```text
slot 0 + lane 777 -> center in page 0
slot 1 + lane 777 -> center in page 1
slot 15 + lane 777 -> center in page 15
```

Это важно: один и тот же surface-lane получает разный смысл в разных слотах.

```text
A в source slot 2
не то же самое, что
A в source slot 5
```

## 4. Action / Operator Centers

Оператор действия тоже кодируется в centers.

Для order/permutation есть operator-pair:

```text
out_slot -> source_slot
```

Для 16 slots:

```text
lane = (out << 4) | src
16 x 16 = 256 possible pair centers
```

Для 32 slots:

```text
lane = (out << 5) | src
32 x 32 = 1024 possible pair centers
```

Для 64 slots:

```text
64 x 64 = 4096
ровно одна 4096-center page
```

То есть operator-pair matrix физически маленькая. Проблема не в ней, а в росте
role-binding edges.

## 5. Active Fringe

В runtime не гоняется огромная плотная матрица. Собирается sparse список
активных центров:

```text
WavePredictorActiveCenter {
  center_id: u32,
  strength: i16
}
```

Это active fringe:

```text
active action centers
active role centers
active condition/demo centers
```

## 6. L3: Role-Binding Edge Table

Главная L3-таблица сейчас такая:

```text
(action_center, output_slot_id, source_slot_id, sign_key) -> weight
```

В коде:

```text
state_delta_role_binding_edges:
  HashMap<(u32, u8, u8, u8), i16>
```

Смысл:

```text
если активен такой action
и мы собираем такой output slot
и нужный source slot содержит похожий lane
и знак lane совпал
то дай pressure за этот target lane
```

Скоринг примерно:

```text
score += abs(action_strength)
       * abs(role_strength)
       * weight
```

## 7. Slot-Scoped Filter

Это важный свежий момент.

Action center должен совпасть не просто "по правилу", а по паре:

```text
output slot
source role slot
```

То есть если action говорит:

```text
out3 <- src7
```

он не должен голосовать за:

```text
out4 <- src7
out3 <- src2
```

Вот это защищает от каши ролей.

## 8. Flat CPU Runtime

После обучения HashMap компилируется в flat table:

```text
edges: Vec<WavePredictorFlatRoleBindingEdge>
action_offsets: Vec<usize>
```

CPU-путь такой:

```text
action_center
  -> action_index
  -> action_offsets[action_index]..action_offsets[action_index + 1]
  -> contiguous slice edges
  -> проверить output_slot/sign/source_slot
  -> найти role_center в active fringe
  -> accumulate i32 score
```

То есть процессор живёт не в "нейросетке", а в обычных массивах:

```text
Vec<Edge>
Vec<usize offsets>
small active_fringe Vec
i32 accumulator
```

## 9. Размеры L3 / Cache

Ключевые размеры:

```text
16-slot layout:
  centers = 32 pages * 4096 = 131072
  base_mass = 131072 * 2 bytes = 262144 bytes

32-slot product target:
  centers = 64 pages * 4096 = 262144
  base_mass = 524288 bytes
```

Текущий 32-slot evidence:

```text
page_count: 64
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64
operator_pair_source_bits: 5

capacity_smoke: PASS
capacity_3_seed_smoke: PASS
latency_smoke: PASS
order_corpus_multi_seed: PASS
mixed_map_corpus: PASS
conditional_branch_corpus: PASS
mixed_conditional_multi_seed: PASS
mixed_conditional_cache_offload_bench: PASS
serialized_role_binding_nwrb_package: PASS
public_role_binding_nwrb_sdk_smoke: PASS
public_sdk_package_runtime_gate: PASS
role_binding_nwrb_cli_inspect_verify: PASS
role_binding_nwrb_cli_score_verify: PASS
independent_corpus_emitted_nwrb_cli_sequence_scoring: PASS
compact_binary_nwreb_eval_pack_suite: PASS
role_binding_release_suite_product_bundle: PASS
serving_only_nwrb_profile_runtime_smoke: PASS

role_binding_edges: 892
hot_bytes_estimate: 600536
flat_runtime_latency_p50_ns: 135476
flat_runtime_latency_p99_ns: 245822
latency_gate_ns: 1000000

Boundary:
  0.246 ms p99 в smoke не блокер для CPU offload;
  max p99 in 32-slot cache/offload bench is 611686 ns;
  max p99 in serialized `.nwrb` package rung is 623242 ns;
  max p99 in public SDK-loaded `.nwrb` package rung is 718891 ns;
  `.nwrb` CLI inspect/verify and score/verify are closed for current artifacts;
  serving-only `.nwrb` profile runtime smoke is closed;
  это ещё не real Codex/API traffic proof и не final product p99 claim.
```

Role-binding edge table:

```text
flat_bytes ~= role_binding_edges * sizeof(edge)
            + action_offsets.len * sizeof(usize)
```

Из текущего capacity-документа:

```text
v4 order:
  role_binding_edges ~ 88k

v4 conditional:
  role_binding_edges ~ 40k

v4 edit:
  role_binding_edges = 136

v4 composed:
  role_binding_edges = 366
```

На CPU это живёт примерно так:

```text
L1d cache: маленькие active lists / текущие slices
L2 cache: offsets / часть hot edges
L3 cache: вся hot table, если не раздули
RAM: cold package / большие shards
```

## 10. Phase-Center Product Scorer

Отдельная линия для продуктового action scorer:

```text
action_tree atoms
  -> phase vector C32
  -> positive center / negative center
  -> flat runtime record
  -> margin
  -> local operator or LLM fallback
```

Размер:

```text
PhaseCenterCell = f64 re + f64 im = 16 bytes

C32 record:
  positive center: 32 cells
  negative center: 32 cells

serialized bytes ~= 2 * 32 * 16 = 1024 bytes per record
in-memory bytes_estimate adds PhaseCenterFlatRecord overhead
```

Скоринг:

```text
margin = dot(correct_vec - wrong_vec,
             positive_center - negative_center) / cells
```

Offload decision:

```text
if margin >= threshold:
  LocalOperator
else:
  FallbackToLlm
```

## Главная Схема

```text
TEXT / TOKENS
|
+-- L1 SurfaceWave4096
|   |
|   +-- 4-gram atoms
|   +-- boundary atoms
|   +-- short-token identity
|   +-- lanes[4096] i16
|
+-- Active Lanes
|   |
|   +-- top action lanes
|   +-- top role lanes
|   +-- signs + strengths
|
+-- Center Addressing
|   |
|   +-- role_center = slot_page * 4096 + lane
|   +-- action_center = action_page * 4096 + lane
|   +-- operator_pair = page17 + ((out << bits) | src)
|
+-- Active Fringe
|   |
|   +-- Vec<(center_id u32, strength i16)>
|
+-- L3 Role Binding
|   |
|   +-- edge(action, out_slot, src_slot, sign) -> weight
|   +-- slot-scoped action filter
|   +-- role lane alignment
|
+-- Decoder
|   |
|   +-- slot gap
|   +-- sequence energy
|   +-- correct > same-bag wrong
|
+-- Flat CPU Runtime
    |
    +-- sorted edges Vec
    +-- action_offsets
    +-- contiguous cache-friendly scan
    +-- i32 score
```

Самая короткая формула:

```text
L1 видит поверхность.
L2/L3 адресуют: где этот surface-lane находится и какая у него роль.
L3 учит: какое действие переносит роли в output slots.
Flat runtime превращает это в быстрый CPU table scan.
```

Важная граница:

```text
это не одна гигантская матрица
это несколько sparse/flat структур:
  SurfaceWave4096
  center pages
  active fringe
  role-binding edges
  phase-center records
```

Вот поэтому это может жить на CPU: горячий путь должен быть маленьким, sparse
и cache-friendly.
