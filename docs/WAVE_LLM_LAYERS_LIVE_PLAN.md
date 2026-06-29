# Wave-LLM Layers Live Plan

Статус: живой план, не закрыт.

Цель: собрать L1/L2/L3 так, чтобы Wave-LLM не была плоским lookup и не
хардкодила смыслы. Каждый слой должен давать проверяемый центр, перенос на
heldout и отказ на ловушках.

## Главная форма

```text
L1 = форма
L2 = мотивы
L3 = смысловые центры и операторы
L4 = план ответа
```

Сейчас фиксируем L1-L3. L4 пока не проектируем глубоко.

## L1 SurfaceWave

L1 кодирует первичный поверхностный импульс.

Базовые параметры:

```text
n = 4
dim = 4096 lanes
k = 3 trits
trit in {-1, 0, +1}
accumulator = i16
```

L1 не должен понимать смысл. Он должен стабильно кодировать форму.

Текущий контракт:

```text
text
-> surface_atoms
-> sparse ternary lane hits
-> SurfaceWave4096
```

Атомы L1:

```text
raw byte 4-grams
boundary atoms with BOS/EOS
service atoms for closed-class function words only
```

Важно:

```text
short word != service word
```

Пример `сыч`:

```text
[BOS, BOS, BOS, с]
[BOS, BOS, с, ы]
[BOS, с, ы, ч]
[с, ы, ч, EOS]
[ы, ч, EOS, EOS]
[ч, EOS, EOS, EOS]
```

`сыч` получает boundary-form, но не service-channel.

Пример `и`:

```text
[BOS, BOS, BOS, и]
[BOS, BOS, и, EOS]
[BOS, и, EOS, EOS]
[и, EOS, EOS, EOS]
+ service("и")
```

`BOS/EOS` - внутренние маркеры. Они не равны реальным символам `^`, `$`.

## L1 частотность

Частые n-граммы нельзя просто выбрасывать.

Они работают как синтаксическая несущая частота. Их вклад должен быть
ослабляемым/нормируемым, а не удаляемым.

Защита:

```text
frequency-aware weight
saturating i16 accumulator
no hard deletion of common surface atoms by default
```

Пока дисперсионный gate показал вред в простом варианте. Значит дисперсию не
добавляем в hot path без нового proof.

## L2 Current State

Текущий L2 в коде:

```text
L1 text
-> surface_atoms
-> L1 center refs: [12, 88, 5, 901, ...]
-> L2 windows over center refs
-> reusable sequence motifs
```

Это уже не exact-word lookup, но еще не финальная волновая физика.

## L2 Target State

Правильный Wave-L2:

```text
L2 motif = sparse ternary prototype + local time phase + normalized resonance
```

Не сравниваем весь плотный `4096 x i16` вектор со всеми мотивами.

Нужный механизм:

```text
L1 sparse trits
-> inverted lane/sign/time index
-> motif votes
-> top-k motif candidates
-> signed score / conflict score
-> gap verification
```

Мотив хранит не весь wave, а устойчивые зубцы:

```text
[(lane, sign, local_t, weight), ...]
```

Score:

```text
score = weighted_matches - weighted_conflicts
score /= motif_norm
gap = best - second_best
```

Нормировка обязательна:

```text
popular lane weight down
long motif length normalized
rare stable lane weight up
```

Иначе длинные/частотные мотивы будут побеждать только массой.

## L2 Time-Phase

Глобальные position buckets ломаются от вводных слов.

Правильное время L2:

```text
local relative phase inside motif
```

Пример:

```text
закрыт  t0
проблем t1
нет     t2
```

Если перед фразой добавили `слушай`, локальная координата мотива не меняется.

Допуск:

```text
exact t      -> full score
t +/- 1     -> partial score
farther t   -> no score
```

Перекрывающиеся окна нужны, но главная защита - локальное время мотива.

## L2 Prototype Mining

Прототипы L2 не задаются руками.

Они вытаскиваются из корпуса:

```text
many texts
-> L1 atoms
-> candidate windows, roughly 3-7 atoms
-> support/conflict statistics
-> heldout/reversed/corrupt filters
-> compact motif prototypes
```

Критерии promotion:

```text
support high
conflict low
heldout repeats
corrupt/reversed worse
compression positive
```

Это Wave-Pattern Mining: не gradient descent, а отбор устойчивых инвариантов.

### Pass 1 Candidate Mining

