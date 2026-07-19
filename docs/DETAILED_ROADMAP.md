# Nando Wave Detailed Roadmap

> **Historical detailed roadmap.** Текущий stage/gate-контракт находится в
> [`../plans/nando-attractor-to-vm-machine-v1/NANDO_ATTRACTOR_TO_VM_ROADMAP_V1.md`](../plans/nando-attractor-to-vm-machine-v1/NANDO_ATTRACTOR_TO_VM_ROADMAP_V1.md).
> Этот файл остаётся доказательной историей и не является authority для новых
> изменений ядра.

Historical detailed roadmap overlay:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Historical P0 at the time:

```text
automatic streaming process:
  L4 opportunity board
  marginal-denominator selector
  bounded HOT/WARM/COLD operator memory
  future-window shadow proof
  verifier-bound promotion
```

Current status:

```text
best shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

not closed:
  product local_accept
  market provider-billing money claim
```

Этот документ - главная защита от расползания проекта. Он задает порядок
работы, входы и выходы этапов, gates, запреты и стоп-сигналы.

Короткая дорожная карта остается в `docs/ROADMAP.md`. Этот файл раскрывает ее
до уровня, по которому можно работать без постоянного пересбора цели.

## Главное правило

Каждый этап отвечает только на один вопрос.

Если во время этапа появляется новая сильная идея, она записывается в
`docs/PARKING_LOT.md`, но не меняет текущий gate.

Переход к следующему этапу разрешен только после одного из двух исходов:

```text
gate_passed
gate_failed_but_documented
```

Плохой исход тоже считается результатом, если он записан и воспроизводим.

## Нельзя перескакивать

Запрещенные прыжки:

```text
сразу к Chat-0
сразу к 1 GB модели
сразу к локальному самообучению
сразу к LLM-дистилляции
сразу к пользовательским приватным логам
```

Причина: такие прыжки дают красивую интерактивность, но ломают проверяемость.

## Единицы проекта

Фиксированные термины:

```text
Cell32   - 32 KB горячая клетка
Expert64 - 2 x Cell32
Organ192 - 6 x Cell32
Mono192  - одиночный монолит 192 KB
CarrierWave - медленная несущая волна
WaveBus  - место интерференции и измерения моды
Snapshot - короткий спектральный снимок состояния
```

Эти термины нельзя менять на ходу. Если термин оказался плохим, создается
отдельное решение в конце этапа.

## Обязательные baseline

Любой значимый результат должен сравниваться минимум с:

```text
random baseline
Mono192
6 x Cell32 без wave bus
6 x Cell32 с wave bus
```

Для текстовых задач дополнительно:

```text
Markov baseline
n-gram baseline
```

Без baseline нельзя писать, что система стала лучше.

## Обязательные метрики

Каждый eval-report должен содержать:

```text
accuracy или task score
loss или error score
false positives, если задача бинарная
coherence
spectral entropy
center-of-mass stability
active cells
top-k setting
ablation drop
synergy over Mono192
CPU time per tick
memory touched per tick
seed
config hash или config dump
```

Если метрики нет в отчете, результат не считается доказательством.

## Этап 0: Рамка проекта

Вопрос этапа:

```text
что именно проверяет Nando Wave?
```

Вход:

```text
идея из README
диалог о Nanda, Фурье, клетках, несущей волне и памяти как snapshot
ограничение T480
```

Работа:

```text
зафиксировать северную звезду
зафиксировать гипотезы
зафиксировать риски
зафиксировать первую архитектурную лестницу
запретить Chat-0 как первый эксперимент
```

Выходные артефакты:

```text
README.md
docs/NORTH_STAR.md
docs/HYPOTHESES.md
docs/ROADMAP.md
docs/DETAILED_ROADMAP.md
docs/RISKS.md
docs/PARKING_LOT.md
```

Gate:

```text
документы существуют
северная звезда явно говорит про частотный организм
roadmap явно запрещает chatbot-first
есть первый контроль 6 x Cell32 против Mono192
```

