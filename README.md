# Nando Wave

> **Обязательный архитектурный канон для людей и ИИ:**
> [`ARCHITECTURE_CANON.md`](ARCHITECTURE_CANON.md). Прочитать до изменений
> miner/Wave/runtime/verifier/admission. Он фиксирует смысл L1/L2/L3,
> teacher/student границу, feedback через anti-centers и запреты, защищающие
> рабочее ядро от повторного разрушительного рефакторинга.

Nando Wave строится как **Attractor-to-VM Machine**: поток опыта формирует
динамическое операторное поле, целый операторный circuit грокается через
межплоскостную фазовую когерентность, а затем кристаллизуется в неизменяемый
пакет VM-bytecode для проверяемого CPU-исполнения. Новый навык должен становиться
данными и bytecode, а не новой веткой Rust-кода.

Авторитетные документы:

```text
ARCHITECTURE_CANON.md
  архитектурное ядро, терминология и запрещённые shortcut-ы

docs/CORE.md
  единая текущая карта ядра, владельцев истины, статуса и блокера

docs/CURRENT_CORE_DECISION.md
  действующее product-core решение и граница target architecture

docs/GOAL.md
  конечная цель и продуктовые критерии

plans/nando-attractor-to-vm-machine-v1/NANDO_ATTRACTOR_TO_VM_ROADMAP_V1.md
  текущая поэтапная программа реализации и gates

docs/README.md
  порядок авторитетности и навигация по доказательствам
```

Любой старый roadmap, эксперимент или отчёт, противоречащий этому маршруту,
является историческим материалом, а не текущим разрешением менять ядро.

Текущая граница:

```text
B1B controlled causal fixture        PASS
independent physical truth           BLOCK
AcceptedBindingLawEvidence           BLOCK
F4 Protocol Compiler                 BLOCKED
production authority                 false
```

Историческое поле `f4_status=UNLOCKED_NOT_STARTED` в машинном receipt
STOP-B1B сохраняется как часть того checkpoint, но не является текущим
разрешением начинать F4.

Исторические и поддерживающие документы:

```text
docs/NORTH_STAR.md   - северная звезда проекта
docs/GOAL.md         - конечная цель, gates и порядок движения
docs/INSPIRATION.md  - статья Nanda et al. как источник вдохновения
docs/ARCHITECTURE.md - черновая архитектура
docs/HYPOTHESES.md   - проверяемые гипотезы
docs/ROADMAP.md      - дорожная карта
docs/DETAILED_ROADMAP.md - детальная дорожная карта с gates
docs/RISKS.md        - тупики и защиты от самообмана
docs/PARKING_LOT.md  - идеи на потом, чтобы не расползаться
```

## Исторический журнал реализации

Всё ниже этого заголовка сохраняет эволюцию экспериментов и команд, но не
определяет текущую цель или roadmap. Текущий маршрут задан авторитетными
документами выше.

Этап 1 из `docs/DETAILED_ROADMAP.md` завершен: Rust workspace.

Этап 2 начат и реализует первый фиксированный wave tick без обучения.

Сейчас реализовано:

```text
Cargo workspace
crates/nando-core
crates/nando-cli
crates/nando-eval
nando-cli status
nando-cli organ128-plan
nando-cli organ128-train-generate
nando-cli organ128-dialog-generate
nando-cli organ128-settle-dialog
nando-cli organ128-response-gate-eval
nando-cli organ128-thought-probe-eval
nando-cli organ128-modadd-eval
nando-cli organ128-modadd-seed-sweep
nando-cli wave-tick
nando-cli snapshot-save
nando-cli snapshot-read
nando-cli eval-one-tick
nando-cli eval-periodic
nando-cli eval-phase-composition
nando-cli eval-phase-holdout
nando-cli eval-carrier-control
nando-cli eval-bus-transfer
nando-cli eval-snapshot-memory
nando-cli eval-snapshot-transition
nando-cli eval-snapshot-dynamics
nando-cli eval-snapshot-multitick
nando-cli eval-snapshot-adapt
nando-cli eval-snapshot-decoder
nando-cli eval-snapshot-keyed
nando-cli eval-snapshot-keyed-transition
nando-cli eval-snapshot-noisy-keyed-transition
nando-cli eval-snapshot-noisy-keyed-transition-sweep
nando-cli eval-snapshot-noisy-keyed-transition-seed-sweep
nando-cli eval-byte-context
nando-cli eval-byte-context-centroid
nando-cli eval-byte-context-offset-centroid
nando-cli eval-byte-context-denoised-centroid
nando-cli eval-byte-context-relative-centroid
nando-cli eval-byte-context-lexical-carrier-centroid
nando-cli eval-byte-context-cellular-carrier-centroid
nando-cli eval-byte-context-trained-carrier-centroid
nando-cli eval-byte-context-prompt-carrier-centroid
nando-cli eval-byte-context-prompt-carrier-diverse-centroid
nando-cli eval-byte-context-centroid-seed-sweep
nando-cli eval-byte-context-offset-centroid-seed-sweep
nando-cli eval-byte-context-denoised-centroid-seed-sweep
nando-cli eval-byte-context-relative-centroid-seed-sweep
nando-cli eval-byte-context-lexical-carrier-centroid-seed-sweep
nando-cli eval-byte-context-cellular-carrier-centroid-seed-sweep
nando-cli eval-byte-context-trained-carrier-centroid-seed-sweep
nando-cli eval-byte-context-prompt-carrier-centroid-seed-sweep
nando-cli eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep
nando-cli eval-byte-context-centroid-ablation
nando-cli eval-byte-context-cellular-carrier-ablation
nando-cli eval-byte-context-trained-carrier-ablation
nando-cli eval-byte-context-prompt-carrier-ablation
nando-cli eval-byte-context-prompt-carrier-diverse-ablation
nando-cli eval-chat0
nando-cli eval-settle-word
nando-cli eval-chat0-route
nando-cli eval-chat0-promote
nando-cli eval-chat0-promoted-holdout
nando-cli chat0-promote-save
nando-cli chat0-once
nando-cli chat0-once-promoted
nando-cli chat0-shell
nando-cli bench-stage2-tick
nando-cli live-byte-train
nando-cli live-byte-learn
nando-cli live-byte-holdout
nando-cli live-byte-holdout-suite
nando-cli live-byte-holdout-seed-sweep
nando-cli live-cell-promote
nando-cli live-architecture-compare
nando-cli live-tissue-diagnose
nando-cli live-grok-trace
nando-cli live-grok-sweep
nando-cli bench-link-tissue
nando-cli phase-package-v4
nando-cli phase-package-inspect
nando-cli phase-package-score-v4
nando-cli phase-eval-pack-v4
nando-cli phase-package-score-pack-v4
nando-cli phase-package-verify
nando-cli phase-action-boundary-v4
nando-cli phase-action-corpus-v1
nando-cli phase-action-domain-corpus-v1
nando-cli phase-action-coverage-corpus-v1
nando-cli phase-action-contract-v1
nando-cli phase-action-shortcut-v1
nando-cli phase-action-operator-coverage-v1
nando-cli phase-action-runtime-v1
nando-cli phase-action-package-v1
nando-cli phase-action-package-inspect-v1
nando-cli phase-action-source-verify-v1
nando-cli phase-action-package-score-v1
nando-cli phase-action-package-verify-v1
scripts/check-goal.sh
scripts/check.sh
scripts/check-push.sh
scripts/check-architecture.sh
Cell32 fixed 32 KB packet
Mono192 fixed 192 KB packet
CarrierWave
WaveBus
TickTrace
SpectrumSnapshot
stable `.nws1` snapshot roundtrip
first Stage 3 periodic baseline report
optimized Stage2Organ tick bench
LinkTissue inner-loop bench
primitive live feedback byte loop
tiny online byte learner
live byte learner holdout gate
live byte learner multi-corpus suite
live byte learner seed sweep
voting baseline
6-cell ablation sweep
phase-composition probe
phase-composition holdout report
CarrierWave control report
delayed bus-transfer report
snapshot-memory replay report
snapshot-transition report
snapshot-dynamics report
snapshot-multitick report
snapshot-adapt report
snapshot-decoder report
snapshot-keyed report
snapshot-keyed-transition report
snapshot-noisy-keyed-transition report
snapshot-noisy-keyed-transition-sweep report
snapshot-noisy-keyed-transition-seed-sweep report
byte-context report
byte-context-centroid report
byte-context-offset-centroid report
byte-context-denoised-centroid report
byte-context-relative-centroid report
byte-context-lexical-carrier-centroid report
byte-context-cellular-carrier-centroid report
byte-context-trained-carrier-centroid report
byte-context-prompt-carrier-centroid report
byte-context-prompt-carrier-diverse-centroid report
byte-context-centroid-seed-sweep report
byte-context-offset-centroid-seed-sweep report
byte-context-denoised-centroid-seed-sweep report
byte-context-relative-centroid-seed-sweep report
byte-context-lexical-carrier-centroid-seed-sweep report
byte-context-cellular-carrier-centroid-seed-sweep report
byte-context-trained-carrier-centroid-seed-sweep report
byte-context-prompt-carrier-centroid-seed-sweep report
byte-context-prompt-carrier-diverse-centroid-seed-sweep report
byte-context-centroid-ablation report
byte-context-cellular-carrier-ablation report
byte-context-trained-carrier-ablation report
byte-context-prompt-carrier-ablation report
byte-context-prompt-carrier-diverse-ablation report
chat-0 eval report
settle-word eval report
```

Открытая текстовая генерация и полноценный Chat-0 еще не реализованы. Первый
eval-controlled Chat-0 loop уже добавлен: он генерирует короткий ответ из
найденной prompt-cloud моды и пишет feedback-счетчик в отчет. Это намеренно
узкий gate, а не маленькая LLM.

В core добавлен первый live feedback loop:

```text
state -> tick -> primitive byte prediction -> feedback -> local runtime update -> snapshot
```

Это пока не обучение весов `Cell32`. Фиксированные клетки остаются стабильными.
Локальное изменение живет в runtime-состоянии `OrganState`: carrier/coupling.
Поверх wave-trace добавлен первый маленький обучаемый слой `LiveByteLearner`:
активные клетки голосуют в next-byte logits, а feedback меняет только локальный
адаптер `byte_bias + cell_byte_weights`.

Такой режим нужен как безопасный первый стенд перед настоящим обучением клеток.
Этап 2 проверяет фиксированную память, детерминированный tick, snapshot-файл и
первый one-tick report; Этапы 3-5 начали baseline/voting/ablation/snapshot-
сравнение на synthetic periodic task, первый byte-context bridge, короткий
Chat-0 output loop и primitive live byte feedback.

Проверка:

```bash
cd /home/ubu/projects/nando-wave
scripts/check.sh
```

Короткая проверка для обычной разработки. Она не гоняет тяжелые seed-sweep и
Chat-0 regression.