Проблема: если собрать все окна `3..7` из всех фраз, кандидатов будет слишком
много.

Решение:

```text
stream corpus
-> L1 atoms
-> sliding windows 3..7
-> hash(window with local_t)
-> Count-Min Sketch / SpaceSaving
-> keep heavy hitters only
```

Pass 1 не хранит все окна.

Хранит только:

```text
sketch counters
top-N candidate hashes
```

Не используем схему "первая фраза стала прототипом". Это слишком шумно.

### Pass 2 Candidate Verification

После Pass 1 повторно прогоняем корпус.

Считаем только top-N candidates:

```text
support
conflict_count
lane/sign/time stats
lane popularity
motif length norm
```

Финальный promoted motif:

```text
motif_hash
prototype_teeth [(lane, sign, local_t, weight), ...]
support
conflict
norm
```

Promotion:

```text
support >= min
conflict low
heldout repeats
corrupt/reversed rejected
compression positive
```

Итог:

```text
Pass 1 = cheaply find candidates
Pass 2 = precisely measure only candidates
```

Это не lookup. Это streaming heavy hitters + verification.

## L3 Target State

L3 не должен быть плоским classifier.

Правильный L3:

```text
L2 top-k motifs
-> sparse votes
-> axis centers
-> interference field
-> composed operator
```

Оси:

```text
Action
Domain
Object
Tone
Style
Constraint
EvidenceNeed
Refusal / AntiWave
```

## L3 Axis-Center Induction

Подход: гибридный базис через именованные аттракторы.

Не используем чистый unsupervised:

```text
SVD/PCA can find components
but cannot honestly name them Action/Domain/Refusal
```

Не используем чистый supervised classifier:

```text
phrase -> fixed label
```

Правильная схема:

```text
axis names = fixed contract
axis centers = learned
motif -> axis weights = learned
```

Минимальный axis contract:

```text
Action
Domain
Object
Tone
Style
Constraint
EvidenceNeed
AntiWave
```

Для каждой оси можно дать маленькие seed anchors. Это не полные правила, а
начальные ядра:

```text
Action: explain, diagnose, disable, compare
Domain: vpn, linux, finance
Tone: angry, calm, brief
```

L2 motifs подтягиваются к осям через:

```text
co-occurrence
contrastive negatives
heldout transfer
phase similarity
```

Вес мотива:

```text
weight(motif, axis_center) =
  attraction_to_seed
+ cooccur_support
- conflict_with_other_axis
- trap_penalty
```

Axis center update:

```text
center += confirmed motifs
center -= conflicting motifs
```

Итог:

```text
L3 axis = named attractor
but its mass and links are learned from corpus
```

L3 не сам придумывает названия осей. L3 сам учит, какие мотивы реально
принадлежат этим осям и с каким весом.

Мотив не имеет жесткого провода к одному классу.

Один мотив дает слабые голоса в несколько осей:

```text
"не работает vpn"
-> Domain: vpn +0.8
-> Action: diagnose +0.6
-> EvidenceNeed: runtime_snapshot +0.7
-> Tone: practical +0.3
```

## L3 Interference

L3 считает поле:

```text
score(center) =
  motif_votes
+ compatibility(other_centers)
- conflict(other_centers)
- anti_wave
```

Оператор ответа:

```text
O_final =
  Action
+ Domain
+ Object
+ Tone
+ Style
+ Constraint
+ EvidenceNeed
```

Пример:

```text
Domain = vpn
Action = disable
Constraint = no_runtime_evidence
```

Должно давать не действие, а запрос evidence / review-only state.

## L3 Acceptance

L3 принимает оператор только если:

```text
axis gaps positive
centers compatible
conflicts suppressed
anti-wave has no veto
evidence is sufficient
heldout passes
role/route traps rejected
```

## L3 Bounded Settle

L3 Field нельзя оставлять крутиться бесконечно.

Защита от limit cycle / chaos:

```text
fixed small settle_steps
+ damping
+ priority axes
+ no authority if gap weak
```

Базовый runtime:

```text
for step in 0..N:
  centers += compatibility
  centers -= conflict
  centers *= damping(step)
  Refusal/Constraint can veto
```

Ориентир:

```text
N = 3..5
temperature falls
damping increases near final steps
```

Если поле не сошлось:

```text
gap < threshold
or oscillation detected
or conflict_energy high
or anti_wave active
```

то оператор не получает authority.

Возврат:

```text
FIELD_UNSETTLED
answer_allowed = false
need_clarification = true
```

Иерархия осей:

```text
Refusal / AntiWave
> Constraint
> EvidenceNeed
> Action
> Domain
> Tone / Style
```

Слабые оси не могут победить safety/evidence.

## L3 Interaction Learning

Pass 3 учит внутреннюю динамику поля как sparse contrastive Hebbian learning.

Не учим полную матрицу:

```text
670 x 670 x 670
```

Учим только активные и пограничные связи:

```text
top-k active centers
near-miss centers
wrong attractor links
```

Три источника обучения:

```text
positive coactivation
contrastive anti-wave
sparse settle correction
```

Positive coactivation:

```text
centers active together in successful example
-> compatibility +=
```

Contrastive anti-wave:

```text
trap / poisoned / unsafe / near-miss example
-> conflict +=
-> anti_wave +=
```

Sparse settle correction:

```text
field settled to wrong attractor
-> penalize only links that pulled into wrong attractor
-> reinforce missing links to correct attractor
```

Важно:

```text
learn observed / near-miss interactions
do not learn all possible combinations
```

Итог:

```text
coactivation gives attraction
traps give repulsion
settle correction gives adaptation
```

## L3 Field Regularization

Проблема: частые пары центров могут стать слишком тяжелыми и начать
затягивать поле даже на зашумленных запросах.

Закон:

```text
frequent != infinitely strong
```

Link update:

```text
raw_link += event
link_weight = raw_link / sqrt(center_mass_a * center_mass_b)
link_weight = clamp(link_weight, -max, +max)
```

Регуляризация:

```text
mass-normalized links
bounded weights
decay for weak / unconfirmed edges
heldout/trap promotion gate
```

Связь считается сильной только если:

```text
appears often
has low conflict
helps heldout
does not break traps
```

## L3 Trap Generation

Цель: модель должна сама искать свои уязвимые места, а не ждать ручного списка
ловушек.

Подход:

```text
axis inversion
domain splice
evidence removal
random impossible combos
active mining of false accepts
```

Цикл:

```text
positive example
-> infer active axes
-> generate near-miss traps
-> run L3 field
-> collect false accepts
-> train conflict / anti-wave
-> retest heldout and traps
```

Near-miss виды:

```text
role swap
action inversion
domain splice
constraint violation
evidence removal
tone/style conflict
```

Комбинаторный генератор:

```text
random axis combo
-> impossible / unsafe / unsupported combo
-> expected FIELD_UNSETTLED or AntiWave
```

Главный механизм:

```text
if L3 falsely accepts a trap:
  trap becomes training negative
  conflict +=
  anti_wave +=
```

Итог:

```text
trap generation is not fully manual
traps grow from failed validation
```

## Hot Memory Layout Draft

Размер hot memory - это не догма. Это профиль.

Цель layout:

```text
minimum cache misses
flat arrays
no pointer chasing
measured heldout/trap gain per byte
```

Запрещено не увеличение объема, а увеличение без пользы.

```text
more bytes are allowed
if they buy better heldout,
lower false accept,
lower latency,
or simpler safer layout
```

## Task-First Matrix Profiles

Не начинаем с размера.

Начинаем с вопроса:

```text
какая матрица нужна, чтобы решить задачу?
```

Размер - следствие выбранной матрицы.

Профиль задается не байтами, а структурной сложностью:

```text
surface_form_profile
  цель: форма слов / шум / опечатки
  нужны: L1 atoms + L2 form motifs
  не нужны: L3 domain/action axes

single_domain_operator_profile
  цель: один домен, устойчивые действия
  нужны: L1 + L2 motifs + L3 Action/Domain/Object/Constraint
  не нужны: широкий multi-domain registry

dialogue_behavior_profile
  цель: болталка / стиль / намерение / тон
  нужны: Action/Tone/Style/Constraint/EvidenceNeed
  нужны: strong AntiWave for refusal/uncertain states

multi_domain_reasoning_profile
  цель: несколько доменов с конфликтами
  нужны: Domain axes, route separation, conflict matrix, evidence matrix
  нужны: domain-splice traps

critical_control_profile
  цель: действия в системе / безопасность
  нужны: Constraint/Refusal/EvidenceNeed priority
  нужны: FIELD_UNSETTLED as normal output
```

Для каждого профиля сначала фиксируем:

