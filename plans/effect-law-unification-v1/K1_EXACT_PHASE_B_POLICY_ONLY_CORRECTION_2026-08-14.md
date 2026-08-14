# K1 Exact Phase B Policy-Only Correction

Date: 2026-08-14

## Authority Baseline

Phase A passed and remains the only rollback target:

```text
source commit       45190058225018e23ed34737dddbceed9965fd41
Phase A receipt     bffecf0093533249cd77e75433b0e290092a649dfe31a4010ea0f2229fbe34e1
writer              OFF
V8 freeze events    0
```

The first Phase B transaction rolled back to Phase A:

```text
attempt receipt     /var/lib/nando-wave/deployments/20260814T071912Z-4519005-phase-b/phase-b-abort-receipt.json
failure             fresh scheduler report did not become available within 600 seconds
V8 freeze events    0
rollback            OFF policy restored
```

This was an operational deployment-harness failure, not a scheduler verdict and
not evidence for or against Law #2.

## Diagnosis

The first transaction restarted cold learner, proof authority, and control.
The authority binds its Unix socket before `recover_authority` completes, so a
systemd `active` state and a listening socket do not prove that the accept loop
is ready. Production replay exceeded the frozen 600-second observation.

The restarts were unnecessary:

- `exact_wake_authoritative` reads and validates the policy document on every
  exact wake;
- health reads the same policy document for every health projection;
- Phase A and Phase B use the same V8-compatible binary bytes;
- the execution plan permits cold, authority, and control to restart, but does
  not require a restart.

## Corrected One-Shot Transaction

The corrected Phase B transaction is policy-only.

```text
stable Phase A readers
-> wait for fresh writer_inactive runtime report
-> freeze PIDs, ledger prefix, active legacy freeze, and connector identity
-> atomically install the exact ON policy
-> restart no service
-> wait for a newer authority-owned exact wake status
-> verify writer is not inactive
-> verify immutable legacy disposition and bounded V8 suffix
-> run composite gate
-> commit Phase B receipt
```

Frozen bounds:

```text
pre-mutation readiness wait      1200 seconds
post-policy exact wake wait       600 seconds
maximum V8 candidate freezes        1
maximum service restarts             0
synthetic requests                    0
```

The new exact wake status must be newer than the Phase A report and its
decision must be one of:

```text
active_generation
waiting_for_evidence
waiting_for_novel_evidence
research_budget_cooldown
candidate_frozen
k1_vocabulary_open
```

`writer_inactive`, missing status, stale report, policy-root mismatch, modified
legacy prefix, unbound legacy generation loss, more than one new V8 freeze,
protected PID change, false accept, parity failure, or composite failure aborts
the transaction.

## Rollback

On failure after mutation, atomically restore the exact Phase A OFF policy.
Do not restart any service. Verify the health projection returns to `OFF` and
write one abort receipt. Do not loop or repeat this corrected transaction.

If a natural V8 event was appended before rollback, the installed readers stay
at Phase A-compatible bytes. A pre-Phase-A binary is never restored.

## Claim Boundary

Phase B PASS proves only that the exact writer is safely enabled over ordinary
captured evidence. It does not prove Law #2, answer quality, independent future,
BundleV4 admission, CPU execution, economics, cleanup, or certification.