Goal-контракт:

```bash
cd /home/ubu/projects/nando-wave
scripts/check-goal.sh
```

Это быстрый документный gate: он проверяет, что `docs/GOAL.md`, `README.md` и
`docs/ROADMAP.md` не разъехались по северной звезде, anti-leakage правилам и
следующему `organ128-modadd-eval` шагу.

Полный pre-push regression:

```bash
cd /home/ubu/projects/nando-wave
scripts/check-push.sh
```

`scripts/check-push.sh` запускается только перед push/релизом или перед большим
архитектурным решением.

Архитектурный gate:

```bash
cd /home/ubu/projects/nando-wave
scripts/check-architecture.sh
```

Внутри короткого `scripts/check.sh` вызывается быстрый режим:

```bash
scripts/check-architecture.sh --contracts-only
```

Последняя проверка Этапа 2: `scripts/check.sh` OK, 2026-06-14.

Ручной статус:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- status
```

Cache-aware план первого L3-организма:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-plan
```

Текущий план под T480 / i7-8650U:

```text
Organ128 = 128 x Cell32 = 4 MiB
L1 active window: 4 Cell32
L2 hot window: 32 Cell32
L3 warm target: 128 Cell32
L3 warm max: 256 Cell32
RAM cold pool: >= 1024 Cell32

roles:
64 FastCell32
32 MidCell32
16 GuardCell32
8 CarrierCell32
8 MemoryCell32
```

Первое обучение и генерация `Organ128`:

```bash
cd /home/ubu/projects/nando-wave
cargo run --release -p nando-cli -- organ128-train-generate 7 64 "nando " 120
```

Базовый корпус теперь вынесен из кода:

```text
data/corpus/organ128_train_v1.txt
data/corpus/organ128_dialog_ru_en_v1.tsv
```

`organ128_train_v1.txt` - компактный структурированный корпус для byte-loop:
Cell32, Organ128, CarrierWave, WaveBus, settle loop, snapshot, feedback,
ablation, holdout и Rust/T480 runtime.

`organ128_dialog_ru_en_v1.tsv` - русско-английские prompt/answer пары для
dialog/settle слоя. Свободный byte-generator пока ASCII-first, поэтому русский
корпус сейчас в первую очередь усиливает prompt-wave/dialog-memory и
settle-dialog, а не полноценную свободную русскую генерацию.

Текущий baseline с prompt-wave:

```text
train_cases: 30720
train_accuracy_before_update: 0.2431
state_abs_mean: 0.001001
mode_status: organ128_generated_text_smoke
```

Prompt теперь переводится в компактное волновое состояние:

```text
prompt_wave_phase
prompt_wave_amplitude
prompt_wave_top_slots
```

Примеры разных prompt:

```text
nando wave -> cell t cell or gen1, says organ wr grows ...
rust cells -> lett t cells make says cells le make cell fast text ...
memory text -> from t cell o gene or cells alive small cells ...
```

Вывод: `Organ128` уже реально обучается и генерирует текстовый поток, но это
первый smoke baseline. Prompt-wave уже меняет режим генерации через фазу,
амплитуду и top-slots, но это еще не осмысленный чат. Следующий шаг -
обучать не только byte-readout, а локальное состояние клеток и контекст длиннее
одного байта.

Первый bilingual dialog-корпус:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-dialog-generate 7 "что такое nando"
```

Пример:

```text
prompt: что такое nando
matched_prompt: что такое nando
answer: nando это маленький волновой организм из клеток cell32.
mode_status: organ128_dialog_memory_answered
```

Похожие запросы тоже идут через prompt-wave + lexical overlap:

```text
prompt: расскажи про organ128
matched_prompt: что такое organ128
answer: organ128 это 128 клеток cell32 в кэшированном теле.
```

Это первый слой `prompt -> answer` на русском и английском. Пока это
dialog-memory поверх Organ128, а не полностью свободная генерация ответа.

Первый settle-dialog контур для Organ128:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-settle-dialog 7 "расскажи про что ты знаешь?" 5
```

Команда добавляет волновую середину между prompt и answer:

```text
PromptWave -> CarrierWave -> WaveBus settle ticks -> stable mode -> answer
```

В отчете печатаются `carrier_phase`, `center_phase`, `coherence`, `entropy`,
`phase_velocity`, `active_cells`, а также cache propagation:

```text
L3 warm organ -> L2 top32 hot pool -> L1 top4 active cells
```

Для каждого tick видны `l2_roles` и `l1_roles`: сколько Fast/Mid/Guard/Carrier/
Memory клеток прошло из L3 в горячий L2-пул и дальше в L1 active window. Также
печатаются control-режимы `no_carrier`, `wrong_carrier`,
`corrupted_prompt_wave`.

L1 active window в settle-dialog теперь квотированный:

```text
2 Fast + 1 Mid/Guard + 1 Carrier/Memory
```

Это нужно, чтобы медленная несущая/памятная мода реально попадала в WaveBus, а
не проигрывала raw-resonance быстрым клеткам каждый tick.

MemoryCell32 в settle-dialog получили первый runtime-state:

```text
8 memory slots
memory phase
memory strength
memory validation
memory pull into CarrierWave
memory update from WaveBus center/coherence
```

Это еще не долговременное обучение весов. Это рабочая runtime-память внутри
одного settle-прохода: память тянет carrier к сохраненному центру, обновляется
после WaveBus и печатается в trace как `memory phase/strength/validation`.
Validation сравнивает текущую рабочую wave с исходным prompt-якорем, поэтому
`corrupted_prompt_wave` должен терять доверие памяти.

Это еще не доказательство осмысленного чата. Это первый прибор, который
показывает, влияет ли несущая волна и settle-loop на выбор ответа, или
dialog-layer остается обычным matching.

Первый eval-gate для wave-state scorer:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-wave-scorer-eval 7 12 5
```

Он обучает маленький линейный scorer по признакам `settled state -> answer id`
на train split dialog-корпуса и проверяет holdout. Цель - постепенно перейти от
`answer_wave_sensitive` к `wave_dominant`, где ответ выбирается settled wave
state, а не lexical overlap.

Текущий ablation result:

```text
full:          0.454545
no_center:     0.272727
no_memory:     0.181818
no_prompt:     0.000000
no_global:     0.454545
no_validation: 0.454545
```

Вывод: scorer-кандидат реально использует memory и center признаки, но пока
сильно зависит от prompt-phase признаков. Global coherence/entropy/stability и
validation на этом split не доказали вклад в holdout accuracy.

Первый response-gate против ложной когерентности:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-response-gate-eval 7 12
```

Идея: одно `settled` состояние еще не значит, что ответ надежный. Система может
сойтись в ложный аттрактор. Поэтому gate проверяет две вещи:

```text
settle_verdict: settled / weak / oscillating / incoherent / rejected_by_memory
thought_verdict: coherent / unsettled / diffuse / detached
response_gate: answer / refuse_unstable_or_low_confidence
```

`ThoughtState` пока является диагностическим внутренним состоянием, а не
ручным правилом ответа. Он агрегирует trajectory settle-прохода:

```text
thought_phase
thought_strength
thought_convergence
thought_drift
thought_verdict
```

Это первый шаг к внутреннему циклу: prompt должен порождать не только внешний
matching, но и наблюдаемый аттрактор в `carrier/bus/memory` динамике.

Отрицательный результат: сырые `ThoughtState` признаки нельзя сразу добавлять в
dialog scorer. Проба `thought -> scorer features` ухудшила holdout:

```text
full with raw thought features: 0.181818
no_thought:                    0.454545
```

Вывод: `ThoughtState` пока остается прибором. Перед участием в ответе ему нужен
отдельный learning/ablation gate, иначе внутренний контур превращается в шумный
ручной readout.

Первый такой отдельный gate добавлен как эксперимент:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- organ128-thought-probe-eval 7 12 24
```

Он учит tiny perceptron только на динамических признаках `ThoughtState` и
settle-метриках, без lexical overlap и без candidate matching. Текущий результат
отрицательный:

```text
holdout_accuracy:    0.142857
known_answer_rate:   0.000000
refusal_refuse_rate: 0.666667
mode_status: not_found_organ128_thought_probe
```

Повторная проба с seed-augmentation и дополнительными динамическими признаками
`prompt_specificity`, `specificity`, `role_balance` не решила перенос:

```text
train_accuracy_before_update: 0.905172
holdout_accuracy:             0.214286
known_answer_rate:            0.000000
refusal_refuse_rate:          1.000000
mode_status: not_found_organ128_thought_probe
```

Вывод: текущий `ThoughtState` не переносится на holdout как самостоятельный
answer/refuse орган. Его нельзя промоутить в readout. Следующий путь - менять
динамику формирования thought, а не добавлять новые ручные признаки.

Текущий smoke-result:

```text
known_answer_rate:   1.000000
refusal_refuse_rate: 1.000000
mode_status: organ128_response_gate_candidate
```

Refusal-набор здесь не является случайным шумом. Это обрывки запросов, слишком
общие формулировки и keyword-cloud prompts, где у модели нет надежной
когерентной опоры для ответа. Этот gate нужен, чтобы отличать настоящий
settle от брожения или ложного аттрактора.

Для этого response-gate печатает `candidate_margin`: разницу между лучшим и
вторым dialog-кандидатом. Малый margin означает, что система видит несколько
почти одинаковых аттракторов и не имеет достаточно конкретной опоры.

Один детерминированный wave tick:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- wave-tick 42 7
```

Сохранить и прочитать snapshot:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- snapshot-save 42 7 target/snapshots/demo.nws1
cargo run -p nando-cli -- snapshot-read target/snapshots/demo.nws1
```

Первый one-tick eval report:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-one-tick 42 7
```

Замер горячего Stage-2 tick:

```bash
cd /home/ubu/projects/nando-wave
cargo run --release -p nando-cli -- bench-stage2-tick 7 100000
```

Замер внутренних циклов клетка/связи:

```bash
cd /home/ubu/projects/nando-wave
cargo run --release -p nando-cli -- bench-link-tissue 7 1000000
```

Этот bench запускается вручную. Быстрый `scripts/check.sh` проверяет контракты
и smoke-диагностику, но не тратит время на микробенчи. Полный
`scripts/check-push.sh` оставлен для pre-push/pre-release.

Минимальный live feedback loop:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-byte-train 7 "let value = value + 1;"
```

Первый обучаемый next-byte adapter:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-byte-learn 7 "let value = value + 1;"
```

Holdout-gate для проверки переноса:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-byte-holdout 7 "let value = value + 1; let value = value + 2;"
```

Этот gate делит поток пополам: первая половина обучает `LiveByteLearner`,
вторая половина проверяется без обновления весов. Это отличает replay-память от
минимального переноса на holdout.

Набор встроенных holdout-gates:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-byte-holdout-suite 7
```