```text
axis_count
centers_per_axis
motif_count
motif_teeth_distribution
interaction_edge_count
anti_wave_edge_count
evidence_need_centers
settle_steps
trap_suite
```

И только потом считаем:

```text
model_hot_bytes
runtime_scratch_bytes
median_inference_us
```

Каждый профиль обязан печатать:

```text
model_hot_bytes
runtime_scratch_bytes
bytes_per_motif
bytes_per_edge
heldout_accuracy
false_accept_rate
trap_rejection_rate
median_inference_us
```

## Primary Matrix Target

Главный рабочий профиль для первой полноценной болталки:

```text
dialogue_behavior_profile_full_v1
```

Цель:

```text
L3-cache-resident working set
not tiny proof toy
```

Важно: cache residency - это цель и метрика, не математическая гарантия.
Нельзя писать "никогда не выйдет в RAM". Правильно:

```text
cache_miss_rate is measured
working_set fits target CPU cache budget
```

Матрица full_v1:

```text
L2 motif_count              = 65_536
avg_teeth_per_motif         = 8
L2 bucket_count             = 65_536
front_refs_per_bucket       = 4
overflow_index_budget       = measured, initial target ~512 KiB

L3 axis_count               = 16
centers_per_axis            = 32
L3 center_count             = 512
interaction_edge_target     = 131_072
anti_wave_edge_target       = 65_536
settle_steps                = 3..5
```

Оси full_v1:

```text
Action
Intent
Topic
Object
Tone
Style
Emotion
Constraint
EvidenceNeed
Refusal
Time
Certainty
MemoryUse
Repair
Safety
DomainBoundary
```

Утвержденный byte audit full_v1:

```text
L1 SurfaceWave scratch [i16;4096]                 8_192
L2 core motif bank, fixed 8 teeth             1_835_008
L2 rare / long motif extension bank             786_432
L2 front inverted index                          524_288
L2 expanded overflow pool                      1_048_576
L3 axis center model data                          4_096
L3 interaction edges, aligned 8-byte           1_048_576
L3 dedicated trap / anti-wave bank               524_288
Metric counters / rebuild / alignment reserve    512_000
--------------------------------------------------------
configured working budget                       6_291_456 bytes
```

6_291_456 bytes = 6 MiB.

Это не "забить память ради красоты". Свободный запас из первичного 3.7 MiB
покупает слабые места:

```text
aligned 8-byte edges           -> speed and simpler traversal
expanded overflow pool         -> fewer important lane-list cuts
rare / long motif bank         -> rare but strong phrases survive
dedicated anti-wave bank        -> more hard negatives and veto patterns
metric / rebuild reserve       -> utility scoring and profile proof
```

Важно: aligned 8-byte edge is chosen for simpler CPU traversal. It does not
magically prove one-cycle SIMD. Benchmark/proof decides the real speed.

## Hardware Benchmark Gate

Главная красная линия - P99 latency полного inference path.

Current proof harness:

```text
crates/nando-core/tests/wave_full_v1_layout_bench.rs
```

Причина: cache-miss и IPC объясняют, почему быстро/медленно, но пользователь и
runtime чувствуют задержку.

Не зашиваем романтический `P99 <= 50us` как первый assert для полного
`L1 -> L2 -> L3` на noisy/adversarial запросах. Это stretch target, не
acceptance gate.

Первый честный gate:

```text
end_to_end_p99_us <= measured_baseline_budget
no_inference_allocations = true
false_accept_rate = 0
FIELD_UNSETTLED returned for unresolved traps
```

Для local/T480-class CPU стартовые redlines:

```text
L3_settle_only_p99_us <= 250
end_to_end_p99_us     <= 500
```

Если реальный первый benchmark окажется быстрее, gate ужесточается. Если
медленнее - не снижаем планку молча, а смотрим cache-miss/overflow/branch
причину.

Hardware diagnostics:

```text
LLC/cache miss rate = primary diagnostic
IPC                 = secondary diagnostic
branch misses       = secondary diagnostic
```

Начальная диагностическая цель:

```text
LLC miss rate <= 5% on hot steady-state profile
warn if LLC miss rate > 3%
```

`LLC miss < 1%` можно оставить как сильную цель для tuned build, но не как
первый обязательный assert: 6 MiB working set конкурирует с кодом, scratch,
prefetch и системным шумом.

First physical release run:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

