# NANDA Task: Phase Stream Shadow Request Gap Audit

## query

Verify the bounded structural claim for the missing `nando_shadow_request`
audit. The audit may rank route gaps, but must not turn null-shadow rows into
accepted savings or product promotion.

## triads

| subject | relation | object | evidence |
|---|---|---|---|
| shadow_gap_report | missing_shadow_not_route_candidate_rows | `16262` | report JSON |
| shadow_gap_report | missing_shadow_rejected_candidate_rows | `294` | report JSON |

## candidate_triads

| subject | relation | object | evidence |
|---|---|---|---|
| shadow_gap_report | missing_shadow_not_route_candidate_rows | `16262` | report JSON |
| shadow_gap_report | missing_shadow_rejected_candidate_rows | `294` | report JSON |

## rejected_boundary

Do not claim CPU10, product promotion, serving change, local accept, market
money, or savings from null-shadow token/cost ceiling. The report only proves
where current real-traffic dry-runs fail to create scoreable phase-center
requests.
