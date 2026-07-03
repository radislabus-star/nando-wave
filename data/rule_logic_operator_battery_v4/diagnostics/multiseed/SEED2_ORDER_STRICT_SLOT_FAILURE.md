# Seed2 Order Strict Slot Failure

Date: 2026-07-01

## Verdict

```text
RED_STRICT_SLOT_READOUT_CASE
```

The seeds 1,2,3 runtime sweep is not strict-green. The runner originally showed
`accuracy_milli: 1000`, but seed2/order contains one concrete strict slot
failure. The parser now treats any `failed_slots > 0` as a red strict gate.

## Evidence

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/order_runtime_gate_release.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/order_runtime_gate_release_strict_red_repro.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed2_order_static_diagnostic.log
data/rule_logic_operator_battery_v4/diagnostics/multiseed/seed_002/order/order_seed2_priem_dynamic_weight_audit.log
```

Failure:

```text
operator_class: order
seed: 2
rule: order_block_reverse_4_len13
source_group: operator_battery_order_heldout_order_block_reverse_4_len13
length: 13
surface: ru_words
noise: clean
output_slot: 12
source_slot: 12
correct_token: priem
wrong_token: oblast
slot_gap: -11426
sequence_energy_gap: 4393850
```

Gate metrics:

```text
order_slot_ordered_sequence_accuracy_milli: 1000
order_sequence_energy_accuracy_milli: 1000
order_energy_pass_slot_fail: 1
order_output_slot_cleanup_failed_slots: 1
order_slot_failure_total: 1
flat_sequence_energy_parity_mismatches: 0
flat_gap_parity_mismatches: 0
state_delta_edges: 0
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
```

The Rust runtime gate now asserts these strict counts directly:

```text
energy.energy_pass_slot_fail == 0
slot_cleanup.failed_slots == 0
```

With the seed2/order corpus and the normal operator-pair action centers enabled,
the gate fails as expected:

```text
thread 'operator_battery_v4_order_must_transfer_without_lookup_or_runtime_phase_hack'
panicked at crates/nando-core/tests/wavepredictor_binding_pressure_l3.rs
assertion `left == right` failed
left: 1
right: 0
```

## Interpretation

This is not a lookup/shortcut failure. The forbidden flags remain false and the
global sequence energy chooses the correct sequence.

The failure is narrower:

```text
sequence/operator energy: correct
strict per-slot readout: one high-slot self-transfer failure
```

The pressure point is now narrowed to role/filler collision in strict readout,
not to operator recognition.

Static comparison for the same heldout row shape:

```text
seed1 same rule/out12 self-transfer:
  correct_token: reyestr
  wrong_token: vybor
  target_wrong_cosine_milli: 58
  positive_target_impulses_hit_other_role: 0 / 22
  positive_target_impulses_multi_role: 0 / 22

seed2 failing row:
  correct_token: priem
  wrong_token: oblast
  target_wrong_cosine_milli: 0
  correct_wrong_role_overlap: 0
  positive_target_impulses_hit_other_role: 8 / 16
  positive_target_impulses_multi_role: 8 / 16

seed3 same rule/out12 self-transfer:
  correct_token: stek
  wrong_token: paket
  target_wrong_cosine_milli: 0
  positive_target_impulses_hit_other_role: 2 / 13
  positive_target_impulses_multi_role: 2 / 13