Стоп-сигналы:

```text
в документах нет CarrierWave
нет Mono192 как контрольного противника
нет правила против расползания
```

## Этап 1: Rust workspace

Статус: реализован.

Вопрос этапа:

```text
можно ли собрать пустой, чистый, проверяемый Rust-стенд?
```

Вход:

```text
завершенный этап 0
нет реализации клеток
нет генератора
```

Работа:

```text
создать Cargo workspace
создать crates/nando-core
создать crates/nando-cli
создать crates/nando-eval
создать минимальную CLI-команду version/status
создать базовый CI-эквивалент локальных checks
```

Не делать:

```text
не писать Cell32
не писать обучение
не писать chatbot
не подключать тяжелые ML-библиотеки
не добавлять Python runtime
```

Выходные артефакты:

```text
Cargo.toml
crates/nando-core
crates/nando-cli
crates/nando-eval
README command для запуска
```

Gate:

```text
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo run -p nando-cli -- status
```

Стоп-сигналы:

```text
workspace уже тянет сложную архитектуру
появился runtime Python
появились тяжелые зависимости без нужды
```

## Этап 2: Форматы памяти и один тик

Статус: начат. Реализованы первый deterministic wave tick без обучения,
стабильный `.nws1` snapshot roundtrip и one-tick eval report без baseline.

Вопрос этапа:

```text
можно ли создать фиксированные структуры и воспроизводимый wave tick?
```

Вход:

```text
пустой Rust workspace
нет обучения
нет задач
```

Работа:

```text
описать Cell32 как фиксированный state packet
описать Mono192 как контрольный state packet
описать CarrierWave
описать WaveBus
описать TickTrace
описать SpectrumSnapshot
реализовать один детерминированный tick
```

Не делать:

```text
не обучать
не генерировать текст
не сравнивать качество
не оптимизировать преждевременно
```

Выходные артефакты:

```text
Cell32 struct
Mono192 struct
CarrierWave struct
WaveBus struct
Snapshot format v0
unit tests размера и roundtrip
```

Gate:

```text
Cell32 имеет проверенный размер
Mono192 имеет проверенный размер
tick с одним seed воспроизводим
snapshot сохраняется и читается
WaveBus считает coherence, entropy и center of mass
```

Стоп-сигналы:

```text
Cell32 содержит heap allocation
размер клетки не контролируется тестом
snapshot стал дампом всей модели
```

## Этап 3: Чистая toy-задача

Статус: начат. Реализован первый synthetic periodic baseline report:
`random`, `Mono192`, `6 x Cell32 без wave bus`, `6 x Cell32 с wave bus`.
Также реализованы voting, ablation sweep и phase-composition probe. Mode
detection пока не доказан полностью.

Вопрос этапа:

```text
видит ли стенд простую гармоническую структуру?
```

Вход:

```text
готов один tick
готовы структуры и метрики
нет обучения
```

Первая задача:

```text
периодическая последовательность
```

Дополнительные задачи только после gate:

```text
phase matching toy task
small modular addition
```

Работа:

```text
создать data/toy с генератором открытых synthetic cases
запустить random baseline
запустить Mono192
запустить 6 x Cell32 без wave bus
запустить 6 x Cell32 с wave bus
сохранить eval report
```

Не делать:

```text
не переходить к тексту
не подбирать задачу под красивый результат
не менять метрики после прогона
```

Gate:

```text
отчет воспроизводится seed-ом
есть все baseline
есть behavioral score
есть spectral metrics
есть CPU time per tick
```

Стоп-сигналы:

```text
wave metrics красивые, но task score не улучшается
нет Mono192 в отчете
нет random baseline
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
`eval-byte-context-cellular-carrier-ablation`.
Критерий
`mode_status: found` еще не достигнут: carrier-control прошел как candidate
gate, delayed bus-transfer пока вернул `not_found_bus_transfer`,
snapshot-memory прошел только как state replay, snapshot-transition пока
вернул `not_found_snapshot_transition`, snapshot-dynamics и
snapshot-multitick прошли по фазовой ошибке, snapshot-adapt пока вернул
`not_found_snapshot_adapt`, snapshot-decoder пока вернул
`not_found_snapshot_decoder`, snapshot-keyed прошел как ограниченный
snapshot-private gate, snapshot-keyed-transition прошел как ограниченный
snapshot-private transition gate, snapshot-noisy-keyed-transition прошел как
менее прямой noisy transition gate,
snapshot-noisy-keyed-transition-sweep прошел как robustness gate.
snapshot-noisy-keyed-transition-seed-sweep прошел как seed robustness gate.
byte-context пока вернул `not_found_byte_context`, byte-context-centroid
прошел как first byte-level candidate gate, но
byte-context-centroid-seed-sweep пока вернул
`not_found_byte_context_centroid_seed_sweep`. На удачной seed-паре
byte-context-centroid-ablation показал чувствительность к `snapshot_offset`,
но на провалившей seed-паре ablation не подтвердила устойчивую feature-моду.
Offset-only seed sweep тоже не найден.
Denoised `offset + top_sin` seed sweep тоже не найден.

Вопрос этапа:

```text
дает ли 6 x Cell32 моду, которой нет у Mono192?
```

Вход:

```text
toy-задача работает
отчеты воспроизводятся
```

Работа:

```text
добавить режим voting
добавить режим wave bus
добавить ablation каждой клетки
добавить synergy calculation
добавить mode_status
```

Сравнения:

```text
Mono192
6 x Cell32 no bus
6 x Cell32 voting
6 x Cell32 wave bus
6 x Cell32 wave bus with ablation
```

Gate pass:

```text
mode_status: found
synergy over Mono192 >= заранее заданного threshold
ablation ключевой клетки снижает score
false positives не растут
```

Gate fail:

```text
mode_status: not_found
результат записан
гипотеза не доказана на этой задаче
```

Стоп-сигналы:

```text
wave bus не отличается от voting
ablation ничего не меняет
Mono192 лучше во всех режимах
```

## Этап 4.5: CarrierWave как аттрактор

Статус: начат. Реализованы `eval-carrier-control` и `eval-bus-transfer`.

Вопрос этапа:

```text
удерживает ли несущая волна организм?
```

Вход:

```text
есть wave bus
есть ансамблевый trace
есть snapshot v0
```

Работа:

```text
сделать CarrierWave явным input/state - сделано
сделать режим without carrier - сделано
сделать режим correct carrier - сделано
сделать wrong carrier - сделано
сделать corrupted carrier - сделано
измерить влияние на entropy и center of mass - сделано
```

Текущий результат:

```text
correct_carrier_wave.accuracy: 0.062500
no_carrier_wave.accuracy: 0.000000
wrong_carrier_wave.accuracy: 0.007812
corrupted_carrier_wave.accuracy: 0.000000
mode_status: carrier_control_passed_candidate_mode
```

Ограничение: это candidate gate, а не финальное доказательство. Следующий
probe должен меньше полагаться на CarrierWave phase прямо в target formula.

Delayed bus-transfer:

```text
correct_carrier_bus.accuracy: 0.007812
wrong_carrier_bus.accuracy: 0.000000
correct_over_best_baseline: 0.000000
correct_over_wrong_carrier: 0.007812
mode_status: not_found_bus_transfer
```

Вывод: без snapshot/warm-state или другой переходной динамики текущий one-tick
организм не предсказывает следующий wave target лучше baseline.

Gate pass:

```text
correct carrier улучшает coherence или снижает entropy
wrong carrier заметно меняет режим
corrupted carrier дает плавную деградацию
carrier входит в snapshot
```

Gate fail:

```text
CarrierWave не влияет на поведение
или работает как обычный bias без волнового trace
```

Стоп-сигналы:

```text
CarrierWave реализована как ручное правило
CarrierWave нельзя испортить экспериментально
CarrierWave не попадает в snapshot
```

## Этап 5: Snapshot как память

Статус: начат. Реализованы `eval-snapshot-memory`,
`eval-snapshot-transition`, `eval-snapshot-dynamics` и
`eval-snapshot-multitick`, `eval-snapshot-adapt`,
`eval-snapshot-decoder`, `eval-snapshot-keyed`.

Вопрос этапа:

```text
может ли спектральный snapshot быть рабочей памятью?
```

Вход:

```text
есть CarrierWave
есть WaveBus metrics
есть успешная или неуспешная toy-задача
```

Работа:

```text
сохранить productive snapshot - сделано для replay
сохранить wrong snapshot - сделано
сделать corrupted snapshot - сделано
сравнить warm start и cold start - сделано для replay
измерить время до восстановления coherence - еще не сделано
проверить transition-memory - сделано, пока not_found
проверить smooth transition dynamics - сделано, passed по phase error
проверить multi-tick state - сделано, passed по phase error
проверить global phase adaptation - сделано, not_found
```

Текущий replay-result:

```text
snapshot_bytes: 148
warm_snapshot.accuracy: 1.000000
wrong_snapshot.accuracy: 0.015625
corrupted_snapshot.accuracy: 0.000000
warm_over_no_snapshot: 1.000000
warm_over_wrong_snapshot: 0.984375
mode_status: snapshot_memory_passed_state_replay
```

Ограничение: это не transition-memory. Следующий gate должен проверить, что
warm snapshot помогает восстановить или предсказать следующий шаг лучше cold
start.

Текущий transition-result:

```text
no_snapshot_transition.accuracy: 0.007812
warm_snapshot_transition.accuracy: 0.000000
wrong_snapshot_transition.accuracy: 0.000000
corrupted_snapshot_transition.accuracy: 0.000000
warm_over_no_snapshot: -0.007812
warm_over_wrong_snapshot: 0.000000
mode_status: not_found_snapshot_transition
```

Вывод: snapshot хранит replay-state, но простая transition-модель через offset
между carrier и center phase не работает. Следующий gate должен вводить
явную переходную динамику или ограниченную локальную адаптацию между тиками.

Текущий dynamics-result:

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

Вывод: smooth CarrierWave дает первую рабочую transition dynamics. Но exact
accuracy низкая, поэтому это не `mode_found`.

Текущий multitick-result:

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

Вывод: snapshot держит слабое фазовое преимущество на горизонте 4 smooth-ticks.
Exact accuracy почти нулевая, поэтому следующий gate должен проверять
локальную адаптацию или более сильный transition decoder.

Текущий adapt-result:

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

Вывод: простая глобальная phase-correction улучшает warm snapshot, но
адаптация без snapshot пока сильнее. Это отрицательный контроль: текущая
адаптация не доказывает клеточную память и не может быть gate pass для Chat-0.

Текущий decoder-result:

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

Вывод: tiny transition decoder извлекает пользу из snapshot относительно
warm-перехода и wrong snapshot, но decoder без snapshot пока сильнее. Это
второй отрицательный контроль: текущая задача или признаки все еще допускают
решение без клеточной памяти.

Текущий keyed-result:

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

Вывод: snapshot-private state проходит no-snapshot, wrong и corrupted
контроли. Ограничение: это keyed-задача; она доказывает наличие переносимого
скрытого состояния, но не доказывает финальную ансамблевую моду.

Текущий keyed-transition-result:

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

Вывод: скрытое snapshot-состояние участвует в transition-задаче вместе с
future WaveBus state. Ограничение: это synthetic keyed-transition, поэтому
следующий gate должен делать задачу менее прямой.

Текущий noisy-keyed-transition-result:

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

Вывод: hidden snapshot modulation снижает идеальность результата, но
snapshot-ветка сохраняет сильное преимущество над future-only, wrong и
corrupted. Это более полезный приборный gate, хотя он все еще synthetic.

Текущий noisy-keyed-transition-sweep-result:

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

Вывод: noisy snapshot advantage держится на нескольких horizons, значит это
не одиночное попадание. Ограничение: synthetic target остается.

Текущий noisy-keyed-transition-seed-sweep-result:

```text
passed_seed_pairs: 4
min_keyed_accuracy_over_future_only: 0.562500
min_error_gain_over_future_only: 13.367188
min_error_gain_over_wrong_snapshot: 4.515625
mode_status: snapshot_noisy_keyed_transition_seed_sweep_passed
```

Вывод: noisy snapshot advantage держится на нескольких seed-парах и horizons.
Это robustness gate, но synthetic target остается.

Gate pass:

```text
warm snapshot быстрее cold start
wrong snapshot уводит в другой режим
corrupted snapshot деградирует плавно
snapshot меньше полного state
```

Gate fail:

```text
snapshot не лучше cold start
snapshot слишком большой
snapshot не влияет на поведение
```

Стоп-сигналы:

```text
snapshot стал полной копией клеток
невозможно повторить восстановление
нет сравнения с cold start
```

## Этап 6: Локальная адаптация

Статус: начат. Реализованы отрицательные контроли `eval-snapshot-adapt`,
`eval-snapshot-decoder` и ограниченные pass-gates `eval-snapshot-keyed`,
`eval-snapshot-keyed-transition`, `eval-snapshot-noisy-keyed-transition`,
`eval-snapshot-noisy-keyed-transition-sweep`,
`eval-snapshot-noisy-keyed-transition-seed-sweep`. Первый переход к
byte-level проверке добавлен как `eval-byte-context`; первый candidate gate -
`eval-byte-context-centroid`; robustness check -
`eval-byte-context-centroid-seed-sweep`; feature diagnostic -
`eval-byte-context-centroid-ablation`; denoising check -
`eval-byte-context-offset-centroid-seed-sweep`; denoised composition check -
`eval-byte-context-denoised-centroid-seed-sweep`; relative seed-normalization
check - `eval-byte-context-relative-centroid-seed-sweep`; lexical carrier-state
bridge - `eval-byte-context-lexical-carrier-centroid-seed-sweep`; cellular
carrier-state bridge -
`eval-byte-context-cellular-carrier-centroid-seed-sweep`; first supervised
harmonic lock bridge -
`eval-byte-context-trained-carrier-centroid-seed-sweep`; full-prompt harmonic
lock bridge - `eval-byte-context-prompt-carrier-centroid-seed-sweep`; diverse
full-prompt lock bridge -
`eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep`.

Вопрос этапа:

```text
может ли клетка улучшаться локально без backprop по всей системе?
```

Текущий trained-lock результат:

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

Ограничение: это supervised lock bank со структурной подсказкой middle-token,
а не общий самообучающийся prompt-organism. Следующий gate должен убрать эту
подсказку или заменить ее более общим harmonic prompt encoder.

Prompt-cloud результат:

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

Вывод: middle-token подсказка убрана. Следующий gate должен увеличить
разнообразие prompt-шаблонов и проверить, что prompt-cloud lock остается
устойчивым до сборки Chat-0 loop.

Diverse prompt-cloud результат:

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

Вывод: prompt-cloud lock выдерживает разные train/holdout шаблоны. Для этого
lock-bank расширен до 32 гармонических признаков: буквенное prompt-cloud
облако плюс токенная половина с пониженным весом. Следующий gate - Chat-0
output loop с feedback и отдельным eval.

Chat-0 output loop добавлен как `eval-chat0`:

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

Settle-word probe добавлен как `eval-settle-word`:

```text
settle_word_mono192.exact_accuracy: 0.179688
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
settle5_over_best_control: -0.054688
settle5_ablation_max_drop: 0.000000
stable_over_best_control: -0.109375
gated_mean_selected_ticks: 7.000000
gated_over_best_control: 0.148438
carrier_integrity_gap: 0.328125
carrier_guard_rejections: 384
mode_status: settle_word_gated_candidate_needs_seed_sweep
```

Вывод: появился первый word-settle candidate. Важный фикс: stateful settle-loop
теперь использует постоянный seed организма, а не пересоздает новый Organ192 на
каждый tick. Gated-settle превосходит `Mono192`, carrier integrity guard
отсекает no/wrong/corrupted carrier, ablation сильный. Следующий v2-шаг -
seed sweep и затем перенос успешного stop-gate ближе к core.

Seed sweep на 128 cases:

```text
seed_pair_0.gated_over_best_control: 0.148438
seed_pair_1.gated_over_best_control: 0.000000
seed_pair_2.gated_over_best_control: 0.093750
seed_pair_3.gated_over_best_control: 0.062500
passed_seed_pairs: 3
min_gated_ablation_max_drop: 0.156250
min_carrier_integrity_gap: 0.164062
mode_status: settle_word_gated_seed_sweep_partial_3_of_4
```

Вывод: candidate устойчив на 3/4 seed-пар, но одна пара только tied with
`Mono192`. Это partial gate, не финальный proof.

Вывод: короткий ответ и feedback log уже проверяются автоматически. Следующий
gate - интерактивный shell с trace-файлом и promote только после eval.

Вход:

```text
есть snapshot
есть baseline и eval
есть rollback состояния
```

Работа:

```text
добавить phase pull - сделано как global phase correction, пока not_found
добавить transition decoder - сделано как tiny online decoder, пока not_found
добавить snapshot-private keyed gate - сделано, passed
добавить snapshot-private transition gate - сделано, passed
добавить noisy snapshot-private transition gate - сделано, passed
добавить noisy horizon sweep - сделано, passed
добавить noisy seed sweep - сделано, passed
добавить amplitude reinforce
добавить decay
добавить drift только в sandbox-режиме
вести training log
сравнивать до и после
```

Режимы:

```text
observe
adaptive-runtime-only
train-confirmed
eval-promote
```

Gate pass:

```text
адаптация улучшает train task
holdout не деградирует выше threshold
можно откатить cell state
eval report показывает, что именно изменилось
```

Gate fail:

```text
адаптация только увеличивает уверенность
качество падает
система закрепляет ошибки
no-snapshot adaptation объясняет эффект лучше snapshot adaptation
no-snapshot decoder объясняет эффект лучше snapshot decoder
```

Стоп-сигналы:

```text
долгая память меняется во время генерации
нет training log
нет rollback
нет holdout
```

## Этап 7: Byte-level prediction

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

Вывод: первый byte-context bridge подключен к eval ladder. Snapshot дает
лучший phase-error, чем wrong snapshot, но проигрывает лучшему контролю.
Chat-0 shell нельзя строить как доказанный принцип до следующего byte gate.

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

Вывод: первый byte-level candidate найден, но это не финальное доказательство.
Следующие обязательные проверки: seed sweep и ablation.

Seed sweep для candidate:

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_centroid_seed_sweep
```

