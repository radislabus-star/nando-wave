# S1C-3E Journal Ownership Repair Critique V1

Status: `ADVERSARIAL REVIEW PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Reviewed artifact: `S1C3E_JOURNAL_OWNERSHIP_REPAIR_PREREGISTRATION_V1.md`

## Findings And Repairs

| Priority | Finding | Risk | Applied repair |
|---|---|---|---|
| P0 | The initial postmortem called the missing journal a valid lazy-empty state. | The repair would bless a runtime that never opened its writer. | Source and boot-log inspection proved `FramedCborLedger::open_with_limits` creates the directory and files immediately; S1C-3E requires exact runtime-created empty segments. |
| P0 | Root could create the files itself and manufacture readiness. | A filesystem fixture would impersonate the runtime writer. | Root may create only the empty final directory. The directory is checked empty before start; only the `e` process may create the three segment files. |
| P0 | Provisioning before rollback is armed would leave an untracked production mutation. | Interruption could strand a writable directory outside the transaction. | Durable `ROLLBACK_ARMED` precedes mkdir, chown, chmod, fsync, install, and restart. |
| P0 | Repeating S1C-3D measurements could spend time and select a luckier latency sample. | Optional stopping could hide the frozen optimization WATCH. | Exact parent resource/parity roots and candidate artifacts are inherited; measurements are not rerun or reclassified. |
| P0 | Empty segment files could be called scientific evidence. | Infrastructure startup would become a fake decision denominator. | Zero-byte startup segments open only S1C-4 `COLLECTING`; scientific authority, K2, training, and phase mutation remain false. |
| P0 | Rollback could delete a natural row that arrived during the forward window. | Correct failback would corrupt future evidence. | Cleanup removes only the exact three expected zero-byte files; any append or extra entry is preserved and reported. |
| P1 | A permissive directory could let unrelated processes inject rows. | Journal provenance would be ambiguous. | Require final directory `e:e 0700` and segment files `e:e 0600`, no symlinks or foreign entries. |
| P1 | A clean health response could hide startup capture failure. | Serving would pass while writer authority remained absent. | Require exact file creation attribution, process env binding, and boot-scoped absence of the known startup error in addition to health. |
| P1 | Replacing the candidate with a newly built binary could mix the ownership fix with runtime code changes. | New behavior would invalidate inherited parity evidence. | Install the exact content-addressed S1C-3D candidate binary and config. |
| P1 | Dashboard work during installation could widen the mutable surface. | Capture repair and presentation changes would share one authority decision. | Dashboard projection is updated only after installation PASS and reads the sealed S1C-3E/S1C-4 receipts. |

## Rejected Alternatives

```text
chmod the parent grounded-meaning-v1 directory
  rejected: widens write authority beyond the journal owner

create a synthetic pre-action goal to force an append
  rejected: manufactures scientific evidence

treat absence of the journal as valid readiness
  rejected: contradicted by runtime source and the PermissionDenied boot log

rerun S1C-3D until latency is below 5 ms
  rejected: optional stopping and unnecessary token/compute spend

modify the Rust writer to use lazy files
  rejected: changes the proven candidate instead of repairing deployment ownership
```

## Verdict

The repair is technically narrow and scientifically non-authoritative:

```text
root provisions ownership boundary
runtime proves writer open by creating exact empty segments
independent verifier binds inherited candidate and parent proof
S1C-4 starts after the empty cursor
natural traffic alone may add rows
```

Implementation may proceed only with the frozen identities, fail-closed
transaction, natural-row-preserving rollback, and one-attempt rule.
