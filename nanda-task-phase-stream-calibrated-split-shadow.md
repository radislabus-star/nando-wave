# NANDA Task: Phase Stream Calibrated Split Shadow

## query

Verify the bounded structural claim for the calibrated split-window
phase-center report.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| calibrated_report | report_kind | `generic_real_traffic_phase_center_guarded_separator_calibrated_split_shadow_v1` | report JSON |
| calibrated_report | split_granularity | `route_local_event_order` | report JSON |
| calibrated_report | selector_compile_calibration_shadow_disjoint | `true` | report JSON |
| calibrated_report | shadow_window_independent | `true` | report JSON |
| calibrated_report | threshold_selected_before_shadow | `true` | calibration window fields |
| calibrated_report | false_accepts | `0` | report JSON |
| calibrated_report | local_accept_enabled | `false` | report JSON |
| calibrated_report | market_claim_allowed | `false` | report JSON |
| frontier_union | unique_accepts | `15` | frontier union report |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| calibrated_report | split_granularity | `route_local_event_order` | report JSON |
| calibrated_report | selector_compile_calibration_shadow_disjoint | `true` | report JSON |
| calibrated_report | shadow_window_independent | `true` | report JSON |
| calibrated_report | threshold_selected_before_shadow | `true` | calibration window fields |
| calibrated_report | local_accept_enabled | `false` | report JSON |
| calibrated_report | market_claim_allowed | `false` | report JSON |

## rejected_boundary

Do not claim CPU10, product promotion, serving change, local accept, market
money, or unsafe threshold lowering from this report.
