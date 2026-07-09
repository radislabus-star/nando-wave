# Wave-LLM Layers Live Plan

Current architecture overlay, 2026-07-06:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Important correction:

```text
L4 is no longer "closed" for the product path.
L4 is now the streaming selector/router/admission/goal-state layer that makes
real agent events visible to L3 phase centers.

L3 is not expected to digest the whole stream alone.
L4 reduces stream entropy; L3 scores compact drifting phase centers.
```

Current proof boundary:

```text
best compatible shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

product local accept: disabled
market money claim: blocked
```

Статус: живой план, не закрыт.

Цель: найти механизм, где Wave-LLM не хардкодит смысл и не вытаскивает строку
lookup-ом, а вынуждена открыть компактный оператор через проверяемую задачу.
Каждый слой должен давать проверяемый центр, перенос на heldout и отказ на
ловушках.

Архитектурная родословная и claim boundary зафиксированы в
`docs/ARCHITECTURE.md`: кирпичи известные, сборка своя; literature novelty claim
не разрешен, engineering/R&D novelty внутри проекта допустима.
Рабочие карточки по отдельным позициям лежат в
`docs/architecture_lineage/README.md`.

## Главная форма

```text
L1 = форма
L2 = мотивы
L3 = смысловые центры и операторы
L4 = план ответа
```

Историческая запись ниже говорила: "Сейчас фиксируем L1-L3. L4 закрыт".
Для текущего product path это больше не так: L4 открыт как streaming
goal-state/router/admission layer.

Ключевой поворот после corpus-trained L3 proof:

```text
fast runtime is not the source of grokking
runtime must be compiled after the grokking objective is proven
```

Текущий corpus-trained L3 field proof показал:

```text
trained edges can be fast and safe
but coactivation + splice traps do not create semantic grokking
```

Главная ошибка предыдущего направления:

```text
we trained center coactivation and rejection
not prediction of a hidden law
```

Новая базовая формула:

```text
grokking requires a task with:
  input
  verifiable target
  hidden compact operator
  heldout transfer
  near-negative rejection
```

Nanda-style lesson:

```text
human semantic labels are not required
but a checkable target law is required
```

## Task Quality Gate

Хорошая задачка для Wave - это не просто фраза и не просто label.

Хорошая задачка:

```text
input -> verifiable target
```

Но главное: между input и target должен быть скрытый компактный оператор,
который выгоднее открыть, чем запомнить все строки.

Good task contract:

```text
has input context
has checkable target
has hidden rule/operator
has heldout split where exact surface is unseen
has near-negative or corruption
has ablation path
does not leak target in input
does not reward exact lookup
```

Хорошая задача отвечает на вопрос:

```text
какой закон нужно открыть, чтобы переноситься на новые случаи?
```

Примеры хороших задач:

```text
prefix -> next sentence/span
context + question -> answer span
dialogue history -> next reply
procedure prefix -> next step
normal sequence -> corrupted sequence rejected
state_t -> state_t+1
```

Почему они хорошие:

```text
target уже существует в данных
не нужна ручная semantic-разметка
heldout может проверить перенос
corrupt/near-negative проверяет, что модель не просто вспыхивает на похожем
```

Плохая задачка:

```text
input -> arbitrary label
```

или:

```text
input contains target
surface token directly maps to answer
train and heldout differ only строкой, но не оператором
negative is too easy and just falls out of L2
label invented by us without external check
success possible by memorizing phrase/table
```

Плохая задача отвечает не на "какой закон открыть?", а на:

```text
какую строку/метку выучить?
```

Red flags:

```text
exact lookup can solve it
Bayesian conditional-frequency baseline can solve it
Markov/bigram baseline can solve it
target leakage
no near-miss
no heldout operator transfer
no measurable ablation drop
corruption too alien to activate the same field
semantic axis names are provided as authority instead of discovered/validated
```

Acceptance:

```text
good_task = target_checkable
          + hidden_operator_pressure
          + heldout_transfer
          + near_negative_rejection
          + no_lookup_shortcut
          + no_bayesian_shortcut
          + no_markov_bigram_shortcut
```

If a task does not satisfy this gate, it may still be useful for L1/L2 surface
training, but it must not be used as proof of semantic grokking.

## Task Factory Proof V1

Implemented proof file:

```text
crates/nando-core/tests/wave_task_factory_gate_proof.rs
```

Purpose:

```text
test the task source before building WavePredictor runtime
reject wordlist/dictionary tasks as semantic grokking sources
accept only tasks that break simple shortcut baselines
```

V1 shortcut baselines:

```text
exact lookup:
  input string -> train target

L2-neighbor:
  nearest input by trigram surface similarity -> its target

Bayesian pairwise:
  choose true target vs near-negative by conditional atom frequencies
  P(target_atom | input_atom)
```

V1 task types:

```text
bad source:
  russian_words_300k consecutive word sequence

candidate source:
  organ128_train_v1 structured next-line tasks
```

Proof run:

```text
command:
  cargo test -p nando-core --test wave_task_factory_gate_proof -- --nocapture

dictionary sequence:
  tasks_total: 1_998
  heldout_tasks: 400
  exact_lookup_accuracy_milli: 0
  l2_neighbor_accuracy_milli: 0
  bayesian_pairwise_accuracy_milli: 442
  target_leak_milli: 4
  near_negative_similarity_milli: 4
  single_token_ratio_milli: 1000
  verdict: REJECT_SINGLE_TOKEN_SEQUENCE

structured next-line:
  tasks_total: 145
  heldout_tasks: 29
  exact_lookup_accuracy_milli: 0
  l2_neighbor_accuracy_milli: 0
  bayesian_pairwise_accuracy_milli: 275
  target_leak_milli: 0
  near_negative_similarity_milli: 365
  single_token_ratio_milli: 0
  verdict: VALID_OPERATOR_PRESSURE_PROVEN
```

Interpretation:

```text
wordlist corpus is rejected for semantic grokking
organ128 next-line tasks are only a small first valid task source
Bayesian shortcut is nonzero, so V1 is not a final strong corpus
this does not prove WavePredictor yet
this proves that Task Quality Gate can separate bad source from candidate source
```

Next Task Factory debt:

```text
add larger real context->target corpora
add QA span tasks
add dialogue next-reply tasks
add stronger L2-neighbor baseline
add stronger Bayesian / Markov baselines
measure heldout transfer by source group
```

## Task Factory Proof V2

Implemented in the same proof file:

```text
crates/nando-core/tests/wave_task_factory_gate_proof.rs
```

V2 changes:

```text
heldout split is now by source_group, not source_id modulo
added Markov/bigram shortcut baseline
added local multi-kind source builder:
  next_line
  procedure_window
  dialogue_reply
```

New shortcut baselines:

```text
source-group heldout:
  whole source groups are withheld from training
  train/heldout no longer share adjacent source chunks

Markov/bigram:
  input last token -> target first token
  target unigram prior
  target bigram chain score
  pairwise choice: true target vs near-negative
```

V2 proof run:

```text
command:
  cargo test -p nando-core --test wave_task_factory_gate_proof -- --nocapture

dictionary sequence:
  tasks_total: 1_998
  heldout_tasks: 400
  source_groups: 10
  heldout_source_groups: 2
  task_kinds: 1
  exact_lookup_accuracy_milli: 0
  l2_neighbor_accuracy_milli: 0
  bayesian_pairwise_accuracy_milli: 467
  markov_bigram_accuracy_milli: 2
  target_leak_milli: 4
  near_negative_similarity_milli: 4
  single_token_ratio_milli: 1000
  verdict: REJECT_SINGLE_TOKEN_SEQUENCE

structured next-line:
  tasks_total: 145
  heldout_tasks: 40
  source_groups: 8
  heldout_source_groups: 2
  task_kinds: 1
  exact_lookup_accuracy_milli: 0
  l2_neighbor_accuracy_milli: 0
  bayesian_pairwise_accuracy_milli: 350
  markov_bigram_accuracy_milli: 325
  target_leak_milli: 0
  near_negative_similarity_milli: 365
  single_token_ratio_milli: 0
  verdict: VALID_OPERATOR_PRESSURE_PROVEN

local multi-kind:
  tasks_total: 328
  heldout_tasks: 64
  source_groups: 23
  heldout_source_groups: 5
  task_kinds: 3
  exact_lookup_accuracy_milli: 0
  l2_neighbor_accuracy_milli: 468
  bayesian_pairwise_accuracy_milli: 359
  markov_bigram_accuracy_milli: 453
  target_leak_milli: 0
  near_negative_similarity_milli: 344
  single_token_ratio_milli: 0
  verdict: REJECT_L2_NEIGHBOR_SHORTCUT
```

Interpretation:

```text
V2 gate is stricter than V1
source-group heldout is active
Markov/bigram shortcut is measured
local multi-kind shape alone is not enough
the local QA/dialogue/procedure-like source is rejected because L2-neighbor solves too much
this is a good failure: the gate caught shortcut leakage before runtime training
```

Updated Task Factory debt:

```text
add larger real QA/dialogue/procedure sources
build heldout by source family, not just chunk
add source provenance metadata
add stronger L2-neighbor baseline over learned L1/L2 motifs, not only trigrams
add span-answer tasks with automatic answer check
add procedure next-step tasks with external checkable order
keep Markov/bigram and Bayesian thresholds visible in every proof
```

## Wave Task Schema V2

Current canonical reference file:

```text
data/task_candidates/wavepredictor_seed_v1/reference_v2.jsonl
```

Machine-readable schema and validator:

```text
data/task_candidates/wave_task_v2.schema.json
data/task_candidates/validate_wave_task_v2.py
```

V2 exists because the early candidate format was useful for probing, but not
stable enough as a long-term corpus standard.

Required fields:

```text
schema_version
task_id
language
task_kind
domain_path
domain_tags
source_family
source_group
input
target
near_negative
operator_family
why_target_is_correct
why_negative_is_wrong
shortcut_risk
quality_status
```

Field rules:

```text
schema_version: wave_task_v2
shortcut_risk: array, not scalar string
operator_family: replaces hidden_operator
quality_status: candidate | reference | accepted | rejected
domain_path: hierarchical corpus metadata
domain_tags: cross-domain/interference metadata
```

Important boundary:

```text
domain_path and domain_tags are metadata for corpus control
they are not runtime answer authority
they must support balancing, heldout splitting, and shortcut audits
```

Current reference set:

```text
reference_v2 tasks: 13
quality_status: reference
accepted training tasks: 0
```

Immediate execution checklist:

```text
[x] 1. Fix Wave Task v2 as the task schema standard.
[x] 2. Build Lexicon Foundation v1.
[x] 3. Build Domain DSL v1 for the first proving ground.
[x] 4. Select and lock the first domain.
[x] 5. Define operators/entities/templates/negative_rules.
[x] 6. Generate compact tasks, not fat JSONL.
[x] 7. Run shortcut gates.
[x] 8. Raise useful-task yield to 30%+.
[x] 9. Build 1k accepted tasks.
[x] 10. Build 10k accepted tasks.
[ ] 11. Train WavePredictor.
[ ] 12. Prove heldout transfer.
```

## Lexicon Foundation V1

Current canonical folder:

```text
data/lexicon_foundation_v1/
```

Files:

```text
ru_hot_100k.txt
ru_cold_300k.txt
en_hot.txt
manifest.json
build_lexicon_foundation.py
validate_lexicon_foundation.py
README.md
```

Purpose:

```text
L1/L2 surface lexicon foundation
not semantic authority
not operator memory
not training-task corpus
```

Build rule:

```text
ru_hot_100k:
  first 100_000 strict Russian entries

ru_cold_300k:
  first 300_000 strict Russian entries

en_hot:
  strict lowercase Latin entries from local system dictionary
```

Audit:

```text
russian_words_300k accepted: 296_387
russian_words_full added: 3_613
ru_hot_100k entries: 100_000
ru_cold_300k entries: 300_000
en_hot entries: 74_960
```

Validation:

```text
python3 data/lexicon_foundation_v1/validate_lexicon_foundation.py
-> lexicon_foundation_v1 validation OK
```

Boundary:

```text
Lexicon Foundation feeds L1/L2 form and motif work.
It must not be treated as L3 semantic grokking evidence.
```

## Domain DSL V1

Current canonical folder:

```text
data/domain_dsl_v1/
```

Locked first proving domain:

```text
linux_networking_vpn
```

Russian translation:

```text
Linux / сеть / VPN / диагностика
```

Lock status:

```text
status: locked_first_proving_domain
lock_scope: first_proving_domain
locked_on: 2026-06-30
```

Files:

```text
domains.json
validate_domain_dsl_v1.py
linux_networking_vpn/domain_lock.json
linux_networking_vpn/entities.json
linux_networking_vpn/operators.json
linux_networking_vpn/templates.json
linux_networking_vpn/negative_rules.json
```

Domain pack v1:

```text
domains: 1
locked_domains: 1
entities: 48
operators: 24
negative_rules: 24
templates: 24
operator_template_coverage: complete
```

Meaning:

```text
entities = reusable domain objects and surface forms
operators = transition rules that tasks should pressure
templates = compact task blueprints
negative_rules = hard-near-negative mistake families
domain_lock = selected scope, boundaries, and required shortcut gates
```

Russian translation:

```text
entities = сущности домена
operators = операторы перехода
templates = шаблоны задач
negative_rules = правила похожих ошибочных ответов
domain_lock = выбранная область, границы и обязательные shortcut-гейты
```

Lock boundary:

```text
included:
  VPN connectivity and tunnel
  Linux routes and route scope
  DNS and internal zones
  Firewall, ACL, and port filtering
  Auth, TLS, RADIUS, and time
  Safe troubleshooting:
    snapshot before mutation
    minimal action
    refusal when evidence is underfilled

excluded:
  general Linux administration
  Windows networking
  full cloud platforms
  broad cybersecurity
  general chat
  using DSL as runtime answer authority
```

Russian translation:

```text
входит:
  VPN-подключение и туннель
  Linux-маршруты и область маршрутизации
  DNS и внутренние зоны
  Firewall, ACL и фильтрация портов
  авторизация, TLS, RADIUS и время
  безопасная диагностика:
    снимок до мутации
    минимальное действие
    отказ при нехватке evidence

не входит:
  Linux-администрирование вообще
  Windows-сети
  облака целиком
  кибербезопасность целиком
  болталка обо всем
  использование DSL как runtime-источника истины
```

Validation:

```text
python3 data/domain_dsl_v1/validate_domain_dsl_v1.py
-> domain_dsl_v1 validation OK: domains=1
```

Boundary:

```text
Domain DSL feeds Task Factory generation and shortcut audits.
It is not a task dataset.
It is not runtime answer authority.
Step 5 is closed: the first domain has a full v1 pack.
Step 6 remains open: no compact task corpus has been generated from this pack yet.
Accepted training tasks remain 0 for this domain.
```

## Linux Networking VPN Compact Task V1

Current canonical folder:

```text
data/task_candidates/linux_networking_vpn_compact_v1/
```

Files:

```text
compact_cases.json
validate_compact_cases.py
materialize_wave_tasks_v2.py
generated_wave_task_v2.jsonl
manifest.json
README.md
```

Purpose:

```text
generate compact candidate tasks from the locked Domain DSL pack
do not store the domain metadata repeatedly by hand
materialize strict wave_task_v2 only as a generated artifact
```

Russian translation:

```text
compact_cases.json = компактный источник задач
generated_wave_task_v2.jsonl = развернутый кандидатный JSONL для валидаторов
manifest.json = граница и счетчики
```

Counts:

```text
compact_cases: 24
templates_covered: 24
source_groups: 24
materialized_wave_task_v2_rows: 24
quality_status: candidate
accepted_training_tasks: 0
useful_candidate_tasks: 24
useful_task_yield_milli: 1000
```

Validation:

```text
python3 data/task_candidates/linux_networking_vpn_compact_v1/validate_compact_cases.py
-> compact task validation OK: domain=linux_networking_vpn cases=24 templates_covered=24

python3 data/task_candidates/linux_networking_vpn_compact_v1/materialize_wave_tasks_v2.py
-> materialized wave_task_v2 rows=24

python3 data/task_candidates/validate_wave_task_v2.py \
  data/task_candidates/linux_networking_vpn_compact_v1/generated_wave_task_v2.jsonl
-> wave_task_v2 validation OK: rows=24
```

Boundary:

```text
Step 6 is closed: compact candidate tasks exist.
Step 7 is closed: shortcut gates were run on this corpus.
Step 8 is closed: current useful candidate yield is above 30%.
No generated task is accepted training data yet.
```

## Linux Networking VPN Shortcut Gate V1

Report:

```text
data/task_candidates/linux_networking_vpn_compact_v1/shortcut_gate_report.json
```

Run:

```text
python3 data/task_candidates/linux_networking_vpn_compact_v1/run_shortcut_gates.py
```

Result:

```text
previous_verdict: REJECT_BAYESIAN_SHORTCUT
current_verdict: VALID_OPERATOR_PRESSURE_CANDIDATE
tasks_total: 24
train_tasks: 19
heldout_tasks: 5
source_groups: 24
heldout_source_groups: 5
task_kinds: 3

exact_lookup_accuracy_milli: 0
l2_neighbor_accuracy_milli: 0
bayesian_pairwise_accuracy_milli: 600
markov_bigram_accuracy_milli: 400
target_leak_milli: 0
near_negative_similarity_milli: 487
single_token_ratio_milli: 0
useful_task_yield_milli: 1000
useful_candidate_tasks: 24
accepted_training_tasks: 0
```

Interpretation:

```text
exact lookup is broken
L2-neighbor is broken
Markov/Bigram is below rejection threshold
target leakage is absent
Bayesian conditional-frequency shortcut is below rejection threshold
near-negatives now stress the same surface field
the corpus is useful as candidate material
the corpus is still not scaled accepted training data
```

Russian translation:

```text
точный lookup не решает
L2-neighbor не решает
Markov/Bigram пока не пробивает порог
утечки target нет
Bayesian baseline теперь ниже порога
near-negative теперь достаточно близкие
```

Repair delta:

```text
Bayesian: 800 -> 600
Markov/Bigram: 600 -> 400
near-negative similarity: 100 -> 487
verdict: REJECT_BAYESIAN_SHORTCUT -> VALID_OPERATOR_PRESSURE_CANDIDATE
```

## Linux Networking VPN Accepted 1k V1

Current canonical folder:

```text
data/task_candidates/linux_networking_vpn_accepted_1k_v1/
```

Files:

```text
build_accepted_1k.py
run_shortcut_gates.py
accepted_wave_task_v2.jsonl
shortcut_gate_report.json
manifest.json
README.md
```

Purpose:

```text
scale the compact VPN task source to 1k accepted Wave Task V2 examples
keep domain metadata compactly generated from Domain DSL
use target-like cross-operator near-negatives instead of alien negatives
```

Result:

```text
rows: 1000
quality_status: accepted
accepted_training_tasks: 1000
shortcut_gate_verdict: VALID_OPERATOR_PRESSURE_CANDIDATE

train_tasks: 790
heldout_tasks: 210
source_groups: 24
heldout_source_groups: 5
task_kinds: 3

exact_lookup_accuracy_milli: 0
l2_neighbor_accuracy_milli: 0
bayesian_pairwise_accuracy_milli: 200
markov_bigram_accuracy_milli: 0
target_leak_milli: 0
near_negative_similarity_milli: 483
single_token_ratio_milli: 0
```

Interpretation:

```text
exact lookup is broken
L2-neighbor is broken
Bayesian conditional-frequency shortcut is below rejection threshold
Markov/Bigram shortcut is broken
target leakage is absent
near-negatives are close enough to stress the same surface field
accepted_training_tasks is now nonzero for this domain
```

Boundary:

```text
Step 9 is closed.
This corpus is accepted training material.
It is not runtime answer authority.
Step 10 is also closed by the 10k corpus below.
```

## Linux Networking VPN Accepted 10k V1

Current canonical folder:

```text
data/task_candidates/linux_networking_vpn_accepted_10k_v1/
```

Files:

```text
build_accepted_10k.py
run_shortcut_gates.py
accepted_wave_task_v2.jsonl
shortcut_gate_report.json
manifest.json
README.md
```

Purpose:

```text
scale the locked VPN task source to 10k accepted Wave Task V2 examples
preserve source-group heldout
keep hard negatives target-like and cross-operator
keep accepted status behind shortcut gate PASS
```

Result:

```text
rows: 10000
quality_status: accepted
accepted_training_tasks: 10000
shortcut_gate_verdict: VALID_OPERATOR_PRESSURE_CANDIDATE

train_tasks: 7915
heldout_tasks: 2085
source_groups: 24
heldout_source_groups: 5
task_kinds: 3

exact_lookup_accuracy_milli: 0
l2_neighbor_accuracy_milli: 0
bayesian_pairwise_accuracy_milli: 200
markov_bigram_accuracy_milli: 0
target_leak_milli: 0
near_negative_similarity_milli: 480
single_token_ratio_milli: 0
```

Interpretation:

```text
exact lookup is broken
L2-neighbor is broken
Bayesian conditional-frequency shortcut is below rejection threshold
Markov/Bigram shortcut is broken
target leakage is absent
near-negatives remain close enough to stress the same surface field
accepted_training_tasks is now 10000 for this domain
```

Boundary:

```text
Step 10 V1 is historically closed.
This corpus is no longer the preferred training material after shortcut audit.
It is not runtime answer authority.
Reason:
  source_group is still too close to operator_family for later state-delta proof.
```

## Linux Networking VPN Accepted 10k V2

Current canonical folder:

```text
data/task_candidates/linux_networking_vpn_accepted_10k_v2/
```

Purpose:

```text
fix the V1 source_group shortcut before training
make source_group a mixed bucket, not an operator label
keep near_negative as real target text from another operator
keep accepted status behind shortcut gate PASS
```

Generator rule:

```text
source_group = vpn_mixed_bucket_00..15
each bucket contains all 24 operator families
operator_family remains the semantic task family
```

Result:

```text
rows: 10000
quality_status: accepted
accepted_training_tasks: 10000
shortcut_gate_verdict: VALID_OPERATOR_PRESSURE_CANDIDATE

train_tasks: 7500
heldout_tasks: 2500
source_groups: 16
heldout_source_groups: 4
task_kinds: 3

exact_lookup_accuracy_milli: 0
source_group_pairwise_accuracy_milli: 0
l2_neighbor_accuracy_milli: 64
bayesian_pairwise_accuracy_milli: 478
markov_bigram_accuracy_milli: 500
target_leak_milli: 0
near_negative_similarity_milli: 480
single_token_ratio_milli: 0
```

Interpretation:

```text
source_group-only shortcut is broken
exact lookup is broken
L2-neighbor is below threshold
Bayesian and Markov are below threshold
target leakage is absent
near-negatives remain close enough to stress the same surface field
```

Boundary:

```text
V2 is the current dataset for the next training gate.
It is not runtime answer authority.
```

Next debt:

```text
Step 9 is closed:
  built 1k accepted tasks
  preserve the same shortcut gates
  accepted_training_tasks = 1000 only after gate pass

Step 10 is closed by V2:
  built 10k accepted tasks
  source_group shortcut is explicitly gated
  accepted_training_tasks = 10000 only after gate pass

Step 11:
  train WavePredictor on V2
  prove heldout transfer before runtime claims
```

## WavePredictor Hebbian Rule V1

Implemented proof/training primitive:

```text
crates/nando-core/src/wave/wavepredictor_hebbian.rs
```

Rule:

```text
Sparse Contrastive Error-Driven Hebbian Rule

if target_gap < margin_required:
  for active in active_fringe:
    compat(active -> target_center) += eta_pos * active_strength
    compat(active -> nearest_wrong_center) -= eta_neg * active_strength
    conflict(active -> nearest_wrong_center) += eta_conflict * active_strength

if trap_accepted:
  for active in active_fringe:
    anti_wave(active -> nearest_wrong_center) += eta_anti * active_strength
```

Hard guard:

```text
base_mass is not changed by this rule
inactive edges are not touched
global field drift is not allowed
weights are clamped
```

Test:

```text
cargo test -p nando-core wavepredictor_hebbian -- --nocapture

3 passed:
  repairs weak target margin locally
  trap accepted reinforces only wrong anti-wave
  settled non-trap case changes no weights or mass
```

Boundary:

```text
This is a reusable learning rule for Step 11.
It is not full WavePredictor training yet.
It is not L4.
It is not runtime answer authority.
```

## WavePredictor Trainer Loop V1

Implemented training-loop primitive:

```text
crates/nando-core/src/wave/wavepredictor_trainer.rs
```

Scheduler:

```text
Dynamic Settle Margin / Margin Ramp

epoch warmup:
  margin_required = start_margin

epoch ramp:
  margin_required rises linearly to target_margin

after ramp:
  margin_required = target_margin
```

Training loop:

```text
for epoch:
  margin_required = margin_schedule.margin_for_epoch(epoch)

  for task:
    current_gap = field.target_gap(task)

    if current_gap < margin_required:
      apply Sparse Contrastive Error-Driven Hebbian Rule

    if task is an accepted trap:
      apply local anti_wave pressure
```

Hard guard:

```text
eta_ratio_scheduler_used = false
base_mass_drift_detected must stay false
l4_opened = false
optional anti_wave cap is a guard only, not the primary scheduler
```

Test:

```text
cargo test -p nando-core wavepredictor_trainer -- --nocapture

3 passed:
  margin ramp warms up, rises, and clamps
  trainer repairs a weak target margin without eta-ratio or base-mass drift
  optional anti-wave brake caps trap pressure per epoch
```

Boundary:

```text
This closes the Step 11 training-loop mechanism.
It does not close Step 11 as a learning proof yet.
Step 11 remains open until the 10k accepted corpus produces a train/heldout report.
It is not L4.
It is not runtime answer authority.
```

## WavePredictor 10k Trainer Gate V1

Implemented corpus trainer gate:

```text
crates/nando-core/tests/wavepredictor_trainer_10k.rs
```

Purpose:

```text
connect accepted_wave_task_v2.jsonl to the WavePredictor Trainer Loop
measure real train/heldout behavior
detect impossible flat-operator heldout before claiming grokking
```

Important finding:

```text
source_group = operator_family in the accepted 10k VPN corpus
source-group heldout therefore withholds entire target operators
a flat target_center model cannot predict a target center that never appeared in train
```

Proof command:

```text
cargo test -p nando-core --test wavepredictor_trainer_10k -- --ignored --nocapture
```

Observed calibration:

```text
source_group_holdout:
  train_tasks: 7915
  heldout_tasks: 2085
  unseen_heldout_target_operators: 5
  flat_operator_accuracy_milli: 0
  median_gap: -29360
  p10_gap: -34720

stratified_control:
  train_tasks: 8000
  heldout_tasks: 2000
  unseen_heldout_target_operators: 0
  flat_operator_accuracy_milli: 1000
  median_gap: 16288
  p10_gap: 10752
```

