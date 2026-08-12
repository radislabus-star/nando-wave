# S1C-3H Completion Repair Critique V1

Status: `ADVERSARIAL REVIEW APPLIED BEFORE RECOVERY CODE`

| Priority | Finding | Failure if ignored | Required repair |
|---|---|---|---|
| P0 | Reading oneshot state again after the settled wait opens a timer race. | A successful service is falsely rejected when the next normal cycle starts. | Validate and receipt-bind the same settled snapshot returned by the wait. |
| P0 | `activating` can expose `Result=success` from the preceding run. | A currently failing run could be accepted using stale success fields. | Accept only `inactive + Result=success + ExecMainStatus=0` in one snapshot. |
| P0 | Repeated rollback can overwrite the primary diagnostic. | The exact candidate blocker is lost and cannot be audited. | Make the first valid rooted diagnostic immutable and reuse its root. |
| P0 | The interrupted orchestrator lost its connector-before artifact. | Reconstructing it from current state would fabricate interval survival evidence. | Seal a scoped recovery verdict with connector survival explicitly `UNKNOWN`. |
| P0 | Starting a new attempt while the old one is nonterminal creates two open transaction identities. | Recovery and installation evidence can be mixed. | Restore and terminally seal the interrupted attempt before a new transaction. |
| P0 | Intentionally stopping an in-flight oneshot leaves `Result=signal/15`. | The installer mistakes its own pause action for a candidate failure. | With all triggers stopped, require inactive, reset only the intentional oneshot failure, then require a later real authority renewal after trigger restoration. |
| P0 | `reset-failed` may retain the preceding `ExecMainStatus`, and timer start time is nondeterministic. | Cleared metadata is mistaken for renewal or a correct candidate is rejected before the timer runs. | Explicitly run both authority oneshots to `success/0` while background triggers remain stopped, then restore triggers. |
| P0 | Immutable remote evidence is mirrored locally with its restrictive modes. | The second mirror refresh cannot remove the first snapshot, aborting orchestration after a valid candidate run. | Strip remote ownership and modes while extracting the local transport copy, and make an existing local mirror owner-writable before replacement. |
| P1 | A recovery receipt could be mistaken for installation PASS. | Capture is falsely reported as installed. | Recovery must set `capture_installed=false`, `scientific_authority=false`, and use a distinct verdict. |
| P1 | Empty natural journals can be mistaken for failed installation. | A valid recorder is removed before ordinary evidence arrives. | Keep installation readiness and natural record count as separate fields. |

No finding permits synthetic traffic, post-hoc goals, K2 promotion, or work
outside S1C-3H.
