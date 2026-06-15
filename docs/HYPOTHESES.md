# Nando Wave Hypotheses

Этот документ фиксирует проверяемые гипотезы. Он нужен, чтобы проект не
превратился в разговор о "почти чатботе" без понятного критерия успеха.

## Главная гипотеза

Клеточный волновой ансамбль может образовать устойчивую ансамблевую моду,
которой нет у одиночной монолитной клетки того же общего размера памяти.

Главная цель проекта - не предположить это, а доказать или честно опровергнуть
на воспроизводимых экспериментах.

Первый честный контроль:

```text
6 x Cell32 = 192 KB
против
1 x Mono192 = 192 KB
```

Победа не обязана означать "текст стал умным". Для первого этапа победа
означает, что ансамбль дал измеримо более устойчивое состояние на узкой задаче.

## H0: Нулевая гипотеза

Клеточная архитектура не дает преимущества. Любой найденный эффект объясняется
обычной емкостью памяти, случайностью или подбором метрик.

Проект обязан пытаться опровергнуть именно эту гипотезу.

Если `Mono192` стабильно не хуже `6 x Cell32`, а ablation клеток ничего не
разрушает, значит ансамблевая мода не найдена.

## H1: Ансамблевая мода

Для некоторого класса задач `C` существует набор клеток `E`, такой что:

```text
quality(E, C) > quality(best_single_or_monolith, C)
```

или ансамбль дает равное качество, но лучше по безопасности и устойчивости:

```text
false positives ниже
coherence выше
spectral entropy ниже
center-of-mass стабильнее
```

Режим считается найденным только если ablation ключевой клетки разрушает эффект:

```text
quality(E) - quality(E without key cell) >= threshold
```

## H2: Волновая шина важнее голосования

Если заменить wave bus на простое голосование клеток, эффект должен ухудшиться.

Проверка:

```text
6 x Cell32 + voting
против
6 x Cell32 + wave bus
```

Если голосование работает так же хорошо, значит проект пока не доказал именно
волновую природу эффекта.

## H3: Спектральный snapshot работает как память

Короткий спектральный снимок состояния должен восстанавливать поведение лучше,
чем холодный старт.

Проверка:

```text
warm snapshot -> продолжение задачи
cold start    -> та же задача
```

Ожидаемый эффект:

```text
быстрее восстанавливается coherence
меньше шагов до правильной моды
ниже ошибка на продолжении
```

Текущий статус: добавлен `eval-snapshot-memory`. Он проверяет serialized
snapshot через `to_bytes -> from_bytes` и сравнивает его с cold/no snapshot,
wrong snapshot и corrupted snapshot.

```text
snapshot_bytes: 148
warm_snapshot.accuracy: 1.000000
wrong_snapshot.accuracy: 0.015625
corrupted_snapshot.accuracy: 0.000000
warm_over_no_snapshot: 1.000000
warm_over_wrong_snapshot: 0.984375
mode_status: snapshot_memory_passed_state_replay
```

Ограничение: это доказывает replay текущего состояния, но не доказывает
переходную память. Следующий H3-gate должен проверять восстановление
следующего шага после warm start.

Этот следующий gate добавлен как `eval-snapshot-transition`. Он использует
previous snapshot как offset между `CarrierWave phase` и `WaveBus center_phase`
и пытается предсказать следующий wave-state без запуска следующего WaveBus в
predictor.

```text
no_snapshot_transition.accuracy: 0.007812
warm_snapshot_transition.accuracy: 0.000000
wrong_snapshot_transition.accuracy: 0.000000
corrupted_snapshot_transition.accuracy: 0.000000
warm_over_no_snapshot: -0.007812
warm_over_wrong_snapshot: 0.000000
mode_status: not_found_snapshot_transition
```

Вывод: snapshot replay работает, но transition-memory пока не найдена.

После этого добавлен `eval-snapshot-dynamics`: тот же смысл переходной памяти,
но уже на smooth `CarrierWave::advance(...)`, а не на независимых carrier
samples.

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

Вывод: H3 получила первую поддержку для динамической памяти по фазовой ошибке,
но exact accuracy пока низкая.

