# Seed2 Order Density Sweep

Date: 2026-07-01

## Verdict

```text
CLASSIFY_AS_DATA_WEIGHT_SPARSITY_UNDER_COLLISION_PRESSURE
```

The seed2/order strict failure is not an architecture ceiling. Reweighting the
target train rule by only one extra copy per train row is enough to remove the
heldout strict slot failure.

This is a diagnostic result, not a final proof corpus. The sweep duplicates
existing train rows for one target rule and keeps heldout unchanged.

## Target

```text
seed: 2
operator_class: order
target_rule: order_block_reverse_4_len13
failure: out12->src12, correct_token=priem, wrong_token=oblast
baseline slot_gap: -11426
baseline sequence_energy_gap: 4393850
```

## Corpora

```text
source:
  data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/accepted_operator_tasks_v4.jsonl

generator:
  data/rule_logic_operator_battery_v4/build_seed2_order_density_sweep.py

output:
  data/rule_logic_operator_battery_v4/diagnostics/density_sweep/seed_002/order/
```

The generator preserves all heldout rows and duplicates only train rows whose
`proof_rule_id` is `order_block_reverse_4_len13`.

## Runtime Results

```text
factor 1 / baseline:
  train_rows: 2048
  target_rule_train_rows: 16
  order_energy_pass_slot_fail: 1
  order_output_slot_cleanup_failed_slots: 1
  slot_failure_total: 1
  result: RED
  log:
    data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/order_runtime_gate_release_strict_red_repro.log

factor 2:
  train_rows: 2064
  target_rule_train_rows: 32
  duplicated_train_rows: 16
  order_slot_ordered_sequence_accuracy_milli: 1000
  order_sequence_energy_accuracy_milli: 1000
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0
  ablations without binding/action/role/active_fringe: 0
  flat sequence-energy parity mismatches: 0
  flat gap parity mismatches: 0
  forbidden flags: false
  result: GREEN
  log:
    data/rule_logic_operator_battery_v4/diagnostics/density_sweep/seed_002/order/factor_002/order_runtime_gate_release.log

factor 4:
  train_rows: 2096
  target_rule_train_rows: 64
  duplicated_train_rows: 48
  order_slot_ordered_sequence_accuracy_milli: 1000
  order_sequence_energy_accuracy_milli: 1000
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0
  ablations without binding/action/role/active_fringe: 0
  flat sequence-energy parity mismatches: 0
  flat gap parity mismatches: 0
  forbidden flags: false
  result: GREEN
  log:
    data/rule_logic_operator_battery_v4/diagnostics/density_sweep/seed_002/order/factor_004/order_runtime_gate_release.log

factor 16:
  train_rows: 2288
  target_rule_train_rows: 256
  duplicated_train_rows: 240
  order_slot_ordered_sequence_accuracy_milli: 1000
  order_sequence_energy_accuracy_milli: 1000
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0
  ablations without binding/action/role/active_fringe: 0
  flat sequence-energy parity mismatches: 0
  flat gap parity mismatches: 0
  forbidden flags: false
  result: GREEN
  log:
    data/rule_logic_operator_battery_v4/diagnostics/density_sweep/seed_002/order/factor_016/order_runtime_gate_release.log
```

## Interpretation

```text
The red baseline had enough architecture to solve the case.
The failure came from underweighted target-rule evidence under local
multi-role lane collision pressure.

Only +16 duplicated train rows for the target rule turn the same runtime green.
No target_id, proof_rule_id authority, concrete_x_lookup, local_out_t, or
hand-coded bind(X) was introduced.
```

This does not prove the final v4 battery is robust. It proves the next fix
should be data policy / train-density, not an immediate architecture mechanism.

## Next Required Proof

```text
1. Replace targeted duplication with a principled corpus policy.
   Candidate: OPERATOR_BATTERY_TRAIN_PER_CELL=2 for the full v4 battery.

2. Rebuild seeds 1,2,3 with that policy.

3. Rerun shortcut gates and runtime gates for order/edit/conditional/composed.

4. Accept the policy only if:
   strict_runtime_issues is empty,
   flat parity remains 0,
   ablations remain 0,
   forbidden flags remain false.
```