Suite проверяет несколько маленьких корпусов: `repeat`, `code_like`, `ru_text`,
`mixed_balanced`, `mixed_shift`. Колонка `gap` показывает разницу между learner
holdout accuracy и простым `last_next_baseline`.

Seed-sweep для проверки устойчивости:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-byte-holdout-seed-sweep
```

Команда прогоняет suite на фиксированных seeds `[1, 7, 13, 29, 97]` и показывает
`wins`, средний holdout accuracy, baseline, средний gap, худший gap и `OOS`.
`OOS` - доля target-байтов holdout, которых не было в train.

Первый eval-gated Cell32 candidate:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-cell-promote 7 "let value = value + 1; let value = value + 2; let value = value + 3;"
```

Эта команда явно разделяет:

```text
candidate update -> holdout eval -> promoted/rejected
```

`Cell32` packets не мутируются. Обновляется только маленькое candidate-состояние
`Cell32Learner`; promotion разрешается только при положительном holdout gap и
не слишком высоком `OOS`.

Архитектурное сравнение клеток против монолита:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-architecture-compare 7
```

Текущий честный результат: отдельный `live-cell-promote` может пройти против
простого baseline, но `3 x Cell32` против `mono96` и `6 x Cell32` против
`mono192` пока не доказывают преимущество клеточной топологии. Это правильный
gate перед развитием: клеточную архитектуру нельзя объявлять найденной модой,
пока она не выигрывает у монолитного контроля.

Первый тканевый слой:

```text
LinkTissue(pair)   - признаки взаимодействия A x B
LinkTissue(triple) - признаки взаимодействия A x B x C
```

Текущий результат `live-architecture-compare 7`:

```text
pair_tissue_wins_over_cell6: 2
triple_tissue_wins_over_pair: 0
mode_status: not_found_cellular_topology_advantage
```

Вывод: pair-ткань уже дает локальный сигнал на части корпусов, но triple-ткань
пока не добавляет новой устойчивой моды сверх pair. Идея тканевого слоя остается
главным направлением, но promotion требует holdout/synergy, а не красивой
аналогии.

Диагностика ткани:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-tissue-diagnose 7
```

Эта команда показывает:

```text
cell6
pair
typed2
typed3
best_pair
pair_drop
best_triple
```

Текущий результат:

```text
typed_pair_wins_over_cell6: 2
typed_triple_wins_over_typed_pair: 0
positive_pair_ablation_cases: 2
mode_status: pair_ablation_signal_needs_typed_gain
```

Вывод: all-pair ткань уже полезна на `code_like` и `ru_text`; ablation показывает
живую пару `0-3` на этих корпусах. Typed-pair теперь включает локальные
same-layer связи и межслойные связи, поэтому догнал all-pair на этих корпусах.
Typed-triple пока не дает прироста сверх pair. Следующий фокус - искать задачу
или update-rule, где тройка нужна реально, а не слепо усиливать triple.

Текущий microbench `bench-link-tissue 7 1000000` в release:

```text
cell3_score.ns_per_tick: 12.9
cell6_score.ns_per_tick: 15.4
pair_score.ns_per_tick: 45.4
typed_pair_score.ns_per_tick: 41.9
triple_score.ns_per_tick: 72.3
typed_triple_score.ns_per_tick: 64.1
typed_pair_vs_pair: 1.082x
typed_triple_vs_triple: 1.128x
```

Typed-топология использует precomputed masks, поэтому профиль связей не
вычисляется внутри горячего score/update цикла.

Ранние признаки grokking:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-grok-trace 7 16 4
```

`live-grok-trace` не ждёт финальной accuracy. Он по эпохам смотрит progress
measures в духе Nanda et al.:

```text
cell6  - одиночные клетки без ткани
full   - cell6 + вся typed tissue
restr  - cell6 + только top-pair tissue
excl   - cell6 + tissue без top-pair
drop   - full - excl, цена удаления top-pair
p_gini - концентрация энергии по pair-связям
signal - none / warmup / circuit_seed / grok_candidate
stable_status - строгий итог по всей трассе
```

`grok_candidate` теперь требует одновременно:

```text
full > cell6
restr > cell6
drop > 0
p_gini >= 0.55
```

`stable_status=stable_grok_candidate` требует еще жестче:

```text
grok_candidate_count >= 3
best_same_pair_streak >= 3
best_full_gain > 0
best_restricted_gain > 0
best_excluded_drop > 0
```

Текущий строгий trace на `code_like`, seed 17, 128 эпох:

```text
circuit_seed_count: 9
grok_candidate_count: 0
best_same_pair_streak: 17
best_full_gain: +0.0588
best_restricted_gain: +0.0294
best_excluded_drop: +0.0588
stable_status: unstable_circuit_seed
```

Sweep по seed `1 3 5 7 11 13 17 19 23 29` на 64 эпохах пока не нашел
`stable_grok_candidate`. Вывод: ранние контуры уже видны, но стабильное
озарение текущим update-rule пока не доказано.

Сравнение update-rule:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- live-grok-sweep 64 8
```

Sweep сравнивает:

```text
perceptron - старый update от ошибки combined prediction
decay      - cleanup/weight-decay перед tissue update
margin     - update, если правильный байт не обгоняет prediction с запасом
boost      - decay + основной update + усиление текущей top-pair связи
```

`LinkTissue` теперь имеет два core-примитива для cleanup/grokking-экспериментов:

```text
apply_decay(retention)
update_pair_from_prediction(...)
```

Текущий sweep `64/8`:

```text
stable_grok_candidate_count: 0
unstable_circuit_seed_count: 7
```

Вывод: cleanup и четыре update-rule реализованы, но устойчивое озарение пока не
появилось. Следующий научный фокус - не новые метрики, а более правильная
задача/датасет или более сильное правило закрепления circuit.

Первый periodic baseline eval:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-periodic 7 64 11 17
```

Отчет уже включает `random`, `Mono192`, `cell32_no_bus`, `cell32_voting`,
`cell32_wave_bus` и ablation по 6 клеткам. Если мода не найдена, это
отображается как `mode_status: not_found_stage_3_ablation`.

Phase-composition probe:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-phase-composition 13 64 19 23 5
```

Это не финальное доказательство, а задача, где target явно требует сочетать
input phase и CarrierWave phase. Статус `candidate_needs_holdout` означает
только кандидата, а не найденную моду.

Phase-composition holdout:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-phase-holdout 13 97 64
```

Если holdout проходит, статус поднимается только до
`candidate_holdout_passed_needs_carrier_test`. Это все еще не `mode_found`:
следующий обязательный контроль - проверить роль CarrierWave.

CarrierWave control:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-carrier-control 13 97 64
```

Этот gate сравнивает правильную, выключенную, чужую и поврежденную несущую.
Текущий результат на 128 случаях:

```text
correct_carrier_wave.accuracy: 0.062500
no_carrier_wave.accuracy: 0.000000
wrong_carrier_wave.accuracy: 0.007812
corrupted_carrier_wave.accuracy: 0.000000
mode_status: carrier_control_passed_candidate_mode
```

Это усиливает кандидата, но еще не является финальным доказательством Nando
Wave: phase-composition probe остается искусственной задачей, где target явно
зависит от CarrierWave phase. Следующий gate должен уменьшить встроенную
подсказку и проверить перенос на более независимой задаче.

Delayed bus-transfer:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-bus-transfer 13 97 64
```

Этот gate строже: decoder не видит `CarrierWave phase`, а использует только
`WaveBus center_phase` текущего тика, чтобы предсказать следующий wave target.
Текущий результат:

```text
correct_carrier_bus.accuracy: 0.007812
wrong_carrier_bus.accuracy: 0.000000
correct_over_best_baseline: 0.000000
correct_over_wrong_carrier: 0.007812
mode_status: not_found_bus_transfer
```

Вывод: текущий one-tick организм умеет показать carrier-dependent state, но
пока не умеет переносить это состояние на следующий шаг лучше baseline. Это
переносит следующий обязательный фокус на snapshot/warm-state или другую
переходную динамику.

Snapshot-memory replay:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-memory 13 97 64
```

Этот gate проверяет, что короткий serialized snapshot реально хранит состояние
WaveBus и проходит путь `to_bytes -> from_bytes` перед использованием.
Текущий результат:

```text
snapshot_bytes: 148
warm_snapshot.accuracy: 1.000000
wrong_snapshot.accuracy: 0.015625
corrupted_snapshot.accuracy: 0.000000
warm_over_no_snapshot: 1.000000
warm_over_wrong_snapshot: 0.984375
mode_status: snapshot_memory_passed_state_replay
```

Ограничение: это replay текущего состояния, а не доказательство переходной
динамики. Следующий gate должен проверить, помогает ли warm snapshot именно
предсказывать или восстанавливать следующий шаг после паузы/повреждения.

Snapshot-transition:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-transition 13 97 64
```

Этот gate использует previous snapshot как носитель offset между
`CarrierWave phase` и `WaveBus center_phase`, затем пытается предсказать
следующий wave-state без запуска следующего WaveBus в predictor.
Текущий результат:

```text
no_snapshot_transition.accuracy: 0.007812
warm_snapshot_transition.accuracy: 0.000000
wrong_snapshot_transition.accuracy: 0.000000
corrupted_snapshot_transition.accuracy: 0.000000
warm_over_no_snapshot: -0.007812
warm_over_wrong_snapshot: 0.000000
mode_status: not_found_snapshot_transition
```

Вывод: текущий snapshot хранит состояние для replay, но простой offset-переход
не дает рабочую transition-memory. Нужна явная переходная динамика или
локальная адаптация между тиками.

Snapshot-dynamics:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-dynamics 13 97 64
```

Этот gate использует smooth `CarrierWave::advance(...)` и проверяет, помогает
ли snapshot предсказывать следующий wave-state по фазовой ошибке.
Текущий результат:

```text
warm_snapshot_dynamics.accuracy: 0.007812
warm_snapshot_dynamics.mean_circular_error: 58.039062
no_snapshot_dynamics.mean_circular_error: 64.023438
wrong_snapshot_dynamics.mean_circular_error: 60.757812
corrupted_snapshot_dynamics.mean_circular_error: 64.062500
warm_error_gain_over_no: 5.984375
warm_error_gain_over_wrong: 2.718750
mode_status: snapshot_dynamics_passed
```

Ограничение: это один переход. Следующий контроль должен проверить, держится
ли старый snapshot несколько smooth-ticks подряд.

