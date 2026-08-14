# K2 Goal Environment Learned Capability Critical Review V1

Status: `ADVERSARIAL REVIEW COMPLETE / REPAIR REQUIRED BEFORE GATE`

Date: `2026-08-14`

Reviewed artifact:
`K2_GOAL_ENVIRONMENT_LEARNED_CAPABILITY_PREREGISTRATION_V1.md` in its initial
`DRAFT FOR ADVERSARIAL REVIEW` state.

This review is preserved separately. Repairs must not erase the findings that
caused them.

## 1. Verdict

The proposed direction is the correct next capability experiment. It moves one
step beyond V1's authored consequences: action effects are inferred from
support executions and transferred to a target that the learner did not see
during law induction. It also retains the exact goal, bwrap executor, oracle,
and all-false authority boundaries.

The initial draft is not yet safe to implement. Several identity, leakage,
independence, persistence, and budget claims are stronger than the proposed
bytes can prove.

```text
route choice                         ACCEPT
bounded scientific question         ACCEPT
claim wording                        REPAIR
implementation authority            DENY UNTIL REPAIRED
deployment authority                false
```

## 2. Findings

| Priority | Finding | Failure if ignored | Required repair |
|---|---|---|---|
| P0 | Learner-visible observations and requests bind the complete experiment freeze root, which itself commits the hidden mapping and target holdout. | Even though SHA-256 is opaque, the learner receives an unnecessary lookup key over forbidden information; a hard-coded or fixture-specific binary could key behavior on it. | Create a separate `K2LearnerPublicContextV1` root containing only catalog, support, grammar, learner identity, and budgets. Learner protocols must never receive the private freeze root, hidden mapping root, target commitment root, or goal root. The terminal verifier binds public and private contexts outside the learner. |
| P0 | The draft says target predictions are durable before the target goal is "read", while the orchestrator must create and freeze that goal before support execution. | Process-local ignorance cannot be proven; the implementation could possess the goal while claiming not to have read it. | Narrow the temporal claim to what receipts can prove: no learner or predictor request contains goal bytes/root, and the V1 selection adapter is not invoked until target predictions are durable. Freeze target bytes in a separate store and test exact learner stdin field absence. |
| P0 | The caller "recomputes" learner output, but no independent verifier owner or non-shared implementation is specified. | The same bug can generate and approve a wrong law or target prediction. | Add `K2LearnedEffectVerifierV1` with an independently structured exhaustive delta checker and manifest transformer. It must not call learner inference/prediction functions. Bind verifier schema/root and require learner/verifier parity for all support laws and target predictions. |
| P0 | The frozen object binds only `hidden_mapping_root_sha256`; no durable private artifact or restart recovery owner is defined for the mapping bytes. | A restart can know that some mapping existed but cannot validate or safely resume exact support/target execution. | Publish the complete private mapping as an immutable canonical artifact before journal freeze; bind its artifact root and path-independent bytes. Restart reopens and validates it before any continuation. It remains absent from learner stdin. |
| P0 | Learner, worker, and oracle hashes are required distinct, but the V1 selection executable is omitted from that identity set. | The learner could also own selection while receipts imply separated roles. | Freeze the V1 selector executable SHA-256 and require learner, selector, worker, and oracle executable hashes to be pairwise distinct. |
| P0 | A single fixed fixture can be solved by hard-coded action IDs or known fixture paths without using support outcomes. | The test may demonstrate fixture recognition rather than effect induction. | Generate action IDs from an experiment-specific nonce owned by the integration harness, keep mapping assignment outside the learner binary, require action IDs to differ across two deterministic replay fixtures, and add an outcome-dependence control: replacing valid support post-manifests with pre-manifests must yield no laws. The claim remains fixed-path transfer, but cannot depend on fixed action hashes. |
| P0 | `K2SupportObservationV1.pre_manifest` is not tied precisely to the three manifests in the Law Lab receipt. | The redactor could pair a valid post-state with a different pre-state and manufacture a delta. | Define pre-manifest as exact `worker_outcome.pre_work_manifest`; require source, pre-work, post-work, request, and receipt cross-validation before redaction. Bind source-manifest root separately and reject any mismatch. |
| P0 | The learning journal lists event names but does not define the event digest, chain, projector, or cross-event replay obligations. | Canonical files can still be rebound across experiments or projected inconsistently after restart. | Freeze a typed event envelope over schema, experiment ID, sequence, kind, payload root, previous entry root, and entry root; define one deterministic projector and exact cross-event identity replay. Terminal outcome and post-event seal must remain acyclic. |
| P1 | Learner byte budgets exist, but wall time, CPU time, process count, and memory limits do not. | A malformed or looping learner can consume unbounded test resources. | Freeze exact learner process limits and enforce them in one process runner; cap stdin, stdout, stderr, wall time, CPU, address space, and process count. |
| P1 | The allowed grammar root commits only to two effect variants, but candidate enumeration and ambiguity counts are not represented in the output. | A learner can announce a unique law without showing the bounded version space it searched. | Add per-action candidate roots/counts, rejection counts by reason, and `version_space_size`; PASS requires complete enumeration and exactly one survivor. Keep candidates bounded to 32. |
| P1 | The target holdout conditions are prose only and lack a typed independence receipt. | A target alias or partially reused content can slip through while the terminal receipt claims independence. | Add `K2TargetIndependenceReceiptV1` over support and target manifests with exact pairwise root/content/length/topology checks, durable before target prediction. |
| P1 | The ablation contract does not freeze exact terminal reason for every mutation, and the constant-output mutation breaks receipt binding before learning. | Tests can accept whichever failure happens and stop checking the intended causal edge. | Define an exact expected verdict per ablation. Separate transport/evidence-integrity controls from learner-identification controls; for a valid-but-adversarial observation set, reseal copied observations under a dedicated ablation provenance, never reuse production-like evidence. |
| P1 | Main execution budgets omit the extra wrong-action oracle control and all ablation learner invocations. | The experiment can exceed its stated probe/process denominator while still claiming budget compliance. | Add separate fixed ablation budgets and report main and ablation executions separately. |
| P1 | Test-local fixture cleanup and retained journal ownership are not part of the terminal contract. | Repeated capability runs can accumulate evidence/workspaces despite bounded per-episode bytes. | Require cleanup receipts for all disposable workspaces and test roots; retain only explicitly requested evidence roots and verify all temporary paths absent. |
| P1 | The target prediction is validated by applying a learned effect, but the draft does not require exact preservation of all unaffected manifest entries in the target verifier. | A prediction can reach the goal root through an incomplete or overbroad tree mutation. | The independent target verifier must compare the complete sorted entry vector and total bytes, preserving every unaffected entry exactly. |
| P2 | "Two deterministic replay fixtures" and experiment-specific IDs can conflict if the nonce source is implicit. | Different implementations can choose incomparable identity material. | Derive nonce from an explicit harness-provided 32-byte commitment, record it only in the private experiment contract, and derive opaque IDs with domain-separated hashes. Replaying the same commitment is byte-identical; the second control uses a separately frozen commitment. |
| P2 | The status list mixes scientific negatives with infrastructure failures. | Reporting can imply a learning failure when the runner or journal failed. | Keep `LEARNING_NEGATIVE`, `INFRASTRUCTURE_FAILURE`, and `INDETERMINATE_AFTER_DISPATCH` terminal classes separate in projections and final reporting. |

