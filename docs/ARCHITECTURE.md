# Nando Wave Architecture v2

Current architecture overlay, 2026-07-06:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current active implementation frame:

```text
L1/L2: source-neutral safe atoms and event shape
L4: streaming selector/router/admission/goal-state layer
L3: drifting phase-center operator memory
Hot runtime: bounded PhaseCenterHotRuntime score path
Verifier: promotion/local-accept gate
```

Current proof boundary:

```text
best shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

local_accept_enabled: false
market_money_claim_allowed: false
```

Этот документ заменяет первый черновик архитектуры. Причина переписывания:
последние promoted-holdout контроли показали, что простые centroid-переносы по
snapshot, active Cell32 ids и короткой фазовой траектории слишком плоские.

Новая архитектура должна удерживать исходную идею:

```text
не маленькая LLM
не набор экспертов с голосованием
а волновой клеточный организм,
который приходит к ответу через динамическую релаксацию
к согласованному гармоническому состоянию
```

Зафиксированная цель проекта и исходная формулировка claim вынесены сюда:

```text
docs/architecture_lineage/00_recorded_project_goal_and_claim.md
```

## Родословная архитектуры и граница claim

Короткая формула:

```text
кирпичи известные, сборка своя
```

Nando Wave нельзя честно описывать как полностью никем не описанную архитектуру:
почти каждый отдельный принцип имеет родню в литературе. Но ее также нельзя
сводить к одной известной архитектуре.

Более точная формула:

```text
Nando Wave = custom sparse wave-associative, role-binding, proof-gated architecture
inspired by associative memory, sparse distributed memory, vector-symbolic binding,
and mechanistic grokking analysis, but not identical to any single standard architecture.
```

Известные семейства идей, к которым проект имеет родство:

```text
associative memory / attractor dynamics
sparse distributed memory
Hebbian updates
role/filler binding
vector-symbolic representations
Fourier / progress-measure grokking analysis
sparse feature superposition
```

Подробные рабочие карточки по каждой позиции вынесены отдельно:

```text
docs/architecture_lineage/README.md
```

Ориентиры, но не готовые доказательства для Nando Wave:

- Hopfield networks: content-addressable memory, где динамика сходится к
  устойчивому состоянию.
- Kanerva Sparse Distributed Memory: высокоразмерная память и чтение по
  близости.
- Smolensky tensor-product binding и Plate HRR: role/filler binding.
- Nanda et al. modular-addition grokking analysis: проверка learned algorithm
  через Fourier/progress measures.
- Toy models of superposition: collision/interference при sparse features.

Что в этой сборке является местным R&D, а не готовым стандартным методом:

```text
exact SurfaceWave/L1 encoding
L2 center/motif promotion gates
L3 state-delta/action-role binding pressure
anti-wave/trap proof loop
strict no-shortcut claim boundary
CPU/cache-resident runtime target
v2/v3 corpus pressure methodology
```

Текущая L1-L2-L3 лестница проекта:

```text
L1 SurfaceWave4096
-> L2 motifs / center sequences
-> L3 state-delta / action-role binding
-> L4 closed until L1-L3 debts are proven
```

Главное отличие проекта не только в математике, но и в proof-gate дисциплине:

```text
не называем grokking,
если есть target_id / proof_rule_id / lookup / shortcut
```

Текущий WavePredictor state-delta путь:

```text
state_before + rule_action_example
-> learned pressure over target_delta
-> role/action slot binding
-> flat parity check
-> ablation checks
```

Это не классический Hopfield, не HRR, не TPR и не Transformer. Это кастомный
sparse Hebbian/contrastive predictor с role-binding readout и явными
anti-shortcut проверками.

Граница заявления:

```text
literature novelty claim: not allowed yet
engineering / R&D novelty inside this repo: yes
```

Самая безопасная формулировка:

```text
Это не открытие из пустоты.
Это новая инженерная сборка известных физических и нейросетевых идей
под конкретную цель:
доказать компактный переносимый оператор без lookup-а и без большой модели.
```