Interpretation:

```text
Trainer Loop can learn and separate target vs near-negative when target
operators are represented in train.

Source-group heldout is the right pressure for future semantic transfer,
but it is not solvable by one flat operator id per source_group.
```

Verdict:

```text
TRAINER_WORKS_BUT_FLAT_OPERATOR_SOURCE_GROUP_HELDOUT_IS_UNTRAINABLE
```

Next architectural debt:

```text
replace flat operator target_center with compositional target centers:
  evidence axis
  layer axis
  action axis
  safety/refusal axis
  scope axis

Then source-group heldout should test unseen combinations of seen micro-centers,
not an unseen atomic label.
```

## WavePredictor Compositional Target V1

Decision:

```text
primary target format = Axis-Index Arrays
derived hot cache may later be TargetMask [u64; 8]
TargetMask is not the source of truth for training
```

Why not primary bitmask:

```text
bitmask is fast but hides which semantic axis failed
axis-index target keeps Action/Layer/Evidence/Safety/Scope separable
Hebbian correction must know target vs nearest_wrong per axis
```

Implemented types:

```text
WAVEPREDICTOR_TARGET_AXIS_CAP = 16
WavePredictorAxisTarget:
  axis_id
  target_center
  nearest_wrong_center

WavePredictorCompositionalTrainTask:
  active_fringe
  axis_targets[16]
  axis_len
  trap_accepted
```

Implemented trainer path:

```text
WavePredictorTrainer::train_compositional
```

Training rule:

```text
for epoch:
  margin_required = margin_schedule.margin_for_epoch(epoch)

  for compositional task:
    for axis target:
      current_gap = score(axis.target) - score(axis.nearest_wrong)

      if current_gap < margin_required:
        apply Sparse Contrastive Error-Driven Hebbian Rule on that axis

      if task is accepted trap:
        apply local anti_wave pressure on that axis nearest_wrong
```

10k source-group compositional gate:

```text
heldout_axis_values_unseen_in_train: 0

flat source_group target:
  exact/operator accuracy_milli: 0
  median_gap: -29360

compositional axis target:
  exact_composition_accuracy_milli: 191
  axis_accuracy_milli: 517
  median_gap: 144
  p10_gap: -2688
```

Interpretation:

```text
compositional target removes the impossible unseen-label wall
but source-group transfer is still weak
axis target is necessary, not sufficient
the remaining debt is stronger learned cue induction from L1/L2 to axes
```

Verdict:

```text
AXIS_TARGETS_ARE_TRAINABLE_BUT_SOURCE_GROUP_TRANSFER_STILL_NEEDS_STRONGER_CUES
```

Stronger correction after shortcut audit:

```text
Axis targets still contain target_center ids.
That means V1 compositional training is still a set of small per-axis classifiers.
It is useful as a diagnostic scaffold.
It is not semantic grokking proof.
```

Hard claim guard:

```text
WavePredictorTrainerReport:
  target_center_id_training_used
  axis_target_id_training_used
  state_delta_training_used
  semantic_grokking_claim_allowed

Current scalar trainer:
  target_center_id_training_used = true
  semantic_grokking_claim_allowed = false

Current compositional trainer:
  axis_target_id_training_used = true
  semantic_grokking_claim_allowed = false
```

Next replacement target:

```text
WavePredictorStateDeltaTarget:
  positive wave impulses
  negative wave impulses

Goal:
  train state_t -> target_state_delta
  not input -> target_center_id
  not input -> per-axis target_center_id

Readout centers may exist for evaluation.
They must not be training authority.
```

Implemented state-delta V1:

```text
WavePredictorStateDeltaTrainTask:
  active_fringe
  target_delta

WavePredictorHebbianField:
  center -> L1 lane sparse projection

Training rule:
  amplify target delta lanes
  suppress near-negative delta lanes
  do not update target_center_id
  do not update axis_target_id
  do not open L4
```

Source-group 10k gate result:

```text
source_group_state_delta_accuracy_milli: 10
source_group_state_delta_tasks: 500
source_group_state_delta_median_gap: -4768
source_group_state_delta_p10_gap: -11616
verdict:
  STATE_DELTA_MECHANISM_ADDED_BUT_CURRENT_SOURCE_GROUP_SPLIT_HAS_UNSEEN_TARGET_DELTAS
```

Meaning:

```text
The mechanism now trains wave deltas, not center ids.
But the current source-group split holds out whole target delta families.
So the task is still untrainable as transfer proof.

Next dataset requirement:
  heldout must hide new combinations
  not all target wave-delta parts
```

Combinatorial delta heldout V1:

```text
split key:
  source_group + scope + evidence_window

train requirement:
  every heldout source_group exists in train
  every heldout scope exists in train
  every heldout evidence_window exists in train

heldout requirement:
  exact source_group + scope + evidence_window combo is absent from train
```

Combinatorial 10k gate + shortcut audit result:

```text
combinatorial_delta_train_tasks: 7996
combinatorial_delta_heldout_tasks: 2004
combinatorial_delta_heldout_combos: 231
combinatorial_delta_leaked_exact_combos: 0
combinatorial_delta_missing_seen_parts: 0
combinatorial_delta_state_delta_accuracy_milli: 1000
combinatorial_delta_state_delta_tasks: 500
combinatorial_delta_state_delta_median_gap: 15216
combinatorial_delta_state_delta_p10_gap: 11440
shortcut_source_group_only_accuracy_milli: 1000
shortcut_scope_only_accuracy_milli: 664
shortcut_window_only_accuracy_milli: 658
shortcut_token_bigram_neighbor_accuracy_milli: 998
shortcut_bayesian_pairwise_accuracy_milli: 914
shortcut_markov_bigram_accuracy_milli: 664
verdict:
  COMBINATORIAL_DELTA_SHORTCUT_LEAK_DETECTED
```

Meaning:

```text
The exact combo lookup is blocked.
But source_group-only, token/bigram neighbor, and Bayesian baselines solve too much.
So the 1000/1000 state-delta result is not semantic proof.

The current corpus still leaks answer shape through surface/operator family.
The next step is to rebuild the task generator so source_group alone cannot predict target_delta.
```

Поэтому следующий правильный узел:

```text
WavePredictor-1:
  context wave -> future wave
  prefix -> continuation
  question/context -> answer span
  state_t + learned_operator -> state_t+1
```

Не следующий узел:

```text
manual Action/Domain/Style labels
runtime-only attraction tuning
L4 decoder
```

## L1 SurfaceWave

L1 кодирует первичный поверхностный импульс.

Базовые параметры:

```text
n = 4
dim = 4096 lanes
k = 3 trits
trit in {-1, 0, +1}
accumulator = i16
```

L1 не должен понимать смысл. Он должен стабильно кодировать форму.

Текущий контракт:

```text
text
-> surface_atoms
-> sparse ternary lane hits
-> SurfaceWave4096
```

Атомы L1:

```text
raw byte 4-grams
boundary atoms with BOS/EOS
service atoms for closed-class function words only
```

Важно:

```text
short word != service word
```

Пример `сыч`:

```text
[BOS, BOS, BOS, с]
[BOS, BOS, с, ы]
[BOS, с, ы, ч]
[с, ы, ч, EOS]
[ы, ч, EOS, EOS]
[ч, EOS, EOS, EOS]
```

`сыч` получает boundary-form, но не service-channel.

Пример `и`:

```text
[BOS, BOS, BOS, и]
[BOS, BOS, и, EOS]
[BOS, и, EOS, EOS]
[и, EOS, EOS, EOS]
+ service("и")
```

`BOS/EOS` - внутренние маркеры. Они не равны реальным символам `^`, `$`.

## L1 частотность

Частые n-граммы нельзя просто выбрасывать.

Они работают как синтаксическая несущая частота. Их вклад должен быть
ослабляемым/нормируемым, а не удаляемым.

Защита:

```text
frequency-aware weight
saturating i16 accumulator
no hard deletion of common surface atoms by default
```

Пока дисперсионный gate показал вред в простом варианте. Значит дисперсию не
добавляем в hot path без нового proof.

## L2 Current State

Текущий L2 в коде:

```text
L1 text
-> surface_atoms
-> L1 center refs: [12, 88, 5, 901, ...]
-> L2 windows over center refs
-> reusable sequence motifs
```

Это уже не exact-word lookup, но еще не финальная волновая физика.

## L2 Target State

Правильный Wave-L2:

```text
L2 motif = sparse ternary prototype + local time phase + normalized resonance
```

Не сравниваем весь плотный `4096 x i16` вектор со всеми мотивами.

Нужный механизм:

```text
L1 sparse trits
-> inverted lane/sign/time index
-> motif votes
-> top-k motif candidates
-> signed score / conflict score
-> gap verification
```

Мотив хранит не весь wave, а устойчивые зубцы:

```text
[(lane, sign, local_t, weight), ...]
```

Score:

```text
score = weighted_matches - weighted_conflicts
score /= motif_norm
gap = best - second_best
```

Нормировка обязательна:

```text
popular lane weight down
long motif length normalized
rare stable lane weight up
```

Иначе длинные/частотные мотивы будут побеждать только массой.

## L2 Time-Phase

Глобальные position buckets ломаются от вводных слов.

Правильное время L2:

```text
local relative phase inside motif
```

Пример:

```text
закрыт  t0
проблем t1
нет     t2
```

Если перед фразой добавили `слушай`, локальная координата мотива не меняется.

Допуск:

```text
exact t      -> full score
t +/- 1     -> partial score
farther t   -> no score
```

Перекрывающиеся окна нужны, но главная защита - локальное время мотива.

## L2 Prototype Mining

Прототипы L2 не задаются руками.

Они вытаскиваются из корпуса:

```text
many texts
-> L1 atoms
-> candidate windows, roughly 3-7 atoms
-> support/conflict statistics
-> heldout/reversed/corrupt filters
-> compact motif prototypes
```

Критерии promotion:

```text
support high
conflict low
heldout repeats
corrupt/reversed worse
compression positive
```

Это Wave-Pattern Mining: не gradient descent, а отбор устойчивых инвариантов.

### Pass 1 Candidate Mining

Проблема: если собрать все окна `3..7` из всех фраз, кандидатов будет слишком
много.

Решение:

```text
stream corpus
-> L1 atoms
-> sliding windows 3..7
-> hash(window with local_t)
-> Count-Min Sketch / SpaceSaving
-> keep heavy hitters only
```

Pass 1 не хранит все окна.

Хранит только:

```text
sketch counters
top-N candidate hashes
```

Не используем схему "первая фраза стала прототипом". Это слишком шумно.

### Pass 2 Candidate Verification

После Pass 1 повторно прогоняем корпус.

Считаем только top-N candidates:

```text
support
conflict_count
lane/sign/time stats
lane popularity
motif length norm
```

Финальный promoted motif:

```text
motif_hash
prototype_teeth [(lane, sign, local_t, weight), ...]
support
conflict
norm
```

Promotion:

```text
support >= min
conflict low
heldout repeats
corrupt/reversed rejected
compression positive
```

Итог:

```text
Pass 1 = cheaply find candidates
Pass 2 = precisely measure only candidates
```

Это не lookup. Это streaming heavy hitters + verification.

## L3 Target State

L3 не должен быть плоским classifier.

Правильный L3:

```text
L2 top-k motifs
-> sparse votes
-> axis centers
-> interference field
-> composed operator
```

Оси:

```text
Action
Domain
Object
Tone
Style
Constraint
EvidenceNeed
Refusal / AntiWave
```

## L3 Axis-Center Induction

Подход: гибридный базис через именованные аттракторы.

Не используем чистый unsupervised:

```text
SVD/PCA can find components
but cannot honestly name them Action/Domain/Refusal
```

Не используем чистый supervised classifier:

```text
phrase -> fixed label
```

Правильная схема:

```text
axis names = fixed contract
axis centers = learned
motif -> axis weights = learned
```

Минимальный axis contract:

```text
Action
Domain
Object
Tone
Style
Constraint
EvidenceNeed
AntiWave
```

Для каждой оси можно дать маленькие seed anchors. Это не полные правила, а
начальные ядра:

```text
Action: explain, diagnose, disable, compare
Domain: vpn, linux, finance
Tone: angry, calm, brief
```

L2 motifs подтягиваются к осям через:

```text
co-occurrence
contrastive negatives
heldout transfer
phase similarity
```

Вес мотива:

```text
weight(motif, axis_center) =
  attraction_to_seed
+ cooccur_support
- conflict_with_other_axis
- trap_penalty
```

Axis center update:

```text
center += confirmed motifs
center -= conflicting motifs
```

Итог:

```text
L3 axis = named attractor
but its mass and links are learned from corpus
```

L3 не сам придумывает названия осей. L3 сам учит, какие мотивы реально
принадлежат этим осям и с каким весом.

Мотив не имеет жесткого провода к одному классу.

Один мотив дает слабые голоса в несколько осей:

```text
"не работает vpn"
-> Domain: vpn +0.8
-> Action: diagnose +0.6
-> EvidenceNeed: runtime_snapshot +0.7
-> Tone: practical +0.3
```

## L3 Interference

L3 считает поле:

```text
score(center) =
  motif_votes
+ compatibility(other_centers)
- conflict(other_centers)
- anti_wave
```

Оператор ответа:

```text
O_final =
  Action
+ Domain
+ Object
+ Tone
+ Style
+ Constraint
+ EvidenceNeed
```

