# K2 Self-Formed Uncertainty Adversarial Critique V1

Status: `REVIEW COMPLETE / V1 BLOCKED BEFORE CODE`

Date: `2026-08-15`

Target commit: `2a48eae813e785b3ea5f7b4d2124659223b4cbd5`

Target: `K2_SELF_FORMED_UNCERTAINTY_PREREGISTRATION_V1.md`

Authority: `FALSE`

## 1. Verdict

The successor question is the correct next bounded question, but V1 is not safe
to implement. It separates model-set formation from the already proved scorer
and forbids Natural K2 overclaiming, yet several contracts remain inferable,
underspecified, or unenforceable.

```text
scientific direction                    ACCEPT
change-one-mechanism boundary           ACCEPT
generated-only claim boundary           ACCEPT
implementation authority                BLOCKED
unresolved P0 findings                   7
unresolved P1 findings                  9
```

V1 remains immutable evidence. Repairs belong in V2.

## 2. P0 Findings

| ID | Finding | Why it can create a false PASS | Required V2 repair |
|---|---|---|---|
| P0-1 | Confirm seed mode `0600` is not independent custody. | The same OS user that writes code can read the seed before executable freeze and tune implementation to confirm cases. A path and file mode are procedure, not a trust boundary. | Discard the V1 confirm commitment for science. Generate the confirm nonce only after source, binaries, tests, thresholds, and V2 root are frozen. Publish its commitment before generator execution and record a seed-custody/access receipt. Do not claim external attestation. |
| P0-2 | Action count and expected model count both vary from three through six. | A shortcut can infer the expected version-space cardinality from the public action count without enumerating support-consistent models. | Freeze the same four action IDs and four path atoms in every confirm case. Vary semantic model cardinality independently. Add paired cases with the same public structural geometry and different private true classes. |
| P0-3 | True-model equality is defined by model identity, not semantic class. | The private generator mapping can be syntactically different from the learner's canonical representative while predicting every state identically. Exact root equality would reject a correct quotient or encourage generator-specific syntax. | Compare the private mapping's complete finite-domain semantic signature with the sole surviving class signature. Keep syntax and class identity separate. |
| P0-4 | Risk and cost are described but not mathematically frozen. | Selector and verifier can agree on a convenient post-hoc cost implementation, changing which probe wins while appearing conformant. | Freeze checked integer formulas, inputs, overflow behavior, robust worst-case aggregation, and golden vectors in V2. |
| P0-5 | A scorer adapter may change effective ranking inputs. | The predecessor source can remain byte-identical while the adapter changes partitions, safety, risk, cost, or tie order, so the scientific mechanism is no longer only model induction. | Require the new route to serialize the exact predecessor request schema and invoke the unchanged selector executable. Independently prove adapter parity over every generated probe and frozen golden vector. |
| P0-6 | The twelve-case attempt lacks a batch barrier before first execution. | Outcomes from early cases can influence model construction, probe enumeration, or selection in later cases even though each local precommit looks valid. | Freeze and independently verify support, model sets, probe sets, baseline rows, selections, and predictions for all confirm cases before any worker dispatch. |
| P0-7 | Robust safety assumes, but does not prove, that the private effect belongs to the frozen grammar. | If the generator's true effect falls outside the grammar, model-relative safety can be wrong before execution. | Add a separate private dispatch-safety owner that validates grammar membership and sandbox confinement after selection verification but before worker dispatch; it emits no bytes back to learner or selector. |

## 3. P1 Findings

| ID | Finding | Consequence | Required V2 repair |
|---|---|---|---|
| P1-1 | The single-model baseline is compared by survivor count. | An unsupported guess reports one survivor and can look as good as verified uncertainty closure. | Score guessed-class correctness separately from residual uncertainty. Never treat an unverified guess as a valid singleton posterior. |
| P1-2 | Exact negative-control denominator is open-ended. | Failed or omitted controls can disappear behind “at minimum.” | Freeze an exact control inventory and denominator before implementation. |
| P1-3 | Probe-state atoms are “derived from support” without an exact census algorithm. | A confirm-specific atom or preferred state can be inserted while still being called derived. | Freeze a public domain-vocabulary receipt before support generation and define complete finite-state enumeration from that receipt. No generator-selected probe basis is allowed. |
| P1-4 | Topology families prescribe expected shapes but no anti-template cases exist. | Code can recognize four prepared geometries instead of performing general bounded consistency and quotient. | Add same-size/different-partition and same-partition/different-cost paired cases, plus counterfactual development cases outside all four confirm geometries. |
| P1-5 | Generator independence is process-level only. | A distinct executable hash does not prove independent origin or prevent shared source assumptions. | State the narrow claim explicitly, forbid external-provenance wording, and require source-import separation plus request-byte exclusions. |
| P1-6 | Semantic quotient completeness depends on a potentially large cross-product but no resource terminal exists. | A timeout can be misreported as scientific failure or silently trigger approximate pruning. | Freeze raw-model, state, prediction, memory, wall-clock, and protocol ceilings. Resource exhaustion is a named terminal scientific result; approximation remains forbidden. |
| P1-7 | V1 mixes a large repository refactor into the successor branch. | A behavior regression in four monoliths can be confused with the new scientific mechanism. | Perform R0 on a separate preparation branch and merge only after parity receipts. The scientific branch starts from that exact merge and adds one mechanism. |
| P1-8 | The old VETO evidence repair is named but not bound to exact files and roots. | Auditors still see contradictory PASS/VETO evidence without a machine-checkable supersession relation. | V2 must name immutable VETO inputs, five replacement receipts, their hashes, and a non-authority supersession index schema. |
| P1-9 | The paper says all disposable paths disappear, but seed and failed-attempt custody artifacts are not classified. | Cleanup can erase attempt evidence or leave private material indefinitely. | Separate retained evidence, retained sealed material, disposable workspaces, and deletion timing. Emit a complete path census after terminal publication. |

