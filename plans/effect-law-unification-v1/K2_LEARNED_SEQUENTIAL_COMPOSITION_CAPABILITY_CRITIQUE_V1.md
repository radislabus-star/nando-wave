# K2 Learned Sequential Composition Capability Critical Review V1

Status: `ADVERSARIAL REVIEW COMPLETE / P0-P1 REPAIRS APPLIED`

Date: `2026-08-15`

Target:
`K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PREREGISTRATION_V1.md` draft.

Structural coherence, tests, and a generated capability PASS do not grant
natural, K2, certification, execution, or deployment authority.

## 1. Verdict

The proposed experiment is the correct next finite step, but the first draft
is not implementation-ready. It proves more than one-step selection only if
the topology, denominator, temporal order, and independent executor are closed
before code.

```text
research question                 valid
generated sequential route        worth implementing after repairs
natural K2 claim                   forbidden
implementation authority          false
unresolved P0/P1 before repair     12
unresolved P0/P1 after repair      0
```

## 2. Findings

| Severity | Finding | Failure mode | Required repair |
|---|---|---|---|
| P0 | The dynamic-ID control preserves the same abstract topology. | A planner hard-coded for `copy -> copy plus remove` can pass despite disjoint hashes and paths. | Add a second complete fixture with a three-copy chain, disjoint IDs/paths, one satisfying schedule, and a different quotient cardinality. |
| P0 | The draft does not freeze exact main-fixture candidate dispositions. | An implementation can silently skip difficult invalid sequences while still reporting `15/15`. | Freeze main counts: 8 valid, 7 inapplicable, 5 semantic classes, one satisfying class with 3 members. Freeze control counts: 3 valid, 12 inapplicable, 3 classes, one satisfying member. |
| P0 | The fixture owner knows mapping and goal while orchestrating all processes. | Test harness code can select the answer and merely ask the planner to serialize it. | Require a planner executable receipt, canonical planner stdout, no selected-sequence field in any input, and independent reconstruction of the entire denominator before mapping reopen. |
| P0 | The new sandbox could reinterpret existing Law Lab V1 operations. | A schema-compatible-looking change could invalidate prior Law Lab proof or production assumptions. | Define a separate generated-only sequential protocol and worker. Freeze all existing Law Lab V1 files byte-identical. |
| P0 | Goal verification trusts a manifest emitted by the same worker that executed the sequence. | A dishonest or defective worker can manufacture the expected manifest. | Require adapter-side independent filesystem scan, source-integrity scan, operation-result parity, and a separate oracle binary over the adapter-observed manifest. |
| P1 | Planner and independent verifier may share transition helpers. | A single bug can agree with itself and create false parity. | Put planner and verifier transition logic in separate modules and add a source/call-route gate that forbids cross-calls. |
| P1 | Learner and planner process budgets are not exact. | Failed controls can be omitted or repeated until passing. | Freeze exactly two complete fixture routes and exact external-process counts; classify all other controls as deterministic verifier-only or one bounded real negative execution. |
| P1 | The draft does not bind target/support independence with a typed receipt. | A target copied from support can look like transfer. | Add an exact receipt over content hashes, lengths, topology, tree roots, and absence from learner bytes before planning. |
| P1 | The journal says mapping is reopened after event 23 but does not bind the persisted private artifact. | A different mapping may be substituted after plan freeze. | Publish the private mapping before event 0, bind its root in the experiment freeze, reopen exact bytes after event 23, and compare artifact receipt and file hash. |
| P1 | Semantic equivalence uses terminal manifests but does not freeze costs. | A longer or side-effect-heavy program can collapse with a shorter program. | Quotient only equal-depth programs with identical exact terminal manifests and identical action multiplicity; report lower-depth goal matches separately and require zero. |
| P1 | `same abstract topology` is not defined source-neutrally. | The control may compare private labels `A/B/C` rather than learned relations. | Define topology from learned effect read/write dependencies and normalize opaque actions by dependency graph, not private labels. |
| P1 | No source-size budget is defined. | Another multi-thousand-line proof module becomes difficult to review and encourages repair loops. | Split model, learner, planner, verifier, sandbox, journal, and integration test; cap every new production source file below 1,800 lines and test file below 2,000 lines. |
| P2 | `BUDGET_REJECTED` appears although all 15 programs are inside the frozen depth. | A valid candidate can be hidden as a budget event. | In the main and control routes, budget-rejected count must be zero. Budget failure is a separate negative control. |
| P2 | The exact output roots will depend on executable identities. | A final logging-only rebuild can stale the evidence report. | Run the real process test after all source edits and record only roots from the final executable set. |

