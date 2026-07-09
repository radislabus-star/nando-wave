# Hybrid Symbiotic Operator Promotion Contract

Дата: 2026-07-09

Статус: active admission/promotion contract.

Этот документ задаёт следующий практический рубеж для майнера Nando Wave.

Цель не в том, чтобы добавить ещё один красивый термин. Цель в том, чтобы
майнер продвигал только такие профили, где скрытый фазовый центр, внешний
наблюдаемый след, verifier и negative memory согласованы между собой.

## 1. Главная Формула

```text
HybridSymbioticOperator =
  hidden transition center
  + explicit observable operator
  + verifier binding
  + negative memory
```

Такой оператор не является:

- exact cache;
- lookup table;
- красивым broad bucket;
- surface-reflex;
- verifier-only rule;
- product claim без future-window evidence.

## 2. Где Он Живёт В Архитектуре

```text
REAL STREAM
|
+-- L1 surface atoms
|   |
|   +-- safe observable atoms
|   +-- no raw prompt/output authority
|
+-- L2 hidden state split
|   |
|   +-- hidden problem state
|   +-- role/context/result shape
|
+-- L3 phase-center memory
|   |
|   +-- positive center
|   +-- negative center / anti-center
|   +-- drift estimate
|
+-- L4 selector/admission
|   |
|   +-- chooses portfolio by marginal denominator delta
|   +-- refuses unsafe broad parents
|   +-- promotes only verifier-bound clean children
|
+-- verifier / collimator
    |
    +-- admission
    +-- quarantine
    +-- chain checkpoints
```

Hot runtime не должен получать новую обязанность доказывать оператор. Hot runtime
только исполняет уже подготовленный compact profile.

## 3. Классы Профилей

Каждый bucket/profile должен получить один из классов.

```text
latent_candidate
  hidden center сильный, но observable/verifier ещё не согласованы

utility_reflex
  observable pattern полезен, но переносимого hidden transition нет

checked_rule
  verifier есть, но phase-center operator не доказан

quarantined_broad_parent
  широкий родитель даёт пользу, но смешивает unsafe cases

hybrid_symbiotic_verified_operator
  hidden center + observable evidence + verifier + negative memory согласованы

portable_operator_chain
  цепочка verified operators с bounded drift и verifier checkpoints
```

Запрещено считать `latent_candidate`, `utility_reflex`, `checked_rule` или
`quarantined_broad_parent` product-safe CPU operator.

## 4. Required Report Fields

Для каждого profile/bucket report обязан иметь такие поля.

```text
profile_id
parent_profile_id
profile_class

rows_seen
exact_cache_overlap_rows
new_unique_accepts_over_exact_cache
calls_saved
tokens_saved

hidden_margin_p50
hidden_margin_p90
hidden_margin_p99
observable_agreement_milli
verifier_true_rows
verifier_false_rows
negative_rejects
false_accepts
wrong_wins

drift_score_milli
trust_score_milli
risk_score_milli
expected_value_microusd

future_shadow_rows
future_shadow_accepts
future_shadow_false_accepts
promotion_verdict
promotion_blocker
```

Если часть поля пока нельзя посчитать, она должна быть явным `not_reported` или
`not_wired`, но такой профиль нельзя продвигать в product-safe class.

## 5. Promotion Ladder

```text
broad parent detected
|
+-- split into candidate children
    |
    +-- hidden center clean?
    |   |
    |   +-- no -> latent_candidate / reject
    |
    +-- observable evidence agrees?
    |   |
    |   +-- no -> latent_candidate / utility_reflex
    |
    +-- verifier binding exists?
    |   |
    |   +-- no -> checked_rule missing / reject
    |
    +-- negative memory rejects similar wrong cases?
    |   |
    |   +-- no -> quarantined_broad_parent
    |
    +-- false_accepts == 0?
    |   |
    |   +-- no -> quarantine + anti-center
    |
    +-- future shadow window stays clean?
        |
        +-- yes -> hybrid_symbiotic_verified_operator
```

Ключевое правило:

```text
promotion is not score.
promotion is score + safety + future proof.
```

## 6. L4 Selector Rule

L4 selector не должен выбирать красивые bucket-ы.

Он должен выбирать portfolio по marginal value:

```text
value =
  new_unique_accepts_over_exact_cache
  + token_value
  + cost_value
  - false_accept_risk
  - hot_bytes_penalty
  - latency_penalty
  - overlap_penalty
  - drift_penalty
```

Но value считается только после hard gates:

```text
verifier_binding_exists
false_accepts == 0
future_shadow_false_accepts == 0
no target/proof/lookup authority
no source-agent hardcode in generic core
```

Если hard gate не пройден, value не имеет права продвигать профиль.

## 7. Negative Memory

Negative memory является first-class signal.