## 4. Strongest Shortcut Attack

The easiest false implementation of V1 is:

```text
public action count
-> infer expected family and model count
-> emit a prepared list with that cardinality
-> adapter constructs predecessor-shaped singleton partitions
-> unchanged scorer selects the expected root
-> private generator uses matching syntax
-> reported PASS
```

That implementation can preserve the predecessor selector hash, execute a real
sandbox action, and produce an exact observation. It still would not show that
support transitions caused formation of the uncertainty set.

V2 must make this shortcut fail through orthogonal cardinalities, complete
witnesses, semantic signatures, support ablations, paired private mappings, and
independent re-enumeration.

## 5. Claim-Naming Critique

“Self-formed” is acceptable only in the following mechanical sense:

```text
the model set was not supplied as input;
it was constructed by a frozen complete consistency procedure from support
```

It does not mean the procedure itself was learned. It does not mean Nanda
invented the grammar, state atoms, causal objective, or scientific method.

The result name should therefore remain
`K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS`, never
`K2_LEARNED_SCIENTIFIC_METHOD_PASS` or `NATURAL_K2_PASS`.

## 6. Corrected Experimental Shape

The narrow repair is:

```text
one fixed public domain vocabulary per case
+ four opaque actions in every case
+ incomplete support transitions
-> exhaustive grammar enumeration
-> exact semantic quotient with 3..6 classes independent of action count
-> complete mechanical probe frontier
-> all-case batch precommit
-> unchanged predecessor selector executable
-> private grammar/sandbox safety check
-> one probe per case
-> independent observation and semantic-class elimination
```

Use sixteen confirm cases: four topology families, each represented by two
matched pairs. Within each pair, public uncertainty geometry is isomorphic but
the private true semantic class differs. This directly attacks fixture-identity
and private-answer leakage while preserving the bounded domain.

## 7. Exact Cost Repair

V2 should define probe cost from the initial state and a predicted effect as:

```text
touched_existing_entries = existing sources + existing destinations/removals
touched_bytes            = exact bytes read + overwritten + removed

risk_units = overwritten_existing_entries
           + removed_existing_entries
           + ceil((overwritten_bytes + removed_bytes) / 4096)

cost_units = 1
           + read_entries
           + written_or_removed_entries
           + ceil(touched_bytes / 4096)
```

The robust probe values are the maxima over the complete grammar effects that
can bind the selected opaque action, not just the current survivor models.
Arithmetic is checked; overflow is VETO. The exact treatment of missing files,
typed failures, and unchanged manifests must be included in golden vectors.

## 8. Attempt Chronology Repair

The confirm nonce should not exist during implementation. The valid chronology
is:

```text
V2 root frozen
-> source and tests frozen
-> all executable hashes frozen
-> development suite and structural gates PASS
-> confirm-read capability armed
-> random confirm nonce created and commitment fsynced
-> all sixteen public cases generated
-> all model/probe/selection precommits independently verified
-> batch barrier root fsynced
-> first worker may start
```

Any code, threshold, grammar, adapter, baseline, or PASS-rule change after nonce
commitment invalidates the preregistration. Any failure after nonce creation is
terminal for that commitment. No seed shopping or rerun is allowed.

## 9. Repository Preparation Repair

R0 is justified as debt cleanup but must not share a commit with model
induction. It requires:

```text
separate branch and worktree
exact 17df25f baseline
move-only/model-protocol-receipt-fixture ownership split
no public schema or behavior change
no sealed predecessor test execution
non-sealed tests + strict Clippy + source-layout parity
machine-readable superseded-evidence index
zero new git diff --check findings
separate merge commit before scientific implementation
```

The seven predecessor whitespace findings and historical VETO artifacts remain
byte-identical. Cleanup means making status explicit, not rewriting evidence.

## 10. V2 Admission Checklist

V2 is ready for structural gating only when it contains all of these:

```text
fixed action/path cardinalities independent of uncertainty cardinality
exact finite grammar and state universe
true semantic-class comparison
exact risk/cost formulas and golden cases
unchanged predecessor request schema and selector executable
sixteen cases with matched private-answer pairs
all-case batch barrier before any execution
post-freeze nonce creation and custody receipt
private grammar/sandbox safety owner
exact negative-control denominator
resource-exhaustion terminal
separate R0 preparation branch
exact superseded-evidence bindings
retained/disposable path classes
explicit null and conjunctive PASS
authority=false everywhere
```

Until those repairs are present, the correct status is
`BLOCKED_BEFORE_CODE`, not almost ready.