Вывод: candidate не выдержал seed robustness. Это оставляет этап открытым:
нужны устойчивые byte-context features и ablation.

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

Вывод: текущий локальный signal в основном держится на `snapshot_offset`, но
не переносится стабильно на все seed-пары. Следующая работа - стабилизировать
offset-mode или убрать шумные top-phase признаки.

Offset-only result:

```text
passed_seed_pairs: 1
min_error_gain_over_best_control: -5.437500
mode_status: not_found_byte_context_offset_centroid_seed_sweep
```

Вывод: один `snapshot_offset` не является достаточной переносимой модой.
Нужно искать устойчивую композицию offset + ограниченные top-phase признаки.

Denoised `offset + top_sin` result:

```text
passed_seed_pairs: 2
min_error_gain_over_best_control: -4.953125
mode_status: not_found_byte_context_denoised_centroid_seed_sweep
```

Вывод: denoised mask лучше offset-only, но не лучше полного centroid по
robustness. Следующая работа - менять формирование состояния или делать
seed-normalization.

Relative seed-normalized centroid result:

```text
passed_seed_pairs: 0
min_error_gain_over_best_control: -5.140625
mode_status: not_found_byte_context_relative_centroid_seed_sweep
```

Вывод: простая seed-normalization top-фаз относительно center/carrier не
помогла. Следующая работа - менять формирование snapshot-state/CarrierWave, а
не только координаты feature-вектора.

