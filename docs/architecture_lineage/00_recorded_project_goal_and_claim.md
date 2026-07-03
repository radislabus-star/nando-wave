# Recorded Project Goal And Claim

This file records the text that was explicitly given for preservation in the
architecture notes. Do not treat this block as a fresh Codex interpretation.

## Central Project Goal

```text
что wave/cell system может сама выделить компактный переносимый оператор,
если это получится тогда что нам даст это в перспективе?
```

Meaning to preserve:

```text
Главная цель Nando Wave не в том, чтобы сделать маленькую копию LLM.

Главная цель:

проверить, может ли wave/cell system сама выделить компактный переносимый
оператор из примеров, удержать его на heldout/traps/ablation, и потом
использовать его как быстрый runtime-механизм без lookup-а и без большой модели.
```

If this works, the expected perspective is:

```text
1. Модель может стать умнее не за счет размера, а за счет выделения оператора.

   То есть знание хранится не как огромная таблица случаев, а как маленький
   переносимый механизм:

   state_t + action/rule -> state_t+1

2. Модель может стать быстрее.

   Если оператор доказан и скомпилирован в компактный runtime, inference может
   идти через sparse centers, локальные pressure/gap вычисления и cache-friendly
   edges, а не через большой dense forward pass.

3. Модель может стать худее.

   Вместо хранения многих похожих примеров и поверхностных шаблонов система
   должна сохранить reusable centers, motifs, bindings and transition edges.

4. Модель может лучше отвергать ловушки.

   Цель не только recall правильного паттерна, а:

   correct transition attractor усиливается,
   near wrong trap подавляется,
   shortcut не считается пониманием.

5. Модель может получить проверяемую механику reasoning.

   Не "accuracy высокий, значит поняла", а:

   heldout transfer,
   no-shortcut gates,
   trap rejection,
   ablation collapse,
   flat/runtime parity,
   gap stability.

6. Если получится, это дает путь к CPU-friendly reasoning runtime.

   Не большая универсальная модель на каждый шаг,
   а маленький проверенный операторный слой, который можно запускать дешево,
   локально и воспроизводимо.
```

Short preserved formula:

```text
Nando Wave должен доказать не запоминание ответа, а выделение компактного
переносимого оператора.

Если это получится, перспектива такая:

умнее — потому что переносится оператор,
быстрее — потому что runtime sparse/cache-friendly,
худее — потому что не нужна большая таблица случаев,
надежнее — потому что wrong traps явно подавляются,
проверяемее — потому что proof идет через gates/ablation, а не через красивый
claim.
```

## Architecture Lineage Claim

```text
Коротко: кирпичи известные, сборка тут своя.

Я бы не говорил: “это полностью никем не описано”. Это опасный claim.
Но и сказать “это просто известная архитектура” тоже нельзя.

Правильнее:

Nando Wave = кастомная гибридная архитектура,
собранная из известных семейств идей,
но с необычной целью, связкой слоёв и proof-gate дисциплиной.
```

Known families:

```text
associative memory
attractor dynamics
sparse distributed memory
Hebbian updates
role/filler binding
vector-symbolic representations
Fourier/progress-measure grokking analysis
sparse feature superposition
```

Custom/new-ish here:

```text
exact SurfaceWave/L1 encoding
L2 center/motif promotion gates
L3 state-delta/action-role binding pressure
anti-wave/trap proof loop
strict no-shortcut claim boundary
CPU/cache-resident runtime target
v2/v3 corpus pressure methodology
```

Safe formula:

```text
Это не открытие из пустоты.
Это новая инженерная сборка известных физических/нейросетевых идей
под очень конкретную цель:
доказать компактный переносимый оператор без lookup-а и без большой модели.
```

Claim boundary:

```text
literature novelty мы пока не имеем права заявлять.
engineering/R&D novelty внутри этого проекта - да, есть.
```

