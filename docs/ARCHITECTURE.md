# Nando Wave Architecture v2

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
