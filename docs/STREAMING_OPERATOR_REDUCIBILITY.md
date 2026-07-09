# Streaming Operator Reducibility

Status: design contract plus active phase-center streaming direction.

Current architecture contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Current live interpretation:

```text
reducible operator
  = repeated verifiable transition
  = compact phase-center with stable margin
  = useful only when it adds unique accepts over exact cache
```

Current best frontier:

```text
unique_cpu_accepts_over_exact_cache: 6_644 / 29_770
calls_saved: 22.3177%
tokens_saved: 72.0541%
false_accepts: 0
local_accept_enabled: false
```

This document defines the allowed direction for online phase-center operator
discovery. The current implementation is limited to `test_output_parse`
shadow mining, quarantine `.nwpc` candidate generation, and an offline
promotion/economics audit. It is not product JIT, does not enable serving
local accept, and does not change `phase_center_runtime.rs`.

## Reducibility Theorem

An operator is reducible when the same action repeats across different states
while preserving the same transition form.

Given samples:

```text
(state_i, action_i, result_i)
```

Encode each transition as a phase vector:

```text
v_i = phase(state_i, action_i, result_i)
```

If a shared action invariant `O` exists, then its compact center is:

```text
center_O = normalize(sum(v_i))
```

For a new transition, the correct candidate must be closer to `center_O` than
the wrong candidate:

```text
margin(v_correct, v_wrong, center_O) > threshold
```

Short form:

```text
many examples -> one compact operator center
```

## Reducibility Conditions

An operator can be treated as reducible only when all of these hold:

1. The action repeats.
2. Fillers, surface forms, and noise change, but the transition form stays the
   same.
3. `state`, `action`, and `result` can be encoded into shared phase atoms.
4. Correct transitions have low intraclass dispersion.
5. Wrong transitions are separated by a stable margin.
6. Heldout transitions pass.
7. Action/role ablation breaks the result.

## Non-Reducible Cases

A transition family is not reducible when any of these hold:

1. There is no repeatable action form.
2. There is no verifier or external check.
3. Wrong transitions cannot be separated.
4. Margin is unstable.
5. The center depends on concrete tokens instead of the action invariant.

## Example

Training transitions:

```text
ABC -> CBA
DEF -> FED
XYZ -> ZYX
```

Shared operator:

```text
reverse
```

Compact center:

```text
center_reverse = normalize(sum(
  phase(ABC -> CBA),
  phase(DEF -> FED),
  phase(XYZ -> ZYX)
))
```

Heldout transition:

```text
correct: KLM -> MLK
wrong:   KLM -> LMK
```

The reducer passes only if `correct` is closer to `center_reverse` than `wrong`
with enough margin.

## Streaming Discovery Pipeline

Allowed mechanism:

```text
event stream
-> extract state/action/result atoms
-> encode phase vector
-> route into rough bucket
-> update positive/negative phase sums
-> monitor count / variance / p10_margin
-> candidate operator
-> shadow verifier
-> compile .nwpc
-> promote only if false_accepts = 0
```

The online stream is allowed to discover candidate centers. It is not allowed
to locally accept without a verifier.

## Streaming Bucket

Each bucket stores:

```text
bucket_key
sum_pos
sum_neg
count_pos
count_neg
recent_ring_buffer
coherence
variance
p10_margin
false_accepts
```

`bucket_key` is a rough route, such as `action_hint` or a rough action hash.
It is not authority. It only decides which candidate center receives the next
sample.

## Online Update

Minimal update:

```text
for event in stream:
  v = encode(event)
  key = rough_route(event)

  bucket[key].sum += v
  bucket[key].count += 1

  center = normalize(bucket[key].sum)

  if bucket[key].count >= 20:
    test_margin(center, heldout_recent)
```

For positive and negative examples:

```text
bucket.sum_pos += v_correct
bucket.count_pos += 1

bucket.sum_neg += v_wrong
bucket.count_neg += 1

positive_center = normalize(bucket.sum_pos)
negative_center = normalize(bucket.sum_neg)
```

The runtime decision remains margin-based:

```text
margin = score(v, positive_center) - score(v, negative_center)
```

## Negative / Wrong Vector Construction

A phase center is not enough unless the wrong side is real. `sum_neg` must be
built only from verifier-checkable wrong transitions.

Allowed negative sources:

1. Verifier-derived wrong labels for the same `state/action`.
   Example: `test_output_parse` has labels such as `pass`, `fail`,
   `compile_error`, and `runtime_panic`. If the verifier proves the event is
   `fail`, then `pass` is a wrong vector for that same event.
2. Same-bucket verifier-false candidates. If a candidate center would accept an
   event but the verifier rejects it, encode that transition into `sum_neg`.
3. Heldout same-action near-negatives. These must use the same action family
   with a different result that the verifier proves false.
4. Older-window base-family negatives. If a bucket has verified positives in
   its train window but no local verified negatives, it may borrow
   verifier-negative events from the same base action family, but only from
   timestamps at or before that bucket train window. These events may update
   only `sum_neg` and threshold calibration. They must never create positive
   accepts by themselves.

Forbidden negative sources:

1. Random garbage negatives.
2. Negatives using `target_id` or `proof_rule_id` authority.
3. Negatives from hidden answer lookup.
4. Negatives that cannot be checked by a verifier.
5. Future-window negatives that would leak heldout evidence into training.

For a labeled verifier event:

```text
v_pos = phase(state, action, verified_result)
v_neg = phase(state, action, verifier_false_result)

bucket.sum_pos += v_pos
bucket.sum_neg += v_neg
```

For a rejected shadow candidate:

```text
if candidate_accepts(event) and verifier_rejects(event):
  bucket.sum_neg += phase(state, action, proposed_result)
  bucket.false_accepts += 1
```

This keeps the negative center tied to the same operational surface as the
positive center. A wrong vector is useful only when it could have fooled the
operator and the verifier proves it false.

## Candidate Gate

A bucket becomes a candidate operator only if:

```text
count >= N
variance <= threshold
p10_margin >= threshold
shadow_false_accepts = 0
```

Recommended first thresholds are intentionally conservative:

```text
N >= 20 for smoke
N >= 100 for a real candidate
N >= 500 before product discussion
```

The gate must report:

```text
candidate_id
bucket_key
count_pos
count_neg
coherence
variance
median_margin
p10_margin
false_accepts
heldout_pass_rate
ablation_action_pass_rate
ablation_role_pass_rate
verifier_name
```

## Fast Path Constraints

To keep discovery cheap:

1. Use 32 or 64 cells for the first stream probes.
2. Track only top buckets.
3. Keep a ring buffer of recent samples per bucket.
4. Update centers incrementally.
5. Recompute full diagnostics only for candidate buckets.

## First MVP

First MVP candidate:

```text
test_output_parse stream
```

Why this route first:

1. `state` can be read from stdout or tool output.
2. `action` is parse test result.
3. `result` is pass, fail, error, or summary class.
4. The verifier is the stdout/tool output itself.
5. False accepts can be checked deterministically.

Example atoms:

```text
state:
  cargo test output
  failing test name
  compiler error code
  panic marker

action:
  parse test result

result:
  pass
  fail
  compile_error
  runtime_panic
  ignored_or_filtered
```

## Current Implementation State

Current narrow CLI surface:

```text
phase-stream-test-output-parse-v1
phase-stream-test-output-raw-log-trace-v1
phase-stream-discovery-v1
phase-stream-online-discovery-v1
phase-stream-real-traffic-online-discovery-v1
phase-stream-test-output-parse-promotion-audit-v1
```

Current implemented path:

```text
real trace or existing raw stdout/stderr logs
-> verifier-derived test_output_parse labels from metadata or raw output
-> phase-center compiler
-> quarantine .nwpc candidate
-> inspect/load/margin parity
-> offline promotion/economics audit
```

Current proven metadata/status narrow report:

```text
trace: target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.trace.jsonl
shadow report: target/nando-wave/streaming/test-output-parse-tool-output-state-v1.shadow-report.json
promotion audit: target/nando-wave/streaming/test-output-parse-tool-output-state-v1.promotion-audit.json

proof_scope: tool_output_state_metadata_parse
metadata_status_shadow_pass: true
raw_output_shadow_pass: false
metadata_status_verified_accepts: 23
raw_output_verified_accepts: 0
metadata_status_claim_allowed: true
raw_output_claim_allowed: false
false_accepts: 0
unique_cpu_accepts_over_exact_cache: 23
local_accept_enabled: false
promoted: false
market_money_claim_allowed: false
billing_evidence_real: false
money_estimate_available: true
token_cost_estimate_used: true
```

Current proven raw stdout/stderr narrow report:

