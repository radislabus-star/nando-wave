# Nando Wave Goal

Этот документ - главный goal проекта. Он нужен, чтобы Nando Wave не расползался
в бесконечные красивые эксперименты и не превращался в обычный набор
hardcoded-подсказок.

## Конечная цель

Довести Nando Wave до состояния:

```text
CPU-friendly wave cellular organism proof
```

Это значит:

```text
малые Cell32/Expert64 клетки
под CarrierWave
через WaveBus
после нескольких settle ticks
создают измеримую ансамблевую моду,
которой нет у одиночной клетки,
нет у простого voting,
и нет у монолита той же памяти
на выбранном классе задач.
```

После этого разрешается делать демонстрационный Chat-0:

```text
input
-> prompt wave
-> carrier lock
-> settle loop
-> memory imprint
-> wave readout
-> short answer
-> feedback
-> checked local update
-> snapshot
```

Chat-0 - не финальная копия GPT. Chat-0 - демонстрация доказанного принципа.

## Главный результат

Финальным полезным результатом считается не один удачный текстовый ответ, а
пакет доказательств:

```text
1. Есть задача, где cellular organism работает лучше controls.
2. Есть key modes/key cells/key links, найденные измерением.
3. Ablation этих modes/cells/links ломает результат.
4. Snapshot сохраняет состояние и восстанавливает поведение.
5. Online feedback улучшает систему только после eval gate.
6. CPU budget на T480 остается приемлемым.
7. Chat-0 использует этот механизм, а не lexical shortcut.
```

Если любой из этих пунктов не проходит, проект не провален, но статус честный:

```text
not_found
candidate
partial
passed
```

Слово `proved` в проекте запрещено, пока нет ablation и controls.

## Северная звезда в одной формуле

```text
не маленькая LLM,
а измеримый волновой клеточный организм.
```

Более строго:

```text
Nando Wave проверяет, может ли динамическая релаксация гармонических клеток
дать переносимую ансамблевую моду и частотную память на CPU.
```

## Что сейчас уже есть

Текущая база проекта:

```text
Rust workspace
Cell32 fixed 32 KB
Mono192 fixed 192 KB
CarrierWave
WaveBus
SpectrumSnapshot
Organ128 = 128 x Cell32
settle-dialog trace
response gate eval
thought probe eval
byte-context evals
Chat-0 narrow eval loop
quick checks and push checks
```

Главный честный вывод текущего состояния:

```text
волновые признаки уже участвуют,
memory/snapshot уже имеют сигнал,
но самостоятельная переносимая thought/mode readout еще не доказана.
```

Значит следующий ход - не наращивать чат, а построить более строгий прибор
по образцу Nanda et al.: task, frequency census, progress measures, ablation.

## Этап 1: Goal freeze

Цель этапа: зафиксировать управление проектом.

Сделать:

```text
docs/GOAL.md как главный goal
README.md со ссылкой на GOAL
docs/ROADMAP.md синхронизировать с GOAL
короткая команда проверки goal-critical evals
```

Gate:

```text
документы отвечают на вопросы:
что строим
что не строим
что считается победой
что считается самообманом
какой следующий эксперимент
```

Стоп-сигнал:

```text
если новый эксперимент не связан с пунктом GOAL, он уходит в PARKING_LOT.
```

## Этап 2: Nanda-style formal task

Цель этапа: убрать субъективность текста и проверить принцип на маленьком
мире, где известен правильный алгоритм.

Первая задача:

```text
modular addition small P
```

Кандидаты:

```text
P = 31
P = 61
byte modular relation
phase composition
```

Нужно реализовать:

```text
train split
holdout split
Fourier/phase feature census
restricted score
excluded score
key frequency ablation
non-key frequency ablation
Mono control
Voting control
Random control
```

Gate:

```text
holdout выше random
cellular >= Mono192 или cellular устойчивее Mono192
restricted key modes сохраняют большую часть качества
excluded key modes ломают качество
non-key ablation ломает меньше key ablation
```

Стоп-сигнал:

```text
если cellular не лучше controls и ablation не показывает key modes,
не переходить к Chat-0 как к доказательству.
```

## Этап 3: Прогресс-меры до озарения

Цель этапа: не ждать финального результата часами, а видеть предвестники.

Метрики:

```text
restricted_loss
excluded_loss
key_mode_energy
non_key_mode_energy
coherence_gain
entropy_drop
phase_velocity_decay
ablation_gap
snapshot_replay_gap
```

Статусы:

```text
none
warmup
circuit_seed
grok_candidate
stable_grok_candidate
not_found
```

Gate:

```text
предвестники stable_grok_candidate появляются раньше финальной holdout accuracy
и коррелируют с последующим улучшением.
```

Стоп-сигнал:

```text
если ранние метрики не предсказывают результат лучше случайности,
они остаются диагностикой, а не управляющим сигналом.
```

## Этап 4: Cellular advantage

Цель этапа: проверить главный вопрос пользователя:

```text
64 + 64 + 64 лучше, чем 192?
```

Сравнить:

```text
1 x Mono192
3 x Expert64
6 x Cell32
6 x Cell32 without WaveBus
6 x Cell32 voting only
6 x Cell32 with WaveBus
Organ128 subset
```

Измерять:

```text
accuracy
false positive rate
coherence
entropy
phase stability
CPU time
cache pressure
ablation drop
seed robustness
```

Gate:

```text
cellular_with_bus > best_single
cellular_with_bus > voting
cellular_with_bus >= Mono192 or safer/faster than Mono192
ablation_drop >= threshold
seed robustness passed
```

