# S1C Transactional Deployment Preregistration V1 Critique

Status: `ADVERSARIAL REVIEW / ACCEPTED REPAIRS IN FINAL CONTRACT / NO DEPLOYMENT`

Date: `2026-08-11 Europe/Tallinn`

Reviewed artifact: `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V1.md`

## 1. Review Scope

This review tries to falsify S1C-3 before any production mutation. It attacks
binary/source identity, two-file installation, restart attribution, rollback,
journal preservation, resource denominators, false live claims, and the
boundary between operational capture and natural decision evidence.

It does not review S1C-4 evidence or claim that an exact pre-action goal exists.

## 2. Findings And Accepted Repairs

| Severity | Finding | Failure mode | Accepted repair in final contract |
|---|---|---|---|
| P0 | Building paper HEAD could include proof files or later source drift. | The deployed binary would not be the accepted S1C-2 candidate. | Build only clean detached `a3ea27a`; bind source tree, lockfile, toolchain, candidate binary hash, and size before mutation. |
| P0 | The existing generic deployment receipt does not bind all S1C-3 facts. | A receipt could pass without config-pair identity, `NRestarts`, journal preservation, resources, or survival. | Require `nando.s1c3-transaction-preparation.v1` and a dedicated final receipt; generic V1 is supporting evidence only. |
| P0 | Binary and config cannot be renamed atomically as one filesystem object. | A crash or automatic restart could expose a torn old/new pair. | Stop only transition-serving before the two-file swap, arm rollback first, fsync both temporary and final files, and recover by restoring the old pair rather than resuming mid-transaction. |
| P0 | A backup could name an old commit but contain different bytes. | Rollback would restore an unproven runtime. | Bind the current deployed receipt, source commit, old binary/config bytes, metadata, and rollback manifest before stop; verify restored bytes before restart. |
| P0 | Rollback could delete the new journal to make state look clean. | Forward evidence and failure evidence would be destroyed. | Preserve every file, byte prefix, and nondecreasing size; rollback changes only binary/config and disables capture. |
| P0 | Feature env values could be present while runtime journal open failed. | Deployment would claim capture active although `grounded_decision_shadow` is absent. | Require process-env proof, boot-scoped log inspection, successful journal recovery, directory/root receipt, and no grounded-decision startup error. |
| P0 | Post-start health alone could hide a serving regression. | HTTP stays 200 while bytes, fallback, package decision, false accepts, or parity drift. | Preserve byte/decision parity, CPU/admission/package identity, false accepts zero, parity zero, and health roots before/after/survival. |
| P0 | A failed resource observation could be called noisy and rerun. | Optional stopping would select a passing deployment. | Exactly three predeployment runs, one build, no retry; missing comparability is `INVALID_ENVIRONMENT`, any absolute breach is VETO. |
| P1 | Manual restart could be confused with a crash restart. | An extra process failure would disappear inside the expected PID change. | Require exactly one old-to-new PID transition, unchanged `NRestarts`, boot timestamps, and a 15-second repeated snapshot. |
| P1 | Other services could restart while transition-serving still recovers. | A wider deployment would masquerade as the scoped slice. | Freeze untouched service PIDs, restart counts, unit/config hashes, and connector identity across every checkpoint. |
| P1 | A moving active journal file could be called a scientific root. | Torn filesystem observation would acquire K2 evidence authority. | Scope the tree root to operational deployment evidence only; S1C-4 owns durable episode roots and census authority. |
| P1 | Old and candidate RSS windows could use different traffic or warmup. | A meaningless delta could pass the 16 MiB gate. | Use isolated same-fixture baseline/candidate schedules; incomparable inputs are invalid, while live RSS remains an additional post-start ceiling. |
| P1 | Continuous ordinary traffic makes a live idle-CPU interval impossible. | The 0.25% gate could be silently omitted or measured under load. | Run the frozen 60-second idle gate in the isolated predeployment process with unchanged inputs; do not relabel live loaded CPU as idle. |
| P1 | Config drift in `phase-center.env` or `authority.env` could be blamed on S1C-3. | Authority or serving changes would have ambiguous ownership. | Hash and require both files unchanged before, during, and after the transaction; change is VETO. |
| P1 | Normal path/timer activity could be stopped for a cleaner snapshot. | The experiment would alter unrelated production behavior. | Observe but never control response-admission and live-transition path/timer units. |
| P1 | Zero goals after deployment could be treated as deployment failure or hidden. | S1C-3 and S1C-4 would be merged, encouraging targeted traffic or false claims. | Empty journal and `MISSING_EXACT_GOAL` remain valid S1C-3 states; only the bounded S1C-4 census classifies the natural surface. |
| P2 | Dashboard text could imply K2 collection immediately after restart. | Operational installation would become a public scientific claim. | No dashboard edit or K2 wording in S1C-3; the immutable deployment receipt is the only new status artifact. |
| P2 | The paper-time baseline may drift before execution. | Stale hashes and PIDs would be treated as current. | Revalidate every baseline value atomically; mismatch returns `STALE_BEFORE_DEPLOYMENT` before mutation. |

