# Drifting Phase-Center Operator Memory

Дата фиксации: 2026-07-06

Назначение: зафиксировать чистую архитектурную формулу, чтобы больше не
смешивать потоковое обучение, hot runtime, operator memory и `.nwpc` snapshot.

## Current Authoritative Contract

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current proof snapshot:

```text
best shadow frontier:
  unique_cpu_accepts_over_exact_cache: 6_644
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0
  local_accept_enabled: false

hot runtime benchmark:
  hot_bytes_estimate: 592
  false_accepts: 0
  unique_cpu_accepts_over_exact_cache: 8

live-store smoke:
  false_accepts: 2
  status: WATCH, no promotion
```

Interpretation:

```text
Drifting phase-center memory is the intended product mechanism.
.nwpc remains snapshot/export/deploy/rollback, not the per-request mechanism.
The next hard part is L4 automatic selection and bounded hot-set management.
```

## Главная Формула

```text
L2-resident mutable phase-center operator
  -> score_before_update
  -> verifier label
  -> in-place center update
  -> safety threshold update
  -> optional snapshot/export
```

Это не файл, не JSONL-комбайн и не RAM-модель как продуктовый термин.

Правильное имя механизма:

```text
Drifting Phase-Center Operator
```

По-русски:

```text
дрейфующий фазовый центр
```

Смысл: оператор не пересобирается заново на каждом шаге. Он живёт как маленькое
cache-resident состояние и плавно сдвигает центр по проверенному потоку.

## Строгие Запреты Терминов И Механизмов

Не говорить и не строить так:

```text
RAM model
compile .nwpc in stream
.nwpc as streaming mechanism
proof daemon as product runtime
JSONL hot path
BTreeMap hot path
String bucket_key hot path
local_accept without verifier
```

Правильные термины:

```text
L2-resident mutable phase-center state
CacheResidentOperatorState
StreamingPhaseCenterState
HotCenterState
Phase Operator Memory
Operator Memory Store
```

## Разделение Слоёв

```text
LIVE STREAM
|
+-- source adapter / event envelope
|   |
|   +-- receives agent/event stream
|   +-- source-specific parsing lives only here
|   +-- emits source-neutral event envelope
|
+-- L1/L2 extraction
|   |
|   +-- safe atoms
|   +-- state atoms
|   +-- action atoms
|   +-- observable result atoms
|   +-- no raw prompt / no raw answer / no raw tool output in core
|
+-- L4 / streaming goal-state layer
|   |
|   +-- consumes L1/L2 safe atoms
|   +-- выделяет route/profile/bucket
|   +-- снижает энтропию для L3
|   +-- не даёт одному phase-center смешивать разные действия
|   +-- не является Codex-hardcode
|   +-- source-specific только в adapter
|
+-- L3 drifting phase center
|   |
|   +-- score old center
|   +-- compare margin/threshold
|   +-- verifier label arrives
|   +-- update center in-place
|   +-- update drift/safety statistics
|
+-- operator memory
|   |
|   +-- stable operators are retained
|   +-- unstable buckets stay shadow-only
|   +-- unsafe buckets are rejected or rolled back
|
+-- optional snapshot/export
    |
    +-- .nwpc
    +-- audit artifact
    +-- deployment package
    +-- rollback version
```

## Роль L4

L4 не заменяет L3 и не является lookup-таблицей.

L4 нужен как потоковый goal-state / routing / admission слой:

```text
raw event stream
|
+-- source adapter
|
+-- L1/L2 safe atoms
|
+-- L4 separates context
|   |
|   +-- route_id
|   +-- profile_id
|   +-- bucket_id
|   +-- verifier_id
|   +-- value/cost priority
|   +-- safety/admission state
|
+-- L3 receives a bounded operator problem
```

То есть L4 помогает L3 не разгребать весь поток одним центром.

Без L4:

```text
L3 sees mixed events
|
+-- sed followup
+-- cargo test status
+-- planning continuation
+-- report sync
+-- edit patch
+-- unrelated dialogue
|
+-- one center starts mixing incompatible operators
```

С L4:

```text
L4 routes:
  command_result_followup -> bucket A
  test_output_parse       -> bucket B
  report_sync             -> bucket C
  edit_patch_small        -> bucket D

L3 learns:
  one bounded drifting phase center per routed operator family
```

Жёсткая граница:

```text
L4 may route, budget, admit, evict, and schedule.
L4 must not become answer authority.
L4 must not use target_id / proof_rule_id / concrete lookup.
L4 must not hardcode Codex as architecture.
```

Коротко:

```text
L4 разгребает поток.
L3 учит оператор.
L2 даёт признаки.
L1 даёт поверхность.
```

