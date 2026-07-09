# Transferable Operators Unified V1

Дата: 2026-07-09

Статус: единый документ по переносимым операторам NANDA CPU / Nando Wave.

Этот документ собирает в одно место:

- определение переносимого оператора;
- фазовую математику;
- свойства и шкалу мощности;
- виды операторов;
- результаты текущих экспериментов;
- границы claim;
- требования к майнеру;
- что считать ошибкой или слабым рефлексом.

Сырые источники:

- `docs/NANDA_CPU_COMPACT_LATENT_TRANSITION_ARCHITECTURE.md`
- `docs/HYBRID_SYMBIOTIC_OPERATOR_PROMOTION_CONTRACT.md`
- `docs/STREAMING_OPERATOR_REDUCIBILITY.md`
- `docs/TRANSFERABLE_OPERATOR_LECTURE_NOTES_V1.md`
- `docs/OPERATOR_PROFILE_RICHNESS_RESEARCH_V1.md`
- `target/nando-wave/research/operator_transferability_lab_v1.json`
- `target/nando-wave/research/operator_richness_report_accumulated.json`

## 1. Короткое Определение

Переносимый оператор — это компактная исполняемая память повторяемого перехода состояния:

```text
state_t + action -> state_t+1
```

Он не хранит готовый ответ.

Он хранит форму действия, которая переносится на новые состояния, поверхности и fillers.

## 2. Чем Оператор Не Является

Переносимый оператор не является:

- exact cache;
- lookup table;
- command string;
- target_id;
- proof_rule_id;
- concrete_x_lookup;
- manual local_out_t;
- простым `exit_code == 0`;
- маленькой LLM;
- ответом на конкретный prompt.

Если система запомнила пару:

```text
input -> output
```

это не оператор.

Если система выучила:

```text
как перевести состояние вперёд
```

это кандидат в оператор.

## 3. Главная Формула NANDA CPU

```text
surface_event
  -> hidden_state z_t

action / operator a_t
  -> transition center C(a)

NANDA CPU:
  z_t + C(a) -> z_hat_t+1

verifier:
  z_hat_t+1 соответствует наблюдаемому результату?
```

Или короче:

```text
state_t + action -> state_t+1
```

## 4. Phase-Center Механизм

Каждый наблюдаемый переход кодируется в фазовый вектор:

```text
v_i = phase(state_i, action_i, result_i)
```

Если существует общий инвариант действия `O`, то центр оператора:

```text
center_O = normalize(sum(v_i))
```

Для нового перехода:

```text
margin = score(v_correct, center_O) - score(v_wrong, center_O)
```

Оператор проходит проверку, если:

```text
margin > threshold
```

Смысл:

```text
много переходов
-> фазовый центр масс
-> compact operator profile
```

## 5. Что Хранит Профиль

Профиль должен хранить компактную исполняемую структуру, не сырой trace:

```text
OperatorProfile
|
+-- operator_id
+-- action_id / route_id
+-- positive center vector
+-- negative/background center vector
+-- optional subcenters
+-- margin threshold
+-- trust / drift score
+-- verifier_id
+-- accept counters
+-- reject counters
+-- false_accept counters
+-- saved_tokens estimate
+-- before/after safe hashes or evidence ids
```

Сильный профиль хранит переход.

Слабый профиль хранит поверхностную привычку.

## 6. Условия Сводимости

Оператор сводим к компактному phase-center, если:

1. действие повторяется;
2. fillers/surface/noise меняются;
3. форма перехода сохраняется;
4. state/action/result кодируются в общие phase atoms;
5. correct transitions имеют низкую внутриклассовую дисперсию;
6. wrong/negative transitions отделяются margin-ом;
7. heldout/future окно проходит;
8. verifier подтверждает безопасность;
9. ablation механизма ломает результат.

Не сводимо, если:

- нет повторяемой формы;
- нет verifier;
- wrong не отделяется;
- margin нестабилен;
- центр зависит от конкретного токена;
- профиль работает только как lookup;
- профиль даёт false_accepts.

## 7. Оси Мощности Оператора

Мощность профиля нельзя мерить только токенами.

Основные оси:

| ось | вопрос | хороший сигнал |
|---|---|---|
| Support | сколько раз переход повторился | много verified positives |
| Portability | переносится ли на разные формы | разные commands/states/projects |
| Negative separation | видел ли wrong/unsafe | не сработал на verifier-negative |
| Margin | есть ли запас | p10 positive > p90 negative |
| Temporal stability | живёт ли во времени | несколько clean windows |
| Symbiotic agreement | согласны ли скрытое и явное | hidden_state + observable_subcenter |
| Verifier safety | есть ли внешний факт | false_accepts = 0 |
| Drift | не уехал ли центр | bounded drift |
| Token value | есть ли польза | tokens saved после safety |

### 7.1 Applied Operator-Power Gate

Статус на 2026-07-09: шкала мощности оператора применена в коде live miner-а.

Реализация:

- `live_store_adapter/operator_power.rs` считает `operator_power_score_milli`,
  класс профиля, blocker, negative-memory status и next auto action.
- `survivor_runtime.rs` применяет `operator_power` перед включением профиля в
  product-hot survivor runtime.
- текущий active/runtime accounting кредитует только профили, проходящие
  `operator_power` gate, `phase trust`, quarantine gate и symbiotic scoring.

Это cold-path selector/promotion logic. Hot path остаётся:

```text
route_id -> phase vector -> preloaded .nwpc -> margin -> accept/fallback
```

То есть operator-power не добавляет JSON, строки или отчётность в core score-loop.

### 7.2 Скрытый Оператор

Скрытый оператор — это не отдельная магия поверх обычного оператора.

Это оператор, у которого основная переносимая форма живёт не в видимой строке,
а в скрытом переходе состояния:

```text
z_t + action -> z_t+1
```

Видимая поверхность может быть разной:

```text
grep output
cargo stdout
git status
planner note
tool result
```

но скрытая форма перехода одна:

```text
состояние стало понятнее
ошибка классифицирована
следующий безопасный шаг выбран
граница правки найдена
риск стал высоким -> fallback
```

Скрытый оператор сильнее поверхностного рефлекса, если:

- он переносится между разными surface forms;
- его hidden_state центр стабилен;
- observable_subcenter подтверждает, что это не галлюцинация;
- verifier проверяет результат;
- negative memory отделяет похожие, но опасные случаи.

Формула:

```text
hidden_operator =
  hidden transition center
  + observable evidence
  + verifier boundary
```

Без observable/verifier части скрытый оператор легко превращается в красивую
галлюцинацию. Поэтому целевой класс не просто `hidden_operator`, а:

```text
symbiotic hidden + observable + verifier operator
```

### 7.2 Цепочка Операторов

Один оператор переносит один переход.

Цепочка операторов переносит траекторию:

```text
state_0
  + operator_1 -> state_1
  + operator_2 -> state_2
  + operator_3 -> state_3
```

Или короче:

```text
OperatorChain =
  O1 ; O2 ; O3 ; ... ; On
```

Цепочка нужна там, где полезное действие не укладывается в один переход:

- прочитать состояние;
- распознать скрытую проблему;
- выбрать правку;
- применить правку;
- проверить результат;
- если надо, продолжить.

Сила цепочки — это не сумма красивых операторов.

Сила цепочки зависит от:

| ось | вопрос | хороший сигнал |
|---|---|---|
| Step validity | каждый шаг доказан? | verifier между шагами |
| Composition | выход шага подходит входу следующего? | state_i согласован с operator_i+1 |
| Drift | ошибка не накапливается? | bounded cumulative drift |
| Recovery | цепь умеет остановиться? | fallback при низком margin |
| Bottleneck | есть слабое звено? | min step trust высокий |
| Goal progress | цепь реально идёт к цели? | distance_to_goal падает |

Главный закон цепочки:

```text
chain_power <= weakest_verified_link
```

Если один шаг не имеет verifier или negative separation, вся цепочка не может
быть product-grade, даже если остальные шаги сильные.

Правильная цепочка:

```text
operator_1
-> verifier/collimator
-> operator_2
-> verifier/collimator
-> operator_3
-> verifier/collimator
```

Неправильная цепочка:

```text
operator_1 -> operator_2 -> operator_3 -> accept
```

без промежуточной проверки. Это может дать длинную красивую траекторию, но не
доказанный переносимый reasoning.

