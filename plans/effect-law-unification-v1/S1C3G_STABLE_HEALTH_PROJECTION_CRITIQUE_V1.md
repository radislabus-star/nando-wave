# S1C-3G Stable Health Projection Critique V1

Status: `ADVERSARIAL REVIEW PASS / AUTHORITY FALSE`

Reviewed artifact: `S1C3G_STABLE_HEALTH_PROJECTION_PREREGISTRATION_V1.md`

| Priority | Finding | Risk | Applied repair |
|---|---|---|---|
| P0 | Calling every health field stable recreates the S1C-3F defect. | A harmless runtime observation can force another production rollback. | Freeze an explicit projection per endpoint and forbid whole-object or raw-hash equality. |
| P0 | Removing health equality entirely would hide a route or admission regression. | Installation could pass while serving is unavailable or authority changed. | Keep `ok`, service identity and each endpoint's published serving/admission fields exact; require `hot == cpu`. |
| P0 | A global field list invents `null` fields on endpoints that do not own them. | Equality would pass for synthetic placeholders rather than endpoint contracts. | Define four endpoint-local stable field sets; missing stable fields are failures. |
| P0 | Capture availability could be treated as an allowed health drift. | The desired installation effect would escape independent checking. | Keep capture env, writer, journal and startup-log checks as separate fail-closed owners. |
| P0 | Patching only `route_probe` leaves `semantic_health_equal` on the old contract. | One hidden equality path can still roll back or accept inconsistently. | Require one pure projection used by both inherited comparison paths and test both call sites. |
| P0 | Dynamic transition profile count could be mistaken for serving-package preservation. | Warm-up timing could censor a safe install, or package drift could be hidden. | Do not compare `transition_active_profiles`; preserve product authority with exact `response_active_profiles`, response cache readiness and admission verdict. |
| P0 | Reusing S1C-3F would rewrite a terminal rollback. | Evidence history and attempt denominator would be corrupted. | Bind all S1C-3F terminal roots and create S1C-3G paper/source/transaction namespaces. |
| P1 | A candidate process replacement necessarily changes PID and raw health bytes. | Exact object equality confuses expected process identity change with route drift. | Check PID replacement/survival separately; retain raw hashes as observations only. |
| P1 | An endpoint could disappear from the projection silently. | The gate could pass on a partial route. | Require exactly `hot`, `control`, `gateway`, and `cpu`, their frozen URLs, all stable keys and `ok == true`. |
| P1 | Relaxed equality could broaden scientific claims. | Capture installation could be reported as grounded meaning or K2. | Keep all scientific/product promotion flags false and open only `S1C-4 COLLECTING` on installation PASS. |
| P1 | The paper could authorize code without real baseline evidence. | A stale local assumption could enter production. | Require mini-PC implementation preflight `READY_TO_IMPLEMENT`, exact baseline bytes/modes, source SHA and runtime comparison coverage before coding. |

## Residual Risks

- The preflight proves contract completeness, not that implementation follows it.
- A stable field can still change for a real operational reason between prepare
  and execute; that correctly yields a stale/preflight or rollback result.
- S1C-3G can prove capture installation only. The first natural decision row and
  all scientific conclusions remain future evidence.

These risks are handled by unit/fault tests, independent receipt verification,
one-attempt discipline, exact rollback, and the narrow result boundary.
