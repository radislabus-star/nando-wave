# Critique Of K2 Self-Formed Uncertainty Preregistration V5 Revision 2

Status: `POST-REPAIR PAPER ACCEPTED / PENDING STRUCTURAL GATES AND PREFLIGHT`

Date: `2026-08-16`

Target: `K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V5.md`

## Verdict

The first V5 repair closed the Development/Confirm schema contradiction and the
one-probe oracle contradiction. A second adversarial pass found four remaining
execution contradictions and three owner-boundary gaps. They are repaired in
the target revision. The paper is now coherent enough to enter owner-bounded
structural gates and implementation preflight, but it grants no code, nonce or
scientific authority by itself.

## Findings And Repairs

| Severity | Finding | Failure if left open | Incorporated repair |
|---|---|---|---|
| P0 | The slot ledger rejected only a reused authorization receipt. | A new receipt could claim another directory and create a second nonce for the same frozen experiment. | The unique slot key is now experiment ID + successor freeze root + V2-V5 aggregate root. Exactly one durable claim is accepted for that key, independent of receipt identity. |
| P0 | `NONCE_COMMITTED` was followed directly by an unjournaled generator pipe write. | A crash after dispatch but before output publication could rerun the generator on restart. | `GENERATOR_DISPATCHED` is durable before the first pipe byte. Any missing or partial result after that event is terminal `GENERATOR_RESULT_INDETERMINATE`; rerun is forbidden. |
| P0 | DevelopmentRehearsal exercised the terminal executable without a distinct terminal schema. | A readiness run could be confused with a sealed scientific PASS. | A closed rehearsal request can emit only `DEVELOPMENT_REHEARSAL_PASS` or a named rehearsal failure and requires zero slots, nonces and sealed attempts. |
| P0 | R9B required K1-K12 while the same controls were defined as attempt-bound after `MODELS_UPDATED`. | A rehearsal receipt could be substituted for the sealed attempt receipt, or R9B could become impossible. | R8B runs separately labelled rehearsal instances; R11B runs the sole attempt-bound K1-K12 set. The sealed terminal accepts only the latter. |
| P1 | The public coordinator's end and the private route's supervisor were implicit. | Resolver effects could flow back through the public selection owner. | The public coordinator exits at `ALL_CASES_PRECOMMITTED`; the confirm owner then supervises only frozen path and receipt transport and cannot revise selection. |
| P1 | The control evaluator lacked sufficient post-outcome read authority for an actual private-byte leak audit. | K4 could pass without comparing public artifacts to nonce and private material. | After `MODELS_UPDATED`, the evaluator receives read-only public and private artifacts with no learner output channel and uses only isolated scratch copies for negative controls. |
| P1 | The cleanup verifier was limited to manifests. | It could not independently hash retained files or census residue. | It receives the complete post-cleanup attempt root and both classified manifests read-only. |
| P0 | The executable list named a closure verifier but no owner for second-probe ranking. | Ranking could fall back into the coordinator or a Cargo test, repeating the R9 failure. | A distinct closure-planner executable owns complete second-probe ranking; the closure verifier remains proof-only. |
| P0 | The executable list named a cleanup verifier but no cleanup mutation owner. | A proof process would have to delete files or cleanup could exist only as test code. | A distinct cleanup-owner executable performs only classified deletion; the cleanup verifier remains read-only. |
| P0 | The terminal proof owner implicitly authorized cleanup and the final public result. | A proof process would participate in live mutation authority and presentation, mixing three role graphs. | A cleanup authorizer now bridges frozen verdict to cleanup mutation, and a result publisher joins only frozen science and cleanup receipts after both proofs. |
| P2 | The four frozen baselines remain one-probe while the model-guided route may use two probes. | PASS would not establish superiority over equally budgeted adaptive baselines. | The asymmetry remains because it is a frozen V2 comparator, but the claim boundary now explicitly excludes that stronger conclusion. The complete one-or-two-probe oracle still prevents a false closure claim. |

## Contradiction Checks

```text
Development wire bytes versus successor executable identity   SEPARATED
one-probe V2 oracle versus two-probe V4 closure                SUPERSEDED
authorization receipt identity versus global attempt slot      SEPARATED
nonce commitment versus irreversible generator dispatch        JOURNALED
R8B readiness controls versus R11B attempt controls             SEPARATED
public selection versus private effect resolution               SEPARATED
rehearsal readiness versus scientific verdict                   SEPARATED
scientific terminal versus cleanup completion                   SEPARATED
```

## Residual Limitations

The experiment remains generated-filesystem, finite, same-host and
`LOCAL_PROCEDURAL`. It does not test natural traffic, external custody,
open-ended strategy learning or an equally budgeted adaptive-baseline family.
Those are claim limits, not hidden PASS exceptions.

## Gate Boundary

The next allowed action is exactly:

```text
four owner-bounded structural worksheets
-> four local structural PASS receipts with zero repairs
-> competing-route code gate
-> implementation preflight READY_TO_IMPLEMENT
```

Any `WATCH`, `VETO`, repair suggestion, `BLOCKED_BEFORE_CODE` or
`safe_to_implement=false` returns the work to paper. No Rust edit, Confirm
nonce, sealed attempt or production mutation is authorized before those gates.
