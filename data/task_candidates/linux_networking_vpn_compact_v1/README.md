# Linux Networking VPN Compact Task V1

This folder is Step 6 output for the locked first proving domain.

Russian translation:

```text
Это компактный корпус кандидатов задач для домена Linux / сеть / VPN /
диагностика.
```

## Boundary

```text
compact_cases.json = compact source
generated_wave_task_v2.jsonl = materialized candidate tasks
quality_status = candidate
accepted_training_tasks = 0
shortcut_gates_pending = false
shortcut_gate_verdict = VALID_OPERATOR_PRESSURE_CANDIDATE
useful_task_yield_milli = 1000
```

It is not accepted training data yet.

Russian translation:

```text
Это еще не принятый обучающий датасет.
Step 7 и Step 8 пройдены для этого маленького candidate corpus.
Следующий масштабный долг: Step 9.
```

## Files

```text
compact_cases.json
validate_compact_cases.py
materialize_wave_tasks_v2.py
generated_wave_task_v2.jsonl
manifest.json
shortcut_gate_report.json
```

## Commands

```bash
python3 data/task_candidates/linux_networking_vpn_compact_v1/validate_compact_cases.py
python3 data/task_candidates/linux_networking_vpn_compact_v1/materialize_wave_tasks_v2.py
python3 data/task_candidates/validate_wave_task_v2.py \
  data/task_candidates/linux_networking_vpn_compact_v1/generated_wave_task_v2.jsonl
python3 data/task_candidates/linux_networking_vpn_compact_v1/run_shortcut_gates.py
```

## Current Counts

```text
compact_cases: 24
templates_covered: 24
source_groups: 24
materialized_wave_task_v2_rows: 24
```

## Shortcut Gate V1

```text
previous_verdict: REJECT_BAYESIAN_SHORTCUT
current_verdict: VALID_OPERATOR_PRESSURE_CANDIDATE
tasks_total: 24
heldout_tasks: 5
exact_lookup_accuracy_milli: 0
l2_neighbor_accuracy_milli: 0
bayesian_pairwise_accuracy_milli: 600
markov_bigram_accuracy_milli: 400
target_leak_milli: 0
near_negative_similarity_milli: 487
single_token_ratio_milli: 0
useful_candidate_tasks: 24
useful_task_yield_milli: 1000
accepted_training_tasks: 0
```

Russian translation:

```text
точный lookup не решает
L2-neighbor не решает
Markov/Bigram проходит порог
Bayesian baseline теперь ниже порога
near-negative достаточно близкие
```

## Next Gate

```text
Step 9:
  build 1k accepted tasks
  keep the same shortcut gates
  keep accepted_training_tasks at 0 until scale acceptance is explicit
```
