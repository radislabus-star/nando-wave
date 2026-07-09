# Nando Wave Risks

Current risk overlay:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current biggest risks:

```text
1. Treating shadow compression as product local accept.
2. Treating placeholder token/cost estimates as market money.
3. Selecting pretty buckets instead of marginal denominator delta.
4. Reintroducing source-specific hardcode into generic core.
5. Letting HOT memory become an unbounded operator scan.
6. Forgetting synthetic/non_synthetic row accounting in compression reports.
```

Current proof boundary:

```text
best shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

still not product local accept.
```

Этот документ нужен, чтобы заранее видеть тупики. Если риск не назвать, проект
легко начнет доказывать красивую идею вместо проверки реальности.

## Риск 1: Красивый шум

Система может генерировать графики coherence, фаз и спектров, которые выглядят
значимо, но не дают улучшения поведения.

Защита:

```text
каждый красивый график должен иметь behavioral metric
каждый mode claim должен иметь ablation
каждый результат сравнивается с Mono192
```

## Риск 2: Обычный ансамбль вместо волновой системы

Клетки могут превратиться в маленьких экспертов с голосованием. Это может быть
полезно, но это не проверяет исходную идею.

Защита:

```text
обязательно сравнивать voting против wave bus
измерять phase coherence
измерять center-of-mass stability
показывать tick trace
```

## Риск 3: Самоусиление ошибки

Если система учится прямо во время генерации, она может закрепить собственную
ошибку как устойчивую моду.

Защита:

```text
runtime меняет только временное состояние
долгая память меняется только после confirmed feedback
каждое обновление проходит eval
есть откат snapshot и cell state
```

## Риск 4: Chatbot trap

Чатбот может отвечать бессмысленно, но из-за интерактивности будет казаться,
что "что-то живое есть".

Защита:

```text
Chat-0 только после toy-задач
сначала byte-level prediction
короткие ответы
фиксированный eval corpus
сравнение с Markov и n-gram
```

## Риск 5: Слишком много клеток слишком рано

1 GB клеток можно держать в RAM, но нельзя честно считать все клетки каждый тик
на T480.

Защита:

```text
сначала 6 x Cell32
потом 64 клетки
потом 1024 клетки
top-k обязателен
full sweep только как диагностика
```

## Риск 6: Плохое определение клетки

Если клетка будет больше похожа на произвольный объект с кучей полей, размер
32 KB потеряет смысл.

Защита:

```text
фиксированный layout памяти
тест размера структуры
без heap allocation внутри горячей клетки
отдельный state packet format
```

## Риск 7: Неправильная дистилляция

Нельзя напрямую снять hidden state закрытой LLM. Если строить план на этом,
проект упрется в недоступные данные.

Защита:

```text
сначала без LLM teacher
потом открытая локальная teacher-модель
или дистилляция по текстам/logits, если доступны
```

## Риск 8: Метрики после результата

Можно случайно выбрать метрику, которая красиво показывает уже получившийся
результат.

Защита:

```text
метрики фиксируются до прогона
отчет хранит конфиг эксперимента
seed обязателен
результат должен повторяться
```

## Риск 9: Смешение краткой и долгой памяти

Если snapshot, runtime state и trained cell state смешать, будет невозможно
понять, что именно обучилось.

Защита:

```text
snapshot = временное состояние
cell state = долгие параметры
training log = события обратной связи
eval report = доказательство изменения
```

## Риск 10: Потеря исходной идеи

Проект может постепенно стать обычным ML-классификатором или toy LLM.

Защита:

```text
каждый этап отвечает на вопрос о модах
каждая победа требует спектральной интерпретации
каждый baseline включает монолит той же памяти
```