```text
trace build report: target/nando-wave/streaming/test-output-parse-raw-log-v1.trace-report.json
trace: target/nando-wave/real-traffic-shadow/test-output-parse-raw-log-v1.trace.jsonl
shadow report: target/nando-wave/streaming/test-output-parse-raw-log-v1.shadow-report.json
promotion audit: target/nando-wave/streaming/test-output-parse-raw-log-v1.promotion-audit.json

source: existing raw stdout/stderr log artifacts
raw_output_classified_events: 13
verifier_metadata_classified_events: 0
synthetic_events: 0
candidate_labels: pass, runtime_panic
proof_scope: raw_output_parse
raw_output_shadow_pass: true
metadata_status_shadow_pass: false
raw_output_verified_accepts: 2
metadata_status_verified_accepts: 0
raw_output_claim_allowed: true
metadata_status_claim_allowed: false
false_accepts: 0
min_margin_micro: 64702
unique_cpu_accepts_over_exact_cache: 2
local_accept_enabled: false
promoted: false
market_money_claim_allowed: false
billing_evidence_real: false
money_estimate_available: true
token_cost_estimate_used: true
```

Current offline discovery registry report:

```text
discovery report: target/nando-wave/streaming/online-phase-center-discovery-v1.report.json
candidate package dir: target/nando-wave/streaming/discovery

mode: offline_shadow_discovery_only
cells: 32
trace inputs:
  target/nando-wave/real-traffic-shadow/test-output-parse-tool-output-state-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/test-output-parse-raw-log-v1.trace.jsonl

total_rows: 117
parsed_events: 110
skipped_unclassified_events: 7
bucket_count: 2
candidate_count: 2
accepted_candidate_count: 2
total_unique_cpu_accepts_over_exact_cache: 25
total_nando_cpu_tokens_saved: 5155
total_nando_cpu_cost_saved_microusd: 5267
local_accept_enabled: false
product_runtime_changed: false
serving_runtime_changed: false
market_money_claim_allowed: false

packages:
  target/nando-wave/streaming/discovery/test_output_parse--raw_output_parse--parse_test_output.nwpc
  target/nando-wave/streaming/discovery/test_output_parse--tool_output_state_metadata_parse--parse_test_output.nwpc
```

Current online-order discovery shadow report:

```text
online report: target/nando-wave/streaming/online-phase-center-streaming-discovery-v1.report.json
candidate package dir: target/nando-wave/streaming/online-discovery

mode: online_shadow_discovery_only
cells: 32
min_bucket_events: 4
margin_threshold_micro: 100000

total_rows: 117
parsed_events: 110
skipped_unclassified_events: 7
bucket_count: 2
compiled_bucket_count: 2
accepted_bucket_count: 1
stream_shadow_events: 79
stream_shadow_accepts: 69
stream_false_accepts: 0
total_unique_cpu_accepts_over_exact_cache: 68
total_nando_cpu_tokens_saved: 6942
total_nando_cpu_cost_saved_microusd: 7214
local_accept_enabled: false
product_runtime_changed: false
serving_runtime_changed: false
market_money_claim_allowed: false

accepted online bucket:
  test_output_parse::tool_output_state_metadata_parse::parse_test_output
  precompile_events: 23
  shadow_events: 74
  unique_cpu_accepts_over_exact_cache: 68
  false_accepts: 0
  runtime_margin_parity_mismatches: 0

rejected online bucket:
  test_output_parse::raw_output_parse::parse_test_output
  reason: no_unique_accepts_over_exact_cache
  false_accepts: 0
  wrong_wins: 0
  min_margin_micro: 64702
```

Current generic real-traffic online discovery report:

```text
generic report: target/nando-wave/streaming/real-traffic-phase-center-online-discovery-v1.report.json
candidate package dir: target/nando-wave/streaming/real-traffic-online-discovery

mode: online_shadow_discovery_only
cells: 32
min_bucket_events: 4
margin_threshold_micro: 300000

trace inputs:
  target/nando-wave/real-traffic-shadow/agent-continue-execute-artifact-progress-v1-current5k.trace.jsonl
  target/nando-wave/real-traffic-shadow/serving-ops-output-evidence-v1-current5k.trace.jsonl
  target/nando-wave/real-traffic-shadow/answer-evidence-output-evidence-v1.trace.jsonl
  target/nando-wave/real-traffic-shadow/read-inspect-output-evidence-v1.trace.jsonl

parsed_candidate_events: 323
skipped_legacy_profile_events: 0
bucket_count: 4
compiled_bucket_count: 4
accepted_bucket_count: 1
stream_shadow_events: 307
stream_shadow_accepts: 4
stream_false_accepts: 1
total_unique_cpu_accepts_over_exact_cache: 2
total_nando_cpu_tokens_saved: 0
total_nando_cpu_cost_saved_microusd: 0
token_cost_evidence_missing_events: 2
local_accept_enabled: false
product_runtime_changed: false
serving_runtime_changed: false
market_money_claim_allowed: false

accepted bucket:
  route_gap_agent_continue_execute_profile_v1::agent_continue_execute
  false_accepts: 0
  unique_cpu_accepts_over_exact_cache: 2
  token_cost_evidence_missing_events: 2

rejected buckets:
  route_gap_answer_evidence_profile_v1::answer_or_explain
  route_gap_read_inspect_profile_v1::read_inspect
  route_gap_serving_ops_profile_v1::serving_ops
```