Lexical CarrierWave centroid result:

```text
passed_seed_pairs: 4
min_error_gain_over_best_control: 4.937500
mode_status: byte_context_lexical_carrier_centroid_seed_sweep_passed
```

Вывод: carrier-state bridge прошел seed robustness. Это лабораторный lock, не
финальная ансамблевая мода. Следующий шаг - заменить ручной lexical lock на
клеточный/wave-bus lock и проверить ablation.

Cellular CarrierWave centroid result:

```text
passed_seed_pairs: 4
min_error_gain_over_best_control: 4.937500
mode_status: byte_context_cellular_carrier_centroid_seed_sweep_passed

min_accuracy_drop: 0.101562
mode_status: byte_context_cellular_carrier_ablation_sensitive
```

Вывод: ручной lexical lock заменен на task-cell resonance layer. Это первый
клеточный lock-gate: seed sweep проходит и ablation чувствительна. Следующий
шаг - сделать lock-клетки обучаемыми гармоническими клетками, а не
детерминированным lexical detector.

Вопрос этапа:

```text
может ли частотный организм предсказывать следующий байт лучше простых baseline?
```

Вход:

```text
toy-задачи пройдены или честно провалены
есть snapshot
есть адаптация или зафиксирован отказ от нее
```

Работа:

```text
создать маленький открытый corpus
сделать byte vocabulary 256
сделать короткий context window
сравнить с Markov
сравнить с n-gram
сравнить с Mono192
сравнить с 6 x Cell32 и большим ансамблем
```