## 3. Strongest Alternatives Considered

### 3.1 Directly author the two effects

This is V1 again. It verifies execution and goal comparison but contributes no
evidence that an effect was inferred from observations.

### 3.2 Learn from natural LLM traffic now

Natural traffic currently exposes no exact pre-action goal surface and only one
genuine K1 law. Forcing this experiment into ordinary traffic would either wait
without an intervention surface or infer goals retrospectively. Both routes
fail to answer the intended question.

### 3.3 Build a general program synthesizer first

That creates a much larger version space before the provenance and holdout
route is proven. The bounded two-effect language is a better first learned
capability test because every candidate and delta can be exhaustively verified.

### 3.4 Use an ML model for the first learner

With six observations, a learned neural model would add optimization and
stochasticity without adding evidential value. A deterministic bounded version
space is the correct baseline. A later Wave or latent learner must beat this
same frozen task without receiving stronger inputs.

## 4. Required Repair Order

```text
1. split learner-public context from private experiment commitments
2. narrow the temporal no-goal claim to serialized process inputs
3. add independent effect/prediction verification
4. make hidden mapping bytes durable and restart-verifiable
5. separate all four executable identities
6. eliminate fixed-ID and no-observation shortcuts
7. bind exact Law Lab pre/post provenance before redaction
8. define typed journal events, projector, and acyclic terminal seal
9. add process, ablation, holdout, and cleanup budgets
10. freeze exact ablation verdicts
11. run three split structural gates
12. run implementation preflight
```

No Rust source edit is allowed until every P0/P1 repair appears in the
preregistration, all split gates return PASS, and implementation preflight
returns `READY_TO_IMPLEMENT` with `safe_to_implement=true`.

## 5. Review Claim Boundary

This review proves only that the initial paper route has concrete repairable
gaps. It does not prove that the repaired contract is structurally coherent,
that the implementation will be correct, that the learner is intelligent, or
that any K1/K2/product claim may be promoted.

## 6. Structural Gate Extraction Obligations

The following one-to-one obligations restate the review findings only for
unambiguous structural evidence binding:

| ID | Required relation |
|---|---|
| SG-P1 | Learner-visible protocol excludes every private freeze, mapping, holdout, and goal commitment. |
| SG-P2 | Private action mapping is an immutable canonical artifact validated across restart. |
| SG-P3 | Opaque action identity varies independently from hidden operation mapping and learner binary. |
| SG-P4 | Support redaction binds exact source, pre-work, post-work, request, and receipt identities. |
| SG-P5 | Learner completely enumerates the bounded two-effect version space. |
| SG-P6 | Independent verifier checks laws and complete predictions without learner inference calls. |
| SG-P7 | Outcome-dependence control removes post-action deltas while preserving opaque IDs. |
| SG-H1 | Exact support worlds, actions, and six-probe schedule freeze before support execution. |
| SG-H2 | Learned law set is durable before the target manifest enters prediction. |
| SG-H3 | Holdout checker emits a typed exact target-independence receipt. |
| SG-H4 | Target predictor input excludes goal, expected output, mapping, and selector evidence. |
| SG-H5 | Target predictions are durable before V1 goal selection and target execution. |
| SG-H6 | Every support dispatch is durable before its sandbox process starts. |
| SG-H7 | A dispatched probe without observation cannot rerun under the same identity. |
| SG-H8 | One deterministic projector validates the typed chain and every cross-event identity. |