queries: 10_000
p50_latency: 1.012511ms
p99_latency: 1.79225ms
false_accepts: 0
unsettled_accuracy_milli: 1000
verdict: LATENCY_GATE_FAILED_SAFETY_GATE_PASSED
```

Не снижаем gate до результата. Цель `end_to_end_p99_us <= 500` остается.

Что это значит:

```text
layout/safety path is alive
full 5-step scan over 131_072 edges is too slow for the first 500us target
next optimization target = L3 settle kernel / active-edge traversal
```

L3 settle optimization decision:

```text
chosen strategy = A-grouped active edge lists
not chosen      = bit-grid over full edge scan
```

Почему:

```text
bit-grid keeps scanning all 131_072 edges
A-grouped layout stores edges contiguously by center_a
active fringe visits only edge blocks for active centers
```

Proof harness implementation:

```text
ACTIVE_CENTER_LIMIT = 32
l3_edge_offsets = [u32; 513]
edge groups = center_a contiguous blocks
offset table bytes = 2_052
offset table is paid from 512_000-byte metric/rebuild reserve
```

Second physical release run after active-edge traversal:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

queries: 10_000
p50_latency: 96.527us
p99_latency: 155.841us
false_accepts: 0
unsettled_accuracy_milli: 1000
verdict: LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Результат:

```text
end_to_end_p99_us target <= 500
measured p99 = 155.841
headroom = about 3.2x
```

Q16 real-fringe distribution decision:

```text
do not choose only one distribution
benchmark both:
  regularized attractor graph
  scale-free hubs graph
```

Regularized graph represents the intended post-pruning field:

```text
max_edges_per_center = 256
```

Scale-free graph is the adversarial Zipf/heavy-hub stress:

```text
hub_count = 16
hub_edges = 2_048
remaining centers = about 198..199 edges each
```

Measured counters added to proof harness:

```text
active_center_p50
active_center_p99
edges_visited_p50
edges_visited_p99
```

Third physical release run with real-fringe simulation:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

Regularized:
  queries: 10_000
  p50_latency: 102.870us
  p99_latency: 173.651us
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 40_704
  edges_visited_p99: 40_960
  false_accepts: 0
  unsettled_accuracy_milli: 1000

ScaleFreeHubs:
  queries: 10_000
  p50_latency: 183.062us
  p99_latency: 272.796us
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 68_708
  edges_visited_p99: 70_562
  false_accepts: 0
  unsettled_accuracy_milli: 1000

verdict: REAL_FRINGE_SIMULATION_LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Честная граница:

```text
this is simulated real-fringe distribution
not yet corpus-trained L3 field
next proof must feed active centers from real L2/L3 training output
```

Fourth physical release run with real L2 training output:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

input:
  corpus = data/corpus/russian_words_300k.txt
  L2 train words = 20_000
  L2 heldout words = 5_000
  timed queries with non-empty L2 motif tokens = 4_717

Regularized:
  p50_latency: 66.815us
  p99_latency: 124.450us
  seed_center_p50: 4
  seed_center_p99: 10
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 33_792
  edges_visited_p99: 35_328
  false_accepts: 0
  unsettled_accuracy_milli: 1000

ScaleFreeHubs:
  p50_latency: 66.856us
  p99_latency: 122.199us
  seed_center_p50: 4
  seed_center_p99: 10
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 32_505
  edges_visited_p99: 46_902
  false_accepts: 0
  unsettled_accuracy_milli: 1000

verdict: REAL_L2_OUTPUT_FRINGE_LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Updated boundary:

```text
real L2-output active-fringe proof = passed
corpus-trained L3 field active-fringe proof = still open
```

## Hot Rebuild / Snapshot Rule

Q14 decision:

```text
inference reads immutable hot snapshot
rebuild/pruning never mutates the live matrix in place
new profile is built in cold/staging memory
swap happens only at request boundary by version/epoch pointer swap
```

Это не "lock-free write into live matrix". И это не stop-the-world посреди
inference loop.

Правильная модель:

```text
active_profile_vN -> inference reads only
builder creates profile_vN+1 in staging memory
gate validates profile_vN+1:
  byte budget
  heldout/trap proof
  false_accept_rate = 0
  latency/cache-miss profile
if gate passes:
  atomic epoch swap at request boundary
else:
  keep active_profile_vN