## 3. Repaired Experiment Shape

The final paper must freeze two complete routes:

```text
MAIN
  hidden effects: copy p0->p1, copy p1->p2, remove p3
  target: p0+p3
  goal: p0+p1+p2 and no p3
  valid / inapplicable: 8 / 7
  semantic classes: 5
  satisfying class members: 3

TOPOLOGY CONTROL
  hidden effects: copy q0->q1, copy q1->q2, copy q2->q3
  target: q0
  goal: q0+q1+q2+q3
  valid / inapplicable: 3 / 12
  semantic classes: 3
  satisfying class members: 1
```

Both routes use disjoint opaque IDs, paths, contents, support roots, target
roots, planner request roots, and experiment roots. Both must be learned and
planned by the same frozen binaries.

## 4. Claim Boundary After Repair

A PASS would establish:

```text
learned arbitrary-path effects
-> complete bounded explicit planning
-> causal dependency respected
-> semantic schedule quotient
-> exact isolated execution
```

It would not establish:

```text
hidden representation
open-ended effect vocabulary
natural decision evidence
natural K2 composition
automatic language growth
production usefulness or authority
```

The strongest honest interpretation is an explicit learned-world-model
baseline and execution substrate for the next hidden-representation test.

## 5. Required Repair Closure

Before implementation:

1. apply every P0/P1 repair to the preregistration;
2. mark the preregistration frozen after review;
3. run split NANDA packets for provenance, planner/quotient, execution, and
   temporal authority;
4. run code-route design gate with separate learner, planner, verifier,
   mapping, execution, oracle, and journal routes;
5. run implementation preflight with exact baseline hashes and module budgets.

Any WATCH, VETO, `BLOCKED_BEFORE_CODE`, source drift, or unresolved P0/P1 keeps
implementation forbidden.

## 6. Repair Closure

All required changes were applied to the frozen preregistration after this
review:

```text
distinct three-copy topology control             APPLIED
exact main/control candidate denominators         APPLIED
external planner process receipt                  APPLIED
separate generated-only sandbox protocol          APPLIED
adapter-side filesystem observation               APPLIED
planner/verifier source separation gate           APPLIED
exact external-process counts                     APPLIED
typed target-independence receipt                  APPLIED
pre-event private mapping artifact binding         APPLIED
equal-depth semantic quotient                      APPLIED
learned read/write topology normalization          APPLIED
per-file and total source budgets                  APPLIED
zero budget-rejected main candidates               APPLIED
final-executable roots rule                        APPLIED
```

This closure grants paper authority only. NANDA and implementation preflight
remain required before source changes.

## 7. Exact Structural Evidence Index

Each line below binds one owner to one invariant so structural packets do not
reuse a broad prose span for incompatible role fillers.

- Sequential transition owner applies each learned effect to the predicted current-work state.
- Minimum-depth owner requires depth three and rejects every satisfying strict prefix.
- Sequential worker owner applies the resolved operations in order to one disposable current-work tree.
- Adapter observation owner independently scans source and current-work after worker exit.
- Exact oracle owner compares only the adapter-observed terminal manifest with the frozen goal.
- Learned-law owner freezes the complete law set before target and goal reveal.
- Target owner freezes target and goal before planning request publication.
- Plan owner freezes independently verified planner output before private mapping reopen.
- Dispatch owner publishes the execution intent durably before sandbox process creation.
- Restart owner forbids same-identity rerun after an unobserved published dispatch.
- Projection owner replays every legal journal prefix and verifies every cross-event root.
- Terminal owner seals the acyclic evidence chain only after all twenty-nine typed events.
- Cleanup owner removes every disposable workspace and journal after evidence extraction.
