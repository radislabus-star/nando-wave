# Slot32 Role-Binding CLI Score/Verify Rung

Date: 2026-07-03

Verdict:

```text
ROLE_BINDING_PACKAGE_SCORE_V1_PASS
ROLE_BINDING_PACKAGE_SCORE_VERIFY_V1_PASS
```

What this closes:

```text
`.nwrb` package bytes -> public SDK runtime -> explicit eval-pack -> score report -> verify report
```

This is a CLI/product plumbing rung for the serialized 32-slot role-binding
runtime. It does not change the L3 architecture and does not introduce
`local_out_t`.

Commands:

```text
cargo run -p nando-cli --release -- role-binding-eval-pack-from-package-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json 128
cargo run -p nando-cli --release -- role-binding-package-score-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json 1
cargo run -p nando-cli --release -- role-binding-package-score-verify-v1 target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json 1
```

Artifacts:

```text
package:
  target/nando-wave/slot32-role-binding/sdk_conditional_branch-seed1.nwrb

eval_pack:
  target/nando-wave/slot32-role-binding/role-binding-eval-pack-v1.json

score_report:
  target/nando-wave/slot32-role-binding/role-binding-package-score-v1.product-proof.json
```

Metrics:

```text
package_fingerprint64: 365065097387925697
eval_pack_fingerprint64: 14619240648419331465
eval_pack_package_fingerprint_matches: true
task_count: 128
expected_local_tasks: 64
expected_fallback_tasks: 64
local_operator_calls: 64
fallback_to_llm_calls: 64
false_local_accepts: 0
missed_expected_local: 0
min_margin: -1024
p10_margin: -1024
median_margin: 992
max_margin: 1024
sdk_load_matches_inspect: true
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
This closes `.nwrb` CLI scoring/verify over an explicit eval-pack interface.
The generated eval-pack in this rung is package-derived and is therefore only a
scoring plumbing smoke, not an independent corpus proof.

Still not closed:
  independent corpus-emitted `.nwrb` eval-pack;
  `.nwrb` daemon/registry routing;
  phase-center `.nwpc` bridge for strict role-binding;
  raw-language action parsing;
  broad workflow reasoning;
  text generation.
```