Read this as a shadow/audit rung only. It is not a production online JIT
operator system and not a market money claim.

Read the first report as a metadata/status operator. Read the second report as
a raw stdout/stderr artifact parser proof for existing log artifacts only. The
raw-output report closes the previous `raw_output_classified_events=0` hole,
but it still does not promote a serving profile and does not prove market
savings on live provider traffic.

`billing_evidence_real` is only an evidence-quality flag. Even if a future
trace carries real provider token and cost rows, this offline audit must still
keep `market_money_claim_allowed=false`. A money claim requires a separate
product-serving approval gate, not this audit command.

## Promotion Rule

Promotion is allowed only after shadow verification:

```text
shadow_events >= threshold
false_accepts = 0
p10_margin >= threshold
ablation_action collapses
ablation_role collapses
heldout passes
```

When promoted, the candidate compiles to:

```text
.nwpc
```

Server runtime loads packages only. It does not compile live traffic inside the
serving hot path.

## Verifier-Bound Package Contract

A promoted `.nwpc` candidate must carry or reference its verifier contract. A
score by itself is not accept authority.

Required package or manifest fields:

```text
verifier_name
verifier_version
verifier_input_kind
verifier_evidence_source
accept_rule
false_accept_threshold = 0
shadow_report_fingerprint
training_window_fingerprint
```

Runtime local accept requires all of these:

```text
score_margin >= threshold
verifier binding exists
request class matches verifier_input_kind
promotion report proves false_accepts = 0
```

Otherwise the action is:

```text
fallback_to_llm
```

The verifier binding is part of the package boundary. If a package cannot prove
which verifier owns its local-accept rule, it is a scoring artifact only, not a
promotable local operator.

## Forbidden

These remain forbidden:

```text
no .nwrb
no role-binding runtime
no payload-builder/verifier/catalog backend revival
no local accept without verifier
no hidden lookup
no target/proof authority
no manual local_out_t
no phase_center_runtime.rs edits in this step
```

## Claim Boundary

Allowed current claim:

```text
Streaming operator reducibility is the approved phase-center discovery
contract, and the current test-output route has a narrow metadata/status
shadow implementation that can build a verifier-bound quarantine .nwpc
candidate and run an offline promotion/economics audit.
```

Forbidden current claim:

```text
Online JIT operators are implemented.
Product runtime local-accept is enabled.
Streaming discovery has proved market savings.
The quarantine .nwpc is a serving/profile artifact.
PROMOTION_ELIGIBLE_REVIEW means product promotion.
Real billing evidence inside the offline audit means market money claim is
allowed.
Current metadata/status PASS proves raw stdout/stderr parsing.
```

The next product-serving step requires a separate reviewer approval.

## Implementation Gate

This document is still a contract for the broader online discovery system, not
blanket implementation approval for product serving.

For future general online phase-center compiler/JIT work, ask for:

```text
OK_TO_IMPLEMENT_ONLINE_PHASE_CENTER_COMPILER
```

Without that explicit reviewer OK:

```text
no JIT code
no package schema change
no phase_center_runtime.rs change
no product runtime change
```

The current accepted implementation remains narrow:

```text
test_output_parse shadow command
test_output_parse raw-log trace command
quarantine .nwpc candidate
offline promotion/economics audit
proof_scope tool_output_state_metadata_parse or raw_output_parse
metadata_status_claim_allowed true only for metadata/status trace
raw_output_claim_allowed true only for raw-output trace
local_accept disabled
promoted false
market_money_claim_allowed false
billing_evidence_real is not claim authority
```
