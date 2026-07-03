# Slot32 Role-Binding Package Rung

Date: 2026-07-03

Verdict:

```text
SLOT32_ROLE_BINDING_PACKAGE_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_role_binding_package_must_roundtrip_and_score_loaded_runtime --nocapture
```

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_package_rung_release.log
```

Runtime:

```text
finished in 735.91s
```

Structural claim-boundary check:

```text
nanda-gate-md /tmp/nanda-task-slot32-role-binding-package-boundary.md --task-id slot32-role-binding-package-boundary --domain code
verdict: PASS
complexity_score: 19
agent_action: SAFE_TO_EDIT
trace_path: /tmp/nanda-structural-gate/slot32-role-binding-package-boundary.trace.json
```

Scope:

```text
32 output slots
32 role slots
64-page paged u32 layout
lengths 17..32
mixed-map + conditional-branch rows
seeds = 3
labels = mixed_map, conditional_branch
package format = serialized role-binding runtime
package magic = NWRB0001
package header bytes = 44
edge bytes = 12
```

Package paths:

```text
target/nando-wave/slot32-role-binding/mixed_map-seed0.nwrb
target/nando-wave/slot32-role-binding/mixed_map-seed1.nwrb
target/nando-wave/slot32-role-binding/mixed_map-seed2.nwrb
target/nando-wave/slot32-role-binding/conditional_branch-seed0.nwrb
target/nando-wave/slot32-role-binding/conditional_branch-seed1.nwrb
target/nando-wave/slot32-role-binding/conditional_branch-seed2.nwrb
```

Per-seed package results:

```text
mixed seed=0 bytes=17948 fingerprint64=16663608610845118970 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=576658 false_local_accepts=0 hot_bytes=607736
conditional seed=0 bytes=26468 fingerprint64=12438530158304250964 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=623242 false_local_accepts=0 hot_bytes=681792
mixed seed=1 bytes=17948 fingerprint64=2007847219677212179 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=603259 false_local_accepts=0 hot_bytes=607736
conditional seed=1 bytes=26468 fingerprint64=365065097387925697 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=583808 false_local_accepts=0 hot_bytes=681792
mixed seed=2 bytes=17948 fingerprint64=5795255548068278463 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=603050 false_local_accepts=0 hot_bytes=607736
conditional seed=2 bytes=26468 fingerprint64=14917641573332468348 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=548132 false_local_accepts=0 hot_bytes=681792
```

Aggregate:

```text
seeds: 3
labels: conditional_branch, mixed_map
local_margin_threshold: 1000000
p99_latency_gate_ns: 1000000
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_flat_gap_parity_mismatches: 0
total_flat_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
nonzero_fingerprints: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 623242
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
corpus_jsonl_used: false
rust_runtime_used: true
```

Interpretation:

```text
This closes the serialized 32-slot role-binding package proof for the current
mixed-map plus conditional-branch Rust runtime path. The test writes package
bytes, inspects the header, reloads the package into a fresh flat runtime,
requires exact rewrite parity, and re-scores heldout rows from loaded bytes.
```

Boundary:

```text
This is a `.nwrb` role-binding runtime package proof.
It is not the phase-center `.nwpc` package path.
It does not close raw-language action parsing, autonomous action_tree induction,
insert-new-constant edit operators, 64-slot capacity, broad workflow reasoning,
text generation, or the packaged daemon/API product p99 path.
```