Snapshot-multitick:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-multitick 13 97 64
```

Этот gate берет snapshot в момент `t`, двигает `CarrierWave` на 4 smooth-ticks
и только потом проверяет будущий wave-state.
Текущий результат:

```text
horizon: 4
warm_snapshot_multitick.accuracy: 0.007812
warm_snapshot_multitick.mean_circular_error: 61.820312
no_snapshot_multitick.mean_circular_error: 63.304688
wrong_snapshot_multitick.mean_circular_error: 62.617188
corrupted_snapshot_multitick.mean_circular_error: 64.195312
warm_error_gain_over_no: 1.484375
warm_error_gain_over_wrong: 0.796875
mode_status: snapshot_multitick_passed
```

Вывод: warm snapshot уже ведет себя как слабая динамическая память на несколько
тиков по фазовой ошибке. Но exact accuracy остается почти нулевой, значит до
Chat-0 нужен следующий слой: локальная адаптация или более сильный transition
decoder.

Snapshot-adapt:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-adapt 13 97 64
```

Этот gate добавляет маленькую онлайн-подстройку фазы после feedback и
сравнивает ее с таким же feedback без snapshot. Это защита от ложного вывода,
что память помогла, если на самом деле помог простой bias.
Текущий результат:

```text
warm_snapshot_no_adapt.mean_circular_error: 61.820312
adapted_snapshot.mean_circular_error: 59.710938
adapted_no_snapshot.mean_circular_error: 53.406250
adapted_wrong_snapshot.mean_circular_error: 62.914062
adapted_error_gain_over_warm: 2.109375
adapted_error_gain_over_no_adapt: -6.304688
adapted_error_gain_over_wrong_adapt: 3.203125
mode_status: not_found_snapshot_adapt
```

Вывод: простая phase-correction улучшает warm snapshot, но no-snapshot
адаптация пока лучше. Значит этот gate не доказывает клеточную память; он
показывает, что нужен transition decoder или адаптация внутри клеток, а не
только глобальный фазовый bias.

Snapshot-decoder:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-decoder 13 97 64
```

Этот gate добавляет tiny online transition decoder с 4 признаками snapshot и
сравнивает его с decoder без snapshot.
Текущий результат:

```text
warm_snapshot_decoder_control.mean_circular_error: 61.820312
decoder_snapshot.mean_circular_error: 56.781250
decoder_no_snapshot.mean_circular_error: 53.320312
decoder_wrong_snapshot.mean_circular_error: 60.828125
decoder_error_gain_over_warm: 5.039062
decoder_error_gain_over_no_decoder: -3.460938
decoder_error_gain_over_wrong_decoder: 4.046875
mode_status: not_found_snapshot_decoder
```

Вывод: decoder использует сигнал snapshot лучше, чем warm-переход и wrong
snapshot, но no-snapshot decoder все еще сильнее. Значит текущая synthetic
задача допускает объяснение без клеточной памяти. Следующий шаг - задача или
decoder, где контроль без snapshot не может вынести основной эффект.

Snapshot-keyed:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-keyed 13 97 64
```

Этот gate проверяет snapshot-private state: target несет небольшой фазовый
ключ из `SpectrumSnapshot`, которого нет у no-snapshot контроля.
Текущий результат:

```text
no_snapshot_keyed.accuracy: 0.000000
keyed_snapshot.accuracy: 1.000000
wrong_snapshot_keyed.accuracy: 0.000000
corrupted_snapshot_keyed.accuracy: 0.000000
keyed_over_no_snapshot: 1.000000
keyed_over_wrong_snapshot: 1.000000
keyed_error_gain_over_no: 69.078125
mode_status: snapshot_keyed_passed
```

Вывод: serialized snapshot действительно может нести скрытое состояние,
которое no-snapshot, wrong snapshot и corrupted snapshot не восстанавливают.
Ограничение: это приборный snapshot-keyed test, а не финальное доказательство
ансамблевой моды или Chat-0.

Snapshot-keyed-transition:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-keyed-transition 13 97 64
```

Этот gate смешивает future `WaveBus center_phase` и snapshot-private state.
То есть snapshot уже не является ответом сам по себе, а участвует в
переходном вычислении.
Текущий результат:

```text
future_only_transition.accuracy: 0.000000
keyed_transition.accuracy: 1.000000
wrong_snapshot_keyed_transition.accuracy: 0.062500
corrupted_snapshot_keyed_transition.accuracy: 0.015625
keyed_over_future_only: 1.000000
keyed_over_wrong_snapshot: 0.937500
keyed_error_gain_over_future_only: 15.312500
mode_status: snapshot_keyed_transition_passed
```

Вывод: snapshot-private state может участвовать в future transition и проходит
future-only, wrong и corrupted контроли. Ограничение остается: это synthetic
keyed-transition, а не финальное доказательство ансамблевой моды.

Snapshot-noisy-keyed-transition:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition 13 97 64
```

Этот gate добавляет скрытую модуляцию из snapshot top-slots. Snapshot-ветка
видит только грубый private phase, поэтому результат уже не идеальный.
Текущий результат:

```text
future_only_noisy_transition.accuracy: 0.007812
keyed_noisy_transition.accuracy: 0.664062
wrong_snapshot_noisy_transition.accuracy: 0.046875
corrupted_snapshot_noisy_transition.accuracy: 0.007812
keyed_accuracy_over_future_only: 0.656250
keyed_error_gain_over_future_only: 14.781250
keyed_error_gain_over_wrong_snapshot: 5.039062
mode_status: snapshot_noisy_keyed_transition_passed
```

Вывод: snapshot-private state помогает в noisy transition без прямого 100%
копирования ответа. Это пока synthetic, но уже ближе к нормальному прибору:
результат частичный, контролируемый и хуже идеального keyed gate.

Snapshot-noisy-keyed-transition sweep:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition-sweep 13 97 64
```

Этот gate гоняет noisy transition на horizons `1, 2, 4, 8`.
Текущий результат:

```text
horizon_1.keyed_accuracy: 0.609375
horizon_2.keyed_accuracy: 0.617188
horizon_4.keyed_accuracy: 0.664062
horizon_8.keyed_accuracy: 0.664062
passed_count: 4
min_keyed_accuracy_over_future_only: 0.601562
min_error_gain_over_future_only: 13.835938
min_error_gain_over_wrong_snapshot: 4.515625
mode_status: snapshot_noisy_keyed_transition_sweep_passed
```

Вывод: noisy snapshot-преимущество держится не на одном выбранном горизонте,
а на нескольких расстояниях. Это все еще synthetic, но уже не
single-horizon coincidence.

Snapshot-noisy-keyed-transition seed sweep:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-snapshot-noisy-keyed-transition-seed-sweep 64
```

Этот gate гоняет noisy horizon sweep на 4 фиксированных seed-парах.
Текущий результат:

```text
passed_seed_pairs: 4
min_keyed_accuracy_over_future_only: 0.562500
min_error_gain_over_future_only: 13.367188
min_error_gain_over_wrong_snapshot: 4.515625
mode_status: snapshot_noisy_keyed_transition_seed_sweep_passed
```

Вывод: noisy snapshot-преимущество держится на нескольких seed-парах и
horizons. Это всё еще synthetic, но уже не single-seed coincidence.

Byte-context bridge:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context 13 97 64
```

Этот gate тренирует маленькие phase-decoders на byte prompt -> answer-byte и
проверяет holdout против `mono192_prompt_decoder`, `no_snapshot_decoder`,
`cell32_voting`, wrong snapshot и corrupted snapshot.

Текущий результат:

```text
snapshot_decoder.accuracy: 0.031250
no_snapshot_decoder.accuracy: 0.046875
mono192_prompt_decoder.accuracy: 0.000000
snapshot_accuracy_over_best_control: -0.015625
snapshot_error_gain_over_best_control: -1.171875
snapshot_error_gain_over_wrong_snapshot: 1.859375
mode_status: not_found_byte_context
```

Вывод: первый byte-level мост к Chat-0 честно не прошел. Snapshot лучше wrong
snapshot по phase-error, но пока хуже лучшего контроля. Следующий шаг - искать
byte-context моду, а не объявлять Chat-0 доказанным.

Byte-context centroid bridge:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-centroid 13 97 64
```

Этот gate тренирует frozen prototype/centroid classifiers на train split и
проверяет holdout. Это проверяет, образует ли snapshot переносимые кластеры
byte-context состояния.

Текущий результат:

```text
snapshot_centroid.accuracy: 0.359375
mono192_prompt_centroid.accuracy: 0.062500
no_snapshot_centroid.accuracy: 0.046875
wrong_snapshot_centroid.accuracy: 0.140625
corrupted_snapshot_centroid.accuracy: 0.140625
snapshot_accuracy_over_best_control: 0.296875
snapshot_error_gain_over_best_control: 3.140625
snapshot_error_gain_over_wrong_snapshot: 2.375000
mode_status: byte_context_centroid_candidate_needs_seed_sweep
```

Вывод: появился первый byte-level candidate gate. Это еще не доказательство
Chat-0, потому что нужны seed sweep и ablation, но это уже не чистый synthetic
transition.

Byte-context centroid seed sweep:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-centroid-seed-sweep 64
```

Текущий результат:

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_centroid_seed_sweep
```

Вывод: byte-context-centroid пока не robust. На 2 seed-парах snapshot
побеждает controls, на 2 seed-парах проигрывает. Это полезный результат:
первая byte-level мода не доказана, следующий шаг - искать более устойчивые
features/ablation, а не строить Chat-0 поверх случайного candidate.

Byte-context centroid ablation:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-centroid-ablation 13 97 64
```

Текущий результат на удачной seed-паре:

```text
snapshot_centroid_full.accuracy: 0.359375
ablate_snapshot_offset.accuracy: 0.140625
ablate_snapshot_top_sin.accuracy: 0.218750
key_feature: ablate_snapshot_offset
max_accuracy_drop: 0.218750
max_error_increase: 5.031250
mode_status: byte_context_centroid_ablation_sensitive
```

Контроль на провалившей seed-паре `29 -> 131`:

```text
snapshot_centroid_full.accuracy: 0.125000
ablate_snapshot_top_cos.accuracy: 0.234375
max_accuracy_drop: 0.000000
mode_status: not_found_byte_context_centroid_ablation
```

Вывод: локальный byte-context signal в удачной паре держится в основном на
`snapshot_offset`, но на провалившей паре этот signal не формируется, а часть
top-phase признаков ведет себя как шум.

Offset-only centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-offset-centroid-seed-sweep 64
```

Текущий результат:

```text
passed_seed_pairs: 1
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -5.437500
min_error_gain_over_wrong_snapshot: -4.437500
mode_status: not_found_byte_context_offset_centroid_seed_sweep
```

Вывод: `snapshot_offset` сам по себе не стабилизирует byte-context моду.
На удачной паре он дает candidate, но seed sweep становится хуже: 1/4 вместо
2/4 у полного centroid. Значит нужна не грубая вырезка top-phase, а более
устойчивая композиция признаков.

Denoised centroid (`offset + top_sin`):

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-denoised-centroid-seed-sweep 64
```

Текущий результат:

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_denoised_centroid_seed_sweep
```

