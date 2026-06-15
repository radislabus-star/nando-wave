# Nando Wave Goal

Этот документ - главный goal проекта. Он нужен, чтобы Nando Wave не расползался
в бесконечные красивые эксперименты и не превращался в обычный набор
hardcoded-подсказок.

## Конечная цель

Довести Nando Wave до состояния:

```text
CPU-friendly wave cellular organism evidence package
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

Chat-0 - не финальная копия GPT. Chat-0 - демонстрация проверенного принципа.

## Главный результат

Финальным полезным результатом считается не один удачный текстовый ответ, а
пакет проверок:

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

Слово `proved` в проекте запрещено, пока нет ablation, controls и seed
robustness. Даже после этого в отчетах лучше писать `passed` или
`evidence_package_passed`, а не объявлять философскую победу.

## Уровни победы

Чтобы не путать первый сигнал с финальным результатом, проект использует
четыре уровня победы:

```text
Bronze   - toy task candidate: есть сигнал выше controls на маленькой задаче
Silver   - key mode: найдены key modes/cells/links и ablation их ломает
Gold     - memory: snapshot/no-shortcut controls подтверждают внутреннее состояние
Platinum - Chat-0: короткий ответ работает поверх Gold-механизма
```

Запрещенная подмена:

```text
Bronze нельзя называть Chat-0
Silver нельзя называть памятью
Gold нельзя называть GPT-like
Platinum нельзя считать универсальной LLM
```

## Формула найденной моды

Ансамблевая мода считается найденной только если одновременно выполняется:

```text
mode_exists =
  ensemble_gain >= threshold
  AND key_ablation_drop >= threshold
  AND key_ablation_drop >= 2 x non_key_ablation_drop
  AND seed_robustness passed
  AND no_shortcut_control passed
  AND false_positive_increase <= limit
```

Минимальные v0-пороги для первых eval:

```text
ensemble_gain >= 0.03
key_ablation_drop >= 0.05
key_ablation_drop >= 2 x non_key_ablation_drop
seed_robustness >= 4/5 seed pairs
false_positive_increase <= 0.02
```

Эти пороги не священные. Их можно менять, но только вместе с объяснением в
документе и повторным прогоном controls.

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
но самостоятельная переносимая thought/mode readout еще не подтверждена.
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
Fourier phase control
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
не переходить к Chat-0 как к подтвержденному механизму.
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
волновая шина не подтверждена.
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

## Этап 9: CPU evidence on T480

Цель этапа: показать, что архитектура реально CPU-friendly.

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
есть CPU evidence
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
называть candidate подтвержденным результатом
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

## Experimental Protocol v0

Этот раздел важнее любого красивого `mode_status`. Он защищает проект от
утечки правильного ответа в код и от самообмана на маленьких выборках.

### Train/holdout

Правила:

```text
train используется для настройки cells/links/readout
holdout используется только для финальной проверки
key modes выбираются только на train
после выбора key modes они freeze
holdout не используется для выбора порогов
seed sweep запускается перед любым claim выше candidate
```

Если threshold был подобран на holdout, этот прогон считается exploratory и не
может давать `passed`.

### Anti-leakage

Для задач с известной формулой, например `(a + b) mod P`, действует жесткое
правило:

```text
label formula разрешена только для генерации dataset и проверки accuracy
runtime/readout не имеет права вызывать label formula
key frequency selection не имеет права смотреть holdout labels
task-specific shortcut не считается mode
```

Обязательные negative controls:

```text
label_shuffle ломает результат
wrong_carrier ухудшает результат
random_snapshot не помогает
no_wavebus снижает или меняет trace
voting_only не объясняет тот же эффект
```

Если label shuffle не ломает результат, значит eval протек или readout
измеряет не задачу.

### Scientific pass и engineering pass

Два вида успеха не смешиваются.

```text
scientific_pass:
  cellular_with_bus > Mono192 по качеству
  AND ablation подтверждает key mode

engineering_pass:
  cellular_with_bus примерно равен Mono192 по качеству
  AND быстрее, устойчивее или дешевле на T480
  AND ablation подтверждает key mode