## 3. Critical Architecture Decision

The safest transaction is a bounded stop, verified pair swap, and start of the
single owner. Keeping the old process running during the two renames appears
less disruptive but leaves a host-crash interval in which systemd could restart
against a torn pair. This review rejects that route.

The intentional interruption is attributed explicitly:

```text
transition-serving       one old PID -> one new PID
NRestarts                unchanged
all other services       same PID and restart count
connector                same process identity
```

The operational gain is capture availability only. Scientific authority stays
false even if the journal immediately receives records.

## 4. Remaining Limitations

The paper contract cannot make the two filesystem renames a single kernel
transaction. Stopping the sole reader and arming byte-exact rollback makes a
torn on-disk state fail-closed and recoverable.

The post-start filesystem root is not a coherent scientific snapshot while an
active segment may append. It proves which bytes the deployment observer read,
not a settled decision episode. The append-cursor census remains S1C-4 work.

A 15-second survival interval does not establish long-term reliability. It is
an installation gate paired with restart recovery tests and the later bounded
natural window.

Ordinary traffic may contain no exact typed goal. In that case S1C-3 can pass
while S1C-4 terminates `EMPTY_GOAL_SURFACE`. That is a valid falsification of
the traffic surface, not a reason to invent a goal.

## 5. Rejected Alternatives

```text
deploy directly from paper HEAD
  rejected: source identity includes non-candidate work

edit the live env file in place
  rejected: torn or partially written config

swap files while old service remains running
  rejected: host/process crash can restart against a mixed pair

trust generic deployment-receipt.v1 alone
  rejected: required S1C-3 fields are absent

declare capture active from environment only
  rejected: runtime journal open can fail closed

delete journal on rollback
  rejected: destroys forward and failure evidence

rerun a failed latency or resource sample
  rejected: optional stopping

wait for a goal before finalizing deployment
  rejected: merges S1C-3 installation with S1C-4 natural evidence

add a dashboard K2 badge
  rejected: no natural decision episode exists
```

## 6. Review Verdict

Every identified P0/P1 route has an explicit fail-closed repair in the final
preregistration. The contract is ready for structural verification, not for
deployment by itself.

```text
candidate identity                    READY FOR STRUCTURAL GATE
owner isolation                       READY FOR STRUCTURAL GATE
transaction chronology                READY FOR STRUCTURAL GATE
rollback and evidence preservation    READY FOR STRUCTURAL GATE
absolute resource route               READY FOR STRUCTURAL GATE
claim authority boundary              READY FOR STRUCTURAL GATE
runtime changed                        no
deployment allowed                    false until paper verification
authority_ready                       false
```

## 7. Exact Chronology Evidence

The immutable rollback pair is armed before transition-serving is stopped.

Transition-serving is fully stopped before either candidate file is swapped.

Both candidate temporary files are fsynced and hash-verified before rename.

The intended start yields one PID change with an unchanged `NRestarts` value.