```

Почему так:

```text
no race conditions in inference
no partial matrix state
no allocations inside inference
no need to fit a full shadow copy inside the 6 MiB hot budget
benchmark remains deterministic
```

Metric / rebuild / alignment reserve is not a full model shadow copy:

```text
512 KiB reserve = counters, ablation hits, local rebuild bookkeeping
full rebuild staging memory lives outside hot inference working set
```

Benchmark must test two paths separately:

```text
steady-state inference:
  no allocations
  immutable active profile
  P99 latency gate

swap stability:
  many request-boundary swaps
  no false accept before/after swap
  no torn profile observed
  old epoch remains valid until readers finish
```

## Model Data vs Runtime Scratch

Нельзя смешивать постоянную модель и рабочий буфер.

Runtime scratch:

```text
L1 SurfaceWave accumulator [i16; 4096] = 8192 bytes
L2 motif score buffer
L3 activation buffer
top-k candidate buffer
```

Model hot data:

```text
L2 motif bank
L2 inverted index
L3 axis centers
L3 interaction edges
AntiWave edges
normalization tables
```

L1 accumulator можно считать в total resident working set, но это не веса
модели.

## L2 Motif Bank Layout

Лучший базовый layout - SoA/CSR, а не фиксированный padded record.

Причина: у разных мотивов разное число зубцов. Фиксированные `8 teeth` просты,
но либо режут богатые мотивы, либо тратят пустоту.

Hot arrays:

```text
motif_offset: [u32; motif_count + 1]
motif_norm:   [u16; motif_count]
motif_flags:  [u16; motif_count]
tooth_pool:   [PackedTooth; total_teeth]
weight_pool:  [i8; total_teeth]
```

PackedTooth:

```text
lane_id: 12 bits
sign:     2 bits
local_t:  2 bits
----------------
packed tooth = u16
```

Motif length is variable:

```text
min_teeth = 4
max_teeth = profile limit, often 8..16
```

Promotion chooses teeth by:

```text
support * idf * heldout_gain / (1 + conflict_count)
```

## L2 Inverted Index Layout

The index must not be keyed by `lane` alone.

Correct key:

```text
tooth_bucket = hash(lane_id, sign, local_t) & (bucket_count - 1)
```

Best layout under free volume:

```text
front_index: fixed [motif_id; 4] per bucket
overflow_index: CSR pool for buckets with more than 4 good refs
```

Why:

```text
front_index gives fast common path
overflow_index prevents useful motif loss
```

Overflow is allowed, but bounded and measured.

Bucket policy:

```text
first 4 refs: highest tooth_score, hot path
overflow refs: sorted by tooth_score, scanned only if needed
low-score refs: rejected
```

Training must report:

```text
bucket_count
avg_refs_per_bucket
max_refs_per_bucket
overflow_bucket_count
overflow_ref_count
motifs_rejected_by_index_saturation
collision_false_candidate_rate
overflow_latency_cost
```

Promotion requires:

```text
heldout still passes
trap proof still passes
collision_false_candidate_rate under budget
overflow_latency_cost under profile budget
```

If overflow improves heldout/traps with acceptable latency, keep it. If it only
hides noisy popular lanes, reject those teeth/motifs.

## L3 Axis Center Layout

Separate immutable center data from runtime activation.

Model hot:

```text
axis_id:       u8
flags:         u8
base_mass:     u16
threshold:     i16
reserved/stat: u16
----------------
8 bytes per center
```

Runtime scratch:

```text
activation: [i16; center_count]
previous_activation: [i16; center_count]
```

The scratch buffers are not model weights.

## L3 Edge Layout

Use flat sorted edges, preferably CSR by source center.

Edge record:

```text
target_center_id: u16
flags:            u8
compat_weight:    i8
conflict_weight:  i8
anti_weight:      i8
--------------------
6 bytes logical
```

Two physical candidates:

```text
packed 6-byte edge  -> smaller
aligned 8-byte edge -> often faster
```

Benchmark decides. Do not assume smaller is faster.

## Layout Decision Rule

Лучший layout выбирается proof-метриками, не эстетикой:

```text
layout_score =
  heldout_gain
- false_accept_penalty
- trap_failure_penalty
- latency_penalty
- cache_miss_penalty
- complexity_penalty
```

Проект может держать несколько task-first compiled profiles:

```text
surface_form_profile
single_domain_operator_profile
dialogue_behavior_profile
multi_domain_reasoning_profile
critical_control_profile
```

Runtime выбирает профиль по задаче, корпусу, домену и устройству.

## Byte Utility / Promotion Control

Каждый байт должен покупать качество.

Не держим матрицу заполненной ради красоты.

Контроль полезности памяти:

```text
edge decay / pruning
motif tooth quantization
entropy_per_byte tracking
heldout/trap contribution
replacement by stronger candidates
```

Для каждого motif считаем:

```text
motif_utility =
  heldout_gain
