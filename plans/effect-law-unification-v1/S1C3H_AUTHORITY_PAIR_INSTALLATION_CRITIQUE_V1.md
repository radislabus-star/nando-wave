# S1C-3H Authority Pair Installation Critique V1

Status: `ADVERSARIAL REVIEW APPLIED BEFORE IMPLEMENTATION`

| Priority | Finding | Failure if ignored | Frozen repair |
|---|---|---|---|
| P0 | Runtime and authority were deployed as independent files. | Candidate authority is rejected before capture starts. | Treat runtime, environment, authority binary, sidecars, and final admission as one compatibility unit. |
| P0 | Starting the candidate runtime before candidate admission creates a build mismatch; running the gate after stopping runtime makes its deployment health fail. | Circular startup dependency or a false VETO. | Generate candidate sidecars and composite admission off-path while the old runtime remains healthy, then install the frozen unit during one stopped interval. |
| P0 | Replacing legacy sidecars without their generation directory and pointer is torn publication. | Gate or future controller sees mixed generations. | Stage and install the complete immutable generation before publishing the pointer and legacy projections. |
| P0 | Rollback of only runtime and config repeats the original defect in reverse. | Old runtime receives new authority or new runtime receives old authority. | Back up and restore the complete compatibility unit. Verify exact pair digest after rollback. |
| P0 | S1C-3G discarded the compared candidate projection before rollback. | The real blocker cannot be proved afterward. | Persist diagnostic and health packets before rollback begins. Diagnostic persistence failure itself forces rollback and remains visible. |
| P0 | Active path/timer units can rewrite authority during installation. | Candidate staging or production publication races the controller. | Pause both response-admission and composite-gate path/timer/service owners and restore their exact active states. |
| P1 | A Git commit alone does not prove runtime compatibility. | Different feature sets or stale artifacts can share a claimed source. | Execute `--print-runtime-contract-sha256` on both candidate binaries and bind binary SHA plus contract SHA in preparation and final receipts. |
| P1 | A multi-file replacement is not power-loss atomic. | Reboot can expose a mixed on-disk unit. | Keep durable `ROLLBACK_ARMED` state before replacement, fsync each backup and install, fail closed on mismatch, and provide idempotent recovery from exact rollback bytes. |
| P1 | The composite gate could write production `admission.json` during preflight. | Preparation would mutate authority before rollback is armed. | Use a staged gate profile and `NANDO_TRANSITION_ADMISSION_JSON` under the transaction directory. |
| P1 | Running staging as root hides production permission errors. | Preparation passes but the service user later fails. | Run response-admission and the composite gate as user `e` against an `e:e` staging directory. |
| P1 | Empty journals can be mistaken for a failed installation. | Valid recorder installation is rolled back merely because no matching ordinary request arrived in 15 seconds. | Separate installation readiness from the first natural record and later S1C-4 census. |
| P1 | Installation can be overstated as grounded meaning. | Infrastructure PASS becomes a false scientific claim. | Keep K2, S1C-4 PASS, model training, phase mutation, and Law #2 explicitly closed. |
| P1 | A blanket one-attempt rule can make a deployment typo terminal science. | Repair work stops before the instrument is installed. | Preserve every attempt immutably, but allow separately committed engineering repairs with new preflight and transaction identity. Natural evidence remains non-retryable. |
| P2 | Candidate generation directories can accumulate after rollback. | Bounded disk leak. | Record their exact bytes; retain the referenced immutable attempt directory and remove only unreferenced staging copies after sealing. |

## Accepted Architecture

```text
S1C-3G immutable failure evidence
-> S1C-3H separately rooted compatibility repair
-> off-path candidate authority preparation
-> complete-unit transactional install or complete-unit rollback
-> capture INSTALLED
-> natural append cursor
-> S1C-4 census, still authority=false
```

No finding authorizes generated traffic or weakens fail-closed execution.
