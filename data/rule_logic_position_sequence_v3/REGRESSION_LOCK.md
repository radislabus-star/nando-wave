# v3 / v3.5 Paged u32 16-Slot Regression Lock

Date: 2026-07-01

This directory contains the frozen green ordered-sequence regression proof for
the paged `u32` 16-slot rung.

Do not rewrite this rung when building later batteries. New pressure suites
must live next to it as separate artifacts.

Canonical green artifact:

```text
data/rule_logic_position_sequence_v3/diagnostics/paged_u32_length_13_16_seed_021/
```

Key evidence:

```text
accepted_position_sequence_tasks_v3.jsonl
manifest.json
shortcut_gate_report.json
combined_objective_paged_u32_rust_gate.log
PAGED_U32_16_SLOT_REPORT.md
```

Frozen claim:

```text
16-slot ordered operator transfer passed on lengths 13..16.
strict ordered slot readout = 1000/1000
sequence energy = 1000/1000
flat gap parity mismatches = 0
flat sequence-energy parity mismatches = 0
ablations collapse to 0
state_delta_edges = 0
role_binding_edges = 143248
target_center_id_training_used = false
proof_rule_id_training_authority_used = false
concrete_x_lookup_used = false
local_out_t_runtime_extension_used = false
```

Boundary:

```text
This proves the 16-slot ordered sequence rung.
It does not prove 32-slot transfer, broad v4 operator battery,
release latency, or general LLMWave reasoning.
```

Next suites must not mutate this evidence. They should be built under separate
directories, starting with:

```text
data/rule_logic_operator_battery_v4/
```
