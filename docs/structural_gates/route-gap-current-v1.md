# route-gap-current-v1

## Claim

After the planning profile is included in the route catalog, the current
no-candidate zone is smaller and the top payload-ready gap family is
`read_inspect`, not `planning_next_step`.

This packet checks measurement coherence only. It must not be used as CPU
Routability 80 proof or as a market savings claim.

## triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| current_route_gap_catalog | registry | profile-registry-planning-next-step-v3.json | route-gap-catalog-current-v1.report.json |
| current_route_gap_catalog | sampled_llm_calls | 1000 | route-gap-catalog-current-v1.report.json |
| current_route_gap_catalog | routed_candidate_events | 462 | route-gap-catalog-current-v1.report.json |
| current_route_gap_catalog | no_candidate_events | 538 | route-gap-catalog-current-v1.report.json |
| current_route_gap_catalog | local_accepts_enabled | false | route-gap-catalog-current-v1.report.json |
| current_route_gap_readiness | payload_ready_events | 35 | route-gap-payload-readiness-current-v1.report.json |
| current_route_gap_readiness | top_payload_ready_family | read_inspect | route-gap-payload-readiness-current-v1.report.json |
| read_inspect | candidate_events | 27 | route-gap-payload-readiness-current-v1.report.json |
| read_inspect | payload_ready_events | 12 | route-gap-payload-readiness-current-v1.report.json |
| read_inspect | recommended_payload_builder | read_inspect_request_payload_builder_v1 | route-gap-payload-readiness-current-v1.report.json |
| read_inspect | recommended_verifier | read_only_path_and_excerpt_verifier_v1 | route-gap-payload-readiness-current-v1.report.json |
| cpu_operator_catalog_v4 | current_verified_cpu_accepts | 17 | cpu-operator-catalog-v4.current-route-gap.report.json |
| cpu_operator_catalog_v4 | verified_gap_to_80_calls | 783 | cpu-operator-catalog-v4.current-route-gap.report.json |
| cpu_operator_catalog_v4 | market_claim_allowed | false | cpu-operator-catalog-v4.current-route-gap.report.json |

## candidate_triads

| subject | relation | object | evidence |
| --- | --- | --- | --- |
| current_route_gap_catalog | registry | profile-registry-planning-next-step-v3.json | candidate claim |
| current_route_gap_catalog | sampled_llm_calls | 1000 | candidate claim |
| current_route_gap_catalog | routed_candidate_events | 462 | candidate claim |
| current_route_gap_catalog | no_candidate_events | 538 | candidate claim |
| current_route_gap_catalog | local_accepts_enabled | false | candidate claim |
| current_route_gap_readiness | payload_ready_events | 35 | candidate claim |
| current_route_gap_readiness | top_payload_ready_family | read_inspect | candidate claim |
| read_inspect | candidate_events | 27 | candidate claim |
| read_inspect | payload_ready_events | 12 | candidate claim |
| read_inspect | recommended_payload_builder | read_inspect_request_payload_builder_v1 | candidate claim |
| read_inspect | recommended_verifier | read_only_path_and_excerpt_verifier_v1 | candidate claim |
| cpu_operator_catalog_v4 | current_verified_cpu_accepts | 17 | candidate claim |
| cpu_operator_catalog_v4 | verified_gap_to_80_calls | 783 | candidate claim |
| cpu_operator_catalog_v4 | market_claim_allowed | false | candidate claim |
