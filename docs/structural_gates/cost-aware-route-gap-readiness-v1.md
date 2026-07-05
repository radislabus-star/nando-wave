# Cost-Aware Route-Gap Readiness V1

Verdict:

```text
COST_AWARE_ROUTE_GAP_READINESS_V1
COUNT_ONLY_ROUTE_PRIORITY_REJECTED
NEXT_CPU80_WORK_FAMILY: git_control
CPU80_NOT_ACHIEVED
```

Why:

```text
After token/cost meter, choosing the next route by call count alone is wrong.

The old count view made project_context_dialogue look like the biggest gap:
  project_context_dialogue candidates: 1314+

But project_context_dialogue is a broad route and remains REJECT_FOR_NOW.
It cannot be promoted whole.

The cost-aware payload-ready view asks a narrower product question:
  among non-routed or under-routed real traffic, which family has
  request-side payload readiness plus the most estimated token/cost pressure?
```

Current5k route-gap payload readiness:

```text
report:
  target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1-current5k.report.json

sampled_llm_calls: 5000
existing_route_candidate_events: 1432
no_candidate_events: 3568
payload_ready_events: 807

top_payload_ready_family: git_control

git_control:
  candidate_events: 126
  payload_ready_events: 90
  candidate_tokens: 102424
  candidate_cost_microusd: 307272
  payload_ready_tokens: 94932
  payload_ready_cost_microusd: 284796
  recommended_payload_builder: git_command_intent_payload_builder_v1
  recommended_verifier: git_status_and_command_outcome_verifier_v1
```

Current5k catalog bridge:

```text
report:
  target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1-current5k.combined.report.json

route_gap_top_payload_ready_family: git_control
route_gap_top_payload_ready_tokens: 94932
route_gap_top_payload_ready_cost_microusd: 284796
```

Manual discovery current5k side note:

```text
report:
  target/nando-wave/real-traffic-shadow/manual-route-discovery-v1-current5k.report.json

top_subfamily: business_party_identity_address
payload_ready_cost_microusd: 4032

This is useful for future domain profiles, but it is much smaller than the
git_control ready-cost pressure and should not preempt the next CPU80 route.
```

Claim boundary:

```text
Allowed:
  Cost-aware route-gap prioritization says git_control is the next highest
  ready-cost route family to improve.

Not allowed:
  Counting git_control ready rows as verified savings.
  Promoting broad project_context_dialogue.
  Claiming CPU80.
  Treating estimated microusd as provider billing truth while
  token_cost_estimate_used=true.
```

Next engineering debt:

```text
Improve git_control payload/evidence geometry.
The current catalog shows git_control is already PROVEN tiny support, but
support is exhausted at 3 incremental accepts. The next work must split a new
git subfamily or improve command outcome evidence before another promote.
```

