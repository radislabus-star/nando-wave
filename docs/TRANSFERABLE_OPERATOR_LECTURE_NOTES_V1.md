# Transferable Operator Lecture Notes V1

Дата: 2026-07-09

Единый документ:

```text
docs/TRANSFERABLE_OPERATORS_UNIFIED_V1.md
```

Источник экспериментов:

- `target/nando-wave/research/operator_transferability_lab_v1.json`
- `target/nando-wave/research/OPERATOR_TRANSFERABILITY_LAB_V1.md`
- `target/nando-wave/research/operator_richness_report_accumulated.json`

## Главная Формула

Переносимый оператор — это не ответ и не command string.

Переносимый оператор — это compact phase center повторяемого перехода:

```text
state_t + action -> state_t+1
```

Если один и тот же переход повторяется на разных состояниях, поверхностях и командах, он может свестись к компактному центру.

## Мощность Оператора

Мощность профиля нельзя мерить только токенами. Жирный профиль может экономить много, но быть опасным широким рефлексом.

Нормальная шкала:

| уровень | имя | смысл | можно считать reasoning claim |
|---:|---|---|---|
| 0 | weak/unproven | мало данных или нет отрицательной проверки | нет |
| 1 | thin_reflex | полезная реакция на узкий surface/state pattern | нет |
| 2 | utility_reflex | экономит CPU-токены, но слабая переносимость | нет |
| 3 | medium_operator | работает на нескольких формах, но ещё мало stability/negative evidence | частично |
| 4 | rich_operator_candidate | переносится через разные commands/states, имеет margin и negative separation | да, как candidate |
| 5 | symbiotic_verified_operator | hidden center + observable evidence + verifier, false_accepts=0 на future window | да, product-grade |

## Оси Мощности

1. Support  
   Сколько раз переход повторился.

2. Portability  
   Сколько разных команд, состояний, поверхностей и проектов профиль пережил.

3. Negative separation  
   Видел ли профиль неправильные/опасные случаи и не сработал ли на них.

4. Margin  
   Нижний край правильных (`p10 positive margin`) должен быть выше верхнего края неправильных (`p90 negative margin`).

5. Temporal stability  
   Профиль должен жить в нескольких временных окнах, а не вспыхнуть случайно.

6. Symbiotic agreement  
   Лучший класс: hidden_state и observable_subcenter согласны, verifier подтверждает.

7. Token value  
   Экономия важна, но только после safety.

## Виды Переносимых Операторов

### 1. File Inspection Transition

Пример:

```text
прочитал файл / grep / sed -> следующий шаг понятен
```

Сейчас:

- rows: 2091
- true: 913
- false: 16
- tokens_true: 1151267

Вывод: большой коммерческий класс, но требует разрезания. `sed_zero` сам по себе даёт false.

### 2. Agent Planning Continuation

Пример:

```text
tool завершился успешно -> можно продолжить ближайший плановый шаг
```

Сейчас:

- rows: 2895
- true: 1448
- false: 7
- tokens_true: 1652216

Вывод: самый жирный класс. Но широкий родитель опасен; нужны subcenters.

### 3. Rust Check/Test Transition

Пример:

```text
cargo check/test/fmt/clippy -> понять статус и следующий безопасный шаг
```

Сейчас:

- rows: 464
- true: 210
- false: 4
- tokens_true: 267243

Вывод: хороший кандидат под verifier-bound профиль, потому что внешний факт проверяем stdout/exit code.

### 4. Git State Transition

Пример:

```text
git diff/status -> понять состояние рабочей копии
```

Сейчас:

- rows: 397
- true: 168
- false: 4
- tokens_true: 211663

Вывод: полезный инженерный operator class, но нужен строгий verifier по `git status/diff`.

### 5. Failure Triage Transition

Пример:

```text
ошибка / nonzero / warning -> выбрать repair/fallback
```

Сейчас:

- rows: 988
- true: 291
- false: 53
- tokens_true: 333131

Вывод: пока слабый и опасный. Не продвигать широко. Нужны отдельные verifier-bound подсемейства.

## Что Доказали Эксперименты

### Простой рефлекс не годится

`exit_code == 0`:

- accepted: 6730
- true: 3030
- false: 69
- precision_milli: 450

Он ловит много, но ошибается. Это не безопасный CPU operator.

### Текущий verifier-bound CPU accept чистый

`current_cpu_accept`:

- accepted: 3030
- true: 3030
- false: 0
- tokens_true: 3615520

Это не значит “всё уже strong operator”. Это значит: текущий admission/verifier слой защищает accept.

### Грязные родители режутся

Из топ-20 `dangerous_broad_or_unclean`:

- checked: 20
- profiles_with_clean_split: 20

Это сильнейшая практическая находка.

Значит часть “бедности” не в phase-center идее, а в слишком широком bucket/profile. Майнер должен автоматически доращивать clean child centers.

### Стабильные кандидаты есть

Temporal stability:

- stable_profile_count: 69
- несколько профилей сохраняют `rich/useful` статус в 6 блоках из 8

Это значит, что сигнал не только одноразовый.

## Главная Ошибка, Которую Нельзя Делать

Нельзя говорить:

```text
профиль экономит много токенов -> профиль умный
```

Правильно:

```text
профиль экономит токены
+ имеет negative separation
+ имеет margin
+ переносится
+ стабилен во времени
+ проходит verifier
= сильный оператор
```

## Что Должен Делать Майнер

Майнер должен не просто собирать жирные центры.

Он должен:

1. находить broad parent center;
2. проверять false/risk;
3. автоматически искать split atoms;
4. создавать clean child centers;
5. хранить verifier-blocked negative memory;
6. измерять operator richness;
7. promote только child/profile с:
   - false_accepts = 0;
   - positive margin gap;
   - temporal stability;
   - verifier-safe evidence.

## Короткая Лекция

NANDA CPU не должен быть кэшем и не должен быть маленькой LLM.

Он должен быть процессором переносимых переходов:

```text
наблюдаем состояние
кодируем hidden/observable atoms
видим повторяемую форму действия
собираем phase center
проверяем wrong/negative separation
разрезаем широкий центр на clean subcenters
исполняем только verifier-safe transition
```

Мощный оператор — это не тот, который много угадывает.  
Мощный оператор — это тот, который переносит форму перехода и не срабатывает на неправильный переход.

## Текущий Вердикт

Да, в системе уже есть переносимые operator candidates.

Нет, не все текущие профили являются сильными операторами.

Главный путь усиления:

```text
broad center
-> auto split
-> clean child center
-> negative memory
-> symbiotic hidden+observable gate
-> verifier-safe promote
```

Это и есть практическая дорога от “экономит токены” к “учит переносимые действия”.