Пример:

```text
Domain = vpn
Action = disable
Constraint = no_runtime_evidence
```

Должно давать не действие, а запрос evidence / review-only state.

## L3 Acceptance

L3 принимает оператор только если:

```text
axis gaps positive
centers compatible
conflicts suppressed
anti-wave has no veto
evidence is sufficient
heldout passes
role/route traps rejected
```

## L3 Bounded Settle

L3 Field нельзя оставлять крутиться бесконечно.

Защита от limit cycle / chaos:

```text
fixed small settle_steps
+ damping
+ priority axes
+ no authority if gap weak
```

Базовый runtime:

```text
for step in 0..N:
  centers += compatibility
  centers -= conflict
  centers *= damping(step)
  Refusal/Constraint can veto
```

Ориентир:

```text
N = 3..5
temperature falls
damping increases near final steps
```

Если поле не сошлось:

```text
gap < threshold
or oscillation detected
or conflict_energy high
or anti_wave active
```

то оператор не получает authority.

Возврат:

```text
FIELD_UNSETTLED
answer_allowed = false
need_clarification = true
```

Иерархия осей:

```text
Refusal / AntiWave
> Constraint
> EvidenceNeed
> Action
> Domain
> Tone / Style
```

Слабые оси не могут победить safety/evidence.

## L3 Interaction Learning

Pass 3 учит внутреннюю динамику поля как sparse contrastive Hebbian learning.

Не учим полную матрицу:

```text
670 x 670 x 670
```

Учим только активные и пограничные связи:

```text
top-k active centers
near-miss centers
wrong attractor links
```

Три источника обучения:

```text
positive coactivation
contrastive anti-wave
sparse settle correction
```

Positive coactivation:

```text
centers active together in successful example
-> compatibility +=
```

Contrastive anti-wave:

```text
trap / poisoned / unsafe / near-miss example
-> conflict +=
-> anti_wave +=
```

Sparse settle correction:

```text
field settled to wrong attractor
-> penalize only links that pulled into wrong attractor
-> reinforce missing links to correct attractor
```

Важно:

```text
learn observed / near-miss interactions
do not learn all possible combinations
```

Итог:

```text
coactivation gives attraction
traps give repulsion
settle correction gives adaptation
```

## L3 Field Regularization

Проблема: частые пары центров могут стать слишком тяжелыми и начать
затягивать поле даже на зашумленных запросах.

Закон:

```text
frequent != infinitely strong
```

Link update:

```text
raw_link += event
link_weight = raw_link / sqrt(center_mass_a * center_mass_b)
link_weight = clamp(link_weight, -max, +max)
```

Регуляризация:

```text
mass-normalized links
bounded weights
decay for weak / unconfirmed edges
heldout/trap promotion gate
```

Связь считается сильной только если:

```text
appears often
has low conflict
helps heldout
does not break traps
```

## L3 Trap Generation

Цель: модель должна сама искать свои уязвимые места, а не ждать ручного списка
ловушек.

Подход:

```text
axis inversion
domain splice
evidence removal
random impossible combos
active mining of false accepts
```

Цикл:

```text
positive example
-> infer active axes
-> generate near-miss traps
-> run L3 field
-> collect false accepts
-> train conflict / anti-wave
-> retest heldout and traps
```

Near-miss виды:

```text
role swap
action inversion
domain splice
constraint violation
evidence removal
tone/style conflict
```

Комбинаторный генератор:

```text
random axis combo
-> impossible / unsafe / unsupported combo
-> expected FIELD_UNSETTLED or AntiWave
```

Главный механизм:

```text
if L3 falsely accepts a trap:
  trap becomes training negative
  conflict +=
  anti_wave +=
```

Итог:

```text
trap generation is not fully manual
traps grow from failed validation
```

## Hot Memory Layout Draft

Размер hot memory - это не догма. Это профиль.

Цель layout:

```text
minimum cache misses
flat arrays
no pointer chasing
measured heldout/trap gain per byte
```

Запрещено не увеличение объема, а увеличение без пользы.

```text
more bytes are allowed
if they buy better heldout,
lower false accept,
lower latency,
or simpler safer layout
```

## Task-First Matrix Profiles

Не начинаем с размера.

Начинаем с вопроса:

```text
какая матрица нужна, чтобы решить задачу?
```

Размер - следствие выбранной матрицы.

Профиль задается не байтами, а структурной сложностью:

```text
surface_form_profile
  цель: форма слов / шум / опечатки
  нужны: L1 atoms + L2 form motifs
  не нужны: L3 domain/action axes

single_domain_operator_profile
  цель: один домен, устойчивые действия
  нужны: L1 + L2 motifs + L3 Action/Domain/Object/Constraint
  не нужны: широкий multi-domain registry

dialogue_behavior_profile
  цель: болталка / стиль / намерение / тон
  нужны: Action/Tone/Style/Constraint/EvidenceNeed
  нужны: strong AntiWave for refusal/uncertain states

multi_domain_reasoning_profile
  цель: несколько доменов с конфликтами
  нужны: Domain axes, route separation, conflict matrix, evidence matrix
  нужны: domain-splice traps

critical_control_profile
  цель: действия в системе / безопасность
  нужны: Constraint/Refusal/EvidenceNeed priority
  нужны: FIELD_UNSETTLED as normal output
```

Для каждого профиля сначала фиксируем:

```text
axis_count
centers_per_axis
motif_count
motif_teeth_distribution
interaction_edge_count
anti_wave_edge_count
evidence_need_centers
settle_steps
trap_suite
```

И только потом считаем:

```text
model_hot_bytes
runtime_scratch_bytes
median_inference_us
```

Каждый профиль обязан печатать:

```text
model_hot_bytes
runtime_scratch_bytes
bytes_per_motif
bytes_per_edge
heldout_accuracy
false_accept_rate
trap_rejection_rate
median_inference_us
```

## Primary Matrix Target

Главный рабочий профиль для первой полноценной болталки:

```text
dialogue_behavior_profile_full_v1
```

Цель:

```text
L3-cache-resident working set
not tiny proof toy
```

Важно: cache residency - это цель и метрика, не математическая гарантия.
Нельзя писать "никогда не выйдет в RAM". Правильно:

```text
cache_miss_rate is measured
working_set fits target CPU cache budget
```

Матрица full_v1:

```text
L2 motif_count              = 65_536
avg_teeth_per_motif         = 8
L2 bucket_count             = 65_536
front_refs_per_bucket       = 4
overflow_index_budget       = measured, initial target ~512 KiB

L3 axis_count               = 16
centers_per_axis            = 32
L3 center_count             = 512
interaction_edge_target     = 131_072
anti_wave_edge_target       = 65_536
settle_steps                = 3..5
```

Оси full_v1:

```text
Action
Intent
Topic
Object
Tone
Style
Emotion
Constraint
EvidenceNeed
Refusal
Time
Certainty
MemoryUse
Repair
Safety
DomainBoundary
```

Утвержденный byte audit full_v1:

```text
L1 SurfaceWave scratch [i16;4096]                 8_192
L2 core motif bank, fixed 8 teeth             1_835_008
L2 rare / long motif extension bank             786_432
L2 front inverted index                          524_288
L2 expanded overflow pool                      1_048_576
L3 axis center model data                          4_096
L3 interaction edges, aligned 8-byte           1_048_576
L3 dedicated trap / anti-wave bank               524_288
Metric counters / rebuild / alignment reserve    512_000
--------------------------------------------------------
configured working budget                       6_291_456 bytes
```

6_291_456 bytes = 6 MiB.

Это не "забить память ради красоты". Свободный запас из первичного 3.7 MiB
покупает слабые места:

```text
aligned 8-byte edges           -> speed and simpler traversal
expanded overflow pool         -> fewer important lane-list cuts
rare / long motif bank         -> rare but strong phrases survive
dedicated anti-wave bank        -> more hard negatives and veto patterns
metric / rebuild reserve       -> utility scoring and profile proof
```

Важно: aligned 8-byte edge is chosen for simpler CPU traversal. It does not
magically prove one-cycle SIMD. Benchmark/proof decides the real speed.

## Hardware Benchmark Gate

Главная красная линия - P99 latency полного inference path.

Current proof harness:

```text
crates/nando-core/tests/wave_full_v1_layout_bench.rs
```

Причина: cache-miss и IPC объясняют, почему быстро/медленно, но пользователь и
runtime чувствуют задержку.

Не зашиваем романтический `P99 <= 50us` как первый assert для полного
`L1 -> L2 -> L3` на noisy/adversarial запросах. Это stretch target, не
acceptance gate.

Первый честный gate:

```text
end_to_end_p99_us <= measured_baseline_budget
no_inference_allocations = true
false_accept_rate = 0
FIELD_UNSETTLED returned for unresolved traps
```

Для local/T480-class CPU стартовые redlines:

```text
L3_settle_only_p99_us <= 250
end_to_end_p99_us     <= 500
```

Если реальный первый benchmark окажется быстрее, gate ужесточается. Если
медленнее - не снижаем планку молча, а смотрим cache-miss/overflow/branch
причину.

Hardware diagnostics:

```text
LLC/cache miss rate = primary diagnostic
IPC                 = secondary diagnostic
branch misses       = secondary diagnostic
```

Начальная диагностическая цель:

```text
LLC miss rate <= 5% on hot steady-state profile
warn if LLC miss rate > 3%
```

`LLC miss < 1%` можно оставить как сильную цель для tuned build, но не как
первый обязательный assert: 6 MiB working set конкурирует с кодом, scratch,
prefetch и системным шумом.

First physical release run:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

queries: 10_000
p50_latency: 1.012511ms
p99_latency: 1.79225ms
false_accepts: 0
unsettled_accuracy_milli: 1000
verdict: LATENCY_GATE_FAILED_SAFETY_GATE_PASSED
```

Не снижаем gate до результата. Цель `end_to_end_p99_us <= 500` остается.

Что это значит:

```text
layout/safety path is alive
full 5-step scan over 131_072 edges is too slow for the first 500us target
next optimization target = L3 settle kernel / active-edge traversal
```

L3 settle optimization decision:

```text
chosen strategy = A-grouped active edge lists
not chosen      = bit-grid over full edge scan
```

Почему:

```text
bit-grid keeps scanning all 131_072 edges
A-grouped layout stores edges contiguously by center_a
active fringe visits only edge blocks for active centers
```

Proof harness implementation:

```text
ACTIVE_CENTER_LIMIT = 32
l3_edge_offsets = [u32; 513]
edge groups = center_a contiguous blocks
offset table bytes = 2_052
offset table is paid from 512_000-byte metric/rebuild reserve
```

Second physical release run after active-edge traversal:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

queries: 10_000
p50_latency: 96.527us
p99_latency: 155.841us
false_accepts: 0
unsettled_accuracy_milli: 1000
verdict: LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Результат:

```text
end_to_end_p99_us target <= 500
measured p99 = 155.841
headroom = about 3.2x
```

Q16 real-fringe distribution decision:

```text
do not choose only one distribution
benchmark both:
  regularized attractor graph
  scale-free hubs graph
```

Regularized graph represents the intended post-pruning field:

```text
max_edges_per_center = 256
```

Scale-free graph is the adversarial Zipf/heavy-hub stress:

```text
hub_count = 16
hub_edges = 2_048
remaining centers = about 198..199 edges each
```

Measured counters added to proof harness:

```text
active_center_p50
active_center_p99
edges_visited_p50
edges_visited_p99
```

Third physical release run with real-fringe simulation:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

Regularized:
  queries: 10_000
  p50_latency: 102.870us
  p99_latency: 173.651us
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 40_704
  edges_visited_p99: 40_960
  false_accepts: 0
  unsettled_accuracy_milli: 1000

ScaleFreeHubs:
  queries: 10_000
  p50_latency: 183.062us
  p99_latency: 272.796us
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 68_708
  edges_visited_p99: 70_562
  false_accepts: 0
  unsettled_accuracy_milli: 1000

verdict: REAL_FRINGE_SIMULATION_LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Честная граница:

```text
this is simulated real-fringe distribution
not yet corpus-trained L3 field
next proof must feed active centers from real L2/L3 training output
```

Fourth physical release run with real L2 training output:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_layout_bench -- --ignored --nocapture

input:
  corpus = data/corpus/russian_words_300k.txt
  L2 train words = 20_000
  L2 heldout words = 5_000
  timed queries with non-empty L2 motif tokens = 4_717

Regularized:
  p50_latency: 66.815us
  p99_latency: 124.450us
  seed_center_p50: 4
  seed_center_p99: 10
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 33_792
  edges_visited_p99: 35_328
  false_accepts: 0
  unsettled_accuracy_milli: 1000

ScaleFreeHubs:
  p50_latency: 66.856us
  p99_latency: 122.199us
  seed_center_p50: 4
  seed_center_p99: 10
  active_center_p50: 32
  active_center_p99: 32
  edges_visited_p50: 32_505
  edges_visited_p99: 46_902
  false_accepts: 0
  unsettled_accuracy_milli: 1000

verdict: REAL_L2_OUTPUT_FRINGE_LATENCY_GATE_PASSED_SAFETY_GATE_PASSED
```

Updated boundary:

```text
real L2-output active-fringe proof = passed
corpus-trained L3 field active-fringe proof = still open
```

## Q18 Early Veto Law

Decision:

```text
Early Veto = weighted anti-energy threshold
not global single-hit veto
```

Why:

```text
single anti-edge hit is too brittle for noisy L1/L2 activations
normal noise must not make the model silent
real traps must accumulate enough anti energy to beat positive support
```

Inference order:

```text
active_centers
-> anti_wave_offsets / anti_wave_edges
-> accumulate anti_energy over active pairs only
-> compare against positive_energy and veto margin
-> if veto wins: FIELD_UNSETTLED before settle
-> else: run ordinary interaction settle
```

Rule:

```text
link_energy = anti_weight(A,B) * min(activation_A, activation_B)

link contributes only if:
  link_energy >= local_margin(A,B)

early veto fires only if:
  total_anti_energy >= veto_threshold
  and total_anti_energy - positive_energy >= veto_margin
```

Segregation:

```text
raw_conflict weak      -> ordinary interaction conflict
raw_conflict medium    -> negative interaction edge
raw_conflict strong    -> anti_wave edge with weight
trap confirmed strong  -> anti_wave edge with high weight
certified hard marker  -> separate immediate safety veto, not semantic AntiWave
```

Binary reuse:

```text
interaction_edges use AlignedEdge as:
  compatibility = attraction weight
  conflict      = soft conflict weight

anti_wave_edges use AlignedEdge as:
  compatibility = anti_weight
  conflict      = local_margin
```

Next proof:

```text
corpus-trained L3 field induction bench
manual_edge_simulation = false
interaction_edges learned from real L2 coactivation
anti_wave_edges learned from contrastive trap coactivation
early_veto_false_silence must stay low
trap_false_accepts must stay zero
```

## Corpus-Trained L3 Field Induction Proof

Implemented proof file:

```text
crates/nando-core/tests/wave_full_v1_corpus_l3_bench.rs
```

What it proves:

```text
manual_edge_simulation = false
interaction_edges come from real Russian L2 center coactivation
anti_wave_edges come from contrastive splice trap coactivation
interaction and anti-wave tables are A-grouped independently
Early Veto uses weighted anti-energy threshold
late anti-check can reject trap tails after settle
```

Release run:

```text
date: 2026-06-29
command:
  cargo test -p nando-core --release --test wave_full_v1_corpus_l3_bench -- --ignored --nocapture

induction:
  train_examples: 18_603
  trap_examples: 9_301
  trained_interaction_edges: 130_946
  trained_anti_wave_edges: 39_250
  center_mass_nonzero: 512
  manual_edge_simulation: false

bench:
  queries: 4_487
  p50_latency: 50.410us
  p99_latency: 101.016us
  seed_center_p99: 10
  active_center_p99: 32
  interaction_edges_visited_p99: 34_816
  anti_edges_visited_p99: 3_090
  total_false_accepts: 0
  trap_rejection_milli: 1000
  trap_early_veto_milli: 430
  normal_early_veto_milli: 284
  normal_success_milli: 0

verdict: CORPUS_TRAINED_L3_ACTIVE_FIELD_SAFETY_LATENCY_GATE_PASSED
```

Honest boundary:

```text
this closes trained-edge active-field safety/latency
this does not grant answer authority
normal_success_milli = 0 means positive semantic convergence did not emerge
next debt is predictive grokking objective, not runtime attraction tuning
do not claim semantic grokking from coactivation/trap field alone
```

Why semantic grokking did not happen:

```text
input was word/L2-center coactivation
target was mostly rejection/trap behavior
hidden predictive law was absent
positive future/operator target was absent
therefore the field learned to refuse, not to continue meaning
```

Corrected interpretation:

```text
corpus-trained L3 field = fast learned safety/settle substrate
not answer-ready semantic Wave-LLM
not proof of semantic grokking
```

Next route:

```text
build a task factory where targets already exist in data:
  text prefix -> next sentence/span
  context + question -> answer span
  dialogue history -> next reply
  procedure prefix -> next step
  normal sample -> corrupted sample rejected

then learn:
  state_t + operator ~= state_t+1
```

## Hot Rebuild / Snapshot Rule

Q14 decision:

```text
inference reads immutable hot snapshot
rebuild/pruning never mutates the live matrix in place
new profile is built in cold/staging memory
swap happens only at request boundary by version/epoch pointer swap
```

Это не "lock-free write into live matrix". И это не stop-the-world посреди
inference loop.

Правильная модель:

```text
active_profile_vN -> inference reads only
builder creates profile_vN+1 in staging memory
gate validates profile_vN+1:
  byte budget
  heldout/trap proof
  false_accept_rate = 0
  latency/cache-miss profile
if gate passes:
  atomic epoch swap at request boundary
else:
  keep active_profile_vN
```

Почему так:

```text
no race conditions in inference
no partial matrix state
no allocations inside inference
no need to fit a full shadow copy inside the 6 MiB hot budget
benchmark remains deterministic
```

Metric / rebuild / alignment reserve is not a full model shadow copy:

```text
512 KiB reserve = counters, ablation hits, local rebuild bookkeeping
full rebuild staging memory lives outside hot inference working set
```

Benchmark must test two paths separately:

```text
steady-state inference:
  no allocations
  immutable active profile
  P99 latency gate

swap stability:
  many request-boundary swaps
  no false accept before/after swap
  no torn profile observed
  old epoch remains valid until readers finish
```

## Model Data vs Runtime Scratch

Нельзя смешивать постоянную модель и рабочий буфер.

Runtime scratch:

```text
L1 SurfaceWave accumulator [i16; 4096] = 8192 bytes
L2 motif score buffer
L3 activation buffer
top-k candidate buffer
```

Model hot data:

```text
L2 motif bank
L2 inverted index
L3 axis centers
L3 interaction edges
AntiWave edges
normalization tables
```

L1 accumulator можно считать в total resident working set, но это не веса
модели.

## L2 Motif Bank Layout

Лучший базовый layout - SoA/CSR, а не фиксированный padded record.

Причина: у разных мотивов разное число зубцов. Фиксированные `8 teeth` просты,
но либо режут богатые мотивы, либо тратят пустоту.

Hot arrays:

```text
motif_offset: [u32; motif_count + 1]
motif_norm:   [u16; motif_count]
motif_flags:  [u16; motif_count]
tooth_pool:   [PackedTooth; total_teeth]
weight_pool:  [i8; total_teeth]
```

PackedTooth:

```text
lane_id: 12 bits
sign:     2 bits
local_t:  2 bits
----------------
packed tooth = u16
```

Motif length is variable:

```text
min_teeth = 4
max_teeth = profile limit, often 8..16
```

Promotion chooses teeth by:

```text
support * idf * heldout_gain / (1 + conflict_count)
```

## L2 Inverted Index Layout

The index must not be keyed by `lane` alone.

Correct key:

```text
tooth_bucket = hash(lane_id, sign, local_t) & (bucket_count - 1)
```

Best layout under free volume:

```text
front_index: fixed [motif_id; 4] per bucket
overflow_index: CSR pool for buckets with more than 4 good refs
```

Why:

```text
front_index gives fast common path
overflow_index prevents useful motif loss
```

Overflow is allowed, but bounded and measured.

Bucket policy:

```text
first 4 refs: highest tooth_score, hot path
overflow refs: sorted by tooth_score, scanned only if needed
low-score refs: rejected
```

Training must report:

```text
bucket_count
avg_refs_per_bucket
max_refs_per_bucket
overflow_bucket_count
overflow_ref_count
motifs_rejected_by_index_saturation
collision_false_candidate_rate
overflow_latency_cost
```

Promotion requires:

```text
heldout still passes
trap proof still passes
collision_false_candidate_rate under budget
overflow_latency_cost under profile budget
```

If overflow improves heldout/traps with acceptable latency, keep it. If it only
hides noisy popular lanes, reject those teeth/motifs.

## L3 Axis Center Layout

Separate immutable center data from runtime activation.

Model hot:

```text
axis_id:       u8
flags:         u8
base_mass:     u16
threshold:     i16
reserved/stat: u16
----------------
8 bytes per center
```

Runtime scratch:

```text
activation: [i16; center_count]
previous_activation: [i16; center_count]
```

The scratch buffers are not model weights.

## L3 Edge Layout

Use flat sorted edges, preferably CSR by source center.

Edge record:

```text
target_center_id: u16
flags:            u8
compat_weight:    i8
conflict_weight:  i8
anti_weight:      i8
--------------------
6 bytes logical
```

Two physical candidates:

```text
packed 6-byte edge  -> smaller
aligned 8-byte edge -> often faster
```

Benchmark decides. Do not assume smaller is faster.

## Layout Decision Rule

Лучший layout выбирается proof-метриками, не эстетикой:

```text
layout_score =
  heldout_gain
- false_accept_penalty
- trap_failure_penalty
- latency_penalty
- cache_miss_penalty
- complexity_penalty
```

Проект может держать несколько task-first compiled profiles:

```text
surface_form_profile
single_domain_operator_profile
dialogue_behavior_profile
multi_domain_reasoning_profile
critical_control_profile
```

Runtime выбирает профиль по задаче, корпусу, домену и устройству.

## Byte Utility / Promotion Control

Каждый байт должен покупать качество.

Не держим матрицу заполненной ради красоты.

Контроль полезности памяти:

```text
edge decay / pruning
motif tooth quantization
entropy_per_byte tracking
heldout/trap contribution
replacement by stronger candidates
```

Для каждого motif считаем:

```text
motif_utility =
  heldout_gain
+ trap_rejection_gain
+ compression_gain
- false_accept_penalty
- collision_penalty
- latency_cost
```

Для каждого edge считаем:

```text
edge_utility =
  settle_gap_gain
+ trap_rejection_gain
- oscillation_penalty
- false_accept_penalty
- activation_deadness_penalty
```

Promotion:

```text
promote only if utility > promotion_threshold
```

Pruning:

```text
if utility decays below keep_threshold:
  remove edge/motif
  free slot for stronger candidate from streaming candidate pool
```

Motif quantization:

```text
if motif is popular but low-discriminative:
  shrink teeth count 8 -> 6 -> 4
  keep only highest tooth_score teeth
```

Tooth score:

```text
tooth_score =
  support
* idf
* heldout_gain
/ (1 + conflict_count + collision_cost)
```

Entropy rule:

```text
low activation entropy + low heldout/trap contribution
-> candidate for replacement
```

Final gate:

```text
bytes_used may grow
only if heldout/trap/latency metrics improve
```

Metrics reserve rule:

```text
training / profiling build:
  reserve stores utility counters, ablation hits, cache-miss samples

compiled inference build:
  reserve may be frozen into read-only utility tables
  or reassigned to rare motifs / anti-wave if counters are not needed
```

Итог по Q12:

```text
best mechanism =
  edge decay / pruning
+ motif tooth quantization
+ entropy_per_byte
+ heldout/trap promotion gate
```

Не выбираем один механизм из трёх. Лучшее решение - связать их в один
promotion economy loop:

```text
новый байт в hot model допускается только если он покупает:
  больше heldout/stress качества
  больше trap rejection
  меньше false accept
  или меньшую latency/cache-miss цену
```

## Что еще не закрыто

1. Task Quality Gate corpus builder.
2. WavePredictor-1 objective: `state_t + operator ~= state_t+1`.
3. Predictive heldout split: unseen surface and unseen operator combinations.
4. Near-negative generator that stays in-field, not alien noise.
5. No-lookup proof for predictive tasks.
6. L1/L2 reuse for predictive context windows.
7. L3 learned transition operator proof.
8. L3 positive convergence proof from predictive target, not manual attraction.
9. L3 trap/corrupt rejection proof after predictive training.
10. Ablation proof: remove learned operator/field and heldout drops.
11. Byte utility / pruning proof under predictive objective.
12. Hot memory layout benchmark/proof for compiled predictive profile.
13. Hot rebuild snapshot / epoch swap proof.
14. Связь L3 operator -> L4 answer plan. Заблокировано до закрытия predictive
    L1-L3 debt gate.

Already passed but not enough for semantic grokking:

```text
6 MiB full_v1 layout proof
synthetic active-fringe proof
real Russian L2-output active-fringe proof
corpus-trained L3 safety/latency proof
```

Why not enough:

```text
they prove fast/safe field mechanics
they do not prove predictive semantic operator discovery
```

## Ближайший Приоритет

Сначала не декодер и не ручное докручивание attraction.

Сначала нужен predictive task protocol:

```text
raw sources
-> Task Quality Gate
-> predictive tasks:
     context -> future
     question/context -> answer span
     procedure prefix -> next step
     dialogue history -> next reply
     normal -> corrupt rejected
-> L1/L2 wave encoding
-> L3 transition operator learning
-> heldout transfer proof
-> near-negative rejection proof
-> no-lookup proof
```

Декодер имеет смысл только после того, как L3 оператор стабильно предсказывает
будущую волну / ответный span на heldout и умеет честно возвращать
`FIELD_UNSETTLED`.

L4 lock:

```text
NO L4 until L1/L2/L3 debts are closed.
```

L4 unlock criteria:

```text
Task Quality Gate passes
predictive target exists and is checkable
heldout transfer beats lookup baseline
near-negative rejection passes
operator ablation drop is strong
L3 transition operator converges on normal heldout
FIELD_UNSETTLED remains available for unresolved/trap cases
compiled hot profile keeps latency/byte gates
```

## Guard

Если новый слой требует ручного хардкода смысла под конкретный домен, дизайн
считается подозрительным.

Если слой проходит только на train, но не переносится на heldout, это не
грокинг.

Если слой хранит строку целиком и потом достает ее обратно, это lookup, а не
Wave.

Если кто-то пытается перейти к L4 до debt gate, это architectural drift.