### Позиция 1: Attractor / Associative Memory

Классическая идея:

```text
частичный / шумный вход
-> сеть динамически сходится
-> к ближайшему устойчивому сохраненному состоянию
```

У Hopfield networks это content-addressable memory: не ищем запись по адресу, а
подаем кусок состояния, и система сама доходит до устойчивого паттерна.

Родство Nando Wave:

```text
active centers
-> compatibility / conflict / anti-wave
-> bounded settle / gap
-> accept or reject
```

L3 field в проекте мыслится не как чистый Hopfield recall, а как typed sparse
attractor-field:

```text
score(center) =
  motif_votes
+ compatibility(other_centers)
- conflict(other_centers)
- anti_wave
```

Settle также должен быть bounded, а не бесконечной релаксацией:

```text
for step in 0..N:
  centers += compatibility
  centers -= conflict
  centers *= damping(step)
```

В текущем `WavePredictorHebbianField` это проявлено через:

```text
base_mass
edges
state_delta_edges
state_delta_role_binding_edges
```

А edge имеет отдельные каналы:

```text
compatibility
conflict
anti_wave
```

Что сильнее для нашей задачи, чем простой nearest-pattern recall:

```text
1. Не только attraction, но и explicit rejection:
   correct attractor усиливается, wrong attractor / trap подавляется.

2. Attractor сам по себе не считается proof:
   нужны heldout, traps, ablation и no-shortcut gates.

3. Цель смещена от pattern recall к transition recall:
   не partial X -> full X,
   а state_t + rule_action -> state_t+1.

4. Attractor становится инженерным объектом проверки:
   accuracy, gap, p10 gap, ablation drop, flat parity,
   shortcut gates, bytes estimate.
```

Что пока упущено / не доказано:

```text
1. Нет строгой global energy function.
   У Hopfield есть energy monotonicity; у нас пока инженерное поле
   compatibility - conflict - anti_wave без формального energy proof.

2. Нет доказанных attraction basins.
   Мы пока не можем сказать:
   "для такого радиуса шума система гарантированно восстановит состояние".

3. V3 показал предел текущего binding/attractor separation:
   ordered_sequence_accuracy_milli = 269
   flat_gap_parity_mismatches = 0
   Значит runtime честный, но field не разделяет dense rule/slot matrix.

4. Текущий WavePredictor больше margin/readout-field, чем полноценный
   recurrent attractor:
   active fringe -> learned sparse pressure -> gap over target_delta.
```

Рабочий вывод:

```text
Классика: вспомнить устойчивый образ.
Nando Wave цель: стабилизировать правильный переход и отвергнуть близкую ловушку.
```

Статус позиции:

```text
Attractor / associative memory:
  родство с классикой: ДА
  улучшение под нашу задачу: ДА
  универсально лучше классики: НЕТ
  доказано до конца: НЕТ
```

Следующий proof/debt для взятия в работу:

```text
basin stability:
  measure which perturbation radius still preserves the correct transition.

gap stability:
  require median and p10 gap to stay positive under controlled noise.

ablation stability:
  remove learned compatibility/conflict/anti-wave channels separately and
  prove the transition collapses in the expected way.

transition stability:
  measure state_t + rule_action -> state_t+1, not only final accuracy.

energy proxy:
  define a monotonic or bounded field-energy proxy before claiming mature
  attractor behavior.
```

## Жесткие решения

```text
ядро только Rust
runtime без Python
первый горячий атом: Cell32
Expert64 = 2 x Cell32
Organ192 = 6 x Cell32
первый честный контроль: Organ192 против Mono192
обучение только через eval-gate
Chat-0 только как демонстрация проверенного принципа
```

## Главный архитектурный сдвиг

Старая схема была слишком похожа на классификатор:

```text
prompt -> features -> centroid -> answer
```

Новая схема должна быть динамической:

```text
input
-> Sense: кодирование импульса
-> Carrier: установка глобальной несущей
-> Settle: несколько wave ticks до согласования
-> Read: чтение устойчивой моды
-> Act: короткий ответ
-> Feedback: событие обратной связи
-> Consolidate: проверенное изменение памяти
```

Ключевое: ответ должен читаться не из одной финальной точки, а из устойчивого
режима после нескольких тиков.

## Три слоя состояния

Архитектура разделяет три памяти. Их нельзя смешивать.

```text
CellState      - долгие параметры клетки
OrganState     - связи, роли, доверие и специализация клеток
RuntimeState   - текущие фазы, CarrierWave, WaveBus и Snapshot
```

`CellState` меняется редко и только после eval.

`OrganState` хранит, какие клетки работают вместе и какие связи оказались
полезны.

Текущая реализация: `OrganState` уже есть в `nando-core` как runtime wrapper
над `Organ192`. Он хранит `CarrierWave`, прошлый центр фазы, прошлую
coherence/entropy и `cell_coupling[6]`. Это первый измеримый слой связей, но
еще не доказанная мода слова.

`RuntimeState` живет во время генерации и может свободно релаксировать, но не
считается обучением.

## Cell32

`Cell32` - горячая физическая единица под T480. Ее смысл не в том, что она сама
умная, а в том, что она быстро входит в резонанс и передает вклад в организм.

Внутри Cell32:

```text
phase slots
amplitudes
decay
local resonance
short trace accumulators
plasticity counters
```

Запрещено для горячей клетки:

```text
heap allocation
сложные Rust-объекты
строковые словари
неограниченные Vec
скрытая мутация во время ответа
```

Клетка должна быть измеримой:

```text
resonance
phase contribution
energy contribution
ablation impact
last useful tick
```

## Expert64 и Organ192

`Expert64` - не “модель на 64 KB”, а маленький орган из двух Cell32.

```text
Cell32 A - fast phase detector
Cell32 B - stabilizer / counter-phase / memory companion
```

`Organ192` - первый организм из шести Cell32:

```text
2 Cell32 для большой моды
2 Cell32 для средней моды
2 Cell32 для малой моды
```

Именно `Organ192` сравнивается с `Mono192`, чтобы проверять не объем памяти, а
эффект организации.

## CarrierWave

`CarrierWave` - не bias и не ручная подсказка. Это медленная глобальная мода,
которая задает область допустимого движения.

Функции:

```text
держит тему / intent / режим
задает фазовую границу для клеток
ограничивает дрейф
модулирует амплитуды нижних мод
попадает в Snapshot
портится отдельным carrier-control gate
```

CarrierWave должна быть проверяемой:

```text
correct carrier
no carrier
wrong carrier
corrupted carrier
carrier ablation
```

Если отключение CarrierWave не меняет поведение, архитектура потеряла
несущую волну.

## WaveBus

`WaveBus` - место интерференции. Это не голосование.

WaveBus собирает:

```text
суммарный фазовый вектор
center of mass
coherence
spectral entropy
active cell ids
energy flow
phase velocity
settle trace
```

Минимальный критерий отличия от voting:

```text
wave bus лучше voting
или равен по accuracy, но лучше по coherence / entropy / safety
и ablation разрушает эффект
```

Если voting дает тот же результат и те же traces, волновая часть не доказана.

## Settle Loop

Главная новая единица вычисления - не один tick, а короткая релаксация.

```text
tick 0: input impulse
tick 1: fast cells react
tick 2: medium cells stabilize
tick 3: CarrierWave clamps drift
tick 4: WaveBus reads stable mode
```

Для T480 стартовый лимит:

```text
settle_ticks: 3..8
active_cells_per_tick: top-8..top-32
full sweep: только диагностика
```

Метрики settle loop:

```text
coherence_gain
entropy_drop
center_phase_stability
phase_velocity_decay
ablation_drop
time_per_answer
```

Признак продуктивного состояния - не высокая уверенность одной клетки, а
снижение хаоса во времени.