Вывод: `offset + top_sin` убирает часть шума `top_cos` и на удачной паре
сохраняет full-centroid accuracy `0.359375`, но robustness не закрывает:
остается 2/4 seed-пары. Это лучше offset-only, но еще не доказанная мода.

Relative centroid (seed-normalized phase relations):

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-relative-centroid-seed-sweep 64
```

Текущий результат:

```text
passed_seed_pairs: 0
min_snapshot_accuracy_over_best_control: -0.203125
min_error_gain_over_best_control: -5.140625
min_error_gain_over_wrong_snapshot: -4.625000
mode_status: not_found_byte_context_relative_centroid_seed_sweep
```

Вывод: простая seed-normalization через относительные фазы не сработала и
хуже полного/denoised centroid. Полезный сигнал текущего snapshot сидит не в
простом относительном аккорде; нужно менять формирование состояния, а не только
систему координат feature-вектора.

Lexical CarrierWave centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-lexical-carrier-centroid-seed-sweep 64
```

Текущий результат:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.796875
min_error_gain_over_best_control: 4.937500
min_error_gain_over_wrong_snapshot: 4.796875
mode_status: byte_context_lexical_carrier_centroid_seed_sweep_passed
```

Вывод: если `CarrierWave` получает устойчивый lexical lock из общего
содержания prompt, `SpectrumSnapshot` становится переносимой byte-context
памятью: train `user:... -> bot:` переносится на holdout `cmd ... answer:`.
Это не финальная ансамблевая мода, потому что lexical lock пока задан
лабораторно. Но это важный положительный gate: проблема предыдущих
byte-context провалов находится в формировании состояния, а не в самом
snapshot/centroid механизме.

Cellular CarrierWave centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-cellular-carrier-centroid-seed-sweep 64
cargo run -p nando-cli -- eval-byte-context-cellular-carrier-ablation 13 97 128
```

Текущий результат:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.796875
min_error_gain_over_best_control: 4.937500
min_error_gain_over_wrong_snapshot: 4.796875
mode_status: byte_context_cellular_carrier_centroid_seed_sweep_passed

min_accuracy_drop: 0.101562
max_error_increase: 1.375000
mode_status: byte_context_cellular_carrier_ablation_sensitive
```

Вывод: ручной lexical lock заменен на маленький task-cell resonance слой:
8 lock-клеток соревнуются за prompt, победившая клетка задает фазу
`CarrierWave`. Seed sweep проходит 4/4, а отключение lock-клеток в ablation
роняет качество. Это ближе к ансамблевой моде, но еще не финальный proof:
lock-клетки пока детерминированные lexical detectors, а не обученные
гармонические клетки.

Trained CarrierWave centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-trained-carrier-centroid-seed-sweep 64
cargo run -p nando-cli -- eval-byte-context-trained-carrier-ablation 13 97 128
```

Текущий результат:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.796875
min_error_gain_over_best_control: 4.937500
min_error_gain_over_wrong_snapshot: 4.796875
mode_status: byte_context_trained_carrier_centroid_seed_sweep_passed

min_accuracy_drop: 0.101562
max_error_increase: 1.328125
mode_status: byte_context_trained_carrier_ablation_sensitive
```

Вывод: добавлен первый supervised harmonic lock bank. 8 обучаемых lock-клеток
накапливают прототипы prompt-сигнала на train split, затем задают фазу
`CarrierWave` на holdout. Это уже не прямой `contains(task) -> phase`, но еще
не финальный proof: текущий gate использует структурную подсказку
middle-token. Следующий критерий - trained lock без такой подсказки и с более
богатым prompt-сигналом.

Prompt-cloud CarrierWave centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-prompt-carrier-centroid-seed-sweep 64
cargo run -p nando-cli -- eval-byte-context-prompt-carrier-ablation 13 97 128
```

Текущий результат:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.406250
min_error_gain_over_best_control: 3.625000
min_error_gain_over_wrong_snapshot: 4.015625
mode_status: byte_context_prompt_carrier_centroid_seed_sweep_passed

min_accuracy_drop: -0.015625
max_accuracy_drop: 0.132812
accuracy_over_all_disabled: 0.492188
error_gain_over_all_disabled: 4.835938
mode_status: byte_context_prompt_carrier_bank_ablation_sensitive
```

Вывод: middle-token подсказка убрана. Prompt-cloud bank учится на всем prompt
как на гармоническом облаке и переносится на holdout. Отключение всего
обученного bank рушит качество, но отключение отдельных клеток не всегда
вредит: `min_accuracy_drop` отрицательный. Это похоже на избыточную
распределенную моду, а не на цепочку, где каждая клетка обязана быть
одиночно незаменимой.

Prompt-cloud diverse CarrierWave centroid:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep 64
cargo run -p nando-cli -- eval-byte-context-prompt-carrier-diverse-ablation 13 97 128
```

Этот gate использует разные train/holdout prompt-шаблоны и расширенный
гармонический lock-bank: 32 признака, где токенная половина добавляет
устойчивость к форме prompt, но имеет пониженный вес. Это не словарь команд:
bank все еще учится centroid-прототипам prompt-сигнала.

Текущий результат:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.250000
min_error_gain_over_best_control: 1.046875
min_error_gain_over_wrong_snapshot: 0.812500
mode_status: byte_context_prompt_carrier_diverse_centroid_seed_sweep_passed

snapshot_prompt_carrier_diverse_full.accuracy: 0.570312
ablate_prompt_diverse_lock_all.accuracy: 0.070312
max_accuracy_drop: 0.085938
accuracy_over_all_disabled: 0.500000
error_gain_over_all_disabled: 2.703125
mode_status: byte_context_prompt_carrier_diverse_bank_ablation_sensitive
```

Вывод: prompt-cloud lock переживает разнообразие шаблонов и остается лучше
`Mono192`, voting, random, wrong snapshot и corrupted snapshot. Это сильнее
предыдущего prompt-cloud gate, но еще не финальный Chat-0: дальше нужен
маленький output loop и feedback/eval-контур.

Chat-0 eval:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-chat0 13 97 128
```

Этот gate берет diverse prompt-cloud lock, восстанавливает mode label через
`SpectrumSnapshot`, генерирует короткий ответ и считает feedback-события для
ошибок. Feedback пока не меняет веса автоматически; это журнал для
контролируемой консолидации через eval.

Текущий результат:

```text
random_chat0.exact_accuracy: 0.109375
mono192_prompt_chat0.exact_accuracy: 0.132812
wrong_snapshot_chat0.exact_accuracy: 0.132812
corrupted_snapshot_chat0.exact_accuracy: 0.046875
prompt_cloud_snapshot_chat0.exact_accuracy: 0.570312
feedback_log_entries: 55
prompt_cloud_over_best_control: 0.437500
prompt_cloud_over_wrong_snapshot: 0.437500
mode_status: chat0_prompt_cloud_loop_passed
```

Вывод: появился первый узкий Chat-0 loop: input -> CarrierWave lock ->
snapshot -> short response -> feedback log -> eval. Это еще не интерактивный
бот и не финальная цель, но инженерная петля уже существует и проверяется.

Settle-word eval:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-settle-word 13 97 128
```

Этот gate проверяет первый вариант рождения короткого слова через multi-tick
settle trace. Readout обучается на train split, но holdout сравнивает
`Organ192` после 1/3/5/8 тиков с `Mono192`, voting, no-carrier, wrong-carrier,
corrupted-carrier и ablation по клеткам.

Текущий результат:

```text
settle_word_random.exact_accuracy: 0.117188
settle_word_mono192.exact_accuracy: 0.179688
settle_word_voting.exact_accuracy: 0.078125
settle_word_organ_one_tick.exact_accuracy: 0.078125
settle_word_organ_settle3.exact_accuracy: 0.000000
settle_word_organ_settle5.exact_accuracy: 0.125000
settle_word_organ_settle8.exact_accuracy: 0.078125
settle_word_organ_stable.exact_accuracy: 0.070312
settle_word_organ_gated.exact_accuracy: 0.328125
settle_word_no_carrier_settle5.exact_accuracy: 0.000000
settle_word_wrong_carrier_settle5.exact_accuracy: 0.000000
settle_word_corrupted_carrier_settle5.exact_accuracy: 0.000000
settle5_mean_coherence_gain: -0.012189
settle5_mean_entropy_drop: 0.000297
settle5_mean_phase_velocity: 0.211230
settle5_over_best_control: -0.054688
settle5_over_voting: 0.046875
settle5_ablation_max_drop: 0.000000
stable_over_best_control: -0.109375
stable_ablation_max_drop: 0.070312
gated_mean_selected_ticks: 7.000000
gated_over_best_control: 0.148438
gated_ablation_max_drop: 0.328125
carrier_integrity_gap: 0.328125
carrier_guard_rejections: 384
mode_status: settle_word_gated_candidate_needs_seed_sweep
```

Вывод: после фикса постоянного seed внутри `OrganState` gated-settle впервые
превысил `Mono192` на holdout. Carrier integrity guard отсекает
no/wrong/corrupted carrier, `carrier_integrity_gap` положительный, ablation
сильный. Это еще не финальное доказательство: статус требует seed sweep.
Следующий шаг - проверить candidate на нескольких seed-парах.

Seed sweep на 128 cases:

```text
passed_seed_pairs: 3
min_gated_over_best_control: 0.000000
min_gated_ablation_max_drop: 0.156250
min_carrier_integrity_gap: 0.164062
mode_status: settle_word_gated_seed_sweep_partial_3_of_4
```

Вывод: candidate не является одиночной вспышкой, но полный proof еще не
получен. Одна seed-пара дала tie с `Mono192`, поэтому следующий gate - убрать
tie через более устойчивый stop-condition.
settle loop и OrganState-связи, а не объявлять Chat-0 доказанным.

Chat-0 one-shot:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- chat0-once "cmd ping #1 answer: " pong target/chat0-traces/check.trace
```

Команда делает один короткий ответ, пишет line-oriented trace и, если указан
ожидаемый ответ, добавляет запись в `target/chat0-feedback/chat0-feedback.log`.
Текущий one-shot route честно выводится в trace: для свободного ручного prompt
ответ может идти через `prompt_cloud_lock_bank`, а snapshot/coherence остаются
наблюдаемыми признаками. Это практический CLI-контур для ручного теста, но не
замена `eval-chat0` и не доказательство открытого chatbot.

