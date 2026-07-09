# Architecture Version Registry

Status: active source of truth for Nando Wave architecture version names.

Rule: every compression/savings report must be tied to explicit architecture
versions. Bump the relevant version when behavior, schema, promotion policy, or
hot-path semantics change. Do not bump for comments or pure formatting.

## Active Versions

| Component | Current version | Meaning |
|---|---:|---|
| Phase-center core | `phase_center_core_v1` | Phase vectors are reduced to positive/negative centers and scored by margin. |
| Online miner | `online_phase_center_miner_v1` | Streaming buckets learn phase centers from real trace events before local accept. |
| Live-tail daemon | `append_live_tail_shadow_daemon_v6` | Follows live append events, scores before update, shadow-only, writes `.nwpc` evidence, appends versioned decisions, and reports restart-safe stable-window compression blockers. |
| Hot runtime | `phase_center_hot_runtime_v1` | Preloaded numeric route/profile scoring path for `.nwpc` packages. |
| Auto-subcenter discovery | `auto_subcenter_discovery_v2_hidden_first` | Hidden-state atoms are generated before visible pair/combo subcenters. |
| Hidden-state profile | `hidden_state_cross_layer_profile_v1` | Cross-layer request/state/tool atoms form the second inferred profile lane. |
| NANDA CPU architecture | `compact_latent_transition_runtime_v1` | L1 surface -> L2 hidden state -> L3 transition center -> L4 selector -> verifier; stores compact verified transitions, not answers. |
| Profile attribution | `profile_attribution_disjoint_v1` | Reports observable-only, hidden-only, mixed, and unknown contribution without double-count. |
| Compression accounting | `restart_safe_claimsafe_stable_window_calls_tokens_cost_milli_accounting_v4` | Reports calls/tokens/cost saved in milli units over the current denominator and blocks compression claims when safety, denominator, minimum-window, or architecture-version gates are red. |
| Package format | `nwpc_v1` | Phase-center package format. |
| Forbidden backend policy | `no_nwrb_no_lookup_no_local_accept_without_verifier_v1` | `.nwrb`, lookup authority, target/proof authority, and local accept without verifier are forbidden. |

## Bump Rules

- `phase_center_core`: bump if vector encoding, center construction, or margin
  math changes.
- `online_miner`: bump if bucket activation, threshold learning, candidate
  admission, or rejection logic changes.
- `live_tail_daemon`: bump if stream order, score-before-update semantics,
  quarantine behavior, or report cadence changes.
- `auto_subcenter_discovery`: bump if atom generation or ordering changes.
- `hidden_state_profile`: bump if hidden-state atom families or allowed source
  evidence change.
- `nanda_cpu_architecture`: bump if the north-star state/action transition
  contract changes, especially L1/L2/L3/L4 responsibility, verifier authority,
  or the allowed phase-center `.nwpc` product path.
- `profile_attribution`: bump if profile-kind classification or disjoint
  accounting changes.
- `compression_accounting`: bump if calls/tokens/cost denominators or milli
  calculation changes, or if claim-allowed/blocker policy changes.
- `package_format`: bump only on incompatible `.nwpc` serialization changes.

## Current Report Contract

`phase_stream_hot_path_daemon_append_live_tail_v1` must include
`architecture_versions` so every live compression snapshot can be traced back to
the exact miner/profile/accounting architecture.
