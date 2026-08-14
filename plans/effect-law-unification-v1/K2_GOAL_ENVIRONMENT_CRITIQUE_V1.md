# K2 Goal-Bearing Law Lab Environment Critical Review V1

Status: `ADVERSARIAL REVIEW COMPLETE / REPAIR REQUIRED BEFORE GATE`

Date: `2026-08-14`

Reviewed artifact:
`K2_GOAL_ENVIRONMENT_PREREGISTRATION_V1.md` in its initial
`DRAFT FOR ADVERSARIAL REVIEW` state.

This review is preserved separately. The findings are not erased when the
canonical preregistration is repaired.

## 1. Verdict

The route is scientifically appropriate: production S1C-4 remains terminal,
the existing sandbox is reused, goals precede actions, laboratory and natural
evidence remain disjoint, and all new authority is false.

The initial draft is not yet safe to implement. It defines the right owners but
leaves several byte, staleness, aliasing, crash, and oracle-independence edges
implicit. Those gaps could produce a convincing capability demo whose receipts
do not prove the temporal and identity claims printed beside it.

```text
route choice                         ACCEPT
scientific claim boundary            ACCEPT
implementation authority             DENY UNTIL REPAIRED
deployment authority                 false
```

## 2. Findings

| Priority | Finding | Failure if ignored | Required repair |
|---|---|---|---|
| P0 | The K1 registry root is captured, but no atomic staleness check is required immediately before decision freeze. | Ranking can use a snapshot whose member was revoked or whose registry revision changed before the pre-action contract became durable. | Re-read the externally owned registry projection under the freeze boundary and require exact revision/root equality. A mismatch terminalizes `STALE_BEFORE_FREEZE` without execution. |
| P0 | Unique action roots are treated as meaningful alternatives. | Two aliases of the same law/effect can manufacture a two-choice denominator. | Require pairwise-distinct LawCertificate roots, semantic-class roots, effect-contract roots, and predicted consequence roots in certificate-bound mode; require pairwise-distinct fixture effect roots in capability mode. |
| P0 | The existing Law Lab request is named in the probe plan, but there is no exact adapter-binding receipt between K2 roots and the V1 sandbox request. | A valid receipt from another candidate, goal, or episode can be replayed as this decision's probe. | Add a canonical `K2LawLabBindingV1` over episode, goal, vocabulary, alternatives, prediction set, selected action, request root, source tree, worker, seed, and budget roots. The evaluator accepts only a receipt matching that request. |
| P0 | `PROBE_PLANNED -> PROBE_EXECUTED` has no durable dispatch marker. | A crash after the external process acted but before its receipt was appended looks identical to a crash before execution; an automatic retry can double-run the scientific probe. | Append and sync `PROBE_DISPATCHED` before process creation. On restart, dispatched without a valid exact receipt is terminal `INDETERMINATE_AFTER_CRASH` and is never rerun under the same episode. |
| P0 | Oracle independence is stated as a field-level rule but not bound to executable identity. | Selector and oracle can accidentally share implementation or predictions while receipts still claim independence. | Freeze an oracle manifest root and executable SHA-256 distinct from selector and sandbox worker identities. Oracle input is exactly goal predicate plus validated terminal manifest; add negative tests rejecting extra selector evidence. |
| P0 | Capability and certificate-bound schemas share broad terminal names such as `GOAL_SATISFIED`. | A laboratory fixture result can be presented as scientific K2 evidence by dropping its provenance column. | Prefix terminal claims by scope: `CAPABILITY_PASS`, `LAB_GOAL_SATISFIED`, and failures. Every receipt and projection repeats provenance and a constant all-false authority block. |
| P0 | Canonical JSON Lines plus append/fsync does not define torn-tail recovery. | A crash can leave a partial final row; silently truncating it rewrites evidence, while accepting it breaks replay. | Store one canonical event per immutable ordinal file using temp file, file sync, no-replace publication, and directory sync. Never rewrite or truncate a published event. |
| P1 | Journal growth is described as append-only but has no exact byte/event ceiling. | Capability tests can become a new unbounded durable store. | Freeze at most 16 events per episode, 64 KiB canonical bytes per event, 1 MiB per episode, and 64 concurrent retained capability episodes; exceeding any limit is terminal/fail-closed. |
| P1 | Predictions bind an evidence root but not predictor identity or provenance. | Human-authored or outcome-informed predictions can masquerade as system selection. | Bind predictor schema, executable/contract root, provenance, input roots, and creation sequence; forbid outcome, oracle receipt, and post-action manifest roots before prediction precommit. |
| P1 | Applicability witnesses are not explicitly bound to the exact initial environment root. | An action valid in a different state can appear as an available alternative. | Every alternative binds the shared initial environment root and a validated applicability receipt root; mismatched environment roots reject the complete set. |
| P1 | The initial draft allows certificate-bound execution in the same implementation slice despite K1 being 1/3. | Untested future authority paths are shipped under a fixture-only result. | V1 source slice validates certificate-bound contracts but keeps their executor closed with `INSUFFICIENT_K1_VOCABULARY` or `CERTIFICATE_BOUND_RUNTIME_CLOSED`. Only generated capability self-test executes. |
| P1 | The exact selector is called a prepared baseline, but its ownership is not separated from future learned K2 selection. | A deterministic fixture selector can later be mislabeled as the meaning model. | Name it `PreparedCapabilitySelectorV1`, freeze `learned=false`, and forbid its receipts from K2 compression/meaning datasets. |
| P1 | The oracle checks the terminal tree but no independent expected-goal artifact snapshot is pinned. | The expected root can be rewritten alongside the fixture after seeing the action. | Goal creation binds an immutable expected-tree manifest root from a read-only goal store; decision freeze verifies it before any prediction. |
| P1 | Production preservation is an out-of-scope statement rather than a measured preflight obligation. | A shared path or inherited adapter setting can still touch production. | Preflight must pin source bytes, existing Law Lab schemas, K1 generation 606 projection, and forbidden paths; source checks veto network, production mounts/writes, natural-ledger paths, service controls, and deployment scripts. |
| P2 | Timestamps participate in receipts without a monotonic ordering rule. | Wall-clock adjustment can make a later event appear pre-action. | Authority comes from journal ordinal and durable publication order, never timestamp ordering. Timestamps are descriptive and must be nondecreasing only when the clock permits. |
| P2 | The draft says "exactly one terminal receipt" but does not say who derives the restart projection. | Two readers can disagree over legal state after a crash. | Define one pure deterministic projector over validated ordinal events; all writers and tests use it before append. |

