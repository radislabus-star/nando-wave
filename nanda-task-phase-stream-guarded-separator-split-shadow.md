# NANDA Task: Phase Stream Guarded Separator Split Shadow

## query

Verify the bounded structural claim for the split-window guarded separator
phase-center report.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| split_report | report_kind | `generic_real_traffic_phase_center_guarded_separator_split_shadow_v1` | report JSON |
| split_report | split_granularity | `route_local_event_order` | report JSON |
| split_report | selector_train_shadow_disjoint | `true` | report JSON |
| split_report | shadow_window_independent | `true` | report JSON |
| split_report | local_accept_enabled | `false` | report JSON |
| split_report | market_claim_allowed | `false` | report JSON |
| split_report | legacy_backend_revived | `false` | active crates search |
| frontier_union | unique_accepts | `13` | frontier union report |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| split_report | split_granularity | `route_local_event_order` | report JSON |
| split_report | selector_train_shadow_disjoint | `true` | report JSON |
| split_report | shadow_window_independent | `true` | report JSON |
| split_report | local_accept_enabled | `false` | report JSON |
| split_report | market_claim_allowed | `false` | report JSON |
| split_report | legacy_backend_revived | `false` | active crates search |

## rejected_boundary

Do not claim CPU10, product promotion, serving change, local accept, or market
money from this split-window report.
