# CPU Call Catalog

Status: review-only working catalog for CPU80.

Source report:

```text
target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json
target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json
```

The catalog is a product filter before building another operator profile. It
answers one question:

```text
Which real call class can add unique verified CPU accepts over exact cache?
```

It is not a market claim. It does not enable local accepts. It does not count
candidate, scoreable, or broad-route rows as savings.

Product rule:

```text
Do not build the next profile because it is technically interesting.
Build or improve it only after BUSINESS_VALUE_GATE shows real traffic, cache
overlap, verifier evidence, expected unique CPU accepts, false_accept risk, and
expected savings.
```

Product identity:

```text
Nando is not sold as a single generic model.
Nando is sold as a per-runtime CPU Call Catalog:

  Codex catalog
  Claude Code catalog
  customer-support-agent catalog
  workflow-agent catalog

Each catalog is mined from that runtime's real traffic. A profile that is
valuable for one runtime can be useless for another.
```

Market trajectory:

```text
real traffic
  -> route classification
  -> cost ranking
  -> exact-cache overlap check
  -> verifier audit
  -> savings forecast
  -> profile/payload/admission work only for rows that pass BUSINESS_VALUE_GATE
```

## Business Value Gate

A profile row passes the gate only when all conditions are true:

```text
call_class appears in non-synthetic real trace
non-exact candidate calls exist
deterministic verifier hook is ready
expected unique CPU accepts over exact cache > 0
false_accepts = 0
```

Before any new payload/profile/admission work, record this eight-field gate:

```text
call_class
traffic_count / traffic_share
exact_cache_overlap
verifier_or_external_fact
expected_unique_cpu_accepts_over_exact_cache
false_accept_risk
expected_savings
next_action
```

Anything else goes to a shelf:

```text
PROVEN          already has unique verified CPU accepts
CANDIDATE       payload/verifier evidence exists, but expected unique accepts are not proven
WATCH           low support, singleton-only, no verifier, or exhausted support
REJECT_FOR_NOW  broad or risky route; split before more work
```

Hard stop:

```text
Do not build 100 elegant CPU/L3 profiles that add only +2.5% on real traffic.
If a call_class has no real traffic count, no verifier, or no expected unique
accepts over exact cache, it does not get engineering time yet.
```

## Online Operator Discovery

Manual profile selection is allowed, but it should become less central over
time. The product path is live action-center mining:

```text
observe LLM calls
extract action/state features
cluster repeated transitions / center-of-mass action groups
rank by traffic and cost
require deterministic verifier
shadow mode
promote only if false_accepts=0 and unique value over exact cache is proven
```

Auto-discovery is allowed. Auto-accept without a verifier is not allowed.

## Current Snapshot

Window:

```text
total_llm_calls: 1000
exact_cache_hits: 53
current_verified_cpu_accepts_route_sum: 42
current_unique_verified_cpu_accepts: 36
current_incremental_unique_cpu_accepts_over_exact_cache: 35
business_value_gate_passed_rows: 8
proven_profile_rows: 8
candidate_profile_rows: 4
watch_profile_rows: 12
rejected_profile_rows: 6
```

## Current 5k Combined Working Snapshot

Source reports:

```text
target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-current5k.combined.report.json
target/nando-wave/real-traffic-shadow/route-gap-catalog-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json
```

Measured on the current non-synthetic 5k Codex history window:

```text
total_llm_calls: 5000
exact_cache_hits: 452
route_candidate_events: 1439
route_gap_no_candidate_events: 3561
route_gap_payload_ready_events: 807

feedback operator_candidate_calls: 3549
feedback scoreable_candidate_calls: 554
feedback verification_hook_ready_events: 301
feedback verified_cpu_accept_eligible_events: 114
feedback verified_cpu_accept_unique_request_fingerprints: 112
feedback incremental_cpu_accept_unique_request_fingerprints: 110
feedback exact_cache_overlap_verified_cpu_accepts: 2
feedback incremental_cpu_accept_unique_reduction_milli: 21
feedback incremental_unique_gap_to_80_calls: 3890

catalog current_verified_cpu_accepts: 112
catalog current_incremental_unique_cpu_accepts_over_exact_cache: 110
catalog business_value_gate_passed_rows: 7
catalog proven_profile_rows: 7
catalog candidate_profile_rows: 1
catalog watch_profile_rows: 16
catalog rejected_profile_rows: 5
```

Latest current5k proven rows:

| rank | call class | candidates | scoreable | hooks | verified eligible | incremental unique | status note |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | `test_output_parse` | 95 | 95 | 95 | 95 | 91 | Narrow previous-tool-output status parser. |
| 2 | `role_binding_mixed_map_seed0` | 477 | 11 | 11 | 6 | 6 | Small safe request-side policy. |
| 3 | `role_binding_conditional_branch_seed0` | 456 | 58 | 53 | 3 | 3 | Tiny safe policy; broad conditional remains unsafe. |
| 4 | `git_control` | 123 | 90 | 74 | 3 | 3 | Request-side v3 policy proven, but support exhausted. |
| 5 | `metrics_report_readout` | 55 | 63 | 51 | 3 | 3 | Safe-policy sidecar proven, but still tiny. |
| 6 | `serving_ops` | 74 | 40 | 33 | 1 | 1 | Tiny server-ops proof; daemon mutations remain disabled. |
| 7 | `role_binding_edit_marker_length_seed0` | 506 | 50 | 42 | 3 | 3 | Edit safe-policy v2: request-side `code_diff_and_question_mark` plus energy threshold, false_accepts=0. |

Git-control request-side admission refresh:

```text
git_control admission audit:
  scoreable_candidate_rows: 90
  hook_ready_rows: 74
  label_true_rows: 35
  label_false_rows: 39
  unverified_rows: 16
  safe_policy_found: true
  best_safe_true_accepts: 3
  best policy:
    no_mutation_verbs AND has_push_terms AND energy >= 1386496
    true_accepts: 3
    false_accepts: 0
    unverified_accepts: 0

catalog git_control existing_profile_route:
  current_status: CANDIDATE
  expected_unique_cpu_accepts_over_exact_cache: 0
  git_control_admission_best_safe_true_accepts: 3
  false_accept_risk: MEDIUM_VERIFIER_READY_POLICY_PENDING_PROMOTE
  business_value_gate_failure_reason:
    expected_unique_cpu_accepts_zero,no_safe_local_accept_policy
```

Boundary:

```text
This is not a market savings claim and not a local-accept promotion.
It only means the request-side admission audit found a tiny safe git subfamily.
The next valid step is a separate promoted shadow trace/registry for that exact
policy, followed by shadow/audit/feedback with provider cost and false_accepts=0.
```

Edit marker-length request-side admission refresh:

```text
edit_marker_length admission calibration:
  hook_ready_rows: 42
  label_true_rows: 14
  label_false_rows: 28
  robust_safe_policy_found: true
  best_robust_true_accepts: 4
  best policy:
    code_diff_and_question_mark
    true_accepts: 4
    false_accepts: 0
    missed_true: 10

edit safe-policy v2 promoted shadow/audit:
  request_side_policy_name: code_diff_and_question_mark
  selected_policy_threshold: 7680
  request_side_policy_accept_rows: 5
  policy_accept_rows: 3
  policy_accept_verified_true_rows: 3
  policy_accept_verified_false_rows: 0
  policy_accept_unverified_rows: 0
  shadow total_requests: 5000
  shadow verified_safe_accepts: 3
  shadow false_accepts: 0
  shadow p99_shadow_score_latency_ns: 863057
  audit verified_cpu_accept_eligible_events: 3

catalog role_binding_edit_marker_length_seed0:
  current_status: PROVEN
  edit_admission_best_robust_true_accepts: 4
  edit_admission_no_safe_policy: false
  expected_unique_cpu_accepts_over_exact_cache: 3
  false_accept_risk: LOW_VERIFIED_POLICY_ZERO_FALSE_ACCEPTS
  business_value_gate_failure_reason: PASSED
```

Boundary:

```text
This is still not a global CPU80 or broad market savings claim.
It means the request-side admission calibration plus promoted shadow/audit
proved a tiny edit subfamily using prompt-side features only:
  has_code_diff_lines && has_question_mark

Count only the 3 incremental unique verified accepts over exact cache. The next
valid edit work is improving evidence/payload coverage or splitting a higher
value artifact-backed edit subfamily, not widening the broad edit route.
```

Latest guardrail refresh:

```text
metrics_report_admission_calibration now treats unverified profile rows as
unsafe for request-side policy selection.

current5k companion reports used by catalog:
  git-control-admission-audit-v1-current5k.report.json
  metrics-report-admission-calibration-v1-5k.report.json
  edit-admission-calibration-v1-current5k.report.json

feedback-loop current5k companion reports now used for artifact-backed
project_context, metrics_report, git_control, and serving_ops:
  project-context-payload-dry-run-v1-current5k.report.json
  project-context-output-evidence-v1-current5k.verification-hook-audit.report.json
  project-context-local-accept-calibration-v1-current5k.report.json
  metrics-report-safe-policy-v1-5k.verification-hook-audit.report.json
  git-control-payload-dry-run-v1-current5k.report.json
  git-control-output-evidence-v1-current5k.verification-hook-audit.report.json
  git-control-safe-policy-v3-current5k.verification-hook-audit.report.json
  git-control-local-accept-calibration-v1-current5k.report.json
  serving-ops-payload-dry-run-v1-current5k.report.json
  serving-ops-output-evidence-v1-current5k.verification-hook-audit.report.json
  serving-ops-safe-policy-v1-current5k.verification-hook-audit.report.json
  serving-ops-local-accept-calibration-v1-current5k.report.json
```

Business Value Gate reason refresh:

```text
cpu_operator_catalog rows now include:
  business_value_gate_failure_reason

This field is diagnostic only. It does not grant local accept authority and
does not change current verified CPU savings. It exists to prevent repeated
work on attractive but blocked routes.
```

Current examples:

```text
role_binding_agent_control_seed0:
  business_value_gate_failure_reason:
    missing_deterministic_verifier_hook,expected_unique_cpu_accepts_zero,no_safe_local_accept_policy,false_accept_risk_unknown
  agent_control_admission_best_robust_true_accepts: 0
  agent_control_current_policy_event_support_exhausted: false
  decision: current5k calibration overrides stale 1000-window v2 support;
    do not repeat the old stop/control promotion.

agent_control_stop:
  business_value_gate_failure_reason:
    missing_deterministic_verifier_hook,expected_unique_cpu_accepts_zero,false_accept_risk_unknown
  agent_control_admission_best_robust_true_accepts: 0
  agent_control_current_policy_event_support_exhausted: false
  decision: WATCH / NO_SAFE_CURRENT_WINDOW_POLICY

git_control:
  business_value_gate_failure_reason:
    expected_unique_cpu_accepts_zero,no_safe_local_accept_policy,safe_policy_missing

role_binding_edit_marker_length_seed0:
  business_value_gate_failure_reason: PASSED
  edit_admission_best_robust_true_accepts: 4
  edit_admission_no_safe_policy: false
  expected_unique_cpu_accepts_over_exact_cache: 3
  false_accept_risk: LOW_VERIFIED_POLICY_ZERO_FALSE_ACCEPTS
  decision: PROVEN tiny / promoted v2 shadow-audit adds 3 incremental unique
    verified accepts over exact cache with false_accepts=0.
```

Git/serving feedback-loop current5k refresh:

```text
SUPERSEDED by the current 5k combined working snapshot above for git_control.

git_control:
  feedback candidate_events: 123
  feedback scoreable_payload_events: 90
  feedback verification_hook_ready_events: 74
  feedback verified_cpu_accept_eligible_events: 0
  catalog status: CANDIDATE
  business_value_gate_failure_reason:
    expected_unique_cpu_accepts_zero,no_safe_local_accept_policy,safe_policy_missing
  decision: do not promote; improve request-side admission, verifier evidence,
    or split the git command-outcome geometry first.

serving_ops:
  feedback candidate_events: 74
  feedback scoreable_payload_events: 40
  feedback verification_hook_ready_events: 33
  feedback verified_cpu_accept_eligible_events: 1
  catalog status: PROVEN
  business_value_gate_failure_reason: PASSED
  decision: keep as tiny proven support only; run non-synthetic soak before any
    market claim and keep daemon mutations disabled.
```

Decision:

```text
Do not re-promote exhausted or no-safe-policy routes by lowering thresholds.
Next profile work must add new verifier evidence, split a narrower
artifact-backed subfamily, or move to a higher-value route.
```

Metrics-report p99 split finding:

```text
initial labeled-only read:
  p99_terms_not_concise: 12 true / 0 false

full trace with unverified rows included:
  hook_ready_rows: 63
  label_true_rows: 31
  label_false_rows: 20
  unverified_rows: 12
  p99_terms_not_concise: 14 accepts, 12 true, 2 unsafe false-or-unverified
  robust_safe_policy_found: false
  best_robust_true_accepts: 0

decision:
  WATCH / NO_SAFE_POLICY
```

Interpretation:

```text
The p99 metrics split is a useful action-center discovery, but not a safe CPU
accept path. Unknown output evidence is treated as unsafe. Do not promote this
route until the missing output evidence is attached or a narrower split shows
false_accepts=0 and unverified_accepts=0.
```

PROVEN rows on current5k:

| rank | call class | candidates | scoreable | hooks | verified eligible | incremental unique | expected savings milli | status note |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | `test_output_parse` | 95 | 95 | 95 | 95 | 91 | 18 | Keep as narrow previous-tool-output status parser; do not treat it as broad answer parsing. |
| 2 | `role_binding_mixed_map_seed0` | 477 | 11 | 11 | 6 | 6 | 1 | Safe request-side admission policy; useful but still small. |
| 3 | `role_binding_conditional_branch_seed0` | 456 | 58 | 53 | 3 | 3 | 0 | Tiny request-side v2 safe policy; broad conditional calibration is still unsafe. |
| 4 | `metrics_report_readout` | 99 | 63 | 51 | 3 | 3 | 0 | Small proven sidecar only; support is exhausted and p99 shadow score was WATCH in the promote run. |
| 6 | `serving_ops` | 74 | 40 | 33 | 1 | 1 | 0 | Tiny safe-policy proof; server mutations remain disabled and this is not a serving market claim. |
| 7 | `role_binding_edit_marker_length_seed0` | 506 | 50 | 42 | 3 | 3 | 0 | Edit safe-policy v2 tiny proof; do not widen broad edit. |

Important non-promotions:

```text
agent_control current5k audit:
  scoreable_payload_events: 540
  output_evidence_matched_events: 476
  verified_true_events: 35
  verified_false_events: 441
  robust_safe_policy_found: false
  best_robust_true_accepts: 0
  current catalog status: WATCH
  current catalog decision: no safe request-side policy; old v2 support is not
    current5k authority
  decision: WATCH / NO_SAFE_POLICY

metrics_report current5k:
  request-side robust admission: no safe policy
  score/readout safe policy: 3 true accepts, 0 false accepts
  decision: PROVEN, but tiny; split or improve numeric evidence before more work

serving_ops current5k:
  score/readout safe policy: 1 true accept, 0 false accepts
  server mutations: disabled
  decision: PROVEN, but tiny; split or improve daemon health evidence before more work

mixed_map current5k:
  request-side admission safe policy: 6 true accepts, 0 false accepts
  decision: PROVEN, but still small; split or improve map evidence before more work

conditional_branch current5k:
  local readout calibration: no safe policy
  request-side v2 safe policy: 3 true accepts, 0 false accepts, 0 unverified
  decision: PROVEN tiny subset only; do not widen the broad conditional route

edit_marker_length current5k:
  route candidate events: 506
  scoreable_payload_events: 50
  verification_hook_ready_events: 42
  admission hook_ready_rows: 42
  admission label_true_rows: 14
  admission label_false_rows: 28
  robust_safe_policy_found: true
  singleton_safe_policy_found: true
  best_robust_true_accepts: 4
  promoted v2 verified_cpu_accept_eligible_events: 3
  promoted v2 false_accepts: 0
  decision: PROVEN tiny / code_diff_and_question_mark plus energy threshold
  next_action: improve edit evidence/payload coverage or split a higher-value
    artifact-backed edit subfamily; do not lower thresholds or widen broad edit

file_path_evidence_answer current5k split:
  broad_split candidate events: 146
  scoreable_payload_events: 44
  verification_hook_ready_events: 39
  verified_true_events: 16
  verified_false_events: 23
  robust_safe_policy_found: false
  singleton_safe_policy_found: true, best_singleton_true_accepts: 1
  decision: WATCH / SINGLETON_ONLY_NO_ROBUST_POLICY

project_context artifact_backed_project_state current5k split:
  project_context candidate events: 1313
  feedback route candidate events: 1314
  payload_ready_events: 15
  scoreable_payload_events: 8
  verification_hook_ready_events: 8
  artifact_evidence_matched_events: 8
  verified_true_events: 1
  verified_false_events: 7
  safe_policy_found: true, best_safe_true_accepts: 1
  minimum_true_support: 3
  local_accept_support_qualified: false
  verified_cpu_accept_eligible_events: 0
  decision: WATCH / SINGLETON_ONLY_BELOW_MIN_SUPPORT
```

Current5k BUSINESS_VALUE_GATE shelves:

| shelf | count | meaning |
| --- | ---: | --- |
| `PROVEN` | 7 | Unique verified CPU accepts exist on this 5k window. |
| `CANDIDATE` | 1 | Evidence exists, but no unique verified accepts yet. |
| `WATCH` | 16 | Low support, exhausted policy, singleton-only, or missing verifier. |
| `REJECT_FOR_NOW` | 5 | Broad route or unsafe route; split before work. |

Decision:

```text
The current5k window is the working commercial filter. It proves that CPU
routability growth is not limited by synthetic gates anymore; it is limited by
finding narrow, verifier-backed real call classes with unique value over exact
cache.

Do not spend cycles on 100 attractive profiles. The next profile must be chosen
by expected unique CPU accepts over exact cache at false_accepts=0.
```

PROVEN rows:

| rank | call class | candidates | non-exact | expected unique | status note |
| ---: | --- | ---: | ---: | ---: | --- |
| 1 | `test_output_parse` | 10 | 10 | 10 | Full-window attribution only: route-specific proof is 97/104, but current 1000-call denominator contains 10 matching rows. |
| 2 | `role_binding_mixed_map_seed0` | 99 | 96 | 7 | Support is already covered; improve payload/evidence before another promote. |
| 3 | `role_binding_agent_control_seed0` | 111 | 74 | 5 | Duplicates/exact-cache overlap constrain unique value; split broader tool-state subfamilies. |
| 4 | `git_control` | 18 | 18 | 4 | Current safe support is exhausted; improve command outcome evidence or split. |
| 5 | `role_binding_conditional_branch_seed0` | 88 | 87 | 3 | Verifier-ready, but policy support is exhausted; split stronger conditional subfamily. |
| 6 | `metrics_report_readout` | 55 | 55 | 3 | Current robust metrics support is exhausted; split stronger report subfamily. |
| 7 | `serving_ops` | 25 | 25 | 3 | Current serving support is exhausted; split stronger daemon/health subfamily. |
| 8 | `role_binding_edit_marker_length_seed0` | 92 | 92 | 1 | Low support; improve edit evidence before another promote. |

## Sidecar: Test Output Parse 5k Window

This is not the canonical catalog snapshot. It is a larger-window attribution
check for the same narrow `test_output_parse` profile.

Source reports:

```text
target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-5k.test-output-window.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-v1-5k.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-shadow-v1-5k.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-v1-5k.verification-hook-audit.report.json
target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1-5k.test-output-window.report.json
```

Measured on the 5k Codex route-candidate window:

```text
total_llm_calls: 5000
exact_cache_hits: 459
test_output_parse promoted_route_rows: 104
test_output_parse promoted_rows_inserted: 97
missing_base_match_rows: 0
exact_cache_overlap_promoted_rows: 2
verified_safe_accepts: 97
false_accepts: 0
verified_cpu_accept_unique_request_fingerprints: 95
incremental_cpu_accept_unique_request_fingerprints: 93
incremental_cpu_accept_unique_reduction_milli: 18
incremental_unique_gap_to_80_calls: 3907
business_value_gate_passed_rows: 1
proven_profile_rows: 1
candidate_profile_rows: 1
watch_profile_rows: 22
rejected_profile_rows: 5
```

Decision:

```text
`test_output_parse` remains PROVEN and more valuable than the 1000-window count
showed, but this sidecar does not replace the canonical 1000-window catalog.
The separate 5k catalog has only one PROVEN row, so the next BUSINESS_VALUE_GATE
target is the best narrow artifact-backed split that can add new verified
unique CPU accepts over exact cache. CPU80 is still not proven.
```

CANDIDATE rows:

```text
uncatalogued / resource_pressure_budget
read_inspect
style_brevity
resource_pressure_budget
```

These do not count as savings. They are work candidates only if expected
unique accepts can be raised by verifier evidence or a narrower split.

REJECT_FOR_NOW rows:

```text
answer_or_explain
project_context_dialogue
agent_continue_execute
```

These are intentionally blocked as broad routes. Work only narrow
artifact-backed subfamilies, never the route as a whole.

## Next Engineering Rule

Do not build the next profile because it is interesting.

Build it only if the catalog row shows one of these:

```text
expected_unique_cpu_accepts_over_exact_cache > 0
or
clear deterministic verifier evidence that can raise expected unique accepts
```

The current highest-leverage pattern is not another generic profile. It is:

```text
1. Keep PROVEN tiny routes only as bounded CPU support.
2. For broad rejected traffic, split by artifact-backed evidence first.
3. Do not promote singleton-only splits; collect more verifier-true rows or
   split again until robust support exists.
```

```text
split high-volume REJECT_FOR_NOW routes into narrow artifact-backed call classes
or improve evidence geometry for exhausted PROVEN routes
```

Immediate safe targets:

```text
metrics_report_readout split
git_control split
serving_ops split
read_inspect verifier/evidence
test_output_parse if found in real trace
```

### Git Control Current5k Probe

Source reports:

```text
target/nando-wave/real-traffic-shadow/git-control-payload-dry-run-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-profile-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-local-accept-calibration-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-admission-audit-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1-current5k.shadow-report.json
target/nando-wave/real-traffic-shadow/git-control-output-evidence-v1-current5k.verification-hook-audit.report.json
target/nando-wave/real-traffic-shadow/git-control-safe-policy-v3-current5k.report.json
target/nando-wave/real-traffic-shadow/git-control-safe-policy-v3-current5k.shadow-report.json
target/nando-wave/real-traffic-shadow/git-control-safe-policy-v3-current5k.verification-hook-audit.report.json
```

Measured on current5k:

```text
git_control_candidate_events: 123
payload_ready_events: 90
payload_built_events: 90
scoreable_payload_events: 90
edge_count: 8
output_evidence_matched_events: 74
verified_true_events: 35
verified_false_events: 39
unverified_rows: 16

local_accept_calibration safe_policy_found: false
local_accept_calibration best_safe_true_accepts: 0
admission_audit safe_policy_found: true
admission_audit best_safe_true_accepts: 3
admission safe policy:
  no_mutation_verbs AND has_push_terms AND energy >= 1386496
  true_accepts: 3
  false_accepts: 0
  unverified_accepts: 0

v3 promoted request_side_policy_accept_rows: 22
v3 promoted policy_accept_rows: 3
v3 promoted policy_accept_verified_true_rows: 3
v3 promoted policy_accept_verified_false_rows: 0
v3 promoted policy_accept_unverified_rows: 0

shadow nando_shadow_accepts: 3
shadow verified_safe_accepts: 3
shadow false_accepts: 0
shadow p99_shadow_score_latency_ns: 386418
verification_hook_ready_events: 21
verified_cpu_accept_eligible_events: 3
market_claim_allowed: true
```

Decision:

```text
`git_control` is now PROVEN as a tiny request-side safe CPU support row:
3 incremental unique CPU accepts over exact cache, false_accepts=0.

The broad git_control route is still not solved. The old local calibration
still fails, and the v3 request-side safe policy exhausts all current support at
3 accepts. Do not lower thresholds and do not execute workspace mutations.
The next git work must improve command outcome evidence or split a new git
subfamily before another promotion attempt.
```

### Edit Marker Length Current5k Probe

Source reports:

```text
target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/edit-output-evidence-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/edit-local-accept-calibration-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/edit-output-evidence-v1-current5k.shadow-report.json
target/nando-wave/real-traffic-shadow/edit-output-evidence-v1-current5k.verification-hook-audit.report.json
```

Measured on current5k:

```text
edit_route_candidate_events: 505
payload_ready_events: 50
payload_built_events: 50
scoreable_payload_events: 50
output_evidence_matched_events: 42
verified_true_events: 14
verified_false_events: 28

local_accept_calibration safe_policy_found: false
local_accept_calibration best_safe_true_accepts: 0
admission_calibration robust_safe_policy_found: false
admission_calibration singleton_safe_policy_found: false
admission_calibration best_robust_true_accepts: 0

shadow nando_shadow_accepts: 0
shadow verified_safe_accepts: 0
shadow false_accepts: 0
shadow p99_shadow_score_latency_ns: 1213283
verification_hook_ready_events: 42
verified_cpu_accept_eligible_events: 0
market_claim_allowed: false
```

Feedback/catalog boundary:

```text
feedback candidate_events: 506
feedback payload_ready_events: 50
feedback payload_built_events: 50
feedback scoreable_payload_events: 0
feedback verified_cpu_accept_eligible_events: 0
catalog status: WATCH
```

Decision:

```text
`edit_marker_length` is a high-volume route but not a safe CPU profile on
current5k. It exposes 50 dry-run payloads and 42 verifier-hook-ready rows, but
the verifier split is 14 true / 28 false and no safe readout or admission policy
exists. Keep it WATCH / NO_SAFE_POLICY_CURRENT5K. Do not lower thresholds; split
the edit family by concrete external edit evidence before another promote.
```

### Mixed Map Current5k Probe

Source reports:

```text
target/nando-wave/real-traffic-shadow/mixed-payload-dry-run-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/mixed-local-accept-calibration-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/mixed-admission-audit-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3-current5k.report.json
target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3-current5k.shadow-report.json
target/nando-wave/real-traffic-shadow/mixed-safe-policy-v3-current5k.verification-hook-audit.report.json
```

Measured on current5k:

```text
mixed_route_candidate_events: 478
payload_ready_events: 37
payload_built_events: 37
scoreable_payload_events: 37
output_evidence_matched_events: 34
verified_true_events: 25
verified_false_events: 9

local_accept_calibration safe_policy_found: true
local_accept_calibration best_safe_true_accepts: 7
admission_audit safe_policy_found: true
admission_audit best_safe_true_accepts: 6

promoted request_side_policy_name: no_question_mark AND has_map_terms AND energy >= 237568
promoted request_side_policy_accept_rows: 11
promoted policy_accept_rows: 6
promoted policy_accept_verified_true_rows: 6
promoted policy_accept_verified_false_rows: 0
promoted policy_accept_unverified_rows: 0

shadow nando_shadow_accepts: 6
shadow verified_safe_accepts: 6
shadow false_accepts: 0
shadow p99_shadow_score_latency_ns: 448249
verification_hook_ready_events: 11
verified_cpu_accept_eligible_events: 6
market_claim_allowed: true
```

Decision:

```text
`mixed_map` is PROVEN on current5k as a small but real value row:
6 incremental unique CPU accepts, false_accepts=0. The safe policy is
request-side constrained and should not be widened. The next mixed work should
split or improve map evidence geometry rather than lower thresholds.
```

### Serving Ops Current5k Probe

Source reports:

```text
target/nando-wave/real-traffic-shadow/serving-ops-payload-dry-run-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/serving-ops-profile-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/serving-ops-local-accept-calibration-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.report.json
target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.shadow-report.json
target/nando-wave/real-traffic-shadow/serving-ops-safe-policy-v1-current5k.verification-hook-audit.report.json
```

Measured on current5k:

```text
serving_ops_candidate_events: 74
payload_ready_events: 40
payload_built_events: 40
scoreable_payload_events: 40
edge_count: 8
output_evidence_matched_events: 33
verified_true_events: 21
verified_false_events: 12

local_accept_calibration safe_policy_found: true
local_accept_calibration best_safe_true_accepts: 6
promoted policy_accept_rows: 1
promoted policy_accept_verified_true_rows: 1
promoted policy_accept_verified_false_rows: 0
promoted policy_accept_unverified_rows: 0

shadow nando_shadow_accepts: 1
shadow verified_safe_accepts: 1
shadow false_accepts: 0
shadow p99_shadow_score_latency_ns: 371226
verification_hook_ready_events: 33
verified_cpu_accept_eligible_events: 1
market_claim_allowed: true
server_mutation_enabled: false
```