После этого добавлен `eval-snapshot-multitick`: snapshot берется в момент `t`,
`CarrierWave` движется на 4 smooth-ticks, и только потом проверяется будущий
wave-state.

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

Вывод: H3 получила вторую поддержку по фазовой ошибке уже на горизонте 4
тиков. Это все еще слабая память: exact accuracy почти нулевая. Следующий gate
должен проверять, можно ли превратить фазовое преимущество в точность через
локальную адаптацию или более сильный transition decoder.

## H4: Top-k активация сохраняет качество

На T480 нельзя считать все клетки каждый тик. Поэтому нужна разреженная
активация.

Гипотеза:

```text
top-k активных клеток сохраняют большую часть качества
при резко меньшем CPU cost
```

Минимальные режимы:

```text
top-8
top-16
top-32
top-64
full sweep
```

Если full sweep сильно лучше, а top-k ломает поведение, архитектуру придется
пересматривать.

## H5: Локальная подстройка может улучшать без backprop

Клетка может улучшать свое состояние через локальные правила:

```text
фазовая автоподстройка
резонансное усиление
затухание неиспользуемых частот
спектральный дрейф
```

Но это считается полезным только если после eval качество растет, а не просто
увеличивается уверенность системы.

Первый gate добавлен как `eval-snapshot-adapt`. Он применяет маленькую
онлайн-подстройку фазы после feedback и сравнивает warm snapshot не только с
cold state, но и с такой же адаптацией без snapshot.

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

Вывод: простая глобальная phase-correction не доказывает H5 в нужном смысле.
Она улучшает warm snapshot, но no-snapshot correction пока сильнее. Это
указывает, что следующий H5-gate должен обучать локальное состояние клеток или
transition decoder, а не один общий фазовый bias.

Второй gate добавлен как `eval-snapshot-decoder`. Он использует tiny online
transition decoder с 4 признаками snapshot и сравнивает его с decoder без
snapshot.

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

Вывод: decoder_snapshot лучше warm snapshot и wrong snapshot, но хуже
decoder_no_snapshot. Значит H5 пока не доказана: текущая задача все еще
позволяет сильное объяснение без snapshot. Следующий H5-gate должен менять
задачу или признаки так, чтобы no-snapshot контроль не мог нести основную
информацию.

После этого добавлен `eval-snapshot-keyed`: приборный gate, где target несет
небольшой фазовый ключ из `SpectrumSnapshot`. Это проверяет не весь интеллект,
а конкретный вопрос: может ли serialized snapshot хранить состояние, которого
нет у no-snapshot контроля.

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

Вывод: snapshot-private state существует и проходит roundtrip; wrong/corrupted
snapshot его не заменяют. Ограничение: это keyed-задача, поэтому она не
доказывает H1/H5 полностью.

Следующий gate добавлен как `eval-snapshot-keyed-transition`. Он смешивает
future `WaveBus center_phase` и snapshot-private phase, поэтому snapshot уже
не является ответом сам по себе, а участвует в переходном вычислении.

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

Вывод: скрытое snapshot-состояние можно перенести в transition-задачу, где
future-only, wrong и corrupted controls не объясняют результат. Ограничение:
это все еще synthetic keyed-transition, не финальное доказательство H1.

Затем добавлен `eval-snapshot-noisy-keyed-transition`. Target содержит скрытую
модуляцию из snapshot top-slots, а predictor использует только грубый
snapshot-private phase. Это убирает идеальные 100% и проверяет более
реалистичное преимущество по ошибке.

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

Вывод: snapshot-private state дает частичное, но сильное преимущество в noisy
transition. Это всё еще synthetic gate, но уже не прямое копирование ответа.

Для проверки, что это не один удачный horizon, добавлен
`eval-snapshot-noisy-keyed-transition-sweep` по horizons `1, 2, 4, 8`.

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

Вывод: noisy snapshot-преимущество устойчиво на нескольких горизонтах. Это
усиливает H3/H5 как приборный результат, но все еще не закрывает H1, потому
что задача остается synthetic.

После этого добавлен seed-sweep:
`eval-snapshot-noisy-keyed-transition-seed-sweep`. Он повторяет horizon sweep
на 4 фиксированных seed-парах.