## 8. Шкала Мощности

| уровень | имя | смысл | reasoning claim |
|---:|---|---|---|
| 0 | weak / unproven | мало данных, нет отрицательной проверки | нет |
| 1 | thin_reflex | узкая реакция на один surface/state pattern | нет |
| 2 | utility_reflex | экономит CPU-токены, но слабо переносится | нет |
| 3 | medium_operator | работает на нескольких формах, но proof неполный | частично |
| 4 | rich_operator_candidate | переносится, имеет margin и negative separation | да, как candidate |
| 5 | symbiotic_verified_operator | hidden + observable + verifier, false_accepts=0 на future window | да, product-grade |

### 8.1 Шкала Мощности Цепочки

| уровень | имя | смысл | chain claim |
|---:|---|---|---|
| 0 | broken_chain | хотя бы один шаг не проверен или даёт false | нет |
| 1 | scripted_sequence | фиксированный порядок действий без переноса | нет |
| 2 | useful_workflow_reflex | полезная повторяемая цепочка, но узкая | нет |
| 3 | verified_short_chain | 2-3 шага, verifier между шагами | частично |
| 4 | portable_operator_chain | 3-10 шагов, перенос между поверхностями/проектами | да, candidate |
| 5 | goal-directed verified chain | L4 выбирает цепь под цель, drift bounded, false_accepts=0 | product-grade |

Цепочка становится сильной не от длины.

Слишком длинная цепочка без collimator хуже короткой.

Сильная цепочка:

```text
short enough
verified at each step
bounded drift
clear goal progress
fallback on uncertainty
```

Слабая цепочка:

```text
long
unverified
accumulates error
cannot stop
explains itself after the fact
```

## 9. Виды Переносимых Операторов

### 9.1 File Inspection Transition

Форма:

```text
прочитал файл / grep / sed / ls
-> следующий шаг понятен
```

Текущие числа:

- rows: 2091
- true: 913
- false: 16
- tokens_true: 1151267

Вывод: большой полезный класс, но широкий parent опасен. Нужны clean subcenters.

### 9.2 Agent Planning Continuation

Форма:

```text
tool завершился успешно
-> можно продолжить ближайший плановый шаг
```

Текущие числа:

- rows: 2895
- true: 1448
- false: 7
- tokens_true: 1652216

Вывод: самый жирный класс по токенам. Но нельзя принимать весь parent целиком.

### 9.3 Rust Check/Test Transition

Форма:

```text
cargo check/test/fmt/clippy
-> понять статус
-> выбрать следующий безопасный шаг
```

Текущие числа:

- rows: 464
- true: 210
- false: 4
- tokens_true: 267243

Вывод: хороший verifier-bound класс, потому что stdout/exit code являются внешним проверяемым фактом.

### 9.4 Git State Transition

Форма:

```text
git diff/status
-> понять состояние рабочей копии
```

Текущие числа:

- rows: 397
- true: 168
- false: 4
- tokens_true: 211663

Вывод: полезный инженерный класс, но нужен строгий verifier по `git status/diff`.

### 9.5 Failure Triage Transition

Форма:

```text
ошибка / nonzero / warning
-> repair или fallback
```

Текущие числа:

- rows: 988
- true: 291
- false: 53
- tokens_true: 333131

Вывод: пока слабый и опасный класс. Его нельзя широко продвигать. Нужны узкие verifier-bound подсемейства.

## 10. Экспериментальная Картина

### 10.1 Richness Snapshot

Accumulated history:

- rows: 6725
- candidate_rows: 6725
- accepted_rows: 2981
- exact_cache_rows: 2514
- false_rows: 84
- total_tokens: 5727411
- profiles_scored: 297

Классы:

- rich_operator_candidate: 19
- useful_reflex_or_medium_operator: 22
- thin_reflex: 3
- weak_or_unproven: 17
- dangerous_broad_or_unclean: 238

Вывод: operator-like centers уже есть, но большинство исторических профилей слишком широкие или грязные.

### 10.2 Dumb Reflex Baseline

Если просто принимать `exit_code == 0`:

- accepted: 6730
- true: 3030
- false: 69
- precision_milli: 450

Это не безопасный CPU operator.

### 10.3 Current CPU Accept

