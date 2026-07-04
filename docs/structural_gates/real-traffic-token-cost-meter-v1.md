# Real Traffic Token/Cost Meter V1

Verdict:

```text
REAL_TRAFFIC_TOKEN_COST_METER_V1
CALLS_ONLY_METRIC_REJECTED
MARKET_MONEY_CLAIM_REQUIRES_TOKEN_COST_SOURCE
STRUCTURAL_TRIADS_PASS
```

Why:

```text
CPU offload cannot be sold from calls_saved alone.

The product-facing denominator is now:
  calls avoided
  tokens avoided
  money saved

For current Codex history traffic, exact provider token counters are not
available in the saved event rows. Therefore current money numbers are explicit
estimates, not billing truth.
```

Schema additions:

```text
real-traffic event/trace rows now carry:
  input_tokens
  output_tokens
  cached_input_tokens
  model_id
  provider
  estimated_input_cost_microusd
  estimated_output_cost_microusd
  estimated_total_cost_microusd
  token_cost_estimate_used
```

Config:

```text
model_price_config_path:
  data/real_traffic/model_price_config.v1.json

The config is user-editable and must be replaced with real provider billing
prices before using money_saved as an external market claim.
```

Estimator:

```text
input_tokens_estimated  = chars / 4
output_tokens_estimated = chars / 4

Current reports set:
  token_cost_estimate_used = true
```

Feedback-loop aggregates:

```text
total_baseline_tokens
total_baseline_cost_microusd
exact_cache_tokens_saved
exact_cache_cost_saved_microusd
nando_cpu_tokens_saved
nando_cpu_cost_saved_microusd
combined_tokens_saved
combined_cost_saved_microusd
nando_calls_saved_pct
nando_tokens_saved_pct
nando_cost_saved_pct
combined_calls_saved_pct
combined_tokens_saved_pct
combined_cost_saved_pct
```

Current5k measured output:

```text
total_llm_calls: 5000
exact_cache_hits: 452
nando incremental unique CPU accepts over exact cache: 110

total_baseline_tokens: 609118
total_baseline_cost_microusd: 1827354

exact_cache_tokens_saved: 4046
exact_cache_cost_saved_microusd: 12138

nando_cpu_tokens_saved: 23224
nando_cpu_cost_saved_microusd: 69672

combined_tokens_saved: 27270
combined_cost_saved_microusd: 81810

nando_calls_saved_pct: 2.20
nando_tokens_saved_pct: 3.8127259414432015
nando_cost_saved_pct: 3.8127259414432015

combined_calls_saved_pct: 11.24
combined_tokens_saved_pct: 4.476965054390118
combined_cost_saved_pct: 4.476965054390118
```

Catalog rule:

```text
cpu-operator-catalog copies the exact token/cost block from feedback-loop.
It does not recalculate a second economics view.
```

Claim boundary:

```text
Allowed:
  On this current5k estimated Codex-history window, Nando incremental CPU
  accepts avoid 110 / 5000 calls and an estimated 23224 / 609118 baseline
  tokens, using the configured chars/4 estimator.

Not allowed:
  Billing-grade customer money saved claim.
  Provider-specific price claim.
  Treating estimated output tokens as observed provider output counters.
```

Structural guard:

```text
Do not mix:
  exact-cache savings
  Nando incremental unique CPU accepts
  combined cache+Nando savings
  estimated tokens
  observed provider billing tokens

token_cost_estimate_used must stay true until trace rows carry real token
counters from the provider/API gateway.
```

NANDA structural gate:

```text
triads_packet: docs/structural_gates/real-traffic-token-cost-meter-v1.triads.json
verdict: PASS
complexity_score: 59
trace_path: /tmp/nanda-structural-gate/real-traffic-token-cost-meter-v1.trace.json
```

Sparse structural triads:

```text
baseline_call -> owns -> baseline_tokens
baseline_call -> owns -> baseline_cost
exact_cache_hit -> saves -> duplicate_call_tokens
exact_cache_hit -> saves -> duplicate_call_cost
nando_incremental_accept -> saves -> unique_non_exact_call_tokens
nando_incremental_accept -> saves -> unique_non_exact_call_cost
combined_savings -> equals -> exact_cache_savings + nando_incremental_savings
feedback_loop -> computes -> token_cost_meter
cpu_operator_catalog -> copies -> feedback_loop.token_cost_meter
model_price_config -> prices -> estimated_token_counts
chars_div_4_estimator -> estimates -> input_tokens
chars_div_4_estimator -> estimates -> output_tokens
provider_billing_counter -> not_present_in -> current_codex_history
token_cost_estimate_used -> must_be_true_for -> current_codex_history
market_money_claim -> blocked_by -> token_cost_estimate_used
provider_billing_counter -> would_unlock -> billing_grade_money_claim
```

Route separation:

```text
exact_cache route:
  repeated request_fingerprint -> exact_cache_tokens_saved

Nando incremental route:
  verified_safe_accept=true
  false_local_accept=false
  exact_cache_hit=false
  unique request_fingerprint
  -> nando_cpu_tokens_saved

Combined route:
  exact_cache route + Nando incremental route
  -> combined_tokens_saved
```