```text
passed_seed_pairs: 4
min_keyed_accuracy_over_future_only: 0.562500
min_error_gain_over_future_only: 13.367188
min_error_gain_over_wrong_snapshot: 4.515625
mode_status: snapshot_noisy_keyed_transition_seed_sweep_passed
```

Вывод: noisy snapshot-преимущество не сводится к одной seed-паре. Это
усиливает robustness, но H1 все еще требует менее synthetic task и сравнения
с Mono192/voting на более прикладной задаче.

Первый менее synthetic bridge добавлен как `eval-byte-context`. Он тренирует
phase-decoders на коротких byte prompts и проверяет holdout против
`mono192_prompt_decoder`, `no_snapshot_decoder`, `cell32_voting`, wrong
snapshot и corrupted snapshot.

```text
snapshot_decoder.accuracy: 0.031250
no_snapshot_decoder.accuracy: 0.046875
mono192_prompt_decoder.accuracy: 0.000000
snapshot_accuracy_over_best_control: -0.015625
snapshot_error_gain_over_best_control: -1.171875
snapshot_error_gain_over_wrong_snapshot: 1.859375
mode_status: not_found_byte_context
```

Вывод: H1 пока не переносится на byte-context. Это важный отрицательный gate:
snapshot-состояние не проигнорировано полностью, но лучшая no-snapshot
контрольная линия пока сильнее.

После отрицательного online-decoder результата добавлен
`eval-byte-context-centroid`: frozen prototype classifier по byte-context
snapshot features.

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

Вывод: H1 получила первый byte-level candidate. Snapshot features образовали
более переносимые кластеры, чем Mono192 prompt centroid, no-snapshot и voting.
Но это пока candidate: нужны seed sweep и ablation, иначе нельзя считать моду
доказанной.

