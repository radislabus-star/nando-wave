# Slot32 Role-Binding Binary Eval-Pack Rung

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_EVAL_PACK_BINARY_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What this closes:

```text
corpus-emitted role-binding eval-pack JSON
-> compact binary `.nwreb` eval-pack
-> same public SDK-loaded `.nwrb` scoring path
-> deterministic verify against package + binary eval-pack
```

This is a packaging/runtime proof. It does not change `.nwrb` runtime
semantics, L3 role-binding, action centers, or operator proof source.

Commands:

```text
cargo run -p nando-cli --release -- role-binding-eval-pack-binary-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb target/nando-wave/slot32-role-binding/role-binding-eval-pack-binary-corpus-v1.product-proof.json
cargo run -p nando-cli --release -- role-binding-package-score-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb target/nando-wave/slot32-role-binding/role-binding-package-score-binary-corpus-v1.product-proof.json 1000000
cargo run -p nando-cli --release -- role-binding-package-score-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb target/nando-wave/slot32-role-binding/role-binding-package-score-binary-corpus-v1.product-proof.json 1000000
```

Artifacts:

```text
source_eval_pack_json:
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.json

binary_eval_pack:
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.corpus-eval-pack-v1.nwreb

binary_pack_report:
  target/nando-wave/slot32-role-binding/role-binding-eval-pack-binary-corpus-v1.product-proof.json

binary_score_report:
  target/nando-wave/slot32-role-binding/role-binding-package-score-binary-corpus-v1.product-proof.json
```

Binary eval-pack metrics:

```text
binary_magic_text: NWRE0001
package_fingerprint64: 365065097387925697
task_count: 0
sequence_count: 4096
source_eval_pack_bytes: 455828420
binary_eval_pack_bytes: 60587229
size_reduction_milli: 867
roundtrip_exact: true
```

Binary score metrics:

```text
eval_pack_format: binary
eval_pack_bytes: 60587229
eval_pack_fingerprint64: 15010148470072679065
margin_threshold: 1000000
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

Boundary:

```text
This closes compact binary role-binding eval-pack packaging and scoring for the
representative 32-slot conditional package.

It does not close all-seed binary eval-pack emission, `.nwrb` daemon/registry
routing, phase-center `.nwpc` bridge for strict role-binding, raw-language
action parsing, broad workflow reasoning, or text generation.
```