```

`engineering_pass` полезен, но он не показывает, что клетки умнее монолита.
Он показывает, что организация может быть практичнее.

### Report artifact

Каждый серьезный eval должен уметь печатать отчет, а позже сохранять его в
`target/reports`.

Минимальный отчет:

```text
command
git_commit
seed
task
dataset_size
train_size
holdout_size
elapsed_ms
random_accuracy
mono_accuracy
voting_accuracy
wavebus_accuracy
ensemble_gain
key_ablation_drop
non_key_ablation_drop
false_positive_increase
fourier_phase_accuracy
cell32_phase_compose_accuracy
cell32_structured_compose_accuracy
cell32_fourier_census_accuracy
phase_compose_gain
structured_compose_gain
fourier_census_gain
wave_over_fourier_gap
compose_over_fourier_gap
structured_over_fourier_gap
census_over_fourier_gap
mode_status
```

Позже добавить JSON, но первый человекочитаемый отчет важнее.

### Failure branches

Если `modadd` не проходит после 3 архитектурных вариантов:

```text
остановить Chat-0 усиление
проверить phase math
проверить readout leakage
проверить CarrierWave не как bias, а как state
сравнить с простым Fourier baseline
```

Fourier baseline в этом этапе - диагностический контроль. Он имеет право
использовать прямую фазовую геометрию задачи, но не считается Nando-readout и
не входит в `scientific_pass`. Если Fourier baseline проходит, а WaveBus нет,
значит задача фазово решаема, но текущая органная динамика или readout не
нашли нужную композицию.

Если `modadd` проходит, а byte-context не проходит:

```text
строить bridge task между modular world и byte world
не перепрыгивать сразу в свободный чат
```

Если Chat-0 держится на matching:

```text
оставить Chat-0 как shell
не называть organism
вернуться к memory imprint и no-direct-prompt control
```

Если CPU budget ломается:

```text
сначала profiler
потом hot path refactor
потом только архитектурное усложнение
```

## Ближайший следующий шаг

Текущий кодовый шаг:

```text
organ128-modadd-eval
organ128-modadd-seed-sweep
```

Срез реализации v0:

```text
1. Сгенерировать deterministic dataset для `(a + b) mod P`.
2. Разделить train/holdout без пересечения пар.
3. Запустить random, Mono192, voting, WaveBus controls.
4. Найти key modes только на train.
5. Заморозить key modes.
6. Проверить restricted/excluded/key-ablation на holdout.
7. Запустить label_shuffle negative control.
8. Напечатать report artifact в stdout.
```

Что v0 не обязан делать:

```text
не обязан сохранять JSON
не обязан обучать полноценный Chat-0
не обязан менять Cell32 weights во время ответа
не обязан подтверждать русский текст
```

Что v0 обязан не делать:

```text
не вызывать `(a + b) mod P` внутри prediction path
не подбирать thresholds на holdout
не объявлять candidate как passed
не скрывать провал controls
```

Минимальная версия команды должна вывести:

```text
task
seed
modulus
train_size
holdout_size
elapsed_ms
random_accuracy
mono192_accuracy
fourier_phase_accuracy
cell32_phase_compose_accuracy
cell32_structured_compose_accuracy
cell32_fourier_census_accuracy
cell32_voting_accuracy
cell32_wavebus_accuracy
ensemble_gain
phase_compose_gain
structured_compose_gain
fourier_census_gain
wave_over_fourier_gap
compose_over_fourier_gap
structured_over_fourier_gap
census_over_fourier_gap
restricted_key_accuracy
excluded_key_accuracy
key_ablation_drop
non_key_ablation_drop
label_shuffle_accuracy
no_shortcut_control
scientific_pass
engineering_pass
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

Первый v0-прогон:

```text
cargo run -q -p nando-cli -- organ128-modadd-eval 7 31 256 256
mode_status: not_found_organ128_modadd
ensemble_gain: -0.003906
phase_compose_gain: -0.007812
structured_compose_gain: 0.000000
fourier_census_gain: -0.015625
fourier_phase_accuracy: 1.000000
cell32_phase_compose_accuracy: 0.035156
cell32_structured_compose_accuracy: 0.042969
cell32_fourier_census_accuracy: 0.027344
wave_over_fourier_gap: -0.960938
compose_over_fourier_gap: -0.964844
structured_over_fourier_gap: -0.957031
census_over_fourier_gap: -0.972656
key_ablation_drop: 0.011719
label_shuffle_accuracy: 0.031250
no_shortcut_control: true
```