Seed sweep для этого candidate добавлен как
`eval-byte-context-centroid-seed-sweep`.

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_centroid_seed_sweep
```

Вывод: H1 пока не доказана на byte-context. Candidate оказался реальным на
части seed-пар, но не robust. Это защищает проект от ложного вывода: найден
интересный сигнал, но не ансамблевая мода.

Feature ablation добавлен как `eval-byte-context-centroid-ablation`.

Удачная seed-пара `13 -> 97`:

```text
snapshot_centroid_full.accuracy: 0.359375
ablate_snapshot_offset.accuracy: 0.140625
ablate_snapshot_top_sin.accuracy: 0.218750
key_feature: ablate_snapshot_offset
max_accuracy_drop: 0.218750
max_error_increase: 5.031250
mode_status: byte_context_centroid_ablation_sensitive
```

Провалившая seed-пара `29 -> 131`:

```text
snapshot_centroid_full.accuracy: 0.125000
ablate_snapshot_top_cos.accuracy: 0.234375
max_accuracy_drop: 0.000000
mode_status: not_found_byte_context_centroid_ablation
```

Вывод: полезный локальный signal завязан на `snapshot_offset`. Но на другой
seed-паре offset уже не является переносимой модой, а `top_cos` выглядит как
шум. Значит H1 требует стабилизации features до Chat-0.

Offset-only проверка добавлена как
`eval-byte-context-offset-centroid-seed-sweep`.

```text
passed_seed_pairs: 1
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -5.437500
min_error_gain_over_wrong_snapshot: -4.437500
mode_status: not_found_byte_context_offset_centroid_seed_sweep
```

Вывод: H1 не спасается простым выделением `snapshot_offset`. Offset-only
хуже полного centroid по seed robustness. Значит локальный signal требует
устойчивой композиции features, а не одного признака.

Denoised mask `offset + top_sin` проверен через
`eval-byte-context-denoised-centroid-seed-sweep`.

```text
passed_seed_pairs: 2
min_snapshot_accuracy_over_best_control: -0.109375
min_error_gain_over_best_control: -4.953125
min_error_gain_over_wrong_snapshot: -5.656250
mode_status: not_found_byte_context_denoised_centroid_seed_sweep
```

Вывод: H1 пока не подтверждена. Denoised mask лучше offset-only и убирает
часть шума, но остается 2/4 seed-пары, как у полного centroid. Значит нужна
более глубокая стабилизация, а не простая feature-mask.

Relative seed-normalized centroid проверен через
`eval-byte-context-relative-centroid-seed-sweep`.

```text
passed_seed_pairs: 0
min_snapshot_accuracy_over_best_control: -0.203125
min_error_gain_over_best_control: -5.140625
min_error_gain_over_wrong_snapshot: -4.625000
mode_status: not_found_byte_context_relative_centroid_seed_sweep
```

Вывод: H1 не спасается простой нормализацией top-фаз относительно
center/carrier. Это ухудшает результат до 0/4, значит устойчивость надо искать
в формировании snapshot-state/CarrierWave, а не только в координатах
центроидного feature-вектора.

Lexical CarrierWave centroid проверен через
`eval-byte-context-lexical-carrier-centroid-seed-sweep`.

```text
passed_seed_pairs: 4
min_snapshot_accuracy_over_best_control: 0.796875
min_error_gain_over_best_control: 4.937500
min_error_gain_over_wrong_snapshot: 4.796875
mode_status: byte_context_lexical_carrier_centroid_seed_sweep_passed
```

Вывод: H1 получает ограниченную поддержку в режиме carrier-state bridge.
Если глобальная несущая явно получает устойчивый lexical lock, snapshot
становится переносимой памятью для byte-context. Ограничение: lock пока задан
лабораторно, поэтому это не доказательство самоорганизованной ансамблевой
моды; следующий критерий - получить похожий carrier lock через клетки/wave bus
и затем разрушить его ablation.

Cellular CarrierWave centroid проверен через
`eval-byte-context-cellular-carrier-centroid-seed-sweep` и
`eval-byte-context-cellular-carrier-ablation`.

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

Вывод: H1 получила более сильную, но все еще ограниченную поддержку. Lock уже
рождается из набора task-cell resonance detectors и рушится при ablation.
Ограничение: эти lock-клетки пока детерминированные lexical detectors, не
обученные гармонические клетки. Следующий критерий - сделать их trainable и
сравнить с monolith/voting без ручного словаря.

Trained CarrierWave centroid проверен через
`eval-byte-context-trained-carrier-centroid-seed-sweep` и
`eval-byte-context-trained-carrier-ablation`.

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

Вывод: появился первый supervised harmonic lock bank. Это сдвигает H1 от
ручного lexical detector к обучаемым клеткам: фаза `CarrierWave` выставляется
по прототипам, накопленным на train split. Ограничение остается существенным:
текущий trainable gate использует структурную подсказку middle-token. Следующий
критерий - обучаемые lock-клетки на более общем prompt-сигнале без такой
подсказки, затем сравнение с monolith/voting и переход к Chat-0.

Prompt-cloud CarrierWave centroid проверен через
`eval-byte-context-prompt-carrier-centroid-seed-sweep` и
`eval-byte-context-prompt-carrier-ablation`.

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

Вывод: H1 получила более реалистичную поддержку без middle-token подсказки.
Обученный prompt-cloud bank нужен целиком: отключение всего bank сильно снижает
качество. Но отдельные клетки частично избыточны, потому что `min_accuracy_drop`
отрицательный. Для Nando Wave это важный результат: ансамбль может вести себя
как распределенная мода, где не каждая клетка обязана быть одиночно
незаменимой.

После этого добавлен diverse prompt-cloud gate:
`eval-byte-context-prompt-carrier-diverse-centroid-seed-sweep` и
`eval-byte-context-prompt-carrier-diverse-ablation`. Train и holdout используют
разные prompt-шаблоны. Prompt-cloud lock-bank расширен до 32 гармонических
признаков: буквенное облако плюс токенная половина с пониженным весом.

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

Вывод: H1 получила более сильную поддержку на varied prompt forms. Эффект
сохраняется против `Mono192`, voting, random, wrong snapshot и corrupted
snapshot; отключение всего learned bank ломает качество. Следующий критерий -
дать этому lock маленький Chat-0 output loop с feedback, но не считать Chat-0
доказанным до отдельного eval.

## H6: Chat-0 возможен только после приборов

Примитивный чатбот возможен как поздний эксперимент:

```text
byte-level input
byte-level output
short context
spectral snapshot
small corpus
```

Но Chat-0 не должен быть первым доказательством. Сначала нужны задачи, где
видно, что система учится и где можно измерить ошибку.

Первый узкий Chat-0 loop добавлен как `eval-chat0`. Он использует diverse
prompt-cloud lock, восстанавливает mode label через `SpectrumSnapshot`,
генерирует короткий фиксированный ответ и записывает feedback-события для
ошибок. Feedback пока не меняет веса автоматически: это защита от хаотичного
самообучения.

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

Вывод: H6 получила первую инженерную поддержку. Это не открытый chatbot, но
уже проверяемый цикл `input -> CarrierWave -> snapshot -> short response ->
feedback log -> eval`. Следующий критерий - сделать feedback-консолидацию
версируемой и сравнить новую версию клеток с прежней через тот же eval.

Ручной контур добавлен как `chat0-once`: один prompt проходит через
prompt-cloud lock bank, CarrierWave/WaveBus snapshot и line-oriented trace.
Если указан expected response, результат записывается в feedback log. Важно:
ручной route может быть `prompt_cloud_lock_bank`, поэтому он служит интерфейсом
наблюдения и сбора feedback, а не самостоятельным доказательством H6.

Первый прибор для рождения слова добавлен как `eval-settle-word`. Он сравнивает
`Organ192` после 1/3/5/8 settle ticks с `Mono192`, voting, no-carrier,
wrong-carrier, corrupted-carrier и ablation по клеткам.

```text
settle_word_mono192.exact_accuracy: 0.179688
settle_word_organ_one_tick.exact_accuracy: 0.078125
settle_word_organ_settle3.exact_accuracy: 0.000000
settle_word_organ_settle5.exact_accuracy: 0.125000
settle_word_organ_settle8.exact_accuracy: 0.078125
settle_word_organ_stable.exact_accuracy: 0.070312
settle_word_organ_gated.exact_accuracy: 0.328125
settle5_mean_coherence_gain: -0.012189
settle5_mean_entropy_drop: 0.000297
settle5_over_best_control: -0.054688
settle5_ablation_max_drop: 0.000000
stable_over_best_control: -0.109375
stable_ablation_max_drop: 0.070312
gated_mean_selected_ticks: 7.000000
gated_over_best_control: 0.148438
carrier_integrity_gap: 0.328125
carrier_guard_rejections: 384
mode_status: settle_word_gated_candidate_needs_seed_sweep
```

Вывод: H6 получила первый candidate, но еще не финальное доказательство.
Ключевой фикс - постоянный seed организма во время settle-loop. Gated-settle
превысил `Mono192`, carrier integrity стал положительным, ablation сильный.
Следующая проверка - seed sweep.

Seed sweep на 128 cases дал:

```text
passed_seed_pairs: 3
min_gated_over_best_control: 0.000000
min_gated_ablation_max_drop: 0.156250
min_carrier_integrity_gap: 0.164062
mode_status: settle_word_gated_seed_sweep_partial_3_of_4
```

Это подтверждает candidate, но не закрывает H6 как доказанную моду.
readout, а явный OrganState-связанный settle loop с phase velocity decay.

`eval-chat0-route` добавлен как отдельная проверка ручных/free prompt. После
route-train augmentation текущий результат:

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

Вывод для H6: route пригоден для ручного Chat-0 one-shot, но не является новым
доказательством ансамблевой моды, потому что lock-bank связан со snapshot.

`chat0-shell` добавлен как практический контур наблюдения: каждый stdin prompt
получает короткий ответ, отдельный trace-файл и feedback log при формате
`<prompt> || <expected>`. Это инженерно приближает Chat-0, но не меняет научный
статус H6: обучение по feedback не применяется без eval/promote.

`eval-chat0-promote` добавлен как первый controlled feedback gate. Он читает
feedback log, строит точечный feedback-replay кандидат, сравнивает его с текущим
replay и держит `eval-chat0-route` как guard. Это еще не доказанное
самообучение: веса не мутируются, но появился проверяемый шлюз.

`chat0-promote-save` и `chat0-once-promoted` добавляют первый сохраняемый
promoted state: `.nwps` хранит exact feedback overlay и применяется только при
совпадении prompt/train seed/cases. Это доказывает воспроизводимый feedback ->
state -> answer контур, но не доказывает обобщение за пределы точного prompt.

`eval-chat0-promoted-holdout` проверяет это явно. Exact overlay на holdout не
дает прироста (`exact_over_base: 0.000000`). Naive harmonic transfer через
promoted carrier snapshot пока хуже base (`harmonic_transfer_over_base:
-0.312500`), но уже чувствителен к ablation
(`harmonic_transfer_ablation_max_drop: 0.132812`). Selective confidence gate
снижает вред (`selective_harmonic_transfer_over_base: -0.210938`), но threshold
sweep не находит прироста выше base (`selective_harmonic_best_over_base:
0.000000`). Cell-signature transfer по активным Cell32 еще хуже
(`cell_signature_transfer_over_base: -0.648438`), значит простые
`active_cell_ids` не являются достаточной переносимой памятью. Trajectory
transfer по компактной истории нескольких wave ticks тоже хуже base
(`trajectory_transfer_over_base: -0.671875`,
`trajectory_ablation_max_drop: 0.054688`), значит простой centroid по фазовой
траектории пока не является переносимой памятью. Task-hint overlay дает прирост
(`task_hint_over_base: 0.148438`) и получает статус
`chat0_promoted_task_hint_holdout_candidate_not_mode`. Вывод: текущий promoted
state полезен как контролируемая память, но научная задача переносимого
обобщения остается открытой.

## H7: Несущая волна удерживает организм

Глобальная несущая волна должна улучшать устойчивость ансамбля по сравнению с
режимом без нее.

Проверка:

```text
6 x Cell32 + wave bus без carrier
против
6 x Cell32 + wave bus + CarrierWave
```

Ожидаемый эффект:

```text
ниже spectral entropy
стабильнее center of mass
меньше дрейф в неправильные режимы
лучше восстановление snapshot
```

Если CarrierWave ничего не меняет, значит исходная идея глобального аттрактора
пока не доказана.

Текущий статус: добавлен `eval-carrier-control`. На phase-composition probe
правильная несущая выигрывает у выключенной, чужой и поврежденной:

```text
correct_carrier_wave.accuracy: 0.062500
no_carrier_wave.accuracy: 0.000000
wrong_carrier_wave.accuracy: 0.007812
corrupted_carrier_wave.accuracy: 0.000000
mode_status: carrier_control_passed_candidate_mode
```

Ограничение: это подтверждает зависимость текущего кандидата от CarrierWave,
но не закрывает H7 полностью, потому что задача синтетическая и target явно
содержит CarrierWave phase.

Следующий более строгий gate `eval-bus-transfer` уже добавлен. Он не дает
decoder читать `CarrierWave phase` напрямую и проверяет переход на следующий
wave target через `WaveBus center_phase`.

Текущий результат:

```text
correct_carrier_bus.accuracy: 0.007812
wrong_carrier_bus.accuracy: 0.000000
correct_over_best_baseline: 0.000000
correct_over_wrong_carrier: 0.007812
mode_status: not_found_bus_transfer
```

Вывод: H7 пока подтверждена только для статического carrier-dependent probe.
Для динамического перехода нужна память состояния, warm snapshot или явная
переходная динамика между тиками.

## Первые классы задач

Порядок задач должен идти от чистых к грязным:

```text
периодические последовательности
модульное сложение
synthetic byte prediction
RU/EN layout toy task
маленькое продолжение текста
Chat-0 shell
```

## Минимальные метрики

Каждый эксперимент должен писать:

```text
accuracy
loss или error score
false positives
coherence
spectral entropy
center-of-mass stability
top-k active cells
ablation drop
synergy over monolith
CPU time per tick
memory touched per tick
```

## Правило честности

Если метрика не была зафиксирована до эксперимента, она не считается
доказательством. Ее можно добавить как новую метрику для следующего прогона,
но не использовать задним числом как победу.