Chat-0 route eval:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-chat0-route 13 97 128
```

Этот gate проверяет ручные/free prompt templates отдельно от `eval-chat0`.
Текущий результат:

```text
snapshot_classifier_chat0_route.exact_accuracy: 0.851562
prompt_cloud_lock_bank_chat0_route.exact_accuracy: 0.851562
hybrid_chat0_route.exact_accuracy: 0.851562
lock_bank_route_count: 128
feedback_log_entries: 19
lock_bank_over_snapshot: 0.000000
hybrid_over_best_control: 0.742188
mode_status: chat0_route_usable_snapshot_tied_or_better
```

Вывод: ручной route стал пригодным для Chat-0 one-shot и сильно лучше
random/Mono192 controls. Но превосходство lock-bank над snapshot здесь не
доказано: они связаны. Это инженерный gate, а не новое доказательство
ансамблевой моды.

Chat-0 shell:

```bash
cd /home/ubu/projects/nando-wave
printf 'manual:2: ping? || pong\n:quit\n' \
  | cargo run -p nando-cli -- chat0-shell target/chat0-traces/shell-check target/chat0-feedback/chat0-shell-check.log
```

Формат строки:

```text
<prompt> || <expected>
```

Если `expected` указан, feedback пишется в log. Каждый ответ получает отдельный
trace-файл вида `chat0-0001.trace`. Shell не меняет веса; он только отвечает,
пишет trace и собирает feedback для будущего eval/promote.

Chat-0 promote eval:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-chat0-promote target/chat0-feedback/chat0-promote-check.log 13 97 128
```

Этот gate читает feedback log, сравнивает текущий replay ответа с точечным
feedback-replay кандидатом и параллельно запускает `eval-chat0-route` как guard.
Он не мутирует runtime-веса и не объявляет обучение доказанным. Его задача -
разрешить следующий слой только тогда, когда исправления реально улучшают replay
и не ломают route-quality.

Chat-0 promoted state:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- chat0-promote-save target/chat0-feedback/chat0-promote-check.log target/chat0-feedback/chat0-promoted-check.nwps 13 97 128
cargo run -p nando-cli -- chat0-once-promoted target/chat0-feedback/chat0-promoted-check.nwps "manual:2: ping?" help target/chat0-traces/promoted-check.trace
```

`chat0-promote-save` сохраняет `.nwps` state только после успешного
`eval-chat0-promote`. `chat0-once-promoted` применяет этот state как явный
overlay: если prompt и параметры совпадают, route становится
`promoted_feedback_state`. Это первый воспроизводимый шаг от feedback log к
контролируемому обучению, но не скрытая самоизменяющаяся модель.

Chat-0 promoted holdout:

```bash
cd /home/ubu/projects/nando-wave
cargo run -p nando-cli -- eval-chat0-promoted-holdout target/chat0-feedback/chat0-promoted-holdout-check.log 13 97 128
```

Этот gate отделяет exact replay от переноса. Текущий результат:

```text
chat0_promoted_holdout_base.exact_accuracy: 0.851562
chat0_promoted_holdout_exact_overlay.exact_accuracy: 0.851562
chat0_promoted_holdout_harmonic_transfer_overlay.exact_accuracy: 0.539062
chat0_promoted_holdout_selective_harmonic_transfer_overlay.exact_accuracy: 0.640625
chat0_promoted_holdout_cell_signature_transfer_overlay.exact_accuracy: 0.203125
chat0_promoted_holdout_trajectory_transfer_overlay.exact_accuracy: 0.179688
chat0_promoted_holdout_task_hint_overlay.exact_accuracy: 1.000000
harmonic_transfer_ablation_min_accuracy: 0.406250
harmonic_transfer_ablation_max_drop: 0.132812
cell_signature_ablation_max_drop: 0.039062
trajectory_ablation_max_drop: 0.054688
selective_harmonic_best_threshold: 8.000000
selective_harmonic_best_over_base: 0.000000
exact_over_base: 0.000000
harmonic_transfer_over_base: -0.312500
selective_harmonic_transfer_over_base: -0.210938
cell_signature_transfer_over_base: -0.648438
trajectory_transfer_over_base: -0.671875
task_hint_over_base: 0.148438
mode_status: chat0_promoted_task_hint_holdout_candidate_not_mode
```

Вывод: `.nwps` exact overlay честно не обобщает на holdout. Naive harmonic
transfer через promoted carrier snapshot пока хуже base, значит его нельзя
считать переносимой модой. При этом transfer уже чувствителен к ablation
(`max_drop: 0.132812`), то есть сигнал не полностью случайный. Selective
confidence gate уменьшает вред, но threshold sweep не находит прироста выше
base. Cell-signature transfer по активным Cell32 еще слабее, значит одних
active-cell ids недостаточно для переноса Chat-0 коррекции. Trajectory transfer
по компактной истории нескольких wave ticks тоже хуже base, значит простой
centroid по фазовой траектории пока не является переносимой памятью. Task-hint overlay
показывает верхнюю границу возможного обобщения по классу задачи, но это пока
не доказательство ансамблевой моды.

## Исходная точка

Стартовая научная опора - статья Neel Nanda et al.:

```text
Progress measures for grokking via mechanistic interpretability
https://arxiv.org/abs/2301.05217
```

Подробнее: `docs/INSPIRATION.md`.

Статья рассматривает grokking и mechanistic interpretability на задаче
модульного сложения.

Главное не в том, что модель решила игрушечную задачу, а в том, что внутри
маленькой сети удалось увидеть конкретный алгоритм: сеть пришла к частотному
представлению через дискретный базис Фурье. Числа фактически кодируются как
фазы на окружности, а правильный ответ появляется через согласованную работу
синусов, косинусов, фаз и интерференции.

Важная мысль для проекта: обобщение может быть не простым знанием примеров,
а появлением правильных мод. Модель сначала может зубрить, но затем в ней
формируется компактная частотная схема, которая решает задачу как устойчивое
состояние.

Цель Nando Wave - пройти дальше: не считать эту статью готовым доказательством,
а построить собственное доказательство для клеточного волнового ансамбля.

## Главная гипотеза

Nando Wave исследует не одну большую сеть, а ансамбль маленьких клеток.

Базовая единица:

```text
одна клетка = примерно 64 KB состояния
```

Такая клетка должна быть дешевой для CPU, хорошо ложиться в кэш и быстро
обновляться. Но смысл не только в размере. Смысл в том, что много маленьких
клеток могут образовать ансамблевую частотную систему.

Вместо одной монолитной модели:

```text
одна большая весовая сеть
```

рассматривается клеточный организм:

```text
много маленьких частотных клеток
связанных через общую волновую шину
```

## Частоты важнее правил

Проект думается не как обычная система правил и не как классический
трансформер. Базовый язык системы - гармонические колебания:

```text
амплитуда
фаза
частота
интерференция
резонанс
биения
несущая волна
```

Клетка не должна просто хранить абстрактные веса. Она хранит и меняет свое
частотное состояние. Если клетка входит в резонанс с входным сигналом, она
усиливает свою моду. Если долго не входит в резонанс, она может дрейфовать
по спектру и искать новую нишу.

## Иерархия мод

Система мыслится как каскад гармонических мод.

Большая мода - низкая частота, глобальный контекст, несущая волна. Она не
дает нижним клеткам уйти за границы состояния. Это не жесткое правило, а
динамический диапазон, внутри которого остальные частоты могут меняться.

Средняя мода - синтаксис, ритм фразы, структура, локальные контуры смысла.
Она живет внутри ограничений большой моды и управляет тем, какие малые моды
могут стать активными.

Малая мода - быстрые микроколебания: символы, токены, локальные реакции,
мгновенные связи.

Грубая схема:

```text
большая мода  -> тема / интент / несущая волна
средняя мода  -> фраза / синтаксис / структура
малая мода    -> символы / токены / быстрые реакции
```

При этом мод может быть больше трех. Три уровня - только первый способ
не захлебнуться на слабом CPU и получить наблюдаемую систему.

## Память как спектральный снимок

Ключевая идея: память может быть не текстовой историей и не огромным массивом
примеров, а частотным состоянием.

То есть память:

```text
не лог всех слов
а аккорд фаз и амплитуд
```

Если состояние системы продуктивно, его можно снять как спектральный снимок:

```text
какие частоты активны
какие фазы согласованы
где центр масс спектра
какая несущая волна держит организм
```

Потом этот снимок можно вернуть на вход и заново возбудить клетки в похожем
состоянии. Система не перечитывает всю историю, а сразу входит в нужную
гармоническую конфигурацию.

## Продуктивное состояние

Продуктивное состояние - это не мистическое слово, а проверяемый режим:

```text
спектр становится менее шумным
фазы лучше согласованы
центр масс устойчивее
ответы стабильнее
ошибочные направления гасятся
```

Если клетки не согласованы, получается размазанный спектр и шум. Если клетки
попали в правильный режим, появляется резонанс: одни волны усиливают нужное
направление, другие гасят лишнее.

Идея близка к динамической релаксации системы к согласованному состоянию.
Мышление здесь рассматривается как поиск устойчивого волнового равновесия,
а не как последовательное применение правил.

## Ансамбль вместо монолита

Один важный контрольный эксперимент:

```text
3 клетки по 64 KB против одной клетки 192 KB
```

По сырой памяти это одно и то же:

```text
64 KB x 3 = 192 KB
```

Но топология разная.

Монолит 192 KB имеет одну общую память. Клетки 64 KB x 3 могут иметь разные
системы координат: одна ловит раскладку, другая держит защиту/границы, третья
ловит контекст. Интерес проекта именно в том, может ли связка клеток дать
ансамблевую моду, которой нет у одиночной клетки того же размера.

Формально:

```text
если 64x3 с волновой связью лучше или безопаснее 192-монолита,
значит есть смысл в клеточном организме
```

## Самообучение клетки

Предполагаемые локальные режимы обучения:

1. Резонансное усиление.
   Если входной сигнал совпал с внутренней гармоникой клетки, клетка чуть
   усиливает эту частоту и подстраивает фазу.

2. Фазовая автоподстройка.
   Клетка сдвигает фазу ближе к успешному сигналу.

3. Спектральный дрейф.
   Если клетка долго не дает полезного резонанса, она медленно меняет свою
   частотную нишу.

4. Межклеточная синхронизация.
   Клетки могут видеть успешные фазы соседей через общую шину и подстраиваться
   не копированием текста, а копированием/смещением частотного состояния.

Важно: это пока гипотеза. Ее надо будет проверять маленькими стендами.

## Дистилляция как частотный перелив

Есть отдельная идея: большую языковую модель можно использовать как источник
частотного состояния.

Не обязательно копировать ее веса. Можно попытаться снять временной ряд ее
ответов, скрытых состояний или выходных распределений и разложить его в спектр.
Затем этот спектральный снимок можно использовать как начальный аккорд для
маленьких клеток.

Это похоже на knowledge distillation, но не через заучивание ответов, а через
попытку передать частотную организацию состояния.

Формула идеи:

```text
большая LLM -> спектральный слепок -> ансамбль маленьких клеток
```

## Почему CPU и T480 важны

Цель не строить огромную GPU-модель. Цель - проверить, может ли слабый ноутбук
быть стендом для клеточного волнового интеллекта.

ThinkPad T480 важен как ограничение:

```text
обычный CPU
обычная RAM
без дорогой видеокарты
```

Первый стенд не должен начинаться с 1 GB модели. Разумнее начать с малого
ансамбля, например:

```text
1024 клетки x 64 KB = 64 MB состояния
```

И ввести разреженное вычисление:

```text
на каждом шаге активны не все клетки,
а только top-k резонансных клеток
```

Так можно наблюдать большие, средние и малые моды и не перегрузить CPU.

## Ядро на Rust

Исполняемое ядро Nando Wave должно быть Rust-first.

Python, notebooks или внешние инструменты теперь считаются архивной или
одноразовой лабораторией. Текущие proof/runtime-рубежи, генераторы корпусов,
shortcut gates, flat packages, спектральные снимки и горячий путь должны жить в
Rust, если отдельный throwaway-анализ явно не помечен как временный.

Причины:

```text
контроль памяти
фиксированный размер клеток
предсказуемая работа CPU cache
нет сборщика мусора в горячем цикле
безопасная параллельность
удобная упаковка бинарного состояния
реальный путь к локальному runtime
```

Базовая инженерная мысль:

```text
Cell32 / Expert64 / Organ192 - это не Python-объекты,
а компактные Rust-структуры с плоской памятью.
```

Горячий путь:

```text
input byte/char
-> phase encoder
-> active Cell32/Expert64 pool
-> wave bus
-> coherence / center-of-mass metrics
-> decoder
-> next byte/char
```

Все, что должно работать быстро и постоянно, должно быть в Rust. Это не
запрет на эксперименты, а защита идеи от случайного превращения в медленный
скрипт.

## Current Rust Package Scorer Commands

Action runtime package proof now has a binary eval-pack step:

Operator-dimension coverage audit is a separate Rust/CLI gate. It is not a
Python demo and not a runtime proof:

```bash
cargo run -p nando-cli --release -- phase-action-operator-coverage-v1 \
  data/rule_logic_operator_battery_v4/action_contract_v1/generated_action_contract_v1.jsonl \
  target/nando-wave/action-contract-v1.generated-operator-coverage-report.json