```

For the full `order_block_reverse_4_len13/out12->src12` cell:

```text
seed1 positive_multi_role_milli: 148
seed2 positive_multi_role_milli: 167
seed3 positive_multi_role_milli: 168
```

So seed2 is not globally more collision-heavy than seed3. The specific failing
row is the local maximum collision row for seed2:

```text
max_row: heldout correct=priem wrong=oblast surface=ru_words noise=clean
max_row_multi_role: 8
max_row_hit_other: 8
```

Interpretation:

```text
operator energy is correct;
target/wrong L1 similarity is not the cause;
correct/wrong role-lane overlap is not the cause;
strict readout fails because many target lanes for priem also receive pressure
from other active role slots.
```

Dynamic weight audit confirms the failure is not missing true binding. The true
slot is present and positive, but learned suppression from other role slots
overpowers it:

```text
target_score: -18810
wrong_score: -7384
slot_gap: -11426
sequence_energy_gap: 4393850
binding_weight_positive: 0
binding_weight_negative: 0
direct lane score: 0 for the audited lanes
self-transfer score: 0 for the audited lanes
target_multi_role_impulses: 8
target_multi_role_total_sum: -35370
target_multi_role_true_slot_sum: 15680
target_multi_role_other_slot_sum: -51050
target_multi_role_slot_totals:
  slot0:  -4416
  slot2:  -4956
  slot5:  -3712
  slot8:   2456
  slot10: -28740
  slot11: -11682
  slot12:  15680
```

The worst lane is `2781`:

```text
roles: [2, 5, 10, 11, 12]
total: -17282
true op_pair(out12->src12) role12 contribution: +3328
other role suppression dominates, especially role10/role11.
```

Cleanup did not repair anything on train:

```text
cleanup_epoch=1..4 repaired_rows=0
train_slot_accuracy_milli=1000
train_energy_accuracy_milli=1000
min_slot_gap=17514
```

Interpretation:

```text
the train set is internally clean;
the heldout failure is local collision pressure under learned role suppression;
more generic cleanup epochs alone are unlikely to fix it;
the next test must isolate whether more examples in this exact cell fix the
collision, or whether the strict readout geometry needs a learned cleanup term.
```

Density sweep result:

```text
factor1 baseline:
  target_rule_train_rows: 16
  order_energy_pass_slot_fail: 1
  order_output_slot_cleanup_failed_slots: 1
  slot_failure_total: 1
  result: RED

factor2 targeted reweighting:
  target_rule_train_rows: 32
  duplicated_train_rows: 16
  order_energy_pass_slot_fail: 0
  order_output_slot_cleanup_failed_slots: 0
  slot_failure_total: 0
  result: GREEN

factor4 targeted reweighting:
  target_rule_train_rows: 64
  result: GREEN

factor16 targeted reweighting:
  target_rule_train_rows: 256
  result: GREEN
```

Classification:

```text
data/weight sparsity under local multi-role collision pressure.
```

This does not make the original multi-seed rung green. Targeted reweighting is
a diagnostic, not the final corpus policy. The next proof must replace it with
a principled train-density policy and rerun all gates.

## Next Diagnostic Steps

Do not add architecture and do not add `local_out_t`.

Run targeted diagnostics for this one case:

```text
1. compare seed1/seed3 order_block_reverse_4_len13 out12->src12;
2. inspect role-lane collision pressure for token priem in source slot 12;
3. inspect wrong token oblast lane pressure and shared lanes;
4. check whether self-transfer edges for high output slots are undertrained;
5. test a narrow corpus density increase for block_reverse_4_len13 only;
6. only after diagnosis decide whether this is data sparsity, collision pressure,
   or a real binding/readout weakness.
```

Completed diagnostics:

```text
1. seed1/seed2/seed3 comparison: done;
2. role-lane collision pressure for priem: done;
3. wrong token oblast lane pressure: done for strongest lanes;
4. high-slot self-transfer undertraining: dynamic audit says train is clean,
   heldout collision remains open;
5. narrow corpus density test: done; factor2 already turns the case green.
```

Next concrete experiments:

```text
1. replace targeted duplication with a principled corpus policy, e.g.
   OPERATOR_BATTERY_TRAIN_PER_CELL=2 for the full v4 battery;
2. rebuild seeds 1,2,3 with that policy;
3. rerun shortcut gates and runtime gates for all four classes;
4. keep v4 multi-seed red until strict_runtime_issues is empty.
```

The claim boundary remains:

```text
v4 is strong on the current operator battery, but seeds 1,2,3 strict robustness
is red until this failure is resolved and strict_runtime_issues is empty.
```