Не делать:

```text
не использовать приватные логи
не заявлять понимание
не делать длинный диалог
```

Gate pass:

```text
лучше random
лучше или не хуже простого Markov на узкой задаче
snapshot влияет на продолжение предсказуемо
trace показывает активные моды
```

Gate fail:

```text
Markov лучше во всем
trace не связан с поведением
snapshot не влияет
```

## Этап 8: Chat-0 shell

Вопрос этапа:

```text
можно ли сделать интерактивный цикл без потери научной честности?
```

Вход:

```text
byte-level prediction имеет измеримый результат
есть режимы observe/adaptive/learn-confirmed
есть rollback
```

Работа:

```text
CLI chat loop
короткий ответ
trace file per answer
snapshot save/load
feedback commands
training log
manual consolidation command
eval before promote
```

Запрещено:

```text
веса не меняются автоматически от каждого ответа
нет скрытого обучения на личных логах
нет обещания GPT-уровня
нет network/distributed режима
```

Gate pass:

```text
бот отвечает коротко
ответ имеет trace
snapshot можно сохранить и восстановить
feedback попадает в log
promote возможен только после eval
```

Текущий статус: `eval-chat0` уже доказывает короткий ответ и feedback log на
проверяемой задаче. `chat0-once` реализует первый ручной CLI-ответ с
trace-файлом и записью feedback. `eval-chat0-route` проверяет свободные prompt:
`hybrid_chat0_route.exact_accuracy: 0.851562`, `feedback_log_entries: 19`,
`mode_status: chat0_route_usable_snapshot_tied_or_better`. При этом
`lock_bank_over_snapshot: 0.000000`, поэтому это инженерное подтверждение
ручного route, а не научное доказательство превосходства над snapshot.
`chat0-shell` реализует интерактивный CLI loop через stdin: строка
`<prompt> || <expected>` дает ответ, trace per answer и feedback log.
`eval-chat0-promote` реализует первый promote/eval flow как replay-кандидат:
исправления из feedback log должны улучшить replay и пройти route-quality guard.
`chat0-promote-save` сохраняет прошедшие gate коррекции в `.nwps` state, а
`chat0-once-promoted` применяет этот overlay к точному prompt. Полный pass этапа
8 еще не достигнут: `eval-chat0-promoted-holdout` показывает
`exact_over_base: 0.000000`, то есть exact overlay не обобщает. Отдельный
`harmonic_transfer_overlay` через promoted carrier snapshot дает
`harmonic_transfer_over_base: -0.312500`, то есть naive transfer хуже base, но
ablation уже показывает чувствительность (`harmonic_transfer_ablation_max_drop:
0.132812`). Selective confidence gate дает
`selective_harmonic_transfer_over_base: -0.210938`, а threshold sweep в лучшем
случае возвращается к base (`selective_harmonic_best_over_base: 0.000000`).
Cell-signature transfer по активным Cell32 дает
`cell_signature_transfer_over_base: -0.648438`, поэтому простая подпись
активных клеток отвергнута как переносимый механизм. Trajectory transfer по
компактной истории нескольких wave ticks дает
`trajectory_transfer_over_base: -0.671875`, поэтому простой centroid по фазовой
траектории тоже отвергнут как переносимая память.
`task_hint_overlay` дает `task_hint_over_base: 0.148438` и статус
`chat0_promoted_task_hint_holdout_candidate_not_mode`. Следующий слой должен
заменить task hint на волновой/клеточный механизм обобщения и проверить его
отдельным holdout eval плюс ablation.