cargo run -p nando-cli --release -- phase-action-operator-coverage-v1 \
  data/rule_logic_operator_battery_v4/action_contract_v1/generated_domain_action_contract_v1.jsonl \
  target/nando-wave/action-contract-v1.generated-domain-operator-coverage-report.json

cargo run -p nando-cli --release -- phase-action-coverage-corpus-v1 \
  data/rule_logic_operator_battery_v5/action_contract_v1/generated_coverage_action_contract_v1.jsonl \
  target/nando-wave/action-contract-v1.generated-coverage-corpus-report.json

cargo run -p nando-cli --release -- phase-action-operator-coverage-v1 \
  data/rule_logic_operator_battery_v5/action_contract_v1/generated_coverage_action_contract_v1.jsonl \
  target/nando-wave/action-contract-v1.generated-coverage-operator-coverage-report.json
```

Current bounded corpora remain WATCH: generated_action has
select/transform/write/condition/check counts `1/10/1/1/8`, and domain_action
has `6/6/1/1/6`. The V5 coverage_action corpus has `6/10/5/5/10` and
`PHASE_ACTION_OPERATOR_COVERAGE_V1_PASS`. Release/regression/freeze now require
all operator-coverage reports to match their sources and at least one artifact
to close full operator-dimension coverage.

```bash
cargo run -p nando-cli --release -- phase-action-eval-pack-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  data/rule_logic_operator_battery_v4/action_contract_v1/generated_domain_action_contract_v1.jsonl \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack

cargo run -p nando-cli --release -- phase-action-package-score-pack-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack \
  target/nando-wave/action-runtime-v1-generated-domain-c32.score-pack-report.json

cargo run -p nando-cli --release -- phase-action-package-verify-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.score-pack-report.json

cargo run -p nando-cli --release -- phase-action-package-bench-pack-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack \
  1000 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.bench-report.json

cargo run -p nando-cli --release -- phase-action-package-bench-verify-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack \
  target/nando-wave/action-runtime-v1-generated-domain-c32.bench-report.json

cargo run -p nando-cli --release -- phase-action-product-proof-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack \
  target/nando-wave/action-runtime-v1-generated-domain-c32.score-pack-report.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.bench-report.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.product-proof.json

cargo run -p nando-cli --release -- phase-action-product-verify-v1 \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc \
  target/nando-wave/action-runtime-v1-generated-domain-c32.nwpc.manifest.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.eval-pack \
  target/nando-wave/action-runtime-v1-generated-domain-c32.score-pack-report.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.bench-report.json \
  target/nando-wave/action-runtime-v1-generated-domain-c32.product-proof.json
```

Required score-pack flags:

```text
eval_task_package_used: true
corpus_jsonl_used_in_score_loop: false
compiler_used: false
```

Required benchmark-pack flags:

```text
eval_task_package_used: true
corpus_jsonl_used_in_bench_loop: false
compiler_used: false
```

Required product-proof flags:

```text
product_proof_kind: phase_action_product_proof_v1
score_report_verdict: PHASE_ACTION_PACKAGE_SCORE_PACK_V1_PASS
bench_report_verdict: PHASE_ACTION_PACKAGE_BENCH_PACK_V1_PASS
compiler_used: false
forbidden_used: false
```

Build and verify the current action release-suite:

```bash
cargo run -p nando-cli --release -- phase-action-release-suite-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json

cargo run -p nando-cli --release -- phase-action-release-verify-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json
```

Required release-suite flags:

```text
release_suite_kind: phase_action_release_suite_v1
artifact_count: 2
distinct_package_fingerprints: true
compiler_used: false
eval_task_package_used: true
corpus_jsonl_used: false
forbidden_used: false
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
commercial_license_closed: false
report_matches_sources: true
```

Build and verify the current non-commercial license package:

```bash
cargo run -p nando-cli --release -- phase-action-license-package-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json

cargo run -p nando-cli --release -- phase-action-license-verify-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json
```

Required license-package flags:

```text
license_package_kind: phase_action_noncommercial_license_package_v1
license_name: Nando Wave Non-Commercial Source License v1.0
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

Build and verify the current local-operator offload audit:

```bash
cargo run -p nando-cli --release -- phase-action-offload-audit-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json \
  300000 \
  1000 \
  target/nando-wave/action-runtime-v1-offload-audit.product-proof.json

cargo run -p nando-cli --release -- phase-action-offload-verify-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json \
  target/nando-wave/action-runtime-v1-offload-audit.product-proof.json
```

Required offload-audit flags:

```text
offload_audit_kind: phase_action_offload_audit_v1
margin_threshold_micro: 300000
simulated_calls: 1000
local_operator_calls: 904
fallback_to_llm_calls: 96
offload_rate_milli: 904
local_accuracy_milli: 1000
false_local_accepts: 0
release_suite_gate_pass: true
license_package_gate_pass: true
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
report_matches_sources: true
```

Policy: use the packaged flat CPU operator only when `margin_micro >= 300000`;
otherwise fall back to the LLM. The decision policy is exported from
`nando_core::PhaseCenterOffloadPolicy`. The lower-level scorer remains
`PhaseCenterFlatRuntime`, and its batch/buffer APIs stay part of the public core
surface, but the product-facing packaged-runtime summary path is now
`PhaseCenterOffloadRuntime::offload_summary_into`. That wrapper computes
decisions and summary from packaged runtime data with caller-owned buffers; this
is not a text-generation or autonomous raw-action-parser claim.

Freeze and verify the current green regression:

```bash
cargo run -p nando-cli --release -- phase-action-regression-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json \
  target/nando-wave/action-runtime-v1-offload-audit.product-proof.json \
  target/nando-wave/action-runtime-v1-regression.product-proof.json

cargo run -p nando-cli --release -- phase-action-regression-verify-v1 \
  target/nando-wave/action-runtime-v1-release-suite.product-proof.json \
  LICENSE-NONCOMMERCIAL.md \
  target/nando-wave/action-runtime-v1-license-package.product-proof.json \
  target/nando-wave/action-runtime-v1-offload-audit.product-proof.json \
  target/nando-wave/action-runtime-v1-regression.product-proof.json
```

Required regression flags:

```text
regression_kind: phase_action_regression_v1
release_verify_pass: true
license_verify_pass: true
offload_verify_pass: true
release_suite_report_fingerprint64: 12340473504052004295
release_suite_report_bytes: 10810
license_package_report_fingerprint64: 17769124418356895286
license_package_report_bytes: 2053
offload_audit_report_fingerprint64: 6122761890799637522
offload_audit_report_bytes: 6281
generated_action_source_contract_fingerprint64: 6845017756676715377
generated_action_source_contract_bytes: 122500
generated_action_source_rebuild_matches_package: true
generated_action_source_rebuild_package_fingerprint64: 14869999570221545448
generated_action_source_rebuild_package_bytes: 10256
domain_action_source_contract_fingerprint64: 9195974547787197795
domain_action_source_contract_bytes: 101612
domain_action_source_rebuild_matches_package: true
domain_action_source_rebuild_package_fingerprint64: 5367415087033800111
domain_action_source_rebuild_package_bytes: 6160
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
offload_runtime_summary_api: nando_core::PhaseCenterOffloadRuntime::offload_summary_into
operator_blueprint_path: docs/OPERATOR_BLUEPRINT.md
operator_blueprint_fingerprint64: 15636335857514266420
operator_blueprint_formula_present: true
operator_blueprint_runtime_package_contract_present: true
operator_blueprint_source_verify_contract_present: true
operator_blueprint_shortcut_report_contract_present: true
operator_blueprint_rust_proof_path_present: true
operator_blueprint_forbidden_invariants_present: true
state_transition_formula: state_t + action_tree -> state_t+1
```

Public Rust SDK consumer proof:

```text
cargo test -p nando-core --test phase_center_offload_sdk_public: PASS
```

## Latest Packaged Runtime Rung

Python demos are not proof artifacts. Current proof authority is the Rust CLI
and flat CPU runtime package chain.

```text
state_t + action_tree -> state_t+1

phase-action-release-suite-v1: PASS
phase-action-release-verify-v1: PASS
phase-action-license-package-v1: PASS
phase-action-license-verify-v1: PASS
phase-action-offload-audit-v1: PASS
phase-action-offload-verify-v1: PASS
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS

release_suite_report_fingerprint64: 6910835921617119291
release_suite_report_bytes: 12128
artifact_count: 2
total_runtime_bytes_estimate: 16896
total_bench_samples: 128000
max_score_p99_latency_ns: 195
max_bench_p99_latency_ns: 110

all_action_contract_source_rebuild_clean: true
total_source_rebuild_accepted_action_tree_rows: 320
total_source_rebuild_rejected_action_tree_rows: 0
total_source_rebuild_forbidden_contract_rows: 0
all_package_report_parity_pass: true
all_action_ablation_collapses: true

offload_rate_milli: 968
local_accuracy_milli: 1000
false_local_accepts: 0
operator_blueprint_fingerprint64: 15803322666366215503
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
product source_rebuild_forbidden_operator_label_rows tamper -> WATCH
release all_action_contract_source_rebuild_clean tamper -> WATCH
regression all_action_contract_source_rebuild_clean tamper -> WATCH
```

