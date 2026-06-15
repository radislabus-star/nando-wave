# Nando Wave Roadmap

Цель дорожной карты - довести идею до проверяемого Rust-стенда на T480, не
провалившись сразу в невозможную цель "сделать GPT".

Подробный рабочий вариант с входами, выходами, gates, стоп-сигналами и
контролем расползания находится в `docs/DETAILED_ROADMAP.md`.

## Принцип

Проект строится как лаборатория, а не как приложение.

Сначала нужен прибор:

```text
создать клетки
запустить волновую шину
измерить моду
сравнить с монолитом
сделать ablation
сохранить snapshot
```

Только после этого появляется Chat-0.

## Этап 0: Фиксация рамки

Статус: текущий этап.

Сделать:

```text
README.md как манифест идеи
docs/HYPOTHESES.md
docs/ROADMAP.md
docs/RISKS.md
docs/NORTH_STAR.md
```

Решения этапа:

```text
ядро только Rust
первый атом Cell32
Expert64 = 2 x Cell32
Organ192 = 6 x Cell32
первый контроль 6 x Cell32 против Mono192
```

Не делать:

```text
не писать чатбот первым
не подключать тяжелые ML-фреймворки
не учить на приватных пользовательских логах
не обещать LLM-уровень
```

## Этап 1: Rust workspace

Статус: реализован.

Цель: создать минимальную структуру проекта без сложной логики.

Предлагаемая структура:

```text
crates/nando-core
crates/nando-cli
crates/nando-eval
docs
data/toy
target/reports
target/snapshots
```

Назначение:

```text
nando-core  - клетки, шина, snapshot, метрики
nando-cli   - ручной запуск экспериментов
nando-eval  - сравнение режимов и отчеты
data/toy     - маленькие открытые fixtures
```

Gate этапа:

```text
cargo test
cargo clippy -- -D warnings
одна CLI-команда показывает пустую версию стенда
```

## Этап 2: Cell32 и Mono192 без обучения

Статус: начат. Реализованы первый deterministic wave tick без обучения,
стабильный `.nws1` snapshot roundtrip и one-tick eval report без baseline.

Цель: создать память и вычисление, но без самообучения.

Сущности:

```text
Cell32
Mono192
WaveBus
TickTrace
SpectrumSnapshot
```

Функции:

```text
encode input byte/number to phase vector
compute resonance
write bus state
measure coherence
measure spectral entropy
measure center of mass
```

Gate этапа:

```text
один тик воспроизводим
размер Cell32 проверяется тестом
размер Mono192 проверяется тестом
snapshot сохраняется и читается
```

## Этап 3: Первая чистая задача

Статус: начат. Реализован первый synthetic periodic baseline report:
`random`, `Mono192`, `6 x Cell32 без wave bus`, `6 x Cell32 с wave bus`.
Mode detection пока не доказан, ablation еще не реализована.

Цель: не текст, а управляемая математическая задача.

Задачи-кандидаты:

```text
периодическая последовательность
modular addition small P
phase matching toy task
```

Режимы сравнения:

```text
random baseline
Mono192
6 x Cell32 без wave bus
6 x Cell32 с wave bus
```

Gate этапа:

```text
отчет показывает accuracy
отчет показывает coherence
отчет показывает entropy
отчет показывает CPU time
результат воспроизводится seed-ом
```

## Этап 4: Ансамблевая мода

