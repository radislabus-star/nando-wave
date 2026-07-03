# Linux Networking VPN Accepted 10k V1

Russian translation:

```text
Это первый 10k-корпус принятых задач для домена Linux / сеть / VPN /
диагностика.
```

## Boundary

```text
accepted_wave_task_v2.jsonl = accepted task corpus only after shortcut gates
accepted_training_tasks = 10000 only if shortcut gate verdict passes
not_runtime_authority = true
```

## Result

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

Near-negatives are target-like cross-operator decoys:

```text
wrong answer surface = another valid VPN target shape
wrong answer reason = wrong operator/layer for the current evidence
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
python3 data/task_candidates/linux_networking_vpn_accepted_10k_v1/build_accepted_10k.py
python3 data/task_candidates/validate_wave_task_v2.py \
  data/task_candidates/linux_networking_vpn_accepted_10k_v1/accepted_wave_task_v2.jsonl
python3 data/task_candidates/linux_networking_vpn_accepted_10k_v1/run_shortcut_gates.py
```