Текущий verifier/admission слой в исследованном срезе:

- accepted: 3030
- true: 3030
- false: 0
- tokens_true: 3615520

Это доказывает не то, что каждый профиль умный, а то, что admission/verifier слой защищает CPU accept.

### 10.4 Split Experiment

Для топ-20 `dangerous_broad_or_unclean`:

- checked: 20
- profiles_with_clean_split: 20

Вывод: многие грязные профили не надо выбрасывать. Их нужно автоматически разрезать на clean child centers.

### 10.5 Temporal Stability

- stable_profile_count: 69
- несколько профилей сохраняют `rich/useful` статус в 6 блоках из 8

Вывод: часть сигнала стабильна во времени, это не только одноразовая вспышка.

## 11. Rich Operator vs Reflex

Reflex:

```text
если exit_code == 0 -> accept
если sed -> accept
если похоже на прошлое -> accept
```

Operator:

```text
форма перехода повторяется
на разных командах
на разных состояниях
с negative evidence
с margin
с verifier
без false_accepts
```

## 12. Broad Parent И Clean Child

Грязный широкий профиль:

```text
broad parent center
```

смешивает несколько разных действий. Он может экономить много токенов, но внутри него несколько операторов.

Правильная процедура:

```text
broad center
-> detect false/risk
-> split by atoms
-> create clean child center
-> add verifier-blocked negative memory
-> shadow future rows
-> promote only clean child
```

Разрезающие atoms:

- command kind;
- shell family;
- state signature;
- output size/error band;
- project/domain family;
- observable evidence;
- hidden state subcenter.

## 13. Symbiotic Operator

Самая сильная форма профиля:

```text
hidden_state center
+ observable_subcenter
+ verifier
= symbiotic_verified_operator
```

Роли:

- hidden_state даёт переносимость;
- observable_subcenter страхует от широкой галлюцинации;
- verifier не даёт принять неправильный переход.

### 13.1 Гибридный Симбиотический Оператор

Гибридный симбиотический оператор — это оператор, который держится сразу на
нескольких разных источниках истины:

```text
HybridSymbioticOperator =
  hidden transition center
  + explicit observable operator
  + verifier binding
  + negative memory
```

Он отличается от обычного скрытого оператора тем, что не доверяет только
внутреннему центру.

Он отличается от observable/reflex оператора тем, что не сводится к видимому
шаблону.

Он отличается от verifier-only правила тем, что переносит форму перехода, а не
просто проверяет итог.

Роли частей:

```text
hidden center
  говорит: какая скрытая форма перехода повторяется

observable operator
  говорит: какой внешний след подтверждает этот переход

verifier
  говорит: результат действительно безопасен

negative memory
  говорит: где похожий переход применять нельзя
```

Пример формы:

```text
tool output / cargo stdout / git state / planner event
  -> L2 hidden state
  -> L3 transition center
  -> observable evidence check
  -> verifier
  -> CPU accept или fallback
```

Такой оператор силён именно как гибрид:

| часть | что даёт | что ломается без неё |
|---|---|---|
| hidden center | переносимость и смысл перехода | остаётся surface-reflex |
| observable operator | привязка к внешнему следу | скрытый центр может галлюцинировать |
| verifier | проверка результата | высокий риск false_accept |
| negative memory | границы применения | broad parent снова начинает ошибаться |

Хранить нужно не сырой trace, а компактный профиль:

```text
HybridSymbioticProfile
|
+-- operator_id
+-- hidden_center
+-- observable_subcenter
+-- verifier_id
+-- positive_center
+-- negative_center / anti-center
+-- margin thresholds
+-- drift score
+-- trust score
+-- accept/reject/false_accept counters
+-- evidence ids / safe hashes
```

Promotion возможен только если:

```text
hidden margin passes
observable evidence agrees
verifier passes
negative memory rejects wrong
false_accepts = 0
future window stays clean
```

Если hidden часть сильная, но observable/verifier не согласны, это:

```text
latent_candidate
```

Если observable часть сильная, но hidden переносимости нет, это:

```text
utility_reflex
```

Если verifier есть, но нет переносимого центра, это:

```text
checked_rule
```

Целевой класс:

```text
hybrid_symbiotic_verified_operator
```