## Потоковый Inner Loop

Потоковый цикл должен быть таким:

```text
event
|
+-- route_id / profile_id / bucket_id
|
+-- score_before_update using current hot center
|
+-- decision
|   |
|   +-- high margin -> shadow accept or product accept if promoted
|   +-- low margin  -> fallback
|
+-- verifier label
|   |
|   +-- true / false / unsafe / unknown
|
+-- update in-place
    |
    +-- positive_center += alpha * (event_vector - positive_center)
    +-- or negative_center += alpha * (event_vector - negative_center)
    +-- update threshold statistics
    +-- update drift budget
```

Нельзя делать в inner loop:

```text
compile .nwpc
serialize package
read/write JSONL
aggregate report
run replay
allocate large maps
hash long strings as routing authority
```

## Центр Масс / Drift Formula

Минимальная формула дрейфа:

```text
center_next = center_old + alpha * (event_vector - center_old)
```

Для безопасного оператора нужны минимум два направления:

```text
positive_center
negative_center
```

Решение:

```text
score = similarity(event, positive_center)
      - similarity(event, negative_center)

if score > threshold:
  accept_or_shadow_accept
else:
  fallback
```

Но threshold не подбирается вручную как продуктовая ручка. Он должен вытекать из
калибровки:

```text
threshold = max_false_margin + safety_gap
```

или более строгого эквивалента, доказанного на future/shadow split.

## Где Физически Хранится Оператор

```text
CPU L1
|
+-- текущие counters
+-- текущий vector slice
+-- score loop

CPU L2
|
+-- active hot operator shard
+-- mutable centers
+-- thresholds
+-- small fixed arrays

CPU L3 / process memory
|
+-- warm operator registry
+-- route_id -> profile_id -> shard
+-- больше operators, чем помещается в один L2 shard

Disk snapshot
|
+-- .nwpc or successor binary format
+-- versioned operator image
+-- restart / deploy / rollback / audit
```

Точно:

```text
горячее - в cache-resident state
тёплое - в process memory registry
вечное - binary snapshot on disk
```

Диск не участвует в каждом запросе.

## Что Такое .nwpc В Этой Архитектуре

`.nwpc` не поток и не механизм обучения.

Правильная роль:

```text
snapshot
export
deploy artifact
audit artifact
rollback version
```

Допустимый цикл:

```text
streaming state learns in-place
|
+-- periodically or on promotion:
    |
    +-- export snapshot
    +-- write .nwpc
    +-- record version/proof
```

Недопустимый цикл:

```text
event -> compile .nwpc -> score -> update -> compile .nwpc
```

Это ломает саму идею поточного оператора.

## Operator Memory

Гибридная формула:

```text
drifting phase center
+
operator memory
+
fallback LLM
```

Смысл:

```text
drifting center discovers stable actions
operator memory retains proven actions
fallback handles uncertain actions
```

Что хранить в operator memory:

```text
operator_id: u32
route_id: u32
profile_id: u32
bucket_id: u32

positive_center: fixed array
negative_center: fixed array
threshold: integer margin

seen_count
accepted_count
fallback_count
false_accept_count
last_seen_timestamp

verifier_id
drift_budget
rollback_version
snapshot_fingerprint
```

## Bounded Operator Memory

Operator memory нельзя строить как бесконечный склад.

Неправильно:

```text
discovered operator -> store forever -> load everything into every worker
```

Так система раздуется до непредсказуемого размера и потеряет cache-resident
свойство.

Правильная форма:

```text
bounded operator cache with admission and eviction
```

Слои:

```text
HOT
|
+-- 1-4 active profile shards per worker
+-- L2/L3 resident
+-- only current high-value routes
+-- fixed arrays, no JSON, no strings in score loop

WARM
|
+-- tens/hundreds of profiles in process memory
+-- route_id -> top-K candidate operators
+-- loaded into HOT only by demand/value

COLD
|
+-- versioned snapshots on disk
+-- .nwpc or successor binary images
+-- restart/deploy/audit/rollback only
+-- never scanned on every request
```

Запрос не должен сканировать всю память операторов.

Правильный request path:

```text
request/event
|
+-- cheap route_id
|
+-- route index selects top-K operators
|   |
|   +-- K = 1..4 by default
|   +-- K must be bounded
|
+-- score only selected hot operators
|
+-- high margin + promoted verifier contract -> accept
+-- otherwise -> fallback
```

## Admission / Eviction Rules

Оператор допускается в HOT/WARM только если он покупает измеримую пользу:

```text
repeat frequency high enough
fallback token/cost weight high enough
exact-cache overlap low enough
verifier exists
shadow success exists
false_accepts = 0
runtime parity = 0 mismatches
value_score above threshold
drift budget not exceeded
```

Оператор должен выгружаться или замораживаться, если:

```text
no recent hits
low accept rate
low token/cost value
route stopped appearing
margin becomes unstable
drift budget exceeded
false_accept observed
better merged operator replaces it
```

Переходы:

```text
new candidate -> shadow only
shadow safe   -> WARM
frequent safe -> HOT
stale         -> WARM -> COLD
unsafe        -> quarantine / rollback
duplicate     -> merge
mixed action  -> split into subcenters
```

## Required Queues

Для product architecture нужны отдельные очереди, а не один комбайн:

```text
EVENT QUEUE
|
+-- live events for scoring/update

MINING QUEUE
|
+-- candidate drifting centers
+-- no product accept

PROMOTION QUEUE
|
+-- verifier-bound operators only
+-- false_accepts = 0 required

LOAD / PREFETCH QUEUE
|
+-- move WARM profiles into HOT by route demand
+-- bounded by worker cache budget

EVICTION QUEUE
|
+-- move HOT -> WARM -> COLD
+-- remove stale/low-value/unsafe operators
```

## Hard Budgets

Каждая реализация должна явно иметь бюджеты:

```text
max_hot_bytes_per_worker
max_hot_profiles_per_worker
max_warm_profiles_per_process
max_profiles_per_route
max_operator_age_without_hits
max_route_top_k
min_tokens_saved
min_accept_rate
false_accepts_must_be_zero
```

Если этих бюджетов нет, это WATCH: операторная память может разрастись и
сломать hot path.

## Safety Rules

Продуктовый accept разрешён только если:

```text
future/shadow split exists
verifier exists
false_accepts = 0
runtime parity = 0 mismatches
exact-cache overlap excluded
denominator exists
drift budget not exceeded
rollback snapshot exists
```

Если любое условие не выполнено:

```text
shadow only
or fallback
```

## Product Boundary

Тяжёлый proof/miner может существовать как отдельный контур, но он не является
product hot path.

```text
PROOF / MINER PATH
|
+-- trace reading
+-- JSONL
+-- reports
+-- replay
+-- verifier audits
+-- package export
+-- denominator accounting

HOT PRODUCT PATH
|
+-- route_id
+-- profile_id
+-- fixed atoms
+-- cache-resident center state
+-- integer score
+-- threshold compare
+-- accept/fallback
```

Цель hot path:

```text
no JSON
no file IO
no package compile
no report aggregation
no source-specific decision authority in core
```

## Current Implementation Audit

Срез проверки: 2026-07-06.

Что уже совпадает с архитектурой:

```text
core PhaseCenterOnlineMiner
|
+-- numeric bucket_id
+-- score_before_update
+-- verifier label input
+-- false_accept rejects bucket
+-- candidate hot runtime export
```

Что пока WATCH, а не product PASS:

```text
CLI online miner daemon
|
+-- uses JSONL trace/report path
+-- uses String/BTreeMap bucket bookkeeping
+-- compiles quarantine .nwpc checkpoints during proof run
+-- correctly marks local_accept=false
+-- correctly marks auto_promote=false
+-- correctly declares shadow-only boundary
|
+-- therefore:
    |
    +-- allowed as proof/miner path
    +-- not allowed as product hot path
```

Главные implementation debts:

```text
P0:
  wire product daemon to core PhaseCenterOnlineMiner / mutable hot state,
  not to repeated package compilation as the streaming mechanism.

P0:
  implement bounded HOT/WARM/COLD operator memory budgets in code,
  not only in this document.

P0:
  add route_id -> bounded top-K profile selection before score loop.
  No request may scan all operators.

P1:
  replace final hot request path string atoms / String bucket_key authority
  with numeric route/profile/bucket/atom ids from the lean adapter.

P1:
  optimize hot center layout when product path is stable:
  fixed arrays / cache-friendly numeric cells before SIMD work.
```

## Reviewer Self-Check

Проверяющий тоже проверяется.

Если в ревью появляется фраза:

```text
RAM model
compile .nwpc in stream
proof path is runtime
manual threshold is product gate
```

то это ошибка ревью. Надо заменить на:

```text
cache-resident mutable phase-center state
.nwpc snapshot/export only
proof/miner path separate from hot runtime
automatic calibrated threshold with future split
```

## Короткая Версия

```text
The product mechanism is not package compilation.

The product mechanism is:
  L2-resident drifting phase-center operator
  plus operator memory
  plus verifier-bound promotion
  plus LLM fallback.

.nwpc is only a snapshot/export/deploy artifact.
```
