# STOP-F5-F Phase Integration

Status: `SAFETY_PASS / WAVE_GAIN_WATCH / F5_G_UNLOCKED`

Implementation commit:

```text
e887349ab34a41ed7dd70173fca255862d22ec19
```

Authority: `false`

## Result

```text
complete structural mapping set
-> typed fixed-point relation phase trace
-> capability-bound action attempts
-> five independent control projections
-> action-class ranking
-> unique existing action or ABSTAIN
```

The binder now preserves the observed and expected complex components that
already produced `phase_fit_fixed`. F5-F does not infer a second score from the
aggregate. Each trace is type-bound, range-checked, and committed by SHA-256 in
the phase report.

The controls are:

```text
full phase
no phase
shuffled observed phase
magnitude-only observed phase
matched deterministic random center
```

All controls consume the same complete structural attempt set. Phase is never
invoked when F5-C/F5-D reports an incomplete binding, missing capability, or
ambiguous action. It therefore cannot repair or bypass structural evidence.

## Honest Gain Boundary

The current F5 relation anchors are neutral and the complete heldout action set
contains one action class. Consequently:

```text
full exact action checks                         1
no-phase exact action checks                     1
measured search reduction                        0
gain verdict                                     WATCH_NO_SEARCH_GAIN
```

This is not a failure of runtime convergence. It means the current evidence
proves safe phase plumbing and causal controls, but does not prove that Wave
reduces search or ambiguity. No Wave performance or applicability claim is
made at STOP-F5-F.

## STOP Matrix

```text
real phase trace recomputes binder score          PASS
phase trace committed in report                   PASS
Wave rescues failed structural binding            0
full-phase wrong actions                          0
all-control wrong actions                         0
distinct-action tie                               ABSTAIN
missing capability under every control            ABSTAIN
ambiguous structural action under every control   ABSTAIN
action changes from structural result             0
full phase search/applicability gain               WATCH
production callers                                0
execution authority                               false
```

## Budgets And Ownership

```text
phase components per mapping                      bounded by relation program
mapping evaluations                               <= 2048
floating runtime randomness                       0
raw request values in phase receipt               0
largest new production module                     215 lines
```

`RuntimeRelationPhaseComponent` is the immutable cross-crate value contract.
`phase_ranking_v3` owns control projection and reporting. It does not own role
binding, action construction, VM execution, verification, persistence, or
admission.

## Verification

Local:

```text
focused F5-F                                      6 PASS / 0 FAIL
runtime role binder                               3 PASS / 0 FAIL
nando-operator-runtime                           39 PASS / 0 FAIL
Clippy -D warnings                                PASS
rustfmt / diff check                              PASS
```

Remote clean detached worktree:

```text
host                     e@192.168.3.94
worktree                 /home/e/projects/nando-wave-f5f-e887349
HEAD                     e887349ab34a41ed7dd70173fca255862d22ec19
target                   /home/e/build/nando-wave-f5f-target
incremental              disabled
nando-core lib           176 PASS / 5 ignored
nando-operator-kernel     13 PASS / 0 FAIL
nando-operator-learning  198 PASS / 0 FAIL
nando-operator-runtime    39 PASS / 0 FAIL
all integration targets  PASS
Clippy -D warnings       PASS
```

Graphify after implementation:

```text
nodes / edges / communities   26,291 / 58,977 / 1,214
```

Live composite gate remained fail-closed:

```text
verdict                         PASS
eligible_for_local_accept       false
response ACTIVE packages        0
response M3                     WATCH
response false accepts          0
response parity mismatches      0
```

No deployment, service restart, registry write, or authority change occurred.
Both user services remained inactive with `NRestarts=0`.

## Next Boundary

Only F5-G is unlocked:

```text
frozen ordinary traffic envelope
-> bounded non-blocking shadow queue
-> generation-pinned F5 attempt
-> exactly one terminal shadow verdict
-> latency, RSS, overload, and projection controls
```

F6 remains locked.
