# Slot32 Role-Binding Public SDK Package Rung

Date: 2026-07-03

Verdict:

```text
SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
```

Command:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_role_binding_public_sdk_must_score_loaded_package_runtime --nocapture
```

Log:

```text
data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_public_sdk_package_rung_release.log
```

Runtime:

```text
finished in 813.60s
```

Structural claim-boundary checks:

```text
runtime route:
  nanda-gate-md /tmp/nanda-task-slot32-role-binding-sdk-package-runtime.md --task-id slot32-role-binding-sdk-package-runtime --domain code --format json
  verdict: PASS
  complexity_score: 23
  agent_action: SAFE_TO_EDIT
  trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-package-runtime.trace.json

boundary route:
  nanda-gate-md /tmp/nanda-task-slot32-role-binding-sdk-package-boundary-local.md --task-id slot32-role-binding-sdk-package-boundary-local --domain code --format json
  verdict: PASS
  complexity_score: 16
  agent_action: SAFE_TO_EDIT
  trace_path: /tmp/nanda-structural-gate/slot32-role-binding-sdk-package-boundary-local.trace.json
```

Structural packaging note:

```text
The first aggregate NANDA packet was intentionally not promoted: it returned
VETO because it mixed runtime proof and several claim-boundary exclusions into
one relation shape. Splitting runtime proof and boundary exclusions into two
route-local packets produced PASS on both routes.
```

Scope:

```text
32 output slots
32 role slots
64-page paged u32 layout
lengths 17..32
mixed-map + conditional-branch rows
seeds = 3
labels = sdk_mixed_map, sdk_conditional_branch
package format = serialized role-binding runtime
package magic = NWRB0001
public SDK load path = inspect_package_bytes -> from_package_bytes -> prepared SDK scoring
```

Per-seed package results:

```text
sdk_mixed_map seed=0 bytes=17948 fingerprint64=16663608610845118970 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=617397 false_local_accepts=0 hot_bytes=607736
sdk_conditional_branch seed=0 bytes=26468 fingerprint64=12438530158304250964 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=517273 false_local_accepts=0 hot_bytes=681792
sdk_mixed_map seed=1 bytes=17948 fingerprint64=2007847219677212179 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=595266 false_local_accepts=0 hot_bytes=607736
sdk_conditional_branch seed=1 bytes=26468 fingerprint64=365065097387925697 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=718891 false_local_accepts=0 hot_bytes=681792
sdk_mixed_map seed=2 bytes=17948 fingerprint64=5795255548068278463 edges=1492 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=517924 false_local_accepts=0 hot_bytes=607736
sdk_conditional_branch seed=2 bytes=26468 fingerprint64=14917641573332468348 edges=2202 rewrite_exact=true slot=1000 energy=1000 parity=0 energy_parity=0 p99_ns=608822 false_local_accepts=0 hot_bytes=681792
```

Aggregate:

```text
seeds: 3
labels: sdk_conditional_branch, sdk_mixed_map
local_margin_threshold: 1000000
p99_latency_gate_ns: 1000000
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_sdk_gap_parity_mismatches: 0
total_sdk_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
nonzero_fingerprints: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 718891
```

Verification after recording:

```text
cargo fmt --check
git diff --check -- <touched slot32 SDK/runtime docs and tests>
cargo check -p nando-core --tests
cargo test -p nando-core --test wavepredictor_role_binding_sdk_public -- --nocapture
cargo clippy -p nando-core --test wavepredictor_role_binding_sdk_public -- -D warnings
cargo clippy -p nando-core --test wavepredictor_binding_pressure_l3 -- -D warnings
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

Red signal before fix:

```text
The first naive public-SDK scoring path was correctness-green but performance-red:
observed p99 was approximately 4.4-4.5 ms, so the run was stopped before final
artifact promotion. The fix was not a mathematical shortcut: the public runtime
now builds a package-derived edge index and prepares the active fringe once per
row before scoring target/wrong lanes. The final gate above is the release
artifact after that fix.
```

Interpretation:

```text
This closes the public SDK-loaded 32-slot `.nwrb` role-binding package proof for
the current mixed-map plus conditional-branch Rust runtime path. The test writes
real package bytes, reloads them only through the public Rust SDK runtime, checks
package fingerprints and rewrite exactness, requires strict slot and sequence
energy 1000/1000, and proves SDK-vs-field parity for both local gap and
sequence-energy decisions.
```

Boundary:

```text
This is a public SDK-loaded `.nwrb` role-binding runtime package proof.
It is not the phase-center `.nwpc` package path.
It is not the CLI/daemon registry product proof.
It does not close raw-language action parsing, autonomous action_tree induction,
insert-new-constant edit operators, 64-slot capacity, broad workflow reasoning,
text generation, or the full operator catalog.
```