Если кто-то называет coactivation/trap safety field "semantic grokking", это
claim drift.

## Rule Logic Corpus v1

New foundation target:

```text
data/rule_logic_corpus_v1
```

Purpose:

```text
build solved rule-transfer tasks before scaling text/domain corpora
```

Current seed corpus:

```text
rows: 12000
rules: 20
surface_families: 8
PROVEN: 10800
UNSETTLED: 600
CONFLICT: 600
file: data/rule_logic_corpus_v1/accepted_rule_tasks_v1.jsonl
```

Shortcut gate:

```text
exact_lookup_accuracy_milli: 0
source_group_majority_accuracy_milli: 6
rule_id_majority_accuracy_milli: 0
surface_majority_accuracy_milli: 30
markov_choice_accuracy_milli: 486
target_in_input_milli: 800
target_negative_jaccard_milli: 175
verdict: VALID_RULE_OPERATOR_PRESSURE_CANDIDATE
```

Interpretation:

```text
This is not yet L3 grokking proof.
It is a usable first task foundation for L3 rule-operator training.
The next proof must train on this corpus and compare WavePredictor against
the same shortcut baselines on heldout rule applications.
```

Training authority:

```text
input
target
near_negative
answer_status
```

Proof-only fields:

```text
proof_rule_id
proof_rule_family
why_target_is_correct
why_negative_is_wrong
```

Guard:

```text
proof_rule_id must never become model authority
```

Rule Logic v1 first L3 training gate:

```text
test:
  crates/nando-core/tests/wavepredictor_rule_logic_trainer.rs

command:
  cargo test -p nando-core --test wavepredictor_rule_logic_trainer -- --ignored --nocapture

train_rows: 4800
heldout_rows: 1200
train_state_delta_accuracy_milli: 954
heldout_state_delta_accuracy_milli: 893
heldout_state_delta_median_gap: 2943
heldout_state_delta_p10_gap: -87

shortcut baselines:
  source_group_prototype_accuracy_milli: 593
  proof_rule_id_prototype_accuracy_milli: 0
  surface_family_prototype_accuracy_milli: 623
  answer_status_prototype_accuracy_milli: 383
  l1_neighbor_accuracy_milli: 213

state_delta_edges: 332241
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
```

Interpretation:

```text
L3 state-delta training now beats the tested shortcut baselines on Rule Logic v1.
This is a useful training signal, not a final grokking claim.
The weak tail remains visible: heldout_state_delta_p10_gap is still negative.
Next hardening target:
  improve weak-tail convergence and add stricter heldout by rule-surface combo
```

## Rule Logic Corpus v2

Why v2 exists:

```text
Rule Logic v1 was too shortcut-friendly.
Best dumb shortcut in the Rust L3 gate reached 623/1000.
That is not acceptable for an intelligence/grokking proof.
```

v2 target contract:

```text
input: rule task + shuffled answer choices
target: choice=<label>
near_negative: choice=<wrong_label>
```

This removes answer-surface leakage:

```text
numbers, symbols, sets, logic, evidence-status tasks all emit the same answer
shape.
```

Corpus gate:

```text
file: data/rule_logic_corpus_v2/accepted_rule_tasks_v2.jsonl
rows: 12000
exact_lookup_accuracy_milli: 0
source_group_majority_accuracy_milli: 246
proof_rule_id_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 256
answer_status_majority_accuracy_milli: 244
markov_choice_accuracy_milli: 497
verdict: VALID_RULE_OPERATOR_PRESSURE_CANDIDATE_V2
```

Current L3 result on v2:

```text
train_rows: 4800
heldout_rows: 1200
train_state_delta_accuracy_milli: 680
heldout_state_delta_accuracy_milli: 498
heldout_state_delta_median_gap: -34
heldout_state_delta_p10_gap: -11041

shortcut baselines:
  source_group_prototype_accuracy_milli: 488
  proof_rule_id_prototype_accuracy_milli: 0
  surface_family_prototype_accuracy_milli: 493
  answer_status_prototype_accuracy_milli: 353
  l1_neighbor_accuracy_milli: 413
  best_shortcut_accuracy_milli: 493

verdict:
  RULE_LOGIC_V2_HARD_CORPUS_CURRENT_L3_NOT_ABOVE_SHORTCUT
```

Interpretation:

```text
The v2 corpus is a better training/evaluation target.
The previous v1 "893/1000" result is no longer accepted as proof.
Current L3 state-delta projection is approximately at shortcut/chance level on
the harder corpus.
```

Next L3 debt:

```text
Learn option-conditioned rule application.
The field must bind:
  detected rule
  candidate option content
  correct choice label

Plain input -> choice-label projection is not enough.
```

## L3 Action Trace Guard

Core correction:

```text
L1 worked because it had a physical object to learn:
  letters / n-grams / word boundaries / short words / service words

L2 worked because it had a physical object to learn:
  repeated local motifs / order / chunks / surface patterns

L3 failed when we gave it only:
  question -> answer
  input -> target_id
  input -> choice_label
```

Fixed L3 object:

```text
L3 must learn action traces.
```

Required training shape:

```text
state_before
rule_action_example
state_after_correct
state_after_wrong
```

Not accepted as L3 grokking:

```text
question -> answer
input -> class_id
input -> choice=A/B/C/D
input -> target_delta only
```

Accepted L3 pressure:

```text
state_t + rule_action -> state_t+1
```

Human analogy:

```text
Do not tell a child only "the ball is in the hoop".
Show:
  ball before
  hand action
  ball trajectory
  ball in hoop
  wrong trajectory
```

Rule-task analogy:

```text
Do not train only:
  row -> answer

Train:
  row before
  demonstrated rule action
  correct transformed row
  near-wrong transformed row
```

Wave interpretation:

```text
L3 grokking object = invariant transition over wave state.

The transition must be reusable across surfaces.
If it cannot be represented as wave/interference/closure, reject it.
```

Guard:

```text
Never call answer-only training L3 grokking.
Never call choice-label training L3 grokking.
Never call proof_rule_id training L3 grokking.
```

## Rule Logic Corpus v3 Action Trace

Why v3 exists:

```text
L3 needs examples of rule action, not only final answers or choice labels.
```

Corpus shape:

```text
state_before
rule_action_example
state_after_correct
state_after_wrong
```

Training interpretation:

```text
active_fringe = state_before + rule_action_example
target_delta = state_before -> state_after_correct
negative_delta = state_before -> state_after_wrong
```

Corpus gate:

```text
file: data/rule_logic_corpus_v3/accepted_action_trace_tasks_v3.jsonl
rows: 12000
exact_lookup_accuracy_milli: 0
source_group_majority_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 0
answer_status_majority_accuracy_milli: 0
markov_pairwise_accuracy_milli: 489
verdict: VALID_ACTION_TRACE_OPERATOR_PRESSURE_CANDIDATE
```

Rust L3 action-trace gate:

```text
test:
  crates/nando-core/tests/wavepredictor_action_trace_l3.rs

command:
  cargo test -p nando-core --test wavepredictor_action_trace_l3 -- --ignored --nocapture

train_rows: 6000
heldout_rows: 1500
train_action_trace_accuracy_milli: 959
heldout_action_trace_accuracy_milli: 891
heldout_action_trace_median_gap: 5007
heldout_action_trace_p10_gap: -82

shortcut baselines:
  source_group_prototype_accuracy_milli: 497
  proof_rule_id_prototype_accuracy_milli: 0
  surface_family_prototype_accuracy_milli: 655
  answer_status_prototype_accuracy_milli: 380
  l1_neighbor_accuracy_milli: 516
  best_shortcut_accuracy_milli: 655

state_delta_edges: 561112
target_center_id_training_used: false
proof_rule_id_training_authority_used: false

error_count: 163
error_by_rule_top:
  required_variable_missing: 93
  alternation_next: 34
  mirror_complete: 22
  odd_one_out: 14
error_by_surface_top:
  evidence_trace: 93
  symbols: 40
  ru_words: 16
  feature_trace: 14
```

Interpretation:

```text
Action-trace training restores a real L3 learning signal:
  891/1000 heldout vs 655/1000 best tested shortcut.

This is not final L3 grokking.
Surface-family prototype shortcut is still too high.
Weak tail remains:
  heldout_action_trace_p10_gap: -82

Next debt:
  fix required_variable_missing action traces first
  reduce surface-family shortcut
  add operator reuse/compression pressure
  export compact trained profile only after shortcut gap and weak tail improve
```

## Rule Logic Binding Pressure v1

Implemented corpus:

```text
data/rule_logic_binding_pressure_v1/accepted_binding_pressure_tasks_v1.jsonl
```

Implemented L3 gate:

```text
crates/nando-core/tests/wavepredictor_binding_pressure_l3.rs
```

Purpose:

```text
Do not hand-code a bind operator.
Create pressure where L3 can win only by transferring X:
  state_before contains X
  rule_action_example demonstrates generic movement
  state_after_correct contains the same X
  state_after_wrong contains a plausible wrong Y
```

Corpus shape:

```text
rows: 2800
train_rows: 2000
heldout_rows: 800
train_heldout_value_overlap: 0
train_heldout_symbol_overlap: 0
train_heldout_word_overlap: 0

rules:
  missing_variable_bind: 700
  conflict_fact_bind: 700
  mirror_first_bind: 700
  alternation_second_bind: 700
```

Shortcut gate:

```text
exact_lookup_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 0
answer_status_majority_accuracy_milli: 0
markov_pairwise_accuracy_milli: 500
verdict: VALID_BINDING_PRESSURE_CANDIDATE
```

Step 12 binding/frame/slot gates:

```text
command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
    -- --ignored binding_pressure_l3_must_induce_transfer_without_target_ids_or_rule_authority --nocapture

train_binding_accuracy_milli: 1000
heldout_binding_accuracy_milli: 1000
flat_binding_accuracy_milli: 1000
heldout_binding_median_gap: 28328
heldout_binding_p10_gap: 10484

shortcut baselines:
  proof_rule_id_prototype_accuracy_milli: 494
  surface_family_prototype_accuracy_milli: 464
  answer_status_prototype_accuracy_milli: 504
  l1_neighbor_accuracy_milli: 325
  markov_pairwise_accuracy_milli: 500
  best_shortcut_accuracy_milli: 504

state_delta_edges: 0
role_binding_edges: 875
role_binding_nonzero_edges: 794
flat_role_binding_edges: 794
flat_role_binding_bytes_estimate: 37540
role_binding_max_abs_weight: 58
train_positive_role_hit_milli: 749
train_negative_role_hit_milli: 324
heldout_positive_role_hit_milli: 756
heldout_negative_role_hit_milli: 333
binding_coprocessor_positive_weight: 0
binding_coprocessor_negative_weight: 0
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
manual_role_slot_bridge_used: false
l2_time_phase_role_slots: 12

ablation_without_binding_accuracy_milli: 0
ablation_without_action_accuracy_milli: 0
ablation_without_role_accuracy_milli: 0

error_count: 0
```

Full state_after binding_trace gate:

```text
command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
    -- --ignored full_state_after_gate_must_compose_frame_slot_and_unknown_x --nocapture

scope: binding_trace rows only
train_rows: 1000
heldout_rows: 400

frame_accuracy_milli: 1000
binding_x_accuracy_milli: 1000
full_state_after_accuracy_milli: 1000
full_state_after_median_gap: 14964
full_state_after_p10_gap: 11952

ablation_without_frame_accuracy_milli: 0
ablation_without_binding_accuracy_milli: 0

frame_delta_edges: 16608
binding_state_delta_edges: 0
binding_role_edges: 537

frame_id_training_used: false
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
```

Noisy full state_after gate:

```text
command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
    -- --ignored noisy_full_state_after_gate_must_survive_marker_relative_noise --nocapture

scope: clean train, noisy binding_trace heldout
train_rows: 1000
noisy_heldout_rows: 400

frame_accuracy_milli: 1000
binding_x_accuracy_milli: 1000
full_state_after_accuracy_milli: 1000
full_state_after_median_gap: 4166
full_state_after_p10_gap: 3218

noisy_marker_relative_roles_used: true
binding_state_delta_edges: 0
concrete_x_lookup_used: false
```

All current binding-pressure rows Step 12 gate:

```text
command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
    -- --ignored step12_all_binding_pressure_rows_must_compose_current_full_state_after --nocapture

scope:
  all binding-pressure rows
  frame required for binding_trace
  prefix + X transfer for position_binding

train_rows: 2000
heldout_rows: 800
frame_train_tasks: 1000

step12_full_state_after_accuracy_milli: 1000
step12_median_gap: 28328
step12_p10_gap: 10484

ablation_without_frame_accuracy_milli: 500
ablation_without_binding_accuracy_milli: 0

binding_state_delta_edges: 0
frame_delta_edges: 16608

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
```

Ordered multi-token position-binding sequence gate:

```text
proof command:
  cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
    -- --ignored ordered_position_binding_must_learn_multi_slot_sequence_not_bag_copy --nocapture

note:
  ordinary cargo test for this file is compile/smoke only because all R&D gates are ignored.
  The proof is the explicit --ignored command above.

scope:
  persisted v2 JSONL rows
  corpus:
    data/rule_logic_position_sequence_v1/accepted_position_sequence_tasks_v1.jsonl
  shortcut report:
    data/rule_logic_position_sequence_v1/shortcut_gate_report.json
  lengths:
    3, 4, 5, 6
  heldout:
    context noise around sequence span
  correct and wrong candidates share the same token bag
  every negative is a same-bag derangement
  only order / output-slot binding separates them

train_rows: 1260
heldout_rows: 504
heldout_rules: 7
heldout_surface_families: 3

ordered_sequence_accuracy_milli: 1000
flat_ordered_sequence_accuracy_milli: 1000
ordered_sequence_median_gap: 35882
ordered_sequence_p10_gap: 17064
flat_gap_parity_checked_slots: 2376
flat_gap_parity_mismatches: 0

ablation_without_binding_accuracy_milli: 0
bag_of_tokens_shortcut_accuracy_milli: 500
exact_train_lookup_accuracy_milli: 0
markov_bigram_pairwise_accuracy_milli: 466
same_bag_derangement_milli: 1000
persisted_position_sequence_corpus_used: true
local_out_t_runtime_extension_used: false
folded_slot_projection_used: true

state_delta_edges: 0
role_binding_edges: 2516
flat_role_binding_edges: 2299
flat_role_binding_bytes_estimate: 112106

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
```

Position-sequence corpus shortcut gate:

```text
command:
  python3 data/rule_logic_position_sequence_v1/run_shortcut_gates.py

rows: 1764
train_rows: 1260
heldout_rows: 504

exact_lookup_accuracy_milli: 0
proof_rule_id_majority_accuracy_milli: 0
surface_family_majority_accuracy_milli: 0
bag_of_tokens_accuracy_milli: 500
same_bag_derangement_milli: 1000
markov_bigram_pairwise_accuracy_milli: 466

verdict: VALID_POSITION_SEQUENCE_CANDIDATE
```

Interpretation:

```text
This is now a Step-12-scope pass for the current binding-pressure corpus,
plus an ordered multi-token position-binding sequence pass.

The previous state-delta table result was only 609/1000 vs 608/1000 shortcut.
That result remains rejected.

The current proof uses:
  learned frame-wave:
    action -> frame_delta_lanes

  learned binding co-processor:
    action-motif + L2 time-phase role-slot -> X transfer

  flat runtime proof:
    HashMap training memory -> A-grouped flat role-binding table

It does not use target_center_id, proof_rule_id authority, concrete X lookup,
manual bind(X), or frame_id.

The binding-token gate passes all shortcut and ablation checks.
The full state_after gate passes for binding_trace rows.
The noisy heldout gate passes after marker-relative L2 role slots are added.
The all-rows gate passes for the current corpus:
  binding_trace uses frame + X
  position_binding uses prefix + X

The new ordered sequence gate closes the previous position-binding debt:
  correct and wrong candidates contain the same tokens
  output order is the only difference
  L3 must bind source roles to target output slots
```

Critical conclusion:

```text
Do not claim broad full text generation solved.

Current Step 12 pass scope:
  binding_trace full frame + X:
    missing_variable_bind
    conflict_fact_bind

  position_binding current corpus shape:
    mirror_first_bind
    alternation_second_bind
    prefix + one appended X

  ordered sequence persisted v2:
    mirror3_full_sequence
    rotate4_left_sequence
    pair_swap4_sequence
    mirror5_full_sequence
    rotate5_right_sequence
    rotate6_left2_sequence
    pair_swap6_sequence

V2 baseline artifact:
  data/rule_logic_position_sequence_v1/baseline_v2_report.json

V3 pressure gate:
  path:
    data/rule_logic_position_sequence_v3/
  schema_version:
    position_sequence_v3
  current report:
    data/rule_logic_position_sequence_v3/baseline_v3_report.json

  historical original baseline, superseded by current best:
    source log:
      data/rule_logic_position_sequence_v3/DIAGNOSTIC_RUNS.md
    note:
      the 2880-row / 269-strict / 831-energy numbers below are preserved
      as the first v3 pressure failure, not as the current best result.

  historical corpus:
    rows: 2880
    train_rows: 1920
    heldout_rows: 960
    matrix_cells: 960
    lengths: 3, 4, 5, 6, 7, 8
    rule_families: 8
    proof_rule_ids: 48
    surface_families: 4
    noise_types: 5
    same_bag_derangement_milli: 1000

  historical shortcut gate:
    exact_lookup_accuracy_milli: 0
    proof_rule_id_majority_accuracy_milli: 0
    surface_family_majority_accuracy_milli: 0
    length_only_accuracy_milli: 0
    output_position_prior_accuracy_milli: 0
    template_without_sequence_accuracy_milli: 0
    bag_of_tokens_accuracy_milli: 500
    markov_bigram_pairwise_accuracy_milli: 500
    l2_neighbor_target_copy_accuracy_milli: 0
    verdict: VALID_POSITION_SEQUENCE_V3_CANDIDATE

  explicit proof command:
    cargo test -p nando-core --test wavepredictor_binding_pressure_l3 \
      -- --ignored ordered_position_binding_v3_balanced_matrix_must_hold_without_runtime_phase_hack --nocapture

  historical result:
    verdict: FAIL_CURRENT_ARCHITECTURE_ON_V3
    ordered_sequence_accuracy_milli: 269
    flat_ordered_sequence_accuracy_milli: 269
    ordered_sequence_median_gap: -11738
    ordered_sequence_p10_gap: -43728
    flat_gap_parity_checked_slots: 5280
    flat_gap_parity_mismatches: 0
    per_matrix_group_failures: 702
    length_group_failures: 6
    rule_group_failures: 41
    surface_group_failures: 4
    noise_group_failures: 5
    output_slot_failures: 8
    sequence_energy_accuracy_milli: 831
    sequence_energy_median_gap: 46908
    sequence_energy_p10_gap: -10438
    energy_pass_slot_fail: 540
    ablation_without_binding_accuracy_milli: 0
    local_out_t_runtime_extension_used: false

  diagnosis:
    flat runtime is not the cause, because field/flat gap parity is exact.
    noise is not the primary cause: failures are spread across all noise types.
    surface is not the primary cause: failures are spread across all surface families.
    local all-slot gating is much weaker than sequence-level energy pressure:
    energy proxy reaches 831 milli while strict slot gate reaches 269 milli.
    current limit is missing global sequence/operator objective plus dense
    action/rule-position separation under 48 rule ids and 8 output slots.

  static diagnostics:
    artifact: data/rule_logic_position_sequence_v3/static_diagnostics_report.json
    action_vectors: 48
    same_rule_action_similarity_milli: 1000
    different_rule_action_similarity_milli: 553
    same_family_different_length_similarity_milli: 551
    different_family_similarity_milli: 553
    max_different_rule_similarity_milli: 1000
    folded_target_impulses_checked: 236962
    folded_multi_role_hit_milli: 65
    folded_wrong_role_hit_milli: 72
    folded_missing_true_role_milli: 114

  static diagnosis:
    primary suspect is weak action-motif separability: some different rule actions
    produce identical top-wave signatures.
    folded projection pressure is real, but currently looks like a secondary
    amplifier, not the whole failure.

  next investigation:
    do not add manual local_out_t.
    first measure action-motif separability, folded-lane collision pressure, and train_per_cell sensitivity.
    only after concrete evidence consider learned output phase centers.
```

V3 follow-up diagnostics:

```text
artifact:
  data/rule_logic_position_sequence_v3/DIAGNOSTIC_RUNS.md

progress output:
  the explicit v3 Rust gate now prints per-epoch progress:
  training_start, epoch, margin, update_steps, touched_edges, margin_repairs,
  margin_fixed, state_delta_edges, role_binding_edges, training_done.

train_per_cell sweep:
  train_per_cell=2:
    ordered_accuracy_milli: 269
    median_gap: -11738
  train_per_cell=4:
    ordered_accuracy_milli: 222
    median_gap: -14232
  conclusion:
    simply adding more train rows per matrix cell does not fix the gate.

factor isolation:
  symbols + clean + all rules:
    ordered_accuracy_milli: 188
    conclusion: surface/noise is not the root cause.
  four rule families + all contexts:
    ordered_accuracy_milli: 365
    conclusion: reducing rule families helps but does not solve.
  lengths 3-6 + all rules:
    ordered_accuracy_milli: 316
    conclusion: long lengths amplify failure but are not the root cause.
  one rule family:
    rejected by shortcut gate, output_position_prior_accuracy_milli: 1000
    conclusion: too-small slices can become invalid shortcut corpora.

current diagnosis order:
  1. missing global sequence/operator objective is now measured by energy proxy.
  2. weak action/operator separability remains a blocker.
  3. role-binding form is brittle under dense maps.
  4. folded collision pressure is a measurable amplifier.
  5. flat readout, text noise, surface family, and train density are not primary.
```

V3 lineage-driven rescue update:

```text
method:
  use architecture_lineage proof-debt instead of declaring "architecture limit".

changes tested:
  role/filler coverage:
    TOP_ACTION_L1_LANES: 24 -> 64
    TOP_ROLE_L1_LANES: 16 -> 32
  de-superposition:
    SEQ_FEATURE_CENTER_COUNT: 2048 -> 4096
    SEQ_ACTION_SLOT_BASE: 256 -> 0
    u16 center space is used as 16 blocks x 4096 = 65536 centers
  corpus quality:
    remove mathematically equivalent rule/length operators from v3 matrix
  action representation:
    use slot-operator action signature, not target tokens and not proof_rule_id

current best honest v3 probe:
  rows: 2520
  train_rows: 1680
  heldout_rows: 840
  matrix_cells: 840
  proof_rule_ids: 42
  shortcut verdict: VALID_POSITION_SEQUENCE_V3_CANDIDATE
  output_position_prior_accuracy_milli: 24
  strict_slot_ordered_accuracy_milli: 507
  sequence_energy_accuracy_milli: 957
  sequence_energy_median_gap: 151224
  sequence_energy_p10_gap: 27348
  flat_gap_parity_mismatches: 0
  flat_sequence_energy_parity_checked_rows: 840
  flat_sequence_energy_parity_mismatches: 0
  flat_sequence_energy_parity_max_abs_gap_delta: 0
  symmetry_sequence_energy_accuracy_milli: 836
  symmetry_p10_energy_gap: -13864
  non_symmetry_sequence_energy_accuracy_milli: 1000
  non_symmetry_p10_energy_gap: 54150
  slot_accuracy_milli_by_output_slot:
    0: 806
    1: 900
    2: 894
    3: 891
    4: 852
    5: 883
    6: 906
    7: 1000
  ablation_without_binding_accuracy_milli: 0
  local_out_t_runtime_extension_used: false
  proof_rule_id_training_authority_used: false
  concrete_x_lookup_used: false

remaining energy failures:
  full_mirror_len3: 8
  full_mirror_len4: 8
  full_mirror_len5: 12
  full_mirror_len7: 4
  pair_swap_len5: 4

interpretation:
  L3 now strongly behaves as a sequence-level operator judge on v3
  (957/1000 without lookup), but it is not yet a full ordered decoder.
  Non-symmetry rows are sequence-energy solved at 1000/1000; the remaining
  energy failures are concentrated in mirror-like symmetric operators.
  Strict ordered decoder remains weaker than sequence energy; normalized slot
  accuracy points to slot 0 and slot 4 as the heaviest local readout debts.
  Next proof-debt is mirror/symmetry operator consistency, not bigger data
  or manual local_out_t.

combined objective probe:
  method:
    local role/filler slot training first
    then sequence/operator energy cleanup
    no target_id, no proof_rule_id authority, no concrete_x_lookup, no local_out_t
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/rust_gate.log
  result:
    strict_slot_ordered_accuracy_milli: 705
    sequence_energy_accuracy_milli: 988
    sequence_energy_p10_gap: 36878
    energy_pass_slot_fail: 238
    symmetry_sequence_energy_accuracy_milli: 955
    symmetry_p10_energy_gap: 9416
    non_symmetry_sequence_energy_accuracy_milli: 1000
    flat_sequence_energy_parity_mismatches: 0
    flat_gap_parity_mismatches: 0
    state_delta_edges: 0
    role_binding_edges: 16357
  interpretation:
    combined local+global training is a real improvement:
      strict slot: 507 -> 705
      sequence energy: 957 -> 988
      symmetry energy: 836 -> 955
    It is still not a completed compact operator proof because strict ordered
    slot readout is not stable at 1000/1000.
```

channel ablation follow-up:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/rust_gate.log
  result:
    ablation_without_binding_accuracy_milli: 0
    ablation_without_action_accuracy_milli: 0
    ablation_without_action_energy_accuracy_milli: 0
    ablation_without_role_accuracy_milli: 0
    ablation_without_role_energy_accuracy_milli: 0
    ablation_without_active_fringe_accuracy_milli: 0
  interpretation:
    The current combined-objective signal depends on the intended binding,
    action, role, and active-fringe channels. It is not being carried by a
    hidden state-delta table (`state_delta_edges: 0`) or by a no-action/no-role
    residual shortcut. This closes the channel-ablation support debt for the
    current best probe, but it does not close the strict ordered decoder debt
    (`strict_slot_ordered_accuracy_milli: 705`).

output-slot cleanup diagnostic:
  result:
    per-slot accuracy: 933 milli
    row-level strict ordered accuracy: 705 milli
    failed local slots: 328
    accuracy by output slot:
      slot0: 937
      slot1: 949
      slot2: 904
      slot3: 968
      slot4: 897
      slot5: 892
      slot6: 966
      slot7: 1000
    symmetry accuracy by output slot:
      slot0: 850
      slot1: 859
      slot2: 727
      slot3: 925
      slot4: 800
      slot5: 775
      slot6: 950
      slot7: 1000
    non-symmetry accuracy by output slot:
      slot0: 968
      slot1: 981
      slot2: 966
      slot3: 984
      slot4: 930
      slot5: 931
      slot6: 971
      slot7: 1000
  interpretation:
    The decoder is not uniformly broken. It is mostly healthy on non-symmetry
    rows, but mirror/pair-swap rows still create local slot ambiguity. The next
    fix must target symmetry/operator consistency and slot disambiguation, not a
    manual output-time coordinate.