## 3. Strongest Alternative Considered

The simpler alternative is to wait for ordinary traffic to expose typed goals.
That route was already tested on a frozen denominator and ended
`EMPTY_GOAL_SURFACE`. Waiting is still useful for K1 natural-law discovery, but
it is not an experiment on goal-conditioned choice.

Another alternative is to inject goals into ordinary LLM traffic. That would
erase the natural/laboratory boundary and make the result uninterpretable.

The separate Law Lab environment remains the best route because it permits
safe interventions and exact outcomes while leaving production evidence
untouched. Its limitation must remain explicit: it can validate the experiment
substrate now, but meaningful K2 research waits for genuine K1 alternatives.

## 4. Required Repair Order

```text
1. close registry staleness and meaningful-alternative identity
2. add exact K2-to-sandbox binding
3. add durable pre-dispatch state and fail-closed crash semantics
4. make oracle executable identity independently pinned
5. replace JSONL with immutable ordinal event files
6. freeze predictor identity, storage budgets, and deterministic projection
7. keep certificate-bound execution closed in the first source slice
8. bind production preservation in implementation preflight
9. run split evidence, execution, and claim-boundary structural gates
```

No code should be written until the repaired preregistration passes all split
gates and the implementation preflight returns `READY_TO_IMPLEMENT`.

## 5. Review Claim Boundary

This critique establishes only that the initial paper route has identifiable
repairs. It does not validate the repaired bytes, authorize implementation,
prove sandbox safety, or grant any K1/K2 claim.

## 6. Pre-Implementation Conformance Finding

After the first repaired paper gate and before any Rust edit, type-level design
exposed one additional P0:

| Priority | Finding | Failure if ignored | Applied repair |
|---|---|---|---|
| P0 | `K2DecisionEpisodeReceiptV1` was required to contain the terminal journal-entry root while that journal entry's payload root would be the same receipt root. | The two SHA-256 values form an uncomputable cycle; an implementation would either omit one promised binding or hash mutable placeholders. | Split the terminal object into a pre-event `K2DecisionOutcomeReceiptV1`, embedded as the terminal event payload, and a post-event deterministic `K2DecisionEpisodeSealV1` that binds outcome, terminal event, and final projection roots. The seal is derived, never fed back into the journal. |
| P0 | A manifest field alone could claim that the exact oracle had an executable identity distinct from selector and worker while evaluation still happened in-process. | The independence receipt would describe an executable that never ran. | Add one isolated exact-oracle binary with a narrow canonical request/outcome protocol. The caller hashes that real binary, freezes the manifest before selection, executes it without selector fields, and verifies its exact response. |

The structural execution packet and implementation preflight must be rerun
after this repair. This finding changes no evidence or authority boundary.