Короткая формула:

```text
hidden понял переход
observable увидел след
verifier подтвердил факт
negative memory запретила похожую ошибку
= CPU-safe transferable action
```

## 14. Negative Memory

Отрицательный пример — это не просто ошибка.

Это материал для анти-центра:

```text
if profile fired on verifier-negative row:
  add negative evidence
  raise threshold or split
  block parent promotion
```

Хороший майнер должен помнить:

- какие профили пытались принять wrong;
- какой margin был у wrong;
- какие atoms отличают wrong от correct;
- какой child center можно выделить без false.

## 15. Drift И Collimator

Цепочка операторов ведёт себя как луч переходов:

```text
operator_1 -> operator_2 -> operator_3
```

Малая фазовая ошибка на каждом шаге может накопиться как drift.

Verifier работает как collimator:

```text
если переход ушёл с оси -> остановить / поправить / fallback
```

Без verifier длинная цепочка может уйти в красивую, но неверную траекторию.

### 15.1 Что Хранит Chain Profile

Одиночный `OperatorProfile` хранит компактный переход.

`OperatorChainProfile` хранит проверенную композицию переходов:

```text
OperatorChainProfile
|
+-- chain_id
+-- goal_state_signature
+-- step profiles:
|   |
|   +-- operator_id_1
|   +-- operator_id_2
|   +-- operator_id_3
|
+-- transition compatibility matrix
+-- cumulative drift budget
+-- per-step margin thresholds
+-- verifier checkpoints
+-- stop/fallback conditions
+-- positive chain traces
+-- negative/broken chain traces
+-- accepted_count
+-- rejected_count
+-- false_accept_count
+-- saved_tokens estimate
```

Это не список команд.

Это память формы траектории:

```text
какие скрытые состояния должны следовать друг за другом
какие переходы совместимы
где нужно остановиться
где verifier обязан коллимировать луч
```

### 15.2 Мощность По Цепочке Действий

Иногда один оператор выглядит слабым, но цепочка сильна.

Пример:

```text
inspect file
-> identify diagnostic class
-> choose patch operator
-> apply in temp copy
-> run verifier
```

Каждый шаг сам по себе может быть простым.

Но вся цепочка переносит полезное действие:

```text
грязное Rust-состояние -> проверенное исправленное состояние
```

Такой объект сильнее отдельного шага, если доказано:

```text
same chain form
different files/projects/surfaces
verifier between steps
negative broken chains seen
cumulative drift bounded
final state verified
```

Именно поэтому надо мерить не только:

```text
operator_accuracy
```

но и:

```text
chain_accepts
chain_false_accepts
chain_drift
chain_goal_progress
chain_tokens_saved
weakest_step_margin
```

## 16. Online Miner

Разрешённый процесс:

```text
real event stream
-> extract safe state/action/result atoms
-> encode phase vector
-> route into rough bucket
-> update positive/negative sums
-> monitor coherence / variance / p10_margin
-> candidate operator
-> quarantine .nwpc
-> shadow future traffic
-> promote only if verifier + false_accepts=0
```

Запрещено:

- online local_accept без verifier;
- manual class picking как product claim;
- `.nwrb`/role-binding backend как commercial path;
- target/proof authority;
- concrete lookup;
- fake savings from synthetic-only trace.

## 17. Promotion Gate

Профиль можно продвигать только если:

```text
support >= threshold
negative_seen >= threshold
false_accepts = 0
false_candidate_negatives = 0
p10_positive_margin > p90_negative_margin
temporal stability passes
verifier exists
exact-cache overlap excluded
future shadow split passes
runtime parity passes
```

Если профиль экономит токены, но не проходит это, он остаётся:

```text
utility_reflex
или
quarantined broad parent
```

### 17.1 Chain Promotion Gate

Цепочку можно продвигать только если:

```text
all steps have verifier or verified compatibility
false_accepts = 0
chain_false_accepts = 0
weakest_step_margin >= threshold
cumulative_drift <= drift_budget
goal_progress > 0 on heldout/future rows
broken-chain negatives are rejected
exact-cache overlap excluded
future shadow split passes
```

Если одиночные шаги сильные, но композиция ломается, цепочка остаётся:

```text
unproven_operator_chain
```