Gate fail:

```text
интерактивность есть, но trace бесполезен
feedback меняет веса без контроля
качество нельзя сравнить с baseline
```

## Этап 9: Масштабирование

Вопрос этапа:

```text
как далеко можно масштабировать T480 без потери управляемости?
```

Вход:

```text
есть работающий Chat-0 или byte predictor
есть метрики CPU/memory
```

Лестница:

```text
6 Cell32
64 Cell32
256 Cell32
1024 Cell32
4096 Cell32 только если top-k работает
```

Не делать:

```text
не прыгать сразу к 1 GB active compute
не считать все клетки каждый tick
не оптимизировать без profiler
```

Gate:

```text
CPU time per tick растет предсказуемо
top-k сохраняет качество
memory touched per tick контролируется
full sweep остается диагностикой
```

## Этап 10: Дистилляция

Вопрос этапа:

```text
можно ли передать частотную организацию от teacher-модели?
```

Вход:

```text
есть собственные задачи
есть snapshot
есть baseline
```

Разрешенные teacher-источники:

```text
открытая локальная модель
доступные logits, если они реально доступны
сгенерированные текстовые trajectories
```

Запрещено:

```text
планировать hidden states закрытой LLM как обязательный источник
```

Gate:

```text
teacher улучшает cold start
teacher snapshot можно испортить и измерить деградацию
улучшение видно на holdout
```