+ trap_rejection_gain
+ compression_gain
- false_accept_penalty
- collision_penalty
- latency_cost
```

Для каждого edge считаем:

```text
edge_utility =
  settle_gap_gain
+ trap_rejection_gain
- oscillation_penalty
- false_accept_penalty
- activation_deadness_penalty
```

Promotion:

```text
promote only if utility > promotion_threshold
```

Pruning:

```text
if utility decays below keep_threshold:
  remove edge/motif
  free slot for stronger candidate from streaming candidate pool
```

Motif quantization:

```text
if motif is popular but low-discriminative:
  shrink teeth count 8 -> 6 -> 4
  keep only highest tooth_score teeth
```

Tooth score:

```text
tooth_score =
  support
* idf
* heldout_gain
/ (1 + conflict_count + collision_cost)
```

Entropy rule:

```text
low activation entropy + low heldout/trap contribution
-> candidate for replacement
```

Final gate:

```text
bytes_used may grow
only if heldout/trap/latency metrics improve
```

Metrics reserve rule:

```text
training / profiling build:
  reserve stores utility counters, ablation hits, cache-miss samples

compiled inference build:
  reserve may be frozen into read-only utility tables
  or reassigned to rare motifs / anti-wave if counters are not needed
```

Итог по Q12:

```text
best mechanism =
  edge decay / pruning
+ motif tooth quantization
+ entropy_per_byte
+ heldout/trap promotion gate
```

Не выбираем один механизм из трёх. Лучшее решение - связать их в один
promotion economy loop:

```text
новый байт в hot model допускается только если он покупает:
  больше heldout/stress качества
  больше trap rejection
  меньше false accept
  или меньшую latency/cache-miss цену
```

## Что еще не закрыто

1. Реальный L2 sparse resonance engine.
2. Inverted index по `(lane, sign, local_t)`.
3. Нормировка lane popularity / motif length.
4. L2 proof: heldout, corrupt, reversed, no exact lookup.
5. L3 axis-center representation.
6. L3 compatibility/conflict matrix.
7. L3 operator composition proof.
8. Bounded L3 settle proof: damping, veto, FIELD_UNSETTLED.
9. Sparse L3 interaction learning proof.
10. L3 field regularization proof.
11. Active adversarial trap generation proof.
12. L3 settle kernel / active-edge traversal optimization proof.
    Initial synthetic full_v1 release proof passed. Real L2-output
    active-fringe release proof passed. Next proof must use corpus-trained L3
    field active centers.
13. Hot memory layout benchmark/proof.
14. L2 inverted index saturation proof.
15. Byte utility / pruning proof.
16. Hot rebuild snapshot / epoch swap proof.
17. Связь L3 operator -> L4 answer plan. Заблокировано до закрытия L1-L3
    debt gate.

## Ближайший Приоритет

Сначала не декодер.

Сначала нужен training protocol:

```text
corpus
-> L1 atoms
-> L2 sparse time-phased motif mining
-> L3 axis-center induction
-> compatibility/conflict/anti-wave learning
-> heldout/trap proof
```

Декодер имеет смысл только после того, как `O_final` стабильно рождается и
умеет честно возвращать `FIELD_UNSETTLED`.

L4 lock:

```text
NO L4 until L1/L2/L3 debts are closed.
```

L4 unlock criteria:

```text
L2 sparse resonance engine works
L2 inverted index saturation is measured
L2 heldout/corrupt/reversed proof passes
L3 axis centers work
L3 compatibility/conflict field works
L3 bounded settle returns O_final or FIELD_UNSETTLED
trap / anti-wave proof passes
6 MiB full_v1 layout has benchmark/proof
byte utility / pruning proof passes
snapshot / epoch swap proof passes
```

## Guard

Если новый слой требует ручного хардкода смысла под конкретный домен, дизайн
считается подозрительным.

Если слой проходит только на train, но не переносится на heldout, это не
грокинг.

Если слой хранит строку целиком и потом достает ее обратно, это lookup, а не
Wave.

Если кто-то пытается перейти к L4 до debt gate, это architectural drift.
