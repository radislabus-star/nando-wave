# Linux Networking VPN Accepted 10k V2

Russian translation:

```text
10k-корпус задач для Linux / сеть / VPN / диагностика.
V2 чинит главный shortcut V1: source_group больше не равен оператору.
```

## Boundary

```text
accepted_wave_task_v2.jsonl = accepted task corpus only after shortcut gates
source_group = mixed bucket, not operator_family
accepted_training_tasks = 10000 only if shortcut gate verdict passes
not_runtime_authority = true
```

## Result

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

## What Changed From V1

```text
V1:
  source_group mapped almost directly to operator_family
  source_group-only shortcut could solve too much

V2:
  source_group is vpn_mixed_bucket_00..15
  every bucket contains all 24 operator families
  near_negative is a real target text from another operator
```

## Files

```text
build_accepted_10k.py
run_shortcut_gates.py
accepted_wave_task_v2.jsonl
shortcut_gate_report.json
manifest.json
```

## Commands

```bash
python3 data/task_candidates/linux_networking_vpn_accepted_10k_v2/build_accepted_10k.py
python3 data/task_candidates/validate_wave_task_v2.py \
  data/task_candidates/linux_networking_vpn_accepted_10k_v2/accepted_wave_task_v2.jsonl
python3 data/task_candidates/linux_networking_vpn_accepted_10k_v2/run_shortcut_gates.py
```
