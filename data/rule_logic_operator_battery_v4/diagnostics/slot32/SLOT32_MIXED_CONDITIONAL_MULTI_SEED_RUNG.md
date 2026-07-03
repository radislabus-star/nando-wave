# Slot32 Mixed/Conditional Multi-Seed Rung

Date: 2026-07-03

Verdict:

```text
SLOT32_MIXED_CONDITIONAL_MULTI_SEED_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_mixed_conditional_multiseed_must_transfer_without_lookup_or_runtime_phase_hack --nocapture
```

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_mixed_conditional_multiseed_rung_release.log
```

Runtime:

```text
finished in 2294.10s
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-mixed-conditional-status.md --task-id slot32-mixed-conditional-status --domain code
verdict: PASS
complexity_score: 23
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-mixed-conditional-status.trace.json
```

Scope:

```text
32 output slots
32 role slots
64 pages
262144 total centers
role_top_l1_lanes = 64
lengths 17..32
seeds = 3
```

Per-seed mixed-map results:

```text
seed=0 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=3106560 role_edges=1492 hot_bytes=607736 flat_avg_ns=159357 classes=3 rules=16 surfaces=4 noise=2 lengths=16 same_bag=1536 edit=512 edit_non_same_bag=512 max_state_reuse=16 token_overlap=0
seed=1 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=3003904 role_edges=1492 hot_bytes=607736 flat_avg_ns=151276 classes=3 rules=16 surfaces=4 noise=2 lengths=16 same_bag=1536 edit=512 edit_non_same_bag=512 max_state_reuse=16 token_overlap=0
seed=2 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=2975744 role_edges=1492 hot_bytes=607736 flat_avg_ns=172809 classes=3 rules=16 surfaces=4 noise=2 lengths=16 same_bag=1536 edit=512 edit_non_same_bag=512 max_state_reuse=16 token_overlap=0
```

Per-seed conditional-branch results:

```text
seed=0 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=3122560 role_edges=2202 hot_bytes=681792 flat_avg_ns=159242 classes=1 rules=8 surfaces=4 noise=2 lengths=16 same_bag=2048 true=1024 false=1024 direct_pair=0 condition_action=50176 state_condition=120832 no_condition_action=0 no_condition_action_energy=0 max_state_reuse=16 token_overlap=0
seed=1 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=3006720 role_edges=2202 hot_bytes=681792 flat_avg_ns=161383 classes=1 rules=8 surfaces=4 noise=2 lengths=16 same_bag=2048 true=1024 false=1024 direct_pair=0 condition_action=50176 state_condition=120832 no_condition_action=0 no_condition_action_energy=0 max_state_reuse=16 token_overlap=0
seed=2 pass=true slot=1000 flat_slot=1000 energy=1000 p10_energy_gap=2991232 role_edges=2202 hot_bytes=681792 flat_avg_ns=154375 classes=1 rules=8 surfaces=4 noise=2 lengths=16 same_bag=2048 true=1024 false=1024 direct_pair=0 condition_action=50176 state_condition=120832 no_condition_action=0 no_condition_action_energy=0 max_state_reuse=16 token_overlap=0
```

Aggregate:

```text
mixed_min_slot_accuracy_milli: 1000
mixed_min_flat_slot_accuracy_milli: 1000
mixed_min_sequence_energy_accuracy_milli: 1000
mixed_min_sequence_energy_p10_gap: 2975744
mixed_total_energy_pass_slot_fail: 0
mixed_total_flat_gap_parity_mismatches: 0
mixed_total_flat_sequence_energy_parity_mismatches: 0

conditional_min_slot_accuracy_milli: 1000
conditional_min_flat_slot_accuracy_milli: 1000
conditional_min_sequence_energy_accuracy_milli: 1000
conditional_min_sequence_energy_p10_gap: 2991232
conditional_total_energy_pass_slot_fail: 0
conditional_total_flat_gap_parity_mismatches: 0
conditional_total_flat_sequence_energy_parity_mismatches: 0
conditional_total_direct_operator_pair_active_centers: 0
conditional_max_ablation_without_condition_action_accuracy_milli: 0
conditional_max_ablation_without_condition_action_energy_accuracy_milli: 0
max_role_binding_edges: 2202
max_hot_bytes_estimate: 681792
max_flat_eval_avg_ns_per_row: 172809
flat_eval_latency_gate_ns: 1000000
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
direct_operator_pair_action_centers_used_for_conditional: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Interpretation:

```text
This closes the 32-slot mixed-map plus conditional-branch multi-seed robustness
rung over Rust-generated symbolic operator tasks. Mixed-map covers order,
edit-map, and composed-map transfer. Conditional branch covers state/action
branch selection without direct operator-pair action centers.
```

Boundary:

```text
This is not the full 32-slot product package proof.
It does not close raw-language action parsing, autonomous action_tree induction,
insert-new-constant edit operators, packed product runtime proof, product p99,
64-slot capacity, broad workflow reasoning, or text generation.
```