Seed robustness command:

```text
cargo run -q -p nando-cli -- organ128-modadd-seed-sweep 31 256 256
```

Первый seed-sweep:

```text
mode_status: not_found_organ128_modadd_seed_sweep
passed_seed_pairs: 0
candidate_seed_pairs: 1
min_ensemble_gain: -0.011719
min_phase_compose_gain: -0.023438
min_structured_compose_gain: -0.023438
min_fourier_census_gain: -0.023438
min_wave_over_fourier_gap: -0.968750
min_compose_over_fourier_gap: -0.984375
min_structured_over_fourier_gap: -0.976562
min_census_over_fourier_gap: -0.972656
min_key_ablation_drop: 0.007812
max_label_shuffle_accuracy: 0.050781
```

Вывод:

```text
прибор собран, leakage control прошел,
но ансамблевая мода на полном v0 split не найдена.
Seed-sweep подтверждает: это не одиночная неудача, текущий readout/dynamics
не seed-robust. Fourier phase control дает accuracy 1.0, значит модульная
задача действительно решается фазовой композицией; провал находится не в
задаче, а в текущем способе, которым Organ128 превращает carrier/bus state в
ответ. Следующий шаг - улучшать wave/readout dynamics и способ записи
композиции в клетки, а не возвращаться к усилению Chat-0 или расширению
корпуса.
```

Проверенный вариант 1:

```text
cell32_phase_compose:
  a -> separate Cell32 trace
  b -> separate Cell32 trace
  compose center_phase(a) + center_phase(b)
  train only global phase offset on train split

result:
  cell32_phase_compose_accuracy: 0.035156
  phase_compose_gain: -0.007812
  min_phase_compose_gain over seeds: -0.023438
  min_compose_over_fourier_gap: -0.984375

decision:
  rejected as architectural direction v1.
```

Вывод по варианту 1:

```text
Раздельные trace для a и b сами по себе не несут линейную фазу, которую можно
просто сложить как Fourier phase. Значит следующая архитектурная попытка должна
не складывать готовые center_phase, а менять способ кодирования входных
компонент в CarrierWave/WaveBus так, чтобы фаза a и фаза b сохранялись как
компонуемые подпространства.
```

Проверенный вариант 2:

```text
cell32_structured_compose:
  a -> Cell32 trace with CarrierWave.phase = phase(a)
  b -> Cell32 trace with CarrierWave.phase = phase(b) + lane shift
  input byte only marks lane, not full value
  compose center_phase(a) + center_phase(b)
  train only global phase offset on train split

result:
  cell32_structured_compose_accuracy: 0.042969
  structured_compose_gain: 0.000000
  min_structured_compose_gain over seeds: -0.023438
  min_structured_over_fourier_gap: -0.976562

decision:
  rejected as architectural direction v2.
```

Вывод по варианту 2:

```text
Структурированная component carrier-фаза немного улучшила одиночный seed
относительно random-compose, но не дала преимущества над voting и не стала
seed-robust. Значит проблема глубже: текущий WaveBus center_phase считает
центр распределения активных slot weights, а не сохраняет carrier/input phase
как линейное компонуемое состояние. Следующая попытка должна менять сам
phase-census/readout: читать не один center_phase, а обучаемый Fourier census
по slots или отдельные sin/cos компоненты bus.
```

Проверенный вариант 3:

```text
cell32_fourier_census:
  expose full WaveBus.phase_sum from core
  each slot votes by slot phase and sign
  train per-slot modular offset on train split
  prediction is weighted circular vote by abs(phase_sum)

result:
  cell32_fourier_census_accuracy: 0.027344
  fourier_census_gain: -0.015625
  min_fourier_census_gain over seeds: -0.023438
  min_census_over_fourier_gap: -0.972656

decision:
  rejected as architectural direction v3.
```

Вывод по варианту 3:

```text
Наивный Fourier census по готовому bus spectrum не достает модульную
композицию. Это подтверждает, что текущий bus.phase_sum является спектром
активированных cell templates, а не устойчивым sin/cos-представлением входных
компонент. Следующая архитектурная попытка должна сохранять в bus отдельные
компонентные sin/cos каналы или обучать projection/readout по raw harmonic
features, а не использовать slot offset по готовому phase_sum.
```