Decision:

```text
`serving_ops` is PROVEN on current5k, but only as a tiny safe-policy row:
1 verified accept, false_accepts=0. This is useful evidence that the value gate
can promote a new profile, but it is not meaningful CPU80 growth. Keep daemon
actions disabled. The next serving work should split a stronger health/metric
subfamily instead of widening thresholds.
```

## Broad Route Split Discovery V1

Source report:

```text
target/nando-wave/real-traffic-shadow/broad-route-split-discovery-v1.report.json
```

Purpose:

```text
Split REJECT_FOR_NOW broad routes into narrow artifact-backed call classes
before building another CPU profile.
```

Measured on the current 5k Codex history window:

```text
sampled_llm_calls: 5000
broad_candidate_events: 3330
non_exact_broad_candidate_events: 3089
candidate_split_rows: 11
watch_split_rows: 6
rejected_split_rows: 3
business_value_gate_passed_rows: 0
```

Top candidate splits:

| rank | parent route | split | candidates | non-exact | payload-ready | verifier-signal | status |
| ---: | --- | --- | ---: | ---: | ---: | ---: | --- |
| 1 | `answer_or_explain` | `file_path_evidence_answer` | 79 | 79 | 70 | 70 | `WATCH` |
| 2 | `project_context_dialogue` | `file_path_evidence_answer` | 65 | 63 | 51 | 52 | `WATCH` |
| 3 | `answer_or_explain` | `test_output_parse` | 57 | 57 | 3 | 35 | `CANDIDATE` |
| 4 | `answer_or_explain` | `metric_from_report` | 27 | 27 | 14 | 14 | `CANDIDATE` |
| 5 | `answer_or_explain` | `git_status_summary` | 10 | 10 | 5 | 10 | `CANDIDATE` |
| 6 | `agent_continue_execute` | `git_status_summary` | 8 | 8 | 4 | 8 | `CANDIDATE` |

Still rejected:

```text
project_context_dialogue / broad_reasoning_requires_llm: 1216 non-exact
answer_or_explain / broad_reasoning_requires_llm: 1029 non-exact
agent_continue_execute / artifact_progress: WATCH, high stateful singleton risk
```

Decision:

```text
The broad routes remain blocked as full routes.
The next high-value narrow work should not promote file_path_evidence_answer
as-is: its output verifier exists, but request-side admission is only
singleton-safe. Work should either collect more verifier-true non-synthetic
rows, split file_path_evidence_answer into a narrower artifact-backed family, or
move to test_output_parse / metric_from_report. All rows still have
expected_unique_cpu_accepts_over_exact_cache = 0 until output evidence,
admission audit, shadow, feedback, and CPU catalog prove false_accepts=0.
```

## Test Output Parse Payload Dry-Run V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-payload-dry-run-v1.report.json
```

Purpose:

```text
Turn the narrow broad-route split `test_output_parse` into request-side
payloads, without promoting the blocked `answer_or_explain` or
`project_context_dialogue` routes as a whole.
```

Measured on the same 5k Codex history window:

```text
test_output_parse_candidate_events: 104
non_exact_candidate_events: 102
exact_cache_overlap_events: 2
payload_ready_events: 3
payload_built_events: 3
scoreable_payload_events: 3
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
raw_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Parent-route provenance:

```text
answer_or_explain: 57
project_context_dialogue: 47
```

Catalog status:

```text
CANDIDATE / REVIEW
```

Decision:

```text
`test_output_parse` is a real CPU-call candidate zone: 102 non-exact calls are
visible in the current trace window. It is not a savings claim yet. The request
payload builder only found 3 scoreable rows because the split needs actual
command-output/tool-output evidence. Next work should attach
test_status_and_error_excerpt/tool_output_validation_result verifier labels,
then compile a disabled-threshold profile and calibrate admission. Do not local
accept without verifier evidence.
```

### Test Output Parse Output Evidence V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-output-evidence-v1.report.json
```

Measured on the 3 scoreable payload rows:

```text
operator_candidate_calls: 3
scoreable_candidate_calls: 3
output_evidence_matched_events: 3
deterministic_verification_events: 3
verified_true_events: 3
verified_false_events: 0
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_verification: true
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Catalog status after evidence:

```text
CANDIDATE / LOW_SUPPORT_REVIEW
```

Decision:

```text
Verifier labels exist for the scoreable subset and are currently clean, but the
support is only 3 rows. This is not enough to count savings or promote local
accepts. For CPU80, the larger debt is agent-loop tool-output state capture:
many test_output_parse candidates refer to command/test results, but the current
request-side payload builder sees explicit status evidence in only 3 of 104
candidate rows.
```

### Test Output Parse Tool-Output State V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.report.json
```

Measured on the same 5k Codex history window:

```text
test_output_parse_candidate_events: 104
non_exact_candidate_events: 102
exact_cache_overlap_events: 2
session_ids_requested: 9
session_files_scanned: 9
codex_turns_indexed: 104
tool_outputs_indexed: 145578
tool_output_state_matched_events: 104
command_status_detected_events: 97
pass_status_events: 90
fail_status_events: 7
unknown_status_events: 7
raw_prompt_text_written: false
raw_tool_output_text_written: false
raw_response_text_written: false
response_text_used: false
target_labels_used: false
proof_labels_used: false
local_accepts_enabled: false
market_claim_allowed: false
```

Catalog status after tool-output state capture:

```text
CANDIDATE / STATE_READY_REVIEW
```

Decision:

```text
`test_output_parse` now has request-time agent-loop state coverage: every
candidate can be linked to the previous tool-output fingerprint, and 97/104 have
deterministic pass/fail/unknown status. This removes the previous support
bottleneck, but still does not count as CPU savings. Next work should build
scoreable payloads from this tool-output state, compile a disabled profile, and
promote only after admission/shadow proves false_accepts=0 over exact cache.
```

### Test Output Parse Tool-State Payload V1

Source report:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-tool-state-payload-v1.report.json
```

