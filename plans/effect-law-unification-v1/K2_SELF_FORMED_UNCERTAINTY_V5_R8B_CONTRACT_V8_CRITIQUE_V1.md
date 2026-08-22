# K2 Self-Formed Uncertainty V5 R8B Contract V8 Critique V1

Status: `CRITIQUE COMPLETE / V8 DRAFT VETO / REPAIR REQUIRED`

Date: `2026-08-21`

## Verdict

V8 closes the V7 cgroup, ledger-size, generic-validator and root-subset defects,
but the first draft is not ready for structural gates. Manual adversarial
review found three P0 and seven P1 defects.

## Findings

| Priority | Finding | Consequence | Required repair |
|---|---|---|---|
| P0 | Section 12 lists nineteen evidence objects and then lists the linked and suite manifests again, although both manifests are already two of the nineteen evidence kinds. It also omits the parent resource receipt and frozen C08 downstream invocation contract needed by M25. | The exact packet census is contradictory and M25 cannot independently compare observed invocation roots with the preregistered downstream schedule or resource denominator. | Freeze exactly 23 packet files: 19 evidence objects including both manifests, C08 contract, resource receipt, process ledger and packet manifest. Give every non-evidence object its own typed descriptor. |
| P0 | Sections 6 and 12 call `process-ledger.json` C08 authority, while chronology and V6 define C08 as the downstream invocation contract frozen before C09. | Expected schedule authority and observed process provenance are merged. A ledger could authorize its own cardinality. | Restore C08 as the immutable expected downstream contract. The ledger is observed provenance only. M25 must require exact C08-to-ledger equality. C08 remains outside the nineteen evidence kinds. |
| P0 | The JSONL contract requires fsync but does not require an exclusive append lock, complete-tail revalidation or fail-stop after an incomplete/unexpected invocation. | Concurrent writers can interleave valid lines or continue after a broken pair. The 7,168-byte pair bound would no longer prove the 128 MiB ledger bound. | Require one no-follow ledger lock, reopen and validate header plus terminal tail under lock, append exactly one canonical line, fsync, unlock. After an incomplete or non-success outcome, no later invocation may be requested. |
| P1 | The M24 child is declared to emit exactly three objects, but M25 also needs the C08 contract. The per-invocation authority descriptor limit is three. | C08 would be fabricated by the parent, omitted from P06 or hidden inside another evidence object. | Make the M24 child emit exactly four immutable objects and raise the bounded per-invocation authority-output descriptor limit to four. |
| P1 | S02 has sixteen total Nando invocations, but they are not all direct. The current suite has ten M01 invocations, three S02-owned setup M02 invocations and three M01-owned nested M02 invocations. N24 incorrectly requires every direct M02 to have an M01 writer pair. | Legal setup dispatches would be rejected, while the actual direct-vs-nested ownership split remains unproved. | Bind the exact S02 partition: `S02 -> 10 M01 + 3 setup M02`; `M01 -> 3 nested M02`. Reject an unjournaled direct setup M02 and reject a nested M02 not owned by its M01. |
| P1 | The tool manifest hashes `/usr/lib/systemd/systemd` on disk but does not bind the already-running user manager process image. | An upgraded or replaced path could differ from the bytes of the manager that physically launches the child. | Before submission, bind the user manager PID, `/proc/<pid>/exe` canonical target, inode/device and SHA-256 together with manager version and D-Bus identity. Recheck after child exit. |
| P1 | The exact `systemd-run` invocation is described semantically but not frozen as a command contract. | `--pipe`, `--wait`, `--collect`, an altered credential name or a different output route could silently change lifecycle and evidence custody. | Freeze exact argv/property order-independent set: no `--pipe`, `--wait` or `--collect`; fixed `--user`, unit, `Type=exec`, `RemainAfterExit`, credential, stdout/stderr and resource/security properties. Add command-normalization parity and forbidden-option negatives. |
| P1 | M25 reconstructs dynamic M10 cardinality from role-specific M04 facts, but the draft does not define the facts schema or explain what authority remains when ephemeral stdout bytes are not retained. | A writer could invent `representative_count`, and the review could overstate M25 as independently revalidating every internal receipt. | Define a closed per-role validated-facts enum. M04 facts are accepted only after exact M04 typed decoding and canonical-byte validation by the request owner. M25 independently validates all retained packet bytes but treats ephemeral facts as writer-attested provenance checked by parity tests, not independent semantic truth. |
| P1 | `all nineteen predecessor evidence bytes unchanged` is stronger than the available baseline. No V7 attempt produced a frozen nineteen-object packet, and V8 revises wrapper schemas. | A future test could claim impossible byte parity or preserve the wrong wrapper. | Require byte parity only for already-frozen predecessor scientific payloads and exact denominators. Version V8 wrapper/process/packet schemas explicitly and compare their semantic projections, not nonexistent V7 packet bytes. |
| P1 | The new module list uses abbreviated `tests/...` paths and does not state that the dirty index and working-tree bytes are separate baseline facts. | The implementation inventory can pass while pointing at the wrong path or silently losing staged/unstaged donor state. | Use exact repository-relative paths. Pin current worktree file bytes/modes for all 23 donor paths and separately record porcelain status plus index blob IDs without changing the index. |

## Ledger Bound Check

The proposed numeric cap remains sufficient after the repairs only if the
fail-stop rule is enforced:

```text
16,668 invocation pairs * 7,168 bytes = 119,476,224 bytes
128 MiB                                 = 134,217,728 bytes
remaining header/seal/newline budget    =  14,741,504 bytes
```

Header and terminal seal remain individually bounded by 4,096 bytes. No event
may be appended after the first incomplete or non-success terminal route.

## Claim Boundary Clarification

Role-specific validation by the request owner is materially stronger than the
V7 generic recursive JSON identity check. It is not an independent rerun of
every ephemeral child output. M25 independently reopens and validates the
nineteen retained evidence objects, C08, the resource receipt, ledger and packet
manifest. Other internal stdout facts remain writer-attested provenance and
must be reported that way.

## Next Legal Action

Repair V8 in place, retain this critique, then perform critique V2. No code or
R8B execution is authorized.