Release-mode benchmark evidence:

```text
generated_action_bench_p99_latency_ns: 110
domain_action_bench_p99_latency_ns: 85
release_suite_max_score_p99_latency_ns: 195
release_suite_max_bench_p99_latency_ns: 110
phase-action-regression-verify-v1: PASS
report_matches_sources: true
```

Boundary:

```text
This closes clean action_tree source-rebuild provenance for the packaged flat
action scorer. It does not close autonomous raw action parsing, strict ordered
decoder, text generation, broad workflow reasoning, or commercial licensing.
```

## Latest Optimized Runtime Gate

Python demos are not proof artifacts. Debug builds are not benchmark proof
artifacts. Current proof authority is the Rust release CLI and packaged flat
CPU runtime report chain.

```text
state_t + action_tree -> state_t+1

phase-action-product-verify-v1 generated_action: PASS
phase-action-product-verify-v1 domain_action: PASS
phase-action-release-suite-v1: PASS
phase-action-release-verify-v1: PASS
phase-action-license-package-v1: PASS
phase-action-license-verify-v1: PASS
phase-action-offload-audit-v1: PASS
phase-action-offload-verify-v1: PASS
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS

release_suite_report_fingerprint64: 15464041285945484691
release_suite_report_bytes: 12234
all_optimized_build_reports_pass: true
max_score_p99_latency_ns: 312
max_bench_p99_latency_ns: 117
total_bench_samples: 128000
total_runtime_bytes_estimate: 16896

all_action_contract_source_rebuild_clean: true
total_source_rebuild_accepted_action_tree_rows: 320
total_source_rebuild_rejected_action_tree_rows: 0
total_source_rebuild_forbidden_contract_rows: 0
all_package_report_parity_pass: true
all_action_ablation_collapses: true

offload_rate_milli: 968
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
debug score-pack optimized_build=false -> WATCH
debug bench-pack optimized_build=false -> WATCH
product optimized_build=false tamper -> WATCH
release all_optimized_build_reports_pass=false tamper -> WATCH
regression all_optimized_build_reports_pass=false tamper -> WATCH
```

Boundary:

```text
This closes optimized release-build evidence for the packaged flat action
scorer. It does not close autonomous raw action parsing, strict ordered
decoder, text generation, broad workflow reasoning, or commercial licensing.
```

## Latest Frozen Regression Checkpoint

The current green regression is now frozen by a separate Rust CLI checkpoint.
This prevents the release proof from existing only as prose or terminal
history.

```text
state_t + action_tree -> state_t+1

phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS

regression_report_fingerprint64: 1510085368394183704
regression_report_bytes: 4060
regression_verdict: PHASE_ACTION_REGRESSION_V1_PASS
regression_gate_pass: true
regression_matches_sources: true
release_suite_report_fingerprint64: 15464041285945484691
license_package_report_fingerprint64: 7703393407740687299
offload_audit_report_fingerprint64: 308995585512256485
operator_blueprint_fingerprint64: 17789125758946021618

all_package_report_parity_pass: true
all_action_contract_source_rebuild_clean: true
all_optimized_build_reports_pass: true
max_bench_p99_latency_ns: 117
offload_rate_milli: 968
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
freeze all_optimized_build_reports_pass=false tamper -> WATCH
regression all_optimized_build_reports_pass=false tamper -> WATCH
```

Boundary:

```text
This freezes the packaged flat action scorer regression checkpoint. It does
not close autonomous raw action parsing, strict ordered decoder, text
generation, broad workflow reasoning, or commercial licensing.
```

## Latest Cache-Enabled Offload Benchmark

The product offload claim is now measured against an exact-cache baseline, not
only against a no-cache LLM baseline.

```text
phase-action-cache-offload-bench-v1: PASS
phase-action-cache-offload-bench-verify-v1: PASS

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
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
incremental_llm_calls_removed_vs_cache=0 tamper -> WATCH
python_demo_used=true tamper -> WATCH
margin_threshold_micro=100 no-fallback policy -> WATCH
```

Boundary:

```text
This proves incremental local CPU operator offload over an exact-cache baseline
for the packaged flat action scorer. It does not close autonomous raw action
parsing, strict ordered decoder, text generation, broad workflow reasoning, or
commercial licensing.
```

## Latest Regression/Freeze Cache-Bench Promotion

The cache-enabled offload benchmark is now part of the regression/freeze proof
chain, not only a separate side benchmark.

```text
phase-action-cache-offload-bench-verify-v1: PASS
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS

cache_offload_bench_report_fingerprint64: 7742563455518673124
cache_offload_bench_report_bytes: 4307
cache_bench_verify_pass: true
cache_bench_report_matches_sources: true
cache_incremental_llm_calls_removed_vs_cache: 124
cache_exact_cache_llm_calls: 128
cache_exact_cache_plus_nando_llm_calls: 4
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
cache benchmark incremental_llm_calls_removed_vs_cache=0 tamper -> regression verify WATCH
regression cache_offload_bench_report_fingerprint64=1 tamper -> regression verify WATCH
freeze cache_bench_verify_pass=false tamper -> freeze verify WATCH
```

Boundary:

```text
This proves incremental local CPU operator offload over the current exact-cache
baseline for the packaged flat action scorer. It does not close strict ordered
decoder, text generation, autonomous raw action parsing, broad workflow
reasoning, every production cache policy, or commercial licensing.
```

## Latest Action-Tree Coverage Gate Promotion

Python demos are not proof artifacts. The packaged flat action scorer must now
prove that source/product/release/regression/freeze reports carry enough
distinct `action_tree` operator keys in both train and heldout splits.

```text
phase-action-source-verify-v1 generated_action: PASS
phase-action-source-verify-v1 domain_action: PASS
phase-action-product-proof-v1 generated_action: PASS
phase-action-product-proof-v1 domain_action: PASS
phase-action-release-suite-v1: PASS
phase-action-release-verify-v1: PASS
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS

generated_action action_tree_key_count: 10
domain_action action_tree_key_count: 6
total_source_rebuild_action_tree_key_count: 16
min_source_rebuild_action_tree_key_count: 6
all_action_tree_key_coverage_pass: true
source_rebuild_min_train_rows_per_action_tree: 12
source_rebuild_min_heldout_rows_per_action_tree: 8
release_suite_report_fingerprint64: 12772053458428771913
regression_report_fingerprint64: 10589187500100786722
operator_blueprint_fingerprint64: 15033540855767578891
python_demo_used: false
forbidden_used: false
```

Negative checks:

```text
source source_rebuild_action_tree_key_count=1 tamper -> release verify WATCH
release all_action_tree_key_coverage_pass=false tamper -> regression verify WATCH
regression all_action_tree_key_coverage_pass=false tamper -> freeze verify WATCH
```

Boundary:

```text
This closes action-tree key coverage promotion for the packaged flat action
scorer proof chain. It does not close strict ordered decoder, 32-slot operator
transfer, autonomous raw action parsing, text generation, broad workflow
reasoning, every production cache policy, or commercial licensing.
```

## Latest V5 Operator-Dimension Coverage Gate

Python demos are not proof artifacts. The current V5 coverage proof is a Rust
corpus factory plus packaged flat runtime release chain.

```text
phase-action-coverage-corpus-v1: PASS
phase-action-operator-coverage-v1 coverage_action: PASS
phase-action-release-suite-v1: PASS
phase-action-release-verify-v1: PASS
phase-action-regression-v1: PASS
phase-action-regression-verify-v1: PASS
phase-action-regression-freeze-v1: PASS
phase-action-regression-freeze-verify-v1: PASS

coverage_action rows: 360
coverage_action action_tree_key_count: 30
coverage_action select/transform/write/condition/check counts: 6/10/5/5/10
coverage_action full_operator_dimension_coverage_pass: true

release artifact_count: 3
release_operator_dimension_coverage_pass: true
all_operator_coverage_reports_match_sources: true
total_source_rebuild_action_tree_key_count: 46
total_runtime_bytes_estimate: 48576
max_bench_p99_latency_ns: 117
offload_rate_milli: 880
local_accuracy_milli: 1000
false_local_accepts: 0
compiler_used: false
corpus_jsonl_used: false
python_demo_used: false
forbidden_used: false
```

Boundary:

```text
This closes V5 operator-dimension coverage for the packaged flat action scorer
release/regression/freeze chain. It does not close strict ordered decoder,
32-slot operator transfer, autonomous raw action parsing, text generation,
broad workflow reasoning, every production cache policy, or commercial
licensing.
```

## Что важно не потерять

Это не просто MoE и не просто набор маленьких экспертов.

Ключ проекта:

```text
частотное состояние
гармонические моды
несущая волна
центр масс спектра
согласование фаз
память как аккорд
ансамблевая мода
```

Если система сведется к обычным правилам или обычному голосованию экспертов,
идея будет потеряна. Нужно проверять именно волновую гипотезу: появляется ли
в ансамбле устойчивое частотное состояние, которое дает поведение лучше, чем
сумма отдельных клеток.

## Пока не делать

Пока не писать код.
Пока не выбирать финальную архитектуру.
Пока не делать громких заявлений.
Пока только фиксировать идею и понимать, что именно надо проверить.

Следующий этап после осмысления:

```text
цели
гипотезы
минимальные эксперименты
метрики
план реализации
```

## Strict Multi-Seed Rust Audit

Python demos are not proof artifacts. The current strict multi-seed robustness
debt is recorded by a Rust log audit:

```bash
cargo run -p nando-cli --release -- strict-multiseed-rust-audit-v1 \
  data/rule_logic_operator_battery_v4/diagnostics/multiseed \
  target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json

cargo run -p nando-cli --release -- strict-multiseed-rust-audit-verify-v1 \
  data/rule_logic_operator_battery_v4/diagnostics/multiseed \
  target/nando-wave/strict-multiseed-rust-audit-v1.product-proof.json
```

Current result:

```text
verdict: STRICT_MULTI_SEED_RUST_AUDIT_RED
verify: STRICT_MULTI_SEED_RUST_AUDIT_VERIFY_PASS
report_matches_sources: true
observed_logs: 12
missing_logs: 0
red issue: seed=2 class=order, energy_pass_slot_fail=1,
  output_slot_cleanup_failed_slots=1, slot_failure_total=1
python_demo_used: false
corpus_jsonl_used: false
```

Boundary: this records the red strict-readout debt. It does not close
multi-seed strict robustness and does not repair the model.