Статус: начат. В `eval-periodic` добавлены `cell32_voting` и ablation sweep
по 6 клеткам. Также добавлен `eval-phase-composition` как probe для задачи,
где target зависит от input phase и CarrierWave phase. Добавлены
`eval-phase-holdout`, `eval-carrier-control`, `eval-bus-transfer`,
`eval-snapshot-memory`, `eval-snapshot-transition` и
`eval-snapshot-dynamics`, `eval-snapshot-multitick`, `eval-snapshot-adapt`,
`eval-snapshot-decoder`, `eval-snapshot-keyed`,
`eval-snapshot-keyed-transition`, `eval-snapshot-noisy-keyed-transition`,
`eval-snapshot-noisy-keyed-transition-sweep`,
`eval-snapshot-noisy-keyed-transition-seed-sweep`, `eval-byte-context`,
`eval-byte-context-centroid`, `eval-byte-context-centroid-seed-sweep`,
`eval-byte-context-centroid-ablation`,
`eval-byte-context-offset-centroid`,
`eval-byte-context-offset-centroid-seed-sweep`,
`eval-byte-context-denoised-centroid`,
`eval-byte-context-denoised-centroid-seed-sweep`,
`eval-byte-context-relative-centroid`,
`eval-byte-context-relative-centroid-seed-sweep`,
`eval-byte-context-lexical-carrier-centroid`,
`eval-byte-context-lexical-carrier-centroid-seed-sweep`,
`eval-byte-context-cellular-carrier-centroid`,
`eval-byte-context-cellular-carrier-centroid-seed-sweep`,
`eval-byte-context-cellular-carrier-ablation`,
`eval-byte-context-trained-carrier-centroid`,
`eval-byte-context-trained-carrier-centroid-seed-sweep`,
`eval-byte-context-trained-carrier-ablation`,
`eval-byte-context-prompt-carrier-centroid`,
`eval-byte-context-prompt-carrier-centroid-seed-sweep`,
`eval-byte-context-prompt-carrier-ablation`,
`eval-byte-context-prompt-carrier-diverse-centroid`,
`eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep`,
`eval-byte-context-prompt-carrier-diverse-ablation`.
Критерий
`mode_status: found` еще не достигнут: carrier-control прошел как candidate
gate, более строгий delayed bus-transfer пока не найден, snapshot-memory
прошел только как state replay, snapshot-transition пока не найден,
snapshot-dynamics и snapshot-multitick прошли по фазовой ошибке,
snapshot-adapt и snapshot-decoder пока не найдены из-за сильного no-snapshot
контроля, snapshot-keyed и snapshot-keyed-transition прошли как ограниченные
snapshot-private gates, snapshot-noisy-keyed-transition прошел как менее
прямой noisy gate, noisy-keyed-transition-sweep прошел как robustness gate.
noisy-keyed-transition-seed-sweep прошел как seed robustness gate.
byte-context пока вернул `not_found_byte_context`, byte-context-centroid
прошел как first byte-level candidate gate, но
byte-context-centroid-seed-sweep пока вернул
`not_found_byte_context_centroid_seed_sweep`. На удачной seed-паре
byte-context-centroid-ablation показал чувствительность к `snapshot_offset`,
но на провалившей seed-паре ablation не подтвердила устойчивую feature-моду.
Offset-only seed sweep тоже не найден.
Denoised `offset + top_sin` seed sweep тоже не найден.
Relative seed-normalized centroid seed sweep тоже не найден.
Lexical CarrierWave centroid seed sweep прошел как carrier-state bridge.
Cellular CarrierWave centroid seed sweep прошел, cellular lock ablation тоже
прошел на 128 cases. Trained CarrierWave centroid seed sweep прошел, trained
lock ablation тоже прошел на 128 cases. Ограничение: trained gate пока
использует структурную подсказку middle-token, поэтому это еще не финальный
proof самоорганизованной моды. Prompt-cloud CarrierWave centroid seed sweep
прошел без middle-token подсказки, а bank-wide ablation показал чувствительность
к обученному bank. При этом per-cell `min_accuracy_drop` отрицательный, то есть
отдельные клетки частично избыточны. Diverse prompt-cloud gate тоже прошел:
train/holdout prompt-шаблоны различаются, seed sweep дает 4/4, а отключение
всего learned bank рушит качество.

Цель: проверить главную гипотезу.

Сравнение:

```text
Mono192
6 x Cell32 + voting
6 x Cell32 + wave bus
6 x Cell32 + wave bus + ablation
```

Критерий:

```text
wave bus лучше voting или monolith
ablation ключевой клетки ломает эффект
false positives не растут
```

Gate этапа:

```text
mode_status: found
или
mode_status: not_found
```

Оба результата полезны. `not_found` означает, что гипотеза пока не доказана.

## Этап 4.5: Несущая волна

Статус: начат. Реализованы `eval-carrier-control` и `eval-bus-transfer`.
Первый сравнивает правильную, выключенную, чужую и поврежденную несущую.
Второй запрещает decoder читать `CarrierWave phase` напрямую и проверяет
переход через `WaveBus center_phase`.

Цель: сделать CarrierWave явной частью эксперимента.

Сравнение:

```text
wave bus без CarrierWave
wave bus с CarrierWave
wrong CarrierWave
corrupted CarrierWave
```

Gate этапа:

```text
правильная несущая снижает entropy
wrong carrier уводит состояние
corrupted carrier дает плавную деградацию
carrier входит в snapshot
```