Стоп-сигнал:

```text
если voting равен WaveBus по качеству и trace,
волновая шина не доказана.
```

## Этап 5: OrganState links

Цель этапа: усилить межклеточную связь, а не добавлять признаки поверх.

Нужно разделить роли:

```text
CellState    - долгие параметры клетки
OrganState   - связи, роли, доверие, coupling
RuntimeState - текущая волна, bus, carrier, snapshot
```

Связи должны быть измеримыми:

```text
coupling strength
link activation
link ablation drop
role transfer
cell co-activation
phase lock between cells
```

Gate:

```text
удаление key link портит результат сильнее удаления random link.
```

Стоп-сигнал:

```text
если links не влияют на результат, не называть это организмом.
```

## Этап 6: Memory imprint

Цель этапа: prompt должен записываться во внутреннее состояние, а не оставаться
внешней подсказкой.

Проверить:

```text
prompt -> settle -> memory imprint
memory imprint -> answer without direct prompt scorer
wrong imprint -> worse
corrupted imprint -> refusal or low confidence
snapshot restore -> same answer mode
```

Gate:

```text
no_direct_prompt остается выше random
full лучше no_memory
wrong_memory заметно хуже full
snapshot replay восстанавливает trace и answer class
```

Стоп-сигнал:

```text
если no_direct_prompt = 0, memory еще не самостоятельная.
```

## Этап 7: Controlled online learning

Цель этапа: система может учиться от feedback, но не портить ядро.

Разрешено менять:

```text
runtime adapter
cell trust
coupling
prompt memory bank
snapshot index
```

Запрещено без eval:

```text
ломать Cell32 weights во время ответа
перезаписывать corpus
принимать любой user feedback как истину
```

Feedback cycle:

```text
generate
receive feedback
write pending update
run local eval
promote only if gate passed
rollback otherwise
```

Gate:

```text
после promote holdout не хуже baseline
новый feedback улучшает целевой класс
false positives не растут выше лимита
```

Стоп-сигнал:

```text
если online learning улучшает train и ломает holdout, это memorization,
а не grokking.
```

## Этап 8: Chat-0

Цель этапа: сделать маленькую демонстрацию, которая пишет короткий ответ
через внутреннее wave-state.

Минимальный Chat-0:

```text
короткий русский/английский prompt
короткий ответ 1-2 предложения
trace по ticks
response gate
refusal при брожении
snapshot save/replay
feedback pending
```

Chat-0 должен показывать:

```text
carrier phase
settle trace
coherence/entropy
memory validation
active cells
answer source
why answered/refused
```

Gate:

```text
known prompts answered
unknown/noisy prompts refused
direct prompt shortcut ablation не убивает все
memory ablation снижает качество
wrong carrier снижает качество
```

Стоп-сигнал:

```text
если Chat-0 держится только на lookup/matching,
это demo shell, а не организм.
```

## Этап 9: CPU proof on T480

Цель этапа: доказать, что архитектура реально CPU-friendly.

Измерять:

```text
time per tick
time per settle
active cells per tick
L1/L2/L3 plan
allocations in hot path
branch-heavy hotspots
cache misses if available
release vs debug speed
```

Инструменты:

```text
cargo bench or custom bench
perf stat
perf record
flamegraph if needed
scripts/check-quick.sh
scripts/check-push.sh only before push
```

Gate:

```text
обычная разработка не гоняет тяжелые sweeps
hot path не делает heap allocation
settle работает в заданном CPU budget
профиль показывает понятные hotspots
```

Стоп-сигнал:

```text
если каждый эксперимент требует часы без early predictor,
сначала улучшать progress measures.
```

## Этап 10: Decision point

После прохождения этапов 2-9 проект должен честно ответить:

```text
Nando Wave found
Nando Wave partial
Nando Wave not found
```

`found` можно сказать только если:

```text
есть cellular advantage
есть key mode ablation
есть snapshot replay
есть no-shortcut control
есть CPU proof
есть Chat-0 demo on top of those mechanisms
```

`partial`:

```text
есть отдельные полезные механизмы,
но они не складываются в переносимый организм.
```

`not_found`:

```text
controls объясняют результат проще, чем волновая клеточная гипотеза.
```

Это тоже нормальный научный результат.

## Правила против расползания

Новые идеи добавляются только в один из слотов:

```text
core mechanism
eval/gate
performance
documentation
parking lot
```

Нельзя делать:

```text
добавлять hardcoded ответы как интеллект
называть matching мышлением
называть candidate доказательством
увеличивать corpus вместо проверки механизма
добавлять признаки без ablation
гонять тяжелые проверки после каждой мелкой правки
```

Можно делать:

```text
маленькие строгие задачи
быстрые checks
ablation
seed sweep перед выводом
профилирование hot path
документирование отрицательных результатов
```

## Ближайший следующий шаг

Следующий кодовый шаг:

```text
organ128-modadd-eval
```

Минимальная версия команды должна вывести:

```text
task
seed
modulus
train_size
holdout_size
random_accuracy
mono192_accuracy
cell32_voting_accuracy
cell32_wavebus_accuracy
restricted_key_accuracy
excluded_key_accuracy
key_ablation_drop
non_key_ablation_drop
mode_status
```

Первый успех:

```text
mode_status: organ128_modadd_candidate
```

Настоящий успех:

```text
mode_status: organ128_modadd_key_mode_ablation_passed
```

Только после этого имеет смысл возвращаться к усилению Chat-0.
