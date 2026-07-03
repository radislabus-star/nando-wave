# Slot32 Order Corpus Multi-Seed Rung

Date: 2026-07-03

This extends the first real 32-slot order corpus rung from one seed to three
seed-varied token surfaces. It keeps the same paged `u32` layout, same flat
runtime path, same no-lookup/no-target/no-local-out-t boundary, and the same
rule-independent token design.

It is not a full 32-slot product proof: edit, conditional, composed, packed
product p99, and broad workflow offload remain open.

## Command

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_order_corpus_multiseed_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Saved log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_order_corpus_multiseed_rung_release.log
```

## Corpus Pressure

```text
seeds: 3
rows_per_seed_train: 1024
rows_per_seed_heldout: 1024
unique_rules: 8
unique_surfaces: 4
unique_noise_types: 2
unique_lengths: 16
lengths: 17..32
same_bag_rows_per_seed: 1024
max_state_reuse_per_seed: 8
train_tokens_overlap_heldout_per_seed: 0
```

Important boundary:

```text
Tokens are independent of rule_name.
The same state key is reused across 8 different rules.
Heldout token surfaces are disjoint from train token surfaces.
Therefore input/state alone is not sufficient; action/operator-pair channel is
required for the transfer.
```

## Result

```text
verdict: SLOT32_ORDER_CORPUS_MULTI_SEED_RUNG_PASS

page_count: 64
total_center_count: 262144
output_slot_count: 32
role_slot_count: 32
role_top_l1_lanes: 64

min_slot_accuracy_milli: 1000
min_flat_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
min_sequence_energy_p10_gap: 2976640
total_energy_pass_slot_fail: 0
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0

max_role_binding_edges: 1354
max_hot_bytes_estimate: 606080
max_flat_eval_avg_ns_per_row: 187982
flat_eval_latency_gate_ns: 1000000

target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Per-seed summaries:

```text
seed 0:
  slot / flat / energy: 1000 / 1000 / 1000
  p10_energy_gap: 3110016
  role_binding_edges: 1354
  hot_bytes: 606080
  flat_avg_ns: 155844
  token_overlap: 0

seed 1:
  slot / flat / energy: 1000 / 1000 / 1000
  p10_energy_gap: 3007232
  role_binding_edges: 1354
  hot_bytes: 606080
  flat_avg_ns: 172744
  token_overlap: 0

seed 2:
  slot / flat / energy: 1000 / 1000 / 1000
  p10_energy_gap: 2976640
  role_binding_edges: 1354
  hot_bytes: 606080
  flat_avg_ns: 187982
  token_overlap: 0
```

## Claim Boundary

This closes:

```text
32-slot order corpus multi-seed robustness;
3 seed-varied token layouts;
lengths 17..32;
8 order/permutation families;
4 surface families;
2 noise/token families;
same-bag heldout negatives;
rule-independent tokens;
disjoint train/heldout token surfaces;
field/flat parity;
binding/action/role/active ablation collapse;
sub-4MiB hot table estimate.
```

This does not close:

```text
full 32-slot operator battery;
32-slot edit / conditional / composed gates;
packed product runtime proof;
product p99 latency proof;
64-slot capacity;
broad product reasoning;
autonomous raw action parsing;
text generation.
```
