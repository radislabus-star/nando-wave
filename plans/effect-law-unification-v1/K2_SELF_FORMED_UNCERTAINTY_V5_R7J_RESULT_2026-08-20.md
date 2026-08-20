# K2 Self-Formed Uncertainty V5 R7J Result

Status: `R7J COMPONENT PASS / R7K NOT EXECUTED`

Date: `2026-08-20`

Base commit: `178614589696a06c8f42eb6b6a393c6df488ca0a`

Contract: `K2_SELF_FORMED_UNCERTAINTY_V5_R7J_CONTRACT_V2.md`

Preflight root:
`34369fe7387bb7efa63a4459e7ccfd0349bdec3c50c317a8da3c993cc86a923e`

## 1. Verdict

R7J closes the independent evaluation component boundary:

```text
manifested case evidence
-> independent frontier reconstruction
-> exact n^2 oracle
-> frozen-baseline comparison
-> process-evidence control evaluator
-> mode-separated terminal evaluator
```

It does not execute R7K K1-K12, create or consume a Confirm nonce, claim an
authorization slot, run a sealed attempt, persist a scientific terminal PASS,
touch production, mutate K1, or prove Natural K2.

## 2. Implemented Owners

Three pairwise-distinct, self-hash-bound executables were added:

```text
nando-k2-self-formed-oracle-baseline
nando-k2-self-formed-control-evaluator
nando-k2-self-formed-terminal-evaluator
```

The oracle receives a compact descriptor and one closed read-only evidence
tree. It reconstructs all raw dispositions, the semantic quotient and the
canonical representatives. It derives private outcomes by applying typed
effects and enumerates all one- and two-probe plans as exactly `n^2` streaming
rows. It does not import selector, closure-planner, final-verifier, control or
terminal decision helpers.

The control evaluator accepts only exact process-outcome rows for the frozen
`32 / 4 / 16 / 12` denominators. The terminal evaluator has separate tagged
development and sealed request schemas and applies the frozen disposition
precedence.

## 3. Evaluation Result

```text
cases                                             16 / 16
raw frontier reconstruction                  1,792 / case
model-guided observation parity                  16 / 16
true class retained                              16 / 16
one-probe frozen plans                             8 / 8
two-probe frozen plans                             8 / 8
oracle denominator                                  n^2 exact
oracle equality                                  16 / 16
baseline aggregate predicates                      4 / 4
baseline 12/16 thresholds                          4 / 4
control substitution negatives                        PASS
terminal cross-mode precedence                        PASS
false accepts                                             0
sealed attempts                                         0 / 1
production, network, K1 and dashboard effects               0
```

The positive terminal result used by the integration test exists only as an
in-memory authority-denied fixture. No `DEVELOPMENT_REHEARSAL_PASS` or
scientific PASS receipt was persisted.

## 4. Defects Found During Evaluation

The first full run found two implementation defects before an oracle result:

1. `public-bindings.json` nested the complete sixteen-case precommit and
   exceeded the frozen 1 MiB transport limit. The binding was replaced with a
   compact preregistration projection while the full case bodies remained in
   separately manifested evidence files. The limit was not raised.
2. Raw prediction witnesses are keyed by class-level world-model roots, while
   the independent evaluator initially searched by syntactic-model roots. The
   evaluator now independently applies each frozen world model for frontier
   parity, then separately uses the complete syntactic partition for true-class
   elimination.

Both repairs were rerun against the retained sixteen-case fixture. The final
full R7J suite passed.

## 5. Verification

All heavy commands ran on the mini-PC with `--jobs 20` in an isolated checkout.

```text
R7J integration                                2 / 2 PASS
R7G generator regressions                      3 / 3 PASS
R7H custody/restart regressions                 9 / 9 PASS
R7I public/private regressions                  2 / 2 PASS
V4 real-owner process regression               1 / 1 PASS
V4 focused controls                            2 / 2 PASS
cargo clippy --all-targets -D warnings                PASS
cargo fmt / git diff --check                         PASS
post-implementation code-route gate                  PASS
```

One additional old combined R7 control test remains VETO:

```text
r7_exact_negative_controls_and_v3_shortcut_controls_pass
-> reject_development_evidence_in_confirm_packet
-> boundary control accepted
```

The same failure reproduces byte-for-byte on clean base `1786145`, where no
R7J files exist. R7J changed none of the vocabulary, support or public-case
validation owners. This is recorded as pre-existing baseline debt, not hidden
and not counted as an R7J regression. The focused V4 controls required by the
R7J route pass.

## 6. Next Authority Boundary

The next permitted stage is R7K: execute the real K1-K12 process controls,
including cleanup, and provide their exact process receipts to the evaluator.
R8B, R9B, R10B and R11B remain locked. R7J alone does not authorize a sealed
attempt or a Natural K2 claim.
