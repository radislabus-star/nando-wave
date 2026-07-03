# Slot32 Role-Binding CLI Corpus Score/Verify Rung

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What this closes:

```text
32-slot heldout corpus rows
-> corpus-emitted `.nwrb` eval-pack
-> public SDK-loaded `.nwrb` runtime
-> CLI strict sequence scoring
-> deterministic verify against package + eval-pack
```

This is stronger than the earlier package-derived scoring smoke. The eval-pack
is emitted from heldout corpus tasks inside the Rust package gate, not from
package edges.

Commands:

```text
cargo test -p nando-core --release --test wavepredictor_binding_pressure_l3 -- --ignored operator_battery_v4_slot32_role_binding_public_sdk_must_score_loaded_package_runtime --nocapture
cargo run -p nando-cli --release -- role-binding-package-score-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json target/nando-wave/slot32-role-binding/role-binding-package-score-corpus-v1.product-proof.json 1000000
cargo run -p nando-cli --release -- role-binding-package-score-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json target/nando-wave/slot32-role-binding/role-binding-package-score-corpus-v1.product-proof.json 1000000
```

Artifacts:

```text
package:
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb

corpus_eval_pack:
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json

score_report:
  target/nando-wave/slot32-role-binding/role-binding-package-score-corpus-v1.product-proof.json

release_log:
  data/rule_logic_operator_battery_v4/diagnostics/slot32/slot32_role_binding_public_sdk_package_rung_release.log
```

CLI corpus score metrics:

```text
package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14754950188000667967
margin_threshold: 1000000
task_count: 0
sequence_count: 4096
expected_local_sequences: 2048
expected_fallback_sequences: 2048
sequence_local_operator_calls: 2048
sequence_fallback_to_llm_calls: 2048
sequence_false_local_accepts: 0
sequence_missed_expected_local: 0
sequence_strict_ordered_accuracy_milli: 1000
sequence_median_energy_margin: 2449664
report_matches_sources: true
```

Release package gate metrics from the same current-source rerun:

```text
verdict: SLOT32_ROLE_BINDING_PUBLIC_SDK_PACKAGE_RUNG_PASS
seeds: 3
labels: {"sdk_conditional_branch", "sdk_mixed_map"}
local_margin_threshold: 1000000
min_slot_accuracy_milli: 1000
min_sequence_energy_accuracy_milli: 1000
total_sdk_gap_parity_mismatches: 0
total_sdk_sequence_energy_parity_mismatches: 0
total_false_local_accepts: 0
rewrite_exact_all: true
max_package_bytes: 26468
max_hot_bytes_estimate: 681792
max_edges: 2202
max_p99_latency_ns: 689788
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

Important pressure finding:

```text
JSON corpus eval-pack is too large for product packaging.

sdk_conditional_branch-seed1.corpus-eval-pack-v1.json: ~456 MB
all six generated seed/label corpus eval-packs in target: target slot32 dir ~2.6 GB

This is acceptable as a proof artifact but not as the final product format.
Next required packaging debt: compact binary role-binding eval-pack, analogous
to the action `.nwpc` binary eval-pack path.
```

Boundary:

```text
This closes independent corpus-emitted `.nwrb` CLI sequence scoring for one
representative 32-slot conditional package and current-source package rerun.

It does not close compact binary `.nwrb` eval-pack packaging, `.nwrb`
daemon/registry routing, phase-center `.nwpc` bridge for strict role-binding,
raw-language action parsing, broad workflow reasoning, or text generation.
```