Текущий candidate-result:

```text
correct_carrier_wave.accuracy: 0.062500
no_carrier_wave.accuracy: 0.000000
wrong_carrier_wave.accuracy: 0.007812
corrupted_carrier_wave.accuracy: 0.000000
mode_status: carrier_control_passed_candidate_mode
```

Ограничение результата: target в phase-composition probe явно использует
CarrierWave phase, поэтому этот gate доказывает зависимость кандидата от
несущей, но еще не доказывает общую архитектуру Nando Wave.

Delayed bus-transfer result:

```text
correct_carrier_bus.accuracy: 0.007812
wrong_carrier_bus.accuracy: 0.000000
correct_over_best_baseline: 0.000000
correct_over_wrong_carrier: 0.007812
mode_status: not_found_bus_transfer
```

Вывод: текущий one-tick организм пока не переносит wave-state на следующий
шаг лучше baseline. Следующий фокус - snapshot/warm-state или явная
переходная динамика.

## Этап 5: Snapshot как память

Статус: начат. Реализованы `eval-snapshot-memory`,
`eval-snapshot-transition`, `eval-snapshot-dynamics` и
`eval-snapshot-multitick`, `eval-snapshot-adapt`,
`eval-snapshot-decoder`, `eval-snapshot-keyed`.

Цель: проверить, является ли спектральный снимок рабочей памятью.

Проверка:

```text
холодный старт
warm snapshot
corrupted snapshot
wrong snapshot
```

Gate этапа:

```text
warm snapshot быстрее возвращает coherence
wrong snapshot мешает или дает другой режим
corrupted snapshot показывает плавную деградацию
```

Текущий replay-result:

```text
snapshot_bytes: 148
warm_snapshot.accuracy: 1.000000
wrong_snapshot.accuracy: 0.015625
corrupted_snapshot.accuracy: 0.000000
mode_status: snapshot_memory_passed_state_replay
```

Ограничение: replay текущего состояния пройден, но переходная память еще не
доказана. Следующий gate должен проверять warm start на следующем шаге.

Текущий transition-result:

```text
no_snapshot_transition.accuracy: 0.007812
warm_snapshot_transition.accuracy: 0.000000
wrong_snapshot_transition.accuracy: 0.000000
corrupted_snapshot_transition.accuracy: 0.000000
mode_status: not_found_snapshot_transition
```

Вывод: простого offset между carrier и center phase недостаточно. Для
следующего шага нужна явная transition dynamics или локальная адаптация.

Текущий dynamics-result:

```text
warm_snapshot_dynamics.mean_circular_error: 58.039062
no_snapshot_dynamics.mean_circular_error: 64.023438
wrong_snapshot_dynamics.mean_circular_error: 60.757812
warm_error_gain_over_no: 5.984375
warm_error_gain_over_wrong: 2.718750
mode_status: snapshot_dynamics_passed
```

Вывод: при smooth carrier sequence snapshot помогает как transition-state по
ошибке фазы. Exact accuracy остается низкой.

Текущий multitick-result:

```text
horizon: 4
warm_snapshot_multitick.mean_circular_error: 61.820312
no_snapshot_multitick.mean_circular_error: 63.304688
wrong_snapshot_multitick.mean_circular_error: 62.617188
warm_error_gain_over_no: 1.484375
warm_error_gain_over_wrong: 0.796875
mode_status: snapshot_multitick_passed
```

Вывод: snapshot держит слабое фазовое преимущество на горизонте 4 тика, но
точность почти нулевая. Следующий этап должен искать локальную адаптацию или
более сильный transition decoder.

## Этап 6: Локальная адаптация

Статус: начат. Реализованы `eval-snapshot-adapt`,
`eval-snapshot-decoder`, `eval-snapshot-keyed`,
`eval-snapshot-keyed-transition`,
`eval-snapshot-noisy-keyed-transition` и
`eval-snapshot-noisy-keyed-transition-sweep`,
`eval-snapshot-noisy-keyed-transition-seed-sweep`.

Цель: добавить ограниченное самообучение.

Разрешенные механизмы:

```text
phase pull
amplitude reinforce
decay
drift
```

Запрет:

```text
не переписывать долгую память во время генерации без eval
```

Gate этапа:

```text
адаптация улучшает toy-задачу
адаптация не ухудшает holdout
можно откатить состояние
```

Текущий adapt-result:

```text
warm_snapshot_no_adapt.mean_circular_error: 61.820312
adapted_snapshot.mean_circular_error: 59.710938
adapted_no_snapshot.mean_circular_error: 53.406250
adapted_error_gain_over_warm: 2.109375
adapted_error_gain_over_no_adapt: -6.304688
mode_status: not_found_snapshot_adapt
```

Вывод: глобальная phase-correction улучшает warm snapshot, но без snapshot
адаптируется еще лучше. Следующий шаг должен переносить адаптацию внутрь
клеточного состояния или отдельного transition decoder.

Текущий decoder-result:

```text
warm_snapshot_decoder_control.mean_circular_error: 61.820312
decoder_snapshot.mean_circular_error: 56.781250
decoder_no_snapshot.mean_circular_error: 53.320312
decoder_error_gain_over_warm: 5.039062
decoder_error_gain_over_no_decoder: -3.460938
mode_status: not_found_snapshot_decoder
```

Вывод: transition decoder улучшает snapshot-ветку, но no-snapshot decoder
пока лучше. Это значит, что текущая toy-задача или признаки слишком легко
объясняются без клеточной памяти.

Текущий keyed-result:

```text
keyed_snapshot.accuracy: 1.000000
no_snapshot_keyed.accuracy: 0.000000
wrong_snapshot_keyed.accuracy: 0.000000
corrupted_snapshot_keyed.accuracy: 0.000000
mode_status: snapshot_keyed_passed
```

Вывод: snapshot-private state проходит no-snapshot/wrong/corrupted контроли.
Это еще не финальная ансамблевая мода, но теперь есть проверенный приборный
сигнал, который no-snapshot контроль не объясняет.

Текущий keyed-transition-result:

```text
future_only_transition.accuracy: 0.000000
keyed_transition.accuracy: 1.000000
wrong_snapshot_keyed_transition.accuracy: 0.062500
corrupted_snapshot_keyed_transition.accuracy: 0.015625
mode_status: snapshot_keyed_transition_passed
```

Вывод: snapshot-private state уже участвует в future transition, а не только
копируется как ответ. Это сильнее replay/keyed, но все еще synthetic gate.

Текущий noisy-keyed-transition-result:

```text
future_only_noisy_transition.accuracy: 0.007812
keyed_noisy_transition.accuracy: 0.664062
wrong_snapshot_noisy_transition.accuracy: 0.046875
corrupted_snapshot_noisy_transition.accuracy: 0.007812
mode_status: snapshot_noisy_keyed_transition_passed
```

Вывод: hidden snapshot modulation дает частичное, но сильное преимущество.
Это уже не идеальный keyed-answer, хотя задача остается synthetic.

Текущий noisy-keyed-transition-sweep-result:

```text
horizon_1.keyed_accuracy: 0.609375
horizon_2.keyed_accuracy: 0.617188
horizon_4.keyed_accuracy: 0.664062
horizon_8.keyed_accuracy: 0.664062
passed_count: 4
mode_status: snapshot_noisy_keyed_transition_sweep_passed
```

Вывод: эффект держится на нескольких horizons, но остается synthetic.

Текущий noisy-keyed-transition-seed-sweep-result:

```text
passed_seed_pairs: 4
min_keyed_accuracy_over_future_only: 0.562500
min_error_gain_over_future_only: 13.367188
min_error_gain_over_wrong_snapshot: 4.515625
mode_status: snapshot_noisy_keyed_transition_seed_sweep_passed
```

Вывод: эффект держится на нескольких seed-парах. Ограничение прежнее:
задача synthetic.

## Этап 7: Byte-level генератор

Цель: перейти от чистых задач к тексту.

Статус: начат через `eval-byte-context`, но gate пока не найден.

```text
snapshot_decoder.accuracy: 0.031250
no_snapshot_decoder.accuracy: 0.046875
mono192_prompt_decoder.accuracy: 0.000000
snapshot_accuracy_over_best_control: -0.015625
snapshot_error_gain_over_best_control: -1.171875
snapshot_error_gain_over_wrong_snapshot: 1.859375
mode_status: not_found_byte_context
```

Вывод: byte-level prompt -> answer-byte уже подключен к eval ladder, но
snapshot decoder пока не обогнал лучший контроль. Следующий шаг - улучшать
byte-context признаки/decoder или менять задачу, пока не появится измеримая
мода с ablation.

После этого добавлен `eval-byte-context-centroid`.

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

