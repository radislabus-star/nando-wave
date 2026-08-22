# R8B V8 Delegated Launch

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | M24 delegated-launch request | submits through | exact user-manager launch tool | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:202-214 | 1.0 | request owner | submission tool | execution | submission-hop |
| t2 | exact systemd-run tool | delegates creation to | authenticated user manager | machine-cardinality-baseline.v8.json:41-56 | 1.0 | submission tool | launch owner | execution | manager-hop |
| t3 | authenticated user manager | launches and owns | transient M24-child service | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V3.md:26-26 | 1.0 | launch owner | service process | execution | service-hop |
| t4 | user-manager identity | requires conjunction of | unprivileged and privileged observation channels | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V3.md:11-14 | 1.0 | observed platform identity | independent observations | observation | manager-identity |
| t6 | pre and post live-image hashes | equal | pinned systemd image hash | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V3.md:52-53 | 1.0 | bounded observations | frozen tool identity | proof | manager-image |
| t9 | M24 delegated-launch parent | stops only | authenticated transient M24-child unit | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:294-299 | 1.0 | cleanup request owner | test service | mutation | exact-stop |
| t10 | transient M24-child unit residue | blocks | P06 delegated-launch packet | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:301-304 | 1.0 | failure observation | packet eligibility | failure | fail-closed |
| t11 | M24 resource receipt | binds | manager image and transient-unit observations | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V3.md:27-27 | 1.0 | observation receipt | platform and service facts | evidence | resource-provenance |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | M24 delegated-launch request | submits through | exact user-manager launch tool | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:216-220 | 1.0 | request owner | submission tool | execution | submission-hop |
| c2 | exact systemd-run tool | delegates creation to | authenticated user manager | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:219-222 | 1.0 | submission tool | launch owner | execution | manager-hop |
| c3 | authenticated user manager | launches and owns | transient M24-child service | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:221-225 | 1.0 | launch owner | service process | execution | service-hop |
| c4 | user-manager identity | requires conjunction of | unprivileged and privileged observation channels | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:227-264 | 1.0 | observed platform identity | independent observations | observation | manager-identity |
| c6 | pre and post live-image hashes | equal | pinned systemd image hash | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:250-258 | 1.0 | bounded observations | frozen tool identity | proof | manager-image |
| c9 | M24 delegated-launch parent | stops only | authenticated transient M24-child unit | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:294-304 | 1.0 | cleanup request owner | test service | mutation | exact-stop |
| c10 | transient M24-child unit residue | blocks | P06 delegated-launch packet | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:301-308 | 1.0 | failure observation | packet eligibility | failure | fail-closed |
| c11 | M24 resource receipt | binds | manager image and transient-unit observations | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md:250-258 | 1.0 | observation receipt | platform and service facts | evidence | resource-provenance |

## notes

- Direct cgroupfs ownership and direct M24-to-child fork claims are forbidden.
- Frozen service properties and observation-tool non-authority remain mandatory contract checks.
- This worksheet checks route coherence only; it grants no execution authority.