Measured result:

```text
operator_candidate_calls: 104
non_exact_candidate_events: 102
exact_cache_overlap_events: 2
tool_output_state_matched_events: 104
command_status_detected_events: 97
payload_ready_events: 97
payload_built_events: 97
scoreable_payload_events: 97
builder_rejected_events: 7
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Catalog status after tool-state payload:

```text
CANDIDATE / PAYLOAD_READY_PROFILE_MISSING
```

Decision:

```text
The prior support bottleneck is materially improved: scoreable payload support
is now 97/104 instead of 3/104, using previous tool-output state rather than
final answers. This is still not a savings claim. The missing artifact is a
disabled-threshold profile plus shadow/admission audit proving false_accepts=0
before any unique CPU accepts can count over exact cache.
```

### Test Output Parse Profile V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-profile-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-profile-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-profile-v1.verification-hook-audit.report.json
```

Measured result:

```text
scoreable_payload_events: 97
package_training_requests: 97
edge_count: 7
runtime_bytes_estimate: 32972
strict_ordered_pass_rows: 97
unexpected_local_accepts_under_disabled_threshold: 0

shadow operator_candidate_calls: 97
shadow nando_shadow_accepts: 0
shadow false_accepts: 0
shadow p99_shadow_score_latency_ns: 484436

verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_explicit_verification: 97
market_claim_allowed: false
```

Catalog status after profile:

```text
CANDIDATE / PROFILE_READY_ACCEPTS_DISABLED / VERIFIER_MISSING
```

Decision:

```text
`test_output_parse` now has a compiled .nwrb profile and overlay registry for
shadow scoring. It is still blocked from verified CPU savings because explicit
verification/admission is missing for all 97 scoreable rows. The next profile
work must add deterministic verifier/admission, not lower the threshold from
score margins alone.
```

### Test Output Parse Safe Policy V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-v1.verification-hook-audit.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/test-output-parse-safe-policy-window-v1.verification-hook-audit.report.json
target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json
```

Measured result:

```text
policy_accept_rows: 97
policy_accept_verified_true_rows: 97
policy_accept_verified_false_rows: 0
policy_accept_unverified_rows: 0
runtime_acceptance_mismatches: 0

shadow total_llm_calls: 104
shadow exact_cache_hits: 2
shadow nando_shadow_accepts: 97
shadow verified_safe_accepts: 97
shadow unverified_shadow_accepts: 0
shadow false_accepts: 0
shadow incremental_savings_over_exact_cache: 95
shadow incremental_reduction_vs_exact_cache_milli: 931
shadow p99_shadow_score_latency_ns: 531326

verification_hook_ready_events: 97
verified_cpu_accept_eligible_events: 97
market_claim_allowed: true

full_window base_window_rows: 1000
full_window promoted_rows_inserted: 10
full_window forced_fallback_rows: 990
full_window missing_base_match_rows: 85
full_window shadow nando_shadow_accepts: 10
full_window shadow verified_safe_accepts: 10
full_window shadow false_accepts: 0
full_window shadow incremental_savings_over_exact_cache: 10
full_window shadow incremental_reduction_vs_exact_cache_milli: 10
full_window audit verification_hook_ready_events: 10
full_window audit verified_cpu_accept_eligible_events: 10
feedback route test_output_parse verified_cpu_accept_eligible_events: 10
feedback route test_output_parse incremental_verified_request_fingerprints: 10
feedback total verified_cpu_accept_eligible_events: 42
feedback total incremental_unique_cpu_accepts: 35
```

Catalog status after safe-policy shadow:

```text
PROVEN / ROUTE_SPECIFIC_97_OF_104 / FULL_WINDOW_ROUTE_DELTA_10_OF_1000
```

Decision:

```text
`test_output_parse` is now a verified narrow CPU profile for request-time
previous-tool-output status parsing. This is not a broad answer/explain route:
the accepted rows are only rows where the agent loop already has deterministic
previous tool-output state and the `.nwrb` score passes the promoted strict
energy threshold.

The route-specific proof is strong, but it is not the CPU80 denominator. The
current feedback window contains only 10 matching request fingerprints from the
97 promoted rows; the other 85 promoted rows are outside the current 1000-call
forecast window and are intentionally not counted. The next debt is to mine the
next high-value call class through BUSINESS_VALUE_GATE, or rerun discovery on a
larger/current trace window before increasing the market claim.
```

## File Path Evidence Payload Dry-Run V1

Source report:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-payload-dry-run-v1.report.json
```

Purpose:

```text
Turn the broad-route split `file_path_evidence_answer` into real request-side
payloads without promoting broad answer/project routes.
```

Measured on the same 5k Codex history window:

```text
file_path_evidence_candidate_events: 146
non_exact_candidate_events: 144
exact_cache_overlap_events: 2
payload_ready_events: 122
payload_built_events: 44
scoreable_payload_events: 44
profile_registered: false
shadow_score_ready: false
expected_unique_cpu_accepts_over_exact_cache: 0
expected_savings_milli: 0
false_accepts: 0
local_accepts_enabled: false
market_claim_allowed: false
```

Parent-route provenance:

```text
answer_or_explain: 79
project_context_dialogue: 65
agent_continue_execute: 2
```

Catalog status:

```text
CANDIDATE
```

Decision:

```text
This row has real traffic and scoreable request-side payloads, but it is not
CPU savings. It stays CANDIDATE until a disabled-threshold profile, deterministic
source/path verifier, admission audit, shadow, feedback, and CPU catalog prove
unique accepts over exact cache with false_accepts=0.
```