Вывод: появился первый byte-level candidate. Следующий gate - seed sweep и
ablation для проверки, что это не single-seed/single-prototype совпадение.

Seed sweep для этого candidate:

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_centroid_seed_sweep
```

Вывод: первый byte-level candidate не выдержал robustness gate. Следующий
шаг - улучшать признаки/разбиение/ablation и искать моду, которая держится на
нескольких seed-парах.

Feature ablation:

```text
13 -> 97:
key_feature: ablate_snapshot_offset
max_accuracy_drop: 0.218750
mode_status: byte_context_centroid_ablation_sensitive

29 -> 131:
max_accuracy_drop: 0.000000
mode_status: not_found_byte_context_centroid_ablation
```

Вывод: удачная пара опирается на `snapshot_offset`, но этот признак пока не
стабилен на разных seed-парах. Следующая работа - стабилизировать offset-mode
или убрать шумные top-phase признаки.

Offset-only result:

```text
passed_seed_pairs: 1
min_error_gain_over_best_control: -5.437500
mode_status: not_found_byte_context_offset_centroid_seed_sweep
```

Вывод: один `snapshot_offset` не является достаточной переносимой модой.
Следующий вариант - искать устойчивую композицию offset + ограниченные
top-phase признаки, а не использовать все features или только offset.

Denoised `offset + top_sin` result:

```text
passed_seed_pairs: 2
min_error_gain_over_best_control: -4.953125
mode_status: not_found_byte_context_denoised_centroid_seed_sweep
```

Вывод: denoised mask лучше offset-only, но не лучше полного centroid по
robustness. Следующая работа - менять формирование состояния или делать
seed-normalization, а не просто подбирать mask.

Relative seed-normalized centroid result:

```text
passed_seed_pairs: 0
min_error_gain_over_best_control: -5.140625
mode_status: not_found_byte_context_relative_centroid_seed_sweep
```

Вывод: простая seed-normalization через относительные top-фазы ухудшила
результат до 0/4. Значит текущий переносимый мост к Chat-0 нельзя получить
обычной сменой координат snapshot-features; следующий шаг - менять сам способ
формирования состояния/несущей, чтобы snapshot кодировал задачу устойчивее.

Lexical CarrierWave centroid result:

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.796875
min_error_gain_over_best_control: 4.937500
min_error_gain_over_wrong_snapshot: 4.796875
mode_status: byte_context_lexical_carrier_centroid_seed_sweep_passed
```

Вывод: когда `CarrierWave` явно locked на устойчивый lexical key из prompt,
snapshot переносит byte-context состояние между разными surface-формами
запроса. Это не финальный proof ансамблевой моды: lock пока лабораторный и
feature-reader читает carrier-state напрямую. Но это положительный gate для
Chat-0: если следующий слой научится получать такой lock через клетки/wave bus,
короткая генерация ответа становится инженерно достижимой.

Cellular CarrierWave centroid result:

```text
passed_seed_pairs: 4
min_error_gain_over_best_control: 4.937500
mode_status: byte_context_cellular_carrier_centroid_seed_sweep_passed

min_accuracy_drop: 0.101562
mode_status: byte_context_cellular_carrier_ablation_sensitive
```

Вывод: lexical lock заменен на слой из 8 task-cell resonance detectors.
Победившая lock-клетка задает фазу `CarrierWave`; seed sweep проходит, а
отключение lock-клеток снижает качество. Это еще не обученный волновой
организм, но уже не простой прямой if-lock: появился проверяемый клеточный
узел, который можно дальше заменять на гармоническое обучение.

Prompt-cloud diverse result:

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

Вывод: обучаемый prompt-cloud lock выдержал разные train/holdout шаблоны.
Первый маленький Chat-0 loop с короткой генерацией и feedback-счетчиком уже
добавлен как отдельный eval-gate, но это не объявление полной цели завершенной.

Chat-0 eval result:

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

Вывод: появился проверяемый output loop `input -> CarrierWave lock -> snapshot
-> short response -> feedback log -> eval`. Следующий шаг - интерактивный shell
и версионированная feedback-консолидация.

Settle-word result:

```text
settle_word_mono192.exact_accuracy: 0.179688
settle_word_organ_one_tick.exact_accuracy: 0.078125
settle_word_organ_settle3.exact_accuracy: 0.000000
settle_word_organ_settle5.exact_accuracy: 0.125000
settle_word_organ_settle8.exact_accuracy: 0.078125
settle_word_organ_stable.exact_accuracy: 0.070312
settle_word_organ_gated.exact_accuracy: 0.328125
settle5_over_best_control: -0.054688
settle5_ablation_max_drop: 0.000000
stable_over_best_control: -0.109375
gated_mean_selected_ticks: 7.000000
carrier_integrity_gap: 0.328125
carrier_guard_rejections: 384
mode_status: settle_word_gated_candidate_needs_seed_sweep
```

Вывод: первый word-settle candidate появился. Постоянный seed внутри
`OrganState` сделал gated-settle сильнее `Mono192`; carrier guard отсекает bad
carriers. Следующий gate - seed sweep.

Seed sweep 128:

```text
passed_seed_pairs: 3
min_gated_over_best_control: 0.000000
min_gated_ablation_max_drop: 0.156250
min_carrier_integrity_gap: 0.164062
mode_status: settle_word_gated_seed_sweep_partial_3_of_4
```

Следующий шаг: сделать stop-condition устойчивее, чтобы seed-pair tie стал
положительным отрывом.

Ограничения:

```text
словарь 256 байтов
короткий контекст
маленький открытый корпус
без обещания понимания
```

Baseline:

```text
Markov
n-gram
Mono192
6 x Cell32
больший ансамбль Cell32
```

Gate этапа:

```text
генератор лучше random и простого Markov на узком корпусе
trace показывает активные моды
snapshot влияет на продолжение предсказуемо
```

## Этап 8: Chat-0

Цель: минимальный интерактивный бот.

Это не GPT-1 по масштабу. Это проверка цикла:

```text
пользовательский ввод
частотное состояние
генерация ответа
feedback
журнал обучения
ручная консолидация
eval
новая версия клеток
```

Режимы:

```text
observe
adaptive
learn-confirmed
```

Gate этапа:

```text
бот отвечает коротко
состояние можно сохранить
состояние можно восстановить
feedback не меняет веса без подтвержденного обучения
```

Текущий статус: `eval-chat0` закрывает короткий ответ и feedback log на
автоматически проверяемой задаче. `chat0-once` добавляет первый ручной CLI
контур: один prompt, короткий ответ, trace-файл и feedback log.
`eval-chat0-route` добавляет route-quality gate для свободных prompt:
`hybrid_chat0_route.exact_accuracy: 0.851562`, `lock_bank_route_count: 128`,
`mode_status: chat0_route_usable_snapshot_tied_or_better`. Важно:
`lock_bank_over_snapshot: 0.000000`, значит ручной route пригоден, но
превосходство над snapshot не доказано. `chat0-shell` добавляет stdin loop:
каждая строка дает короткий ответ, отдельный trace-файл и feedback log при
формате `<prompt> || <expected>`. `eval-chat0-promote` добавляет первый
controlled promote gate: feedback replay кандидат сравнивается с текущим
replay и route-quality guard. `chat0-promote-save` сохраняет проверенный
`.nwps` overlay, а `chat0-once-promoted` применяет его к точному prompt.
Runtime-веса еще не мутируются; обучение остается явным state-слоем.
`eval-chat0-promoted-holdout` отделяет exact replay от переноса:
`exact_over_base: 0.000000`, `harmonic_transfer_over_base: -0.312500`,
`harmonic_transfer_ablation_max_drop: 0.132812`,
`selective_harmonic_best_over_base: 0.000000`,
`cell_signature_transfer_over_base: -0.648438`,
`trajectory_transfer_over_base: -0.671875`,
`task_hint_over_base: 0.148438`,
`mode_status: chat0_promoted_task_hint_holdout_candidate_not_mode`.
Значит `.nwps` уже воспроизводим, но обобщение пока существует только как
task-hint верхняя граница. Naive harmonic transfer уже проверен и пока хуже
base, хотя его можно разрушить ablation. Confidence threshold снижает вред, но
лучший sweep только возвращается к base. Cell-signature transfer по
`active_cell_ids` хуже base. Trajectory transfer по компактной истории wave
ticks тоже хуже base, поэтому следующий шаг - усилить полезность
клеточного переноса, а не объявлять promoted state модой.

## Что можно делать параллельно

Параллельно можно вести:

```text
глоссарий терминов
заметки по статье Nanda
маленькие fixtures
визуализацию спектра
тесты размера структур
формат snapshot
```

Нельзя параллельно раздувать цель до большой LLM.