Текущий результат `eval-settle-word`: 3 ticks сравнялись с `Mono192`, но 5/8
ticks разваливают траекторию. Поэтому stop-condition должен быть обучаемым или
gate-driven, а не фиксированным числом тиков.

## Snapshot

`Snapshot` - это частотный аккорд состояния, а не дамп модели.

Snapshot v2 должен хранить:

```text
format version
CarrierWave state
WaveBus center
dominant phase slots
coherence
entropy
phase velocity
settle trace summary
active cell sketch
organ id / role id
checksum
```

Snapshot считается памятью только если:

```text
warm snapshot лучше cold start
wrong snapshot хуже warm
corrupted snapshot хуже warm
snapshot помогает на transition / holdout
```

Replay exact state полезен, но не достаточен. Нужна transition-memory.

## Обучение

Во время ответа разрешена только runtime-релаксация.

Долгая память меняется так:

```text
feedback log
-> candidate consolidation
-> holdout eval
-> ablation eval
-> compare Mono192 / voting / no-carrier
-> promote or reject
```

Запрещено:

```text
мутировать CellState без eval
учиться на собственном ответе без feedback
считать exact overlay доказательством обучения
считать task hint доказательством моды
```

Promoted state может существовать как отдельный слой, но он не должен
маскироваться под найденную ансамблевую моду.

## Chat-0

Chat-0 - демонстрационный контур, а не цель сам по себе.

Минимальный путь:

```text
prompt
-> byte/phrase impulse
-> CarrierWave lock
-> Organ settle loop
-> WaveBus mode
-> tiny decoder
-> short response
-> trace
-> feedback log
```

Chat-0 считается полезным только если каждый ответ оставляет trace:

```text
route
carrier state
settle ticks
active cells
coherence / entropy
selected mode
snapshot bytes
feedback status
```

Пока нет переносимого promoted-holdout эффекта, Chat-0 остается узким
eval-controlled shell, а не самообучающимся ботом.

## Почему последние negative gates важны

Проверено:

```text
exact overlay не обобщает
naive harmonic transfer хуже base
selective harmonic gate только возвращается к base
active Cell32 signature хуже base
compact trajectory centroid хуже base
task hint дает верхнюю границу, но не является модой
```

Вывод для v2:

```text
искать надо не один snapshot centroid
и не финальный active-cell набор,
а устойчивую settle-динамику организма:
как фазы сходятся,
какие клетки взаимно стабилизируются,
и что ломается при ablation
```

## Первый v2 milestone

Следующий прибор должен называться условно:

```text
eval-settle-mode
```

Он проверяет:

```text
Mono192 one-shot
Organ192 one-shot
Organ192 settle-3
Organ192 settle-5
Organ192 settle-8
Organ192 voting
Organ192 no-carrier
Organ192 wrong-carrier
Organ192 ablate-cell
Organ192 ablate-link
```

Gate pass:

```text
settle loop улучшает качество или устойчивость
wave bus отличается от voting
carrier нужен
ablation разрушает эффект
результат повторяется на seed sweep
```

Gate fail:

```text
settle не улучшает ничего
voting равен wave bus
carrier не влияет
ablation не меняет результат
Mono192 не хуже Organ192
```

## Масштабирование

Порядок роста:

```text
6 Cell32
16 Cell32
64 Cell32
256 Cell32
1024 Cell32
```

Переход к следующему размеру разрешен только если предыдущий дал:

```text
mode metric
ablation sensitivity
runtime budget
seed robustness
```

Иначе увеличение числа клеток будет прятать слабую архитектуру.

## Что не входит в v2

```text
большая LLM
GPU training
скрытая онлайн-мутация весов
обучение на приватных логах
дистилляция hidden states закрытых моделей
объявление Chat-0 доказательством
```

## Самая короткая формула

```text
Nando Wave v2 =
Cell32 organs
+ CarrierWave
+ multi-tick settle loop
+ WaveBus mode readout
+ Snapshot transition memory
+ eval-gated consolidation
```

Это направление ближе к исходной северной звезде, чем простая цепочка
`features -> centroid -> answer`.