Если цепочка экономит много токенов, но не имеет verifier между шагами, она
остаётся:

```text
workflow_reflex
```

Product-grade chain:

```text
portable_operator_chain
+ verifier checkpoints
+ negative chain memory
+ bounded drift
+ false_accepts=0
```

## 18. Главный Закон

Нельзя говорить:

```text
профиль экономит много токенов -> профиль умный
```

Правильно:

```text
профиль экономит токены
+ переносится
+ отделяет wrong
+ стабилен во времени
+ имеет verifier
+ false_accepts = 0
= переносимый оператор
```

## 19. Что Должен Делать Майнер Дальше

Майнер должен стать не сборщиком жирных bucket-ов, а фабрикой clean operators:

1. находить broad parent center;
2. измерять operator richness;
3. искать split atoms;
4. создавать clean child centers;
5. добавлять negative memory;
6. измерять drift/trust;
7. проверять future window;
8. собирать verifier-safe operator chains;
9. продвигать только verifier-safe children или verifier-safe chains.

Короткая формула:

```text
find repeat
-> build center
-> attack with negatives
-> split if broad
-> verify
-> promote clean child
```

Для цепочек:

```text
find repeated trajectory
-> verify every step
-> measure drift
-> attack with broken-chain negatives
-> promote only bounded verified chain
```

## 20. Текущий Вердикт

1. Идея phase-center operator не выглядит слабым местом.
2. Слабые места сейчас:
   - extraction quality;
   - broad parent mixing;
   - verifier coverage;
   - negative memory;
   - automatic split/promotion discipline.
3. В системе уже есть переносимые operator candidates.
4. Не все текущие профили являются сильными операторами.
5. Самая важная дорога:

```text
broad center
-> auto split
-> clean child center
-> negative memory
-> symbiotic hidden+observable gate
-> verifier-safe promote
```

## 21. Мини-Глоссарий

`state_t`  
Текущее скрытое состояние.

`action`  
Действие или оператор, который должен перевести состояние.

`state_t+1`  
Следующее состояние после действия.

`phase vector`  
Фазовое кодирование перехода.

`center`  
Нормализованный центр масс похожих переходов.

`margin`  
Запас между correct и wrong / negative.

`hidden_state`  
Скрытая форма состояния.

`observable_subcenter`  
Явный наблюдаемый подцентр: command, stdout shape, exit code, evidence.

`verifier`  
Внешняя проверка, которая не даёт принять ошибку.

`broad parent`  
Слишком широкий профиль, смешавший несколько правил.

`clean child`  
Разрезанный профиль с нулевыми false accepts.

`utility_reflex`  
Полезный CPU shortcut без полноценного reasoning claim.

`symbiotic_verified_operator`  
Целевой класс: hidden + observable + verifier.

`hybrid_symbiotic_operator`  
Гибридный оператор, который объединяет hidden center, observable operator,
verifier binding и negative memory.

`hidden_operator`  
Оператор, чья переносимая форма живёт в скрытом переходе `z_t -> z_t+1`,
а не в одной видимой строке.

`operator_chain`  
Композиция операторов, которая переносит траекторию из нескольких переходов.

`chain_drift`  
Накопленная фазовая ошибка по цепочке.

`collimator`  
Verifier или checkpoint, который не даёт цепочке уехать с правильной оси.

`weakest_verified_link`  
Самое слабое проверенное звено цепи. Верхняя граница силы всей цепочки.

## 22. One-Screen Summary

```text
Переносимый оператор =
  compact phase center повторяемого перехода
  state_t + action -> state_t+1

Сильный оператор =
  support
  + portability
  + negative separation
  + margin
  + temporal stability
  + verifier
  + false_accepts=0

Слабый оператор =
  жирный shortcut без negative proof
  или broad parent без split

Скрытый оператор =
  hidden transition center
  + observable evidence
  + verifier boundary

Гибридный симбиотический оператор =
  hidden center
  + observable operator
  + verifier
  + negative memory

Сильная цепочка =
  несколько переносимых переходов
  + verifier между шагами
  + bounded drift
  + weakest link не ломает claim

Правильный майнер =
  broad center
  -> clean child
  -> negative memory
  -> verifier-safe child/chain promote
```