### Disabled Profile V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-profile-v1.verification-hook-audit.report.json
```

Measured profile:

```text
profile_id: split_file_path_evidence_answer_profile_v1
package_bytes: 128
edge_count: 7
runtime_bytes_estimate: 32972
threshold: 2147483647
scoreable_payload_events: 44
package_training_requests: 44
positive_margin_rows: 44
strict_ordered_pass_rows: 44
unexpected_local_accepts_under_disabled_threshold: 0
median_energy_margin: 1123328
```

Measured shadow/audit:

```text
operator_candidate_calls: 44
nando_shadow_accepts: 0
nando_shadow_fallbacks: 44
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
p99_shadow_score_latency_ns: 259065
verification_hook_ready_events: 0
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 44
market_claim_allowed: false
```

Catalog status remains:

```text
CANDIDATE
```

Decision:

```text
The profile/scoring path is connected, but the route still adds zero CPU80
savings. The blocking item is verifier evidence, not score geometry. Do not
lower threshold or promote until source_path_or_url_presence_verifier_v1 proves
safe accepts and the feedback/catalog path reports unique value over exact cache.
```

### Output Evidence V1

Source reports:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-shadow-v1.report.json
target/nando-wave/real-traffic-shadow/file-path-evidence-output-evidence-v1.verification-hook-audit.report.json
```

Measured verifier labels:

```text
operator_candidate_calls: 44
scoreable_candidate_calls: 44
output_evidence_matched_events: 39
no_session_output_match_events: 5
deterministic_verification_events: 39
verified_true_events: 15
verified_false_events: 24
raw_prompt_text_written: false
raw_response_text_written: false
local_accepts_enabled: false
market_claim_allowed: false
```

Measured shadow/audit:

```text
nando_shadow_accepts: 0
nando_shadow_fallbacks: 44
verified_safe_accepts: 0
unverified_shadow_accepts: 0
false_accepts: 0
incremental_reduction_vs_exact_cache_milli: 0
verification_hook_ready_events: 39
verified_cpu_accept_eligible_events: 0
candidates_missing_output_evidence: 5
provider_cost_events: 0
```

Catalog status becomes:

```text
WATCH
```

Decision:

```text
Verifier evidence exists now, but the disabled profile still accepts zero rows.
The route remains non-savings until request-side admission calibration finds a
robust safe policy and a promoted shadow run proves unique verified accepts over
exact cache with false_accepts=0.
```

### Admission Calibration V1

Source report:

```text
target/nando-wave/real-traffic-shadow/file-path-evidence-admission-calibration-v1.report.json
```

Measured request-side admission:

```text
hook_ready_rows: 39
rows_with_prompt_features: 39
history_prompt_missing_rows: 0
label_true_rows: 15
label_false_rows: 24
minimum_true_support: 3
robust_safe_policy_found: false
singleton_safe_policy_found: true
best_robust_true_accepts: 0
best_singleton_true_accepts: 1
raw_prompt_text_written: false
raw_response_text_written: false
response_text_used_for_features: false
target_labels_used_for_runtime: false
proof_labels_used_for_runtime: false
local_accepts_enabled: false
market_claim_allowed: false
```

Catalog status:

```text
WATCH
```

Decision:

```text
This split has verifier labels, but no robust request-side admission policy.
The strongest zero-false policy accepts only one verifier-true row, below the
minimum robust support of 3. Do not promote. Either collect more verifier-true
non-synthetic rows, split the family more narrowly, or choose the next
higher-value artifact-backed profile.
```

Blocked for now:

```text
answer_or_explain as a whole
project_context_dialogue as a whole
agent_continue_execute as a whole
IME singleton-only routes
resource_pressure without verifier-true evidence
```

## Current5k Discovery Triage

Manual route discovery and broad-route split discovery are review-only. They
can nominate the next profile; they do not prove savings.

Current manual discovery top rows:

```text
ime_input_state_debug:
  candidate_events: 16
  payload_ready_events: 6
  output_evidence_matched_events: 5
  verified_true_events: 3
  verified_false_events: 2
  robust_safe_policy_found: false
  best_singleton_true_accepts: 1
  decision: WATCH / SINGLETON_ONLY_NO_ROBUST_POLICY

document_stamp_layout_edit:
  candidate_events: 5
  payload_ready_events: 4
  decision: WATCH until document/file verifier evidence exists

resource_pressure_budget:
  manual candidate_events: 7
  payload_ready_events: 3
  current scoreable_payload_events: 1
  verifier_true_events: 0
  decision: WATCH until real verifier-true support exists
```

Current broad split top candidates:

```text
file_path_evidence_answer:
  candidate_events: 146
  scoreable_payload_events: 44
  verified_true_events: 16
  verified_false_events: 23
  robust_safe_policy_found: false
  decision: WATCH / SINGLETON_ONLY_NO_ROBUST_POLICY

test_output_parse:
  broad split candidates: 55 answer_or_explain + 46 project_context
  existing full-window route already contributes 91 incremental unique accepts
  decision: PROVEN for previous-tool-output status parsing, not broad answer parsing

metric_from_report:
  answer_or_explain candidate_events: 27
  project_context candidate_events: 11
  current metrics_report proven support: 3 incremental unique
  decision: WATCH unless a stronger numeric-report subfamily is split

git_status_summary:
  combined broad candidates: 26
  existing git_control: 90 scoreable / 74 hooks / no safe policy
  decision: WATCH until command-outcome subfamily separates true from false

report_sync:
  combined broad candidates: 30
  payload_ready_events: 5
  decision: CANDIDATE only after artifact-diff verifier is implemented
```

Next selection rule:

```text
Prefer a new narrow split only if it can plausibly add >= 3 verified unique
accepts over exact cache with false_accepts=0. Otherwise record WATCH and move
to the next higher-value route instead of squeezing a singleton.
```

## Claim Boundary

Allowed:

```text
On the current non-synthetic 1000-call Codex trace, the CPU call catalog finds
7 proven call classes and 25 incremental unique verified CPU accepts over exact
cache, with false_accepts=0.
```

Not allowed:

```text
Nando saves 80%
Nando saves market traffic
scoreable rows are savings
broad answer routes are safe
candidate rows are verified CPU accepts
```