attractor basin stability diagnostic:
  perturbation sweep over active-fringe input:
    clean:
      slot: 705
      energy: 988
      p10_energy_gap: 36878
    weaken_x2:
      slot: 683
      energy: 995
      p10_energy_gap: 19326
    drop_mod_11:
      slot: 654
      energy: 985
      p10_energy_gap: 26454
    drop_mod_7:
      slot: 624
      energy: 983
      p10_energy_gap: 25494
    drop_mod_5:
      slot: 631
      energy: 980
      p10_energy_gap: 23548
    drop7_distract8:
      slot: 626
      energy: 987
      p10_energy_gap: 24894
    drop5_distract16:
      slot: 633
      energy: 980
      p10_energy_gap: 23548
  interpretation:
    The sequence/operator energy basin is robust under controlled active-fringe
    weakening, dropout, and distractors. The local ordered readout degrades
    much earlier than the global operator-energy decision, so the next blocker
    is decoder/slot crystallization rather than loss of the learned operator
    basin.

proxy energy monotonicity diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/proxy_energy_monotonicity_report.json
  cleanup trace:
    epoch1:
      energy_accuracy: 961
      p10_energy_gap: 25148
      min_energy_gap: -32876
    epoch2:
      energy_accuracy: 961
      p10_energy_gap: 30572
      min_energy_gap: -20566
    epoch3:
      energy_accuracy: 965
      p10_energy_gap: 26346
      min_energy_gap: -5602
    epoch4:
      energy_accuracy: 992
      p10_energy_gap: 36536
      min_energy_gap: -168
  interpretation:
    This is a proxy sequence-energy diagnostic, not a formal thermodynamic
    proof. The p10 curve is not strictly monotonic, but the worst-row
    `min_energy_gap` improves monotonically toward zero, energy accuracy is
    non-decreasing, and p10 remains positive/bounded during cleanup.

capacity curve diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/capacity_curve_report.json
  by length:
    len3: slot 513, energy 925, p10 3718
    len4: slot 729, energy 1000, p10 24034
    len5: slot 700, energy 971, p10 33412
    len6: slot 706, energy 1000, p10 73546
    len7: slot 663, energy 1000, p10 115548
    len8: slot 825, energy 1000, p10 165934
  by rule family:
    full_mirror: slot 83, energy 917, p10 3488
    pair_swap: slot 670, energy 1000, p10 38808
    even_odd_split: slot 1000, energy 1000, p10 50996
  interpretation:
    The current capacity limit is not monotonic sequence length. Length 8 is
    stronger than length 3. The collapse axis is rule-family geometry, centered
    on `full_mirror`, with `pair_swap` as a weaker related debt. This points the
    next mechanism search toward symmetry/operator separability rather than more
    memory or a manual phase coordinate.

address-radius diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/address_radius_report.json
  input-surface perturbations:
    clean:
      slot: 705
      energy: 988
      p10_energy_gap: 36878
    action_wrapped:
      slot: 382
      energy: 904
      p10_energy_gap: 1218
    source_slot0_suffix:
      slot: 682
      energy: 975
      p10_energy_gap: 30694
    source_all_suffix:
      slot: 638
      energy: 983
      p10_energy_gap: 12482
    action_wrapped_source_slot0_suffix:
      slot: 354
      energy: 889
      p10_energy_gap: -4358
  interpretation:
    The source/role address radius is fairly robust, even when every source
    token gets a suffix typo. The action/operator address is fragile: wrapping
    the action text almost erases the p10 gap, and combining action wrapper with
    one source typo turns p10 negative. This reinforces action/operator
    separability as the next true bottleneck.

collision audit diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/combined_objective_probe/collision_audit_report.json
  l1/action signature collision:
    action_vectors: 42
    max_different_rule_similarity: 969
    nearest collision:
      full_mirror_len3 vs rotate_left_1_len3
  l2/folded role pressure:
    wrong_role_hit_milli: 64
    missing_true_role_milli: 0
  l3/role-binding polysemy:
    flat_nonzero_role_binding_edges: 14600
    raw_role_binding_edges: 16357
    action_centers_with_edges: 1176
    action_centers_with_multi_slot_edges: 1176
    max_slots_per_action_center: 8
  interpretation:
    The main collision path is not missing source-role recall. The true role is
    always present in the folded audit. The pressure is action/operator
    ambiguity plus compact shared L3 role-binding edges. That compactness is
    useful, but under near-identical action signatures it becomes polysemantic
    pressure, matching the `full_mirror` collapse.

multi-seed robustness diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/multi_seed_robustness_report.json
  status:
    PARTIAL_ENERGY_ROBUST_STRICT_DECODER_NOT_CLOSED
  runtime seeds:
    1, 2, 3
  result:
    shortcut gates: all valid
    forbidden-authority guards: all zero
    sequence_energy_accuracy: 974..992
    strict_slot_accuracy: 586..621
    full_mirror_energy_accuracy: 842..958
  interpretation:
    Multi-seed now supports the sequence-energy operator claim across three
    independently generated corpora. It does not close the final proof:
    strict ordered slot readout remains weak and `full_mirror` remains the
    unstable family.

beyond-v3 generalization diagnostic:
  artifact:
    data/rule_logic_position_sequence_v3/diagnostics/generalization_beyond_v3_report.json
  status:
    PARTIAL_LENGTH_9_12_LEARNED_OUTPUT_SLOT_STRICT_DECODER_IMPROVED_NOT_CLOSED
  new length slice:
    artifact:
      data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011/
    lengths: [9, 10, 11, 12]
    rows: 1920
    train_rows: 1280
    heldout_rows: 640
    seed: 11
    shortcut_verdict: VALID_POSITION_SEQUENCE_V3_CANDIDATE
    exact_lookup_accuracy_milli: 0
    l2_neighbor_target_copy_accuracy_milli: 0
    markov_bigram_pairwise_accuracy_milli: 500
    bag_of_tokens_accuracy_milli: 500
    same_bag_derangement_milli: 1000
    folded_missing_true_role_milli: 239
    strict_slot_ordered_accuracy_milli: 0
    flat_strict_slot_ordered_accuracy_milli: 0
    sequence_energy_accuracy_milli: 1000
    sequence_energy_p10_gap: 131766
    symmetry_sequence_energy_accuracy_milli: 1000
    non_symmetry_sequence_energy_accuracy_milli: 1000
    output_slot_cleanup_accuracy_milli: 693
    output_slots_8_to_11_accuracy_milli: 0
    flat_sequence_energy_parity_mismatches: 0
    ablations_without_binding_action_role_active: 0
    state_delta_edges: 0
    forbidden_authority_flags: false
  learned_output_slot_key_follow_up:
    mechanism:
      (action_center, output_slot_id, source_role_slot_id, sign_key) -> learned weight
    manual_local_out_t: false
    base_v3_preserved:
      strict_slot_ordered_accuracy_milli: 705
      sequence_energy_accuracy_milli: 988
      flat_sequence_energy_parity_mismatches: 0
    length_9_12_result:
      strict_slot_ordered_accuracy_milli: 734
      flat_strict_slot_ordered_accuracy_milli: 734
      sequence_energy_accuracy_milli: 1000
      sequence_energy_p10_gap: 233720
      symmetry_sequence_energy_accuracy_milli: 1000
      output_slot_cleanup_accuracy_milli: 970
      output_slots_8_to_11_accuracy_milli: [955, 894, 969, 1000]
      ablations_without_binding_action_role_active: 0
      flat_sequence_energy_parity_mismatches: 0
      flat_gap_parity_mismatches: 0
      state_delta_edges: 0
      forbidden_authority_flags: false
    length_9_12_cleanup8_result:
      cleanup_epochs: 8
      strict_slot_ordered_accuracy_milli: 778
      flat_strict_slot_ordered_accuracy_milli: 778
      sequence_energy_accuracy_milli: 1000
      sequence_energy_p10_gap: 267892
      symmetry_sequence_energy_accuracy_milli: 1000
      output_slot_cleanup_accuracy_milli: 976
      output_slots_8_to_11_accuracy_milli: [958, 946, 994, 1000]
      length_strict_accuracy_milli:
        len9: 825
        len10: 638
        len11: 750
        len12: 900
      rule_strict_accuracy_milli:
        full_mirror: 300
        rotate_left_1: 988
        rotate_left_2: 938
        even_odd_split: 950
      ablations_without_binding_action_role_active: 0
      flat_sequence_energy_parity_mismatches: 0
      flat_gap_parity_mismatches: 0
      state_delta_edges: 0
      forbidden_authority_flags: false
  not validated:
    new rule families, new token/noise families, multi-seed beyond-v3,
    strict ordered decoder at 1000 milli, learned readout for output slots
    strict cleanup for length 9/10/11 slots, fixed full_mirror family on base
    v3 multi-seed.
  interpretation:
    Length 9..12 now proves a stronger sequence-energy operator-judge signal
    without lookup or manual output time. It also exposes a concrete ordered
    decoder limit in the pre-learned-output-slot readout.
    Two direct packing attempts were measured and rejected as defaults:
    `slot16_span2048` regressed base v3 to strict 679 / energy 985, and
    `slot12_span2730` regressed base v3 to strict 649 / energy 983. Both remove
    missing-slot pressure but increase folded wrong-role pressure. The learned
    output-slot key then breaks the 8-slot ceiling without manual `local_out_t`
    and raises length 9..12 strict readout from 0 to 734 milli while preserving
    base v3. The next debt is strict slot cleanup to 1000, not a hand-coded
    output phase.
    Cleanup8 improves strict readout further to 778 milli, but also shows the
    remaining blocker clearly: full_mirror strict readout is only 300 milli.
  verdict:
    `BEYOND_V3_LENGTH_GENERALIZATION_PARTIAL_LEARNED_OUTPUT_SLOT_BREAKTHROUGH_STRICT_DECODER_NOT_PROVEN`

operator-pair action motif follow-up:

  reason:
    The measured blocker was action/operator motif separability. Extra
    same-bag candidate pressure was tested first and rejected:
    cleanup4+candidate1 regressed base v3 to strict 587 / energy 981 /
    full_mirror strict 200; cleanup8+candidate1 regressed to strict 624 /
    energy 974 / full_mirror strict 242.

  accepted mechanism:
    derive operator-pair action centers from the `rule_action_example`
    demonstration and feed them into the existing learned role/filler binding:

      (action_center, output_slot_id, source_role_slot_id, sign_key)
        -> learned weight

    This is not target_id, not proof_rule_id authority, not concrete_x lookup,
    and not a manual runtime `local_out_t` extension. It is a stronger L2 -> L3
    action representation. Claim boundary: the current gate still uses a
    deterministic extractor for formal `operator_slots:` text; a later gate
    must prove learned L2 induction from less formal demonstrations.

  base_v3 artifact:
    data/rule_logic_position_sequence_v3/diagnostics/operator_pair_action_base_v3/combined_objective_rust_gate.log

  base_v3 result:
    strict_slot_ordered_accuracy_milli: 1000
    flat_strict_slot_ordered_accuracy_milli: 1000
    sequence_energy_accuracy_milli: 1000
    sequence_energy_p10_gap: 225820
    symmetry_sequence_energy_accuracy_milli: 1000
    full_mirror_strict_accuracy_milli: 1000
    output_slot_cleanup_accuracy_milli: 1000
    ablations_without_binding_action_role_active: 0
    flat_sequence_energy_parity_mismatches: 0
    flat_gap_parity_mismatches: 0
    state_delta_edges: 0
    forbidden_authority_flags: false

  beyond_v3 length 9..12 artifact:
    data/rule_logic_position_sequence_v3/diagnostics/beyond_v3/length_9_12_seed_011_operator_pair_action/combined_objective_rust_gate.log

  beyond_v3 length 9..12 result:
    heldout_rows: 640
    strict_slot_ordered_accuracy_milli: 1000
    flat_strict_slot_ordered_accuracy_milli: 1000
    sequence_energy_accuracy_milli: 1000
    sequence_energy_p10_gap: 1166244
    symmetry_sequence_energy_accuracy_milli: 1000
    non_symmetry_sequence_energy_accuracy_milli: 1000
    output_slots_0_to_11_accuracy_milli: all 1000
    rule_family_accuracy_milli: all 1000, including full_mirror
    basin_energy_accuracy_milli: all 1000 in tested perturbations
    source_all_suffix_slot_accuracy_milli: 983
    ablations_without_binding_action_role_active: 0
    flat_sequence_energy_parity_mismatches: 0
    flat_gap_parity_mismatches: 0
    state_delta_edges: 0
    forbidden_authority_flags: false

  updated status:
    Current v3 and one beyond-v3 length slice are green with operator-pair
    action motifs. Final compact transferable operator claim is still bounded
    by remaining proof-debts: multi-seed beyond-v3, new rule/token/noise
    families, length > 12 or dynamic slot capacity, and learned L2 induction of
    operator-pair motifs without a test-only parser.

  verdict:
    `CURRENT_V3_AND_LENGTH_9_12_TRANSFER_GREEN_WITH_OPERATOR_PAIR_ACTION_MOTIFS_BROAD_PROOF_STILL_OPEN`
