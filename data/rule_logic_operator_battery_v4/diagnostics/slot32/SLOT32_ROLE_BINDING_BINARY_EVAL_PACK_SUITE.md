# Slot32 Role-Binding Binary Eval-Pack Suite

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_BINARY_EVAL_PACK_SUITE_V1_PASS
ROLE_BINDING_BINARY_EVAL_PACK_SUITE_VERIFY_V1_PASS
```

What this closes:

```text
Current slot32 role-binding corpus eval-packs are now available as
compact `.nwreb` binary eval-packs and score green through the serialized
`.nwrb` role-binding runtime.
```

Artifacts:

```text
suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json

binary_eval_packs:
  target/nando-wave/slot32-role-binding/sdk_mixed_map-seed0.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_mixed_map-seed1.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_mixed_map-seed2.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed0.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed2.corpus-eval-pack-v1.nwreb
  target/nando-wave/slot32-role-binding/sdk_edit_marker_length-seed0.corpus-eval-pack-v1.nwreb
```

Commands:

```text
cargo run -p nando-cli --release -- role-binding-binary-eval-pack-suite-v1 target/nando-wave/slot32-role-binding target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json 1000000
cargo run -p nando-cli --release -- role-binding-binary-eval-pack-suite-verify-v1 target/nando-wave/slot32-role-binding target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json 1000000
```

Metrics:

```text
suite_items: 7
total_source_eval_pack_bytes: 2790622842
total_binary_eval_pack_bytes: 369676909
suite_size_reduction_milli: 867
total_sequence_count: 27648
total_expected_local_sequences: 13824
total_expected_fallback_sequences: 13824
total_sequence_local_operator_calls: 13824
total_sequence_fallback_to_llm_calls: 13824
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 6144
all_binary_gate_pass: true
all_binary_reports_match_sources: true
all_score_gate_pass: true
all_score_reports_match_sources: true
all_eval_pack_format_binary: true
all_package_fingerprints_match: true
```

Additional current item:

```text
sdk_edit_marker_length seed0:
  margin_threshold: 1
  binary_eval_pack_bytes: 9980071
  sequence_median_energy_margin: 6144
```

Forbidden flags:

```text
target_center_id_training_used: false
proof_rule_id_training_authority_used: false
concrete_x_lookup_used: false
local_out_t_runtime_extension_used: false
python_demo_used: false
rust_runtime_used: true
```

Boundary:

```text
This closes compact binary `.nwreb` role-binding eval-pack packaging and
scoring for the current 32-slot role-binding package set with per-item margin
thresholds, including the bounded EDIT marker/length package.

It does not close the full 32-slot operator battery, `.nwpc` bridge,
raw-language action parsing, broad workflow reasoning, or text generation.
```