Если профиль сработал на verifier-negative row:

```text
store anti-center
store split atoms
increase parent risk
block parent promotion
search clean child split
```

Negative memory не должна быть просто логом ошибки. Она должна влиять на:

- selector risk;
- split pressure;
- quarantine;
- promotion blocker;
- future accept boundary.

## 8. Chain Promotion Gate

Цепочка операторов не доказывается суммой красивых одиночных шагов.

Для цепочки нужны отдельные поля:

```text
chain_id
step_count
weakest_verified_link
cumulative_drift
chain_false_accepts
broken_chain_negatives
verifier_checkpoints
goal_progress_rows
```

Продвижение цепочки разрешено только если:

```text
all steps have verifier or verified compatibility
false_accepts == 0
chain_false_accepts == 0
weakest_verified_link >= threshold
cumulative_drift <= drift_budget
broken_chain_negatives are rejected
future shadow split passes
exact-cache overlap excluded
```

Иначе цепочка остаётся:

```text
unproven_operator_chain
или
workflow_reflex
```

## 9. Implementation Order

Исполнитель должен двигаться в таком порядке.

```text
R1 report schema
  add profile_class and hybrid report fields

R2 classification
  classify current profiles:
    latent_candidate
    utility_reflex
    checked_rule
    quarantined_broad_parent
    hybrid_symbiotic_verified_operator

R3 negative memory
  make verifier-negative rows affect risk/split/quarantine

R4 selector
  rank by marginal denominator delta after hard safety gates

R5 chain gate
  add weakest_verified_link and cumulative_drift

R6 reviewer view
  show TREE / SCOREBOARD / DEBT QUEUE with profile classes
```

Do not change hot runtime while doing R1-R4 unless a report proves the hot path
is the blocker.

## 10. Relation To Current Operator-Power Gate

Текущий `operator_power_*` слой разрешён как cold precursor signal.

Он может помогать ранжировать кандидаты, но сам по себе не является финальным
product class.

Mapping должен быть явным:

```text
operator_power_class
  -> candidate strength / precursor signal

profile_class
  -> admission truth used by promotion
```

Требуемый мост:

```text
rich_transfer_operator
  -> may become hybrid_symbiotic_verified_operator
  -> only after hidden/observable/verifier/negative/future gates

useful_transfer_operator
  -> may become checked_rule, utility_reflex, or hybrid candidate
  -> not product-safe by name alone

thin_reflex_or_low_evidence
  -> utility_reflex or watch

dangerous_broad_or_unclean
  -> quarantined_broad_parent

weak_or_unproven
  -> latent_candidate / checked_rule / watch
```

Запрещено:

```text
operator_power_class == product claim
operator_power_score_milli == promotion proof
tokens_saved == safety proof
```

Разрешено:

```text
operator_power_score_milli helps priority
profile_class decides admission
verifier/negative/future window decides promotion
```

## 11. Acceptance Criteria

Контракт считается внедрённым, когда report показывает:

```text
profile_class present for every selected profile
hidden/observable/verifier/negative fields present
quarantined broad parents are separated from clean children
false_accepts == 0 for promoted profiles
future_shadow_false_accepts == 0 for promoted profiles
exact_cache_overlap excluded from saved counts
local_accept still gated by verifier policy
```

Для chain profiles дополнительно:

```text
weakest_verified_link reported
cumulative_drift reported
chain_false_accepts reported
broken_chain_negatives reported
```

## 12. Hard Bans

```text
no .nwrb revival
no source-agent hardcode in generic core
no manual class list as product logic
no target_id authority
no proof_rule_id authority
no concrete_x_lookup
no output hash as answer authority
no local_accept without verifier and false_accepts=0
no synthetic-only market claim
no broad parent promotion when clean child split is required
```

## 13. Reviewer View

```text
PIPELINE TREE
  real stream
    -> L1 safe atoms
    -> L2 hidden state split
    -> L3 phase centers
    -> L4 hybrid selector
    -> verifier / negative memory
    -> promoted profile or quarantine

SCOREBOARD
  profile_class coverage
  hybrid_symbiotic_verified_operator count
  quarantined_broad_parent count
  new_unique_accepts_over_exact_cache
  tokens_saved
  false_accepts
  future_shadow_false_accepts
  weakest_verified_link
  cumulative_drift

DEBT QUEUE
  P0: report missing hybrid fields
  P0: negative memory not wired into selector
  P1: chain gate missing
  P1: future-window proof missing
  P2: status/dashboard display
```

## 14. Short Command For Executor

```text
Stop promoting fat buckets.
Classify every profile.
Promote only hybrid symbiotic verified operators:
hidden center + observable evidence + verifier + negative memory.
Use negative rows as anti-centers.
For chains, report weakest link and drift.
No hot runtime change unless evidence says runtime is the blocker.
```
