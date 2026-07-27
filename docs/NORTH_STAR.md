# Nando Wave North Star

Текущий инженерный контракт проекта:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current product-facing interpretation:

```text
Nando Wave studies reasoning as verified transferable state transitions.
The current implementation path is phase-center operator memory:
  real stream -> L4 selector/router -> L3 phase centers -> CPU shadow/runtime.

Best current shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0
  local_accept_enabled: false
```

Этот документ отвечает на главный вопрос: куда идет проект и что нельзя
потерять, даже если первые эксперименты будут маленькими и сухими.

## Северная звезда

Nando Wave проверяет идею:

```text
мышление можно моделировать как динамическую релаксацию
клеточного частотного организма
к согласованному гармоническому состоянию
```

Проект не начинается с утверждения, что это уже доказано. Проект строит
прибор, который может показать:

```text
возникает ли ансамблевая мода
можно ли ее измерить
можно ли ее разрушить ablation
можно ли сохранить ее как snapshot
можно ли восстановить ее позже
```

Научная искра проекта - статья Nanda et al.:

```text
Progress measures for grokking via mechanistic interpretability
https://arxiv.org/abs/2301.05217
```

Она вдохновляет частотной трактовкой learned circuit, но готовое
доказательство архитектуры Nando Wave должен дать сам проект.

Цель Nando Wave - пройти от вдохновения к собственному доказательству:

```text
найти воспроизводимую ансамблевую моду
показать ее лучше или устойчивее монолита той же памяти
разрушить ее ablation
сохранить ее как snapshot
восстановить ее позже
```

## Что является сутью

Суть не в количестве параметров и не в маленьком chatbot.

Суть:

```text
частота
фаза
амплитуда
гармоническая интерференция
несущая волна
центр масс спектра
согласование клеток
память как частотный аккорд
```

Если это исчезает, проект теряет смысл.

## Запрещенный старый коммерческий путь

`.nwrb role-binding profiles -> payload builder -> verifier -> catalog`
больше не является допустимым путем к CPU80, коммерческому offload или market
claim.

Это зафиксировано как forbidden legacy:

```text
docs/FORBIDDEN_LEGACY_NWRB_COMMERCIAL_BACKEND.md
```

Допустимый путь дальше:

```text
real traffic
-> action/state atoms
-> phase-center / Fourier center of mass
-> compact operator runtime
-> deterministic verifier
-> feedback/catalog
```

## Несущая волна

В исходной идее очень важна несущая волна.

Она не просто еще одна метрика. Это глобальный аттрактор состояния:

```text
она задает диапазон допустимых фаз
она ограничивает дрейф клеток
она удерживает контекст
она модулирует средние и малые частоты
она помогает отличить продуктивное состояние от шума
```

В терминах архитектуры несущая волна должна стать явной сущностью, а не
побочным эффектом:

```text
CarrierWave
GlobalMode
phase boundary
amplitude envelope
context attractor
```

Без этого Nando Wave рискует стать обычным ансамблем маленьких моделей.

## Иерархия частот

Минимальная иерархия:

```text
большая мода  - несущая, тема, контекст, границы
средняя мода  - структура, синтаксис, устойчивые паттерны
малая мода    - символы, токены, быстрые реакции
```

Эта иерархия нужна не для красоты. Она защищает систему от хаоса. Малые клетки
не должны свободно "думать обо всем". Они должны колебаться внутри диапазона,
который задают более медленные моды.

## Память

Память проекта - это не просто сохраненные веса.

Главная идея памяти:

```text
snapshot = частотный аккорд состояния
```

Он должен быть намного меньше полного состояния и хранить только то, что
помогает восстановить режим:

```text
доминирующие частоты
фазы
амплитуды
coherence
center of mass
состояние несущей волны
активные клетки
```

Если snapshot не восстанавливает поведение лучше холодного старта, гипотеза
памяти пока не доказана.

## Чатбот не является северной звездой

Chat-0 нужен как демонстрация, но не как первый смысл проекта.

Правильная последовательность:

```text
сначала прибор мод
потом доказательство ансамблевой моды
потом snapshot-память
потом локальная адаптация
потом byte-level text
потом Chat-0
```

Если начать с chatbot, проект утонет в субъективной оценке "похоже или нет".

## Почему 32 KB и 64 KB не конфликтуют

32 KB - физический горячий атом для T480.

64 KB - удобный орган и пакет совместимости с исходной идеей Expert64.

192 KB - первый контрольный масштаб:

```text
6 x Cell32
против
1 x Mono192
```

Это не отказ от 64 KB, а уточнение уровней:

```text
Cell32   - клетка
Expert64 - пара клеток
Organ192 - малый организм
```

## Дистилляция

Идея частотного перелива от большой модели важна, но не должна быть первым
этапом.

Причина простая: hidden states закрытых моделей недоступны. Поэтому ранние
эксперименты должны доказать принцип без внешнего учителя.

Позже возможны варианты:

```text
открытая локальная teacher-модель
дистилляция по logits, если доступны
дистилляция по текстовым траекториям
частотный анализ generated traces
```

## Главная формула проекта

```text
не "маленькая LLM"
а "волновой клеточный организм"
```

И еще точнее:

```text
малые гармонические клетки
под несущей волной
через wave bus
образуют измеримую ансамблевую моду
которую можно сохранить как память
и использовать для генерации поведения
```

Это и есть северная звезда.

## Текущая доказательная граница

По состоянию исходников на 2026-07-27 Северная звезда имеет отдельный
fail-closed proof contract:

```text
frozen MS3 natural-law contract
-> independently verified phase receipts
-> relation fragments
-> CircuitSynthesizer
-> FrozenSynthesizedCircuitSet
-> disjoint future waves
-> OperatorGrokkingConsolidator
-> five fixed seeds and seven fixed arms
-> snapshot and remote restore
-> NorthStarProofV1
```

Обязательные arms:

```text
cellular wave ensemble
equal-budget monolith
exact structural search
no phase
shuffled phase
magnitude only
random center
```

Proof требует как минимум четыре проходящих seed из пяти, нулевые wrong
accepts и runtime parity failures, отсутствие support/future overlap,
delayed-transition и cleanup evidence, exact snapshot restore и отдельный
remote restore. Каждый arm связывает метрики с корнями experiment report,
frozen future и snapshot.

`NorthStarProofV1` не является admission receipt. Даже его будущий `PASS`
оставляет `authority_ready=false`: право исполнения по-прежнему выдаёт только
внешний admission после отдельного operator proof.

Честный текущий статус:

```text
MS3 generation 1 independent future   CONTRADICTION
blocker                               physical_transition_mismatch
immutable generation registry         IMPLEMENTED
verified phase -> circuit bridge       SHADOW TEST PASS
five-seed ensemble experiment          NOT EVALUATED
NorthStarProofV1 report                NOT ISSUED
new authority                          0
```

Exact structural MS3 остаётся контрольной веткой. Он не может быть
переименован в cellular-wave evidence. Cellular arm принимает только
`VerifiedDeltaReceipt` с независимым verifier и никогда не принимает готовую
`ResponseProgram` как доказательство grokking.
