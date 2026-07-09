# Operator Profile Richness Research V1

Дата: 2026-07-09

Цель: проверить, являются ли текущие `.nwpc` профили переносимыми операторами или бедными рефлексами, которые просто ловят узкий поверхностный паттерн.

Это исследовательский отчёт. Он не включает новые `local_accept`, не разрешает money claim и не возвращает `.nwrb` / role-binding backend.

## Что Измеряли

Есть два полезных среза:

1. `latest` — последний свежий segment daemon-а, показывает текущее состояние.
2. `accumulated` — вся накопленная decision history, лучше отвечает на вопрос “профиль устойчив или уже когда-то ошибался”.

Latest window:

- rows: 1309
- candidate_rows: 1309
- accepted_rows: 691
- exact_cache_rows: 541
- false_rows: 1
- total_tokens: 1222006
- profiles_scored: 119

Accumulated history:

- rows: 6725
- candidate_rows: 6725
- accepted_rows: 2981
- exact_cache_rows: 2514
- false_rows: 84
- total_tokens: 5727411
- profiles_scored: 297

Проверки:

- diversity: сколько разных command/cwd/state/surface/evidence/output видел профиль;
- negative exposure: видел ли профиль отрицательные случаи;
- false candidate negatives: срабатывал ли профиль на verifier-negative rows;
- margin robustness: p10 positive candidate margin против p90 negative margin;
- concentration: не сидит ли профиль почти целиком на одном command/state;
- dumb reflex baseline: насколько далеко мы ушли от простого правила `exit_code == 0`.

## Тупой Базовый Рефлекс

На accumulated history, если просто принимать `exit_code == 0`, получилось бы:

- exit_zero_rows: 6620
- exit_zero_accepts: 2981
- exit_zero_false_rows: 69

То есть простой рефлекс ловит много, но ошибается. Это нельзя считать безопасным CPU operator.

## Классы Профилей

Latest window:

- useful_reflex_or_medium_operator: 64
- thin_reflex: 2
- weak_or_unproven: 3
- dangerous_broad_or_unclean: 50

Accumulated history:

- rich_operator_candidate: 19
- useful_reflex_or_medium_operator: 30
- thin_reflex: 3
- weak_or_unproven: 16
- dangerous_broad_or_unclean: 238

Главный вывод: в системе уже есть кандидаты на переносимый оператор, но большинство исторически scored profiles слишком широкие или грязные. Их нельзя продвигать как “мышление”; их надо резать, карантинить или усиливать verifier-negative memory.

## Сильные Кандидаты

Примеры rich operator candidates:

| profile_id | class | positives | negative_seen | false_negative_fires | command_unique | state_unique | p10_pos_margin | p90_neg_margin | tokens |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1931602119 | rich_operator_candidate | 196 | 34 | 0 | 15 | 59 | 76893 | -14864 | 194841 |
| 2690547446 | rich_operator_candidate | 196 | 34 | 0 | 15 | 59 | 76893 | -14864 | 194841 |
| 1571125015 | rich_operator_candidate | 111 | 34 | 0 | 15 | 59 | 133482 | 27132 | 25798 |
| 3113373750 | rich_operator_candidate | 111 | 34 | 0 | 15 | 59 | 133482 | 27132 | 25798 |
| 2416821403 | rich_operator_candidate | 67 | 34 | 0 | 15 | 59 | 165294 | 20966 | 9877 |

Почему это похоже на оператор:

- профиль переносится через разные команды и состояния;
- видел verifier-negative rows;
- не срабатывал на отрицательных candidate rows;
- positive p10 margin остаётся выше negative p90 margin;
- нет концентрации на одном единственном surface pattern.

## Опасные Широкие Профили

Пример проблемы:

- profile_id: 2925751819
- positive_candidate_rows: 729
- false_candidate_negatives: 1
- p10_pos_margin: 79283
- p90_neg_margin: 150440
- margin_gap_p10pos_p90neg: -71157

Это важная находка. Профиль продуктивный по токенам, но слишком широкий: отрицательные случаи могут иметь margin выше, чем нижний край положительных. Такой профиль нельзя считать сильным переносимым оператором без дополнительного split / negative memory / symbiotic gate.

