# Slot32 Role-Binding Release Suite

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_RELEASE_SUITE_V1_PASS
ROLE_BINDING_RELEASE_SUITE_VERIFY_V1_PASS
```

What this closes:

```text
The current 32-slot role-binding `.nwrb` package set, all-seed `.nwreb`
eval-packs, per-row binary/score reports, and aggregate binary suite are tied
into one product-proof bundle with source verification.
```

Artifacts:

```text
binary_suite_report: target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json
release_suite_report: target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
```

Commands:

```text
cargo run -p nando-cli --release -- role-binding-release-suite-v1 target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
cargo run -p nando-cli --release -- role-binding-release-suite-verify-v1 target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json
```

Metrics:

```text
binary_suite_report_fingerprint64: 3795565268872355408
package_count: 7
binary_eval_pack_count: 7
score_report_count: 7
total_package_bytes: 134912
total_binary_eval_pack_bytes: 369676909
total_sequence_count: 27648
total_expected_local_sequences: 13824
total_expected_fallback_sequences: 13824
total_sequence_false_local_accepts: 0
total_sequence_missed_expected_local: 0
min_sequence_strict_ordered_accuracy_milli: 1000
min_sequence_median_energy_margin: 6144
all_packages_magic_match: true
all_packages_bytes_match_inspect: true
all_package_fingerprints_match_suite: true
all_eval_pack_magic_match: true
all_eval_pack_fingerprints_match_suite: true
all_binary_reports_match_suite_rows: true
all_score_reports_match_suite_rows: true
all_forbidden_flags_false: true
report_matches_sources: true
```

Per package:

```text
sdk_mixed_map seed0:
  package_bytes: 17948
  package_edge_count: 1492
  binary_eval_pack_bytes: 59208629
  sequence_median_energy_margin: 2330624

sdk_mixed_map seed1:
  package_bytes: 17948
  package_edge_count: 1492
  binary_eval_pack_bytes: 59015429
  sequence_median_energy_margin: 2411776

sdk_mixed_map seed2:
  package_bytes: 17948
  package_edge_count: 1492
  binary_eval_pack_bytes: 59267573
  sequence_median_energy_margin: 2352640

sdk_conditional_branch seed0:
  package_bytes: 26468
  package_edge_count: 2202
  binary_eval_pack_bytes: 60779485
  sequence_median_energy_margin: 2354176

sdk_conditional_branch seed1:
  package_bytes: 26468
  package_edge_count: 2202
  binary_eval_pack_bytes: 60587229
  sequence_median_energy_margin: 2449664

sdk_conditional_branch seed2:
  package_bytes: 26468
  package_edge_count: 2202
  binary_eval_pack_bytes: 60838493
  sequence_median_energy_margin: 2380544

sdk_edit_marker_length seed0:
  margin_threshold: 1
  package_bytes: 1664
  package_edge_count: 135
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
This closes a product-proof release bundle for the current strict 32-slot
role-binding package/eval-pack set.

It does not close the full 32-slot operator battery, `.nwpc` bridge,
raw-language action parsing, broad workflow reasoning, text generation, or
commercial license. The serving-only `.nwrb` profile runtime is tracked in a
separate profile-runtime smoke artifact.
```