## Change control

Любое изменение цели проходит через три вопроса:

```text
это помогает проверить северную звезду?
это относится к текущему этапу?
это имеет gate и baseline?
```

Если хотя бы один ответ "нет", идея идет в `docs/PARKING_LOT.md`.

## Что считать расползанием

Распознать расползание можно по признакам:

```text
появилась новая большая цель без gate
появилась новая библиотека без причины
появился chatbot до byte-level predictor
появилось обучение без holdout
появился красивый график без task score
появилось "почти работает" без baseline
появился distributed/network режим до локального стенда
```

Действие:

```text
остановить этап
записать отклонение
вернуться к текущему gate
```

## Definition of done

Этап завершен, если:

```text
есть артефакты этапа
есть команды проверки
есть отчет или документированный провал
есть seed/config для повторения
нет скрытого расширения scope
следующий этап не начат раньше gate
```

## Definition of not done

Этап не завершен, если:

```text
результат держится только на ощущениях
нет baseline
нет ablation, когда заявлена мода
нет snapshot test, когда заявлена память
нет CPU/memory report, когда заявлена масштабируемость
```

## Первый реализационный коммит

Первый кодовый коммит должен делать только Этап 1:

```text
Cargo workspace
пустые crates
status CLI
минимальные tests
README команд
```

В нем не должно быть:

```text
Cell32 logic
обучения
генерации текста
самообучения
дистилляции
```

Так проект начнет движение без расползания.