## Что Значит “Бедный Оператор”

Профиль бедный, если:

- даёт accepts только на одном command или одном state signature;
- не видел отрицательных случаев;
- держится на `exit_code == 0`;
- не имеет запаса margin против wrong/negative;
- зависит от конкретного surface, а не от формы перехода.

Такой профиль может быть полезным CPU shortcut, но не доказывает NANDA CPU как compact latent transition runtime.

## Что Нужно Внести В Майнер

Следующий правильный механизм:

1. `operator_richness_score` должен стать частью promotion gate.
2. `dangerous_broad_or_unclean` нельзя держать в final_hot как сильный operator claim.
3. Для широких профилей нужен automatic split по атомам:
   - command kind;
   - shell family;
   - state signature;
   - output size/error band;
   - project/domain family.
4. Добавить verifier-blocked negative memory: если profile fired на negative row, это не просто “ошибка”, а обучающий отрицательный центр.
5. Symbiotic gate должен требовать согласия:
   - hidden_state center;
   - observable_subcenter;
   - verifier-safe evidence.
6. Тонкие рефлексы можно оставлять для экономии, но маркировать как `utility_reflex`, не как `portable_operator`.

## Applied In Code

Статус на 2026-07-09: выводы этого исследования перенесены в cold survivor/promotion path.

Кодовые точки:

- `crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/operator_power.rs`
  - считает `operator_power_score_milli`;
  - классифицирует профиль как `rich_transfer_operator`, `useful_transfer_operator`,
    `thin_reflex_or_low_evidence`, `weak_or_unproven` или
    `dangerous_broad_or_unclean`;
  - выдаёт `operator_power_blocker`, `operator_power_next_auto_action`,
    `operator_power_negative_memory`.
- `crates/nando-cli/src/phase_streaming_cmd/live_store_adapter/survivor_runtime.rs`
  - добавляет `operator_power_*` поля в `clean_candidate_reports`;
  - применяет `live_store_operator_power_allows_product_hot` перед включением
    профиля в product-hot survivor runtime.
- `crates/nando-cli/src/phase_streaming_cmd/live_store_adapter.rs`
  - current/active manifest scoring теперь кредитует только профили, которые
    проходят `operator_power` gate;
  - `product_hot_score_only_active_profile_count` считает power-allowed profiles,
    а не просто все loaded non-quarantined IDs.

Граница:

- это не включает `.nwrb`;
- не включает manual class list;
- не разрешает money claim;
- не включает local accept без verifier/false_accepts=0;
- hot score-loop не раздут JSON/report логикой.

## Итог

Ответ на вопрос “оператор слабый или сильный?”:

- не весь текущий набор сильный;
- 19 профилей на accumulated history выглядят как переносимые phase-center operator candidates;
- 30 профилей полезны, но пока ближе к medium/reflex;
- 238 профилей исторически слишком широкие или грязные и требуют авто-разрезания;
- один `exit_code == 0` не работает: даёт 69 false rows.

Система уже нашла operator-like centers, но майнер должен стать строже: продвигать не самые жирные профили, а профили с переносимостью, отрицательной устойчивостью и нулевым verifier-negative срабатыванием.

Серия `operator_transferability_lab_v1` дополнительно показала:

- stable_profile_count: 72;
- top dangerous profiles checked: 20;
- profiles_with_clean_split: 20;
- min_child_true_accepts: 5.

Полные артефакты:

- `target/nando-wave/research/operator_richness_report_strict.json`
- `target/nando-wave/research/OPERATOR_RICHNESS_REPORT_STRICT.md`
- `target/nando-wave/research/operator_richness_report_latest.json`
- `target/nando-wave/research/OPERATOR_RICHNESS_REPORT_LATEST.md`
- `target/nando-wave/research/operator_richness_report_accumulated.json`
- `target/nando-wave/research/OPERATOR_RICHNESS_REPORT_ACCUMULATED.md`

Следующая серия экспериментов:

- `docs/TRANSFERABLE_OPERATOR_LECTURE_NOTES_V1.md`
- `target/nando-wave/research/operator_transferability_lab_v1.json`
- `target/nando-wave/research/OPERATOR_TRANSFERABILITY_LAB_V1.md`
