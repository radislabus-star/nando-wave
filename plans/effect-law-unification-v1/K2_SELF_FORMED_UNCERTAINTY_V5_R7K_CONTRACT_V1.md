# K2 Self-Formed Uncertainty V5 R7K Contract V1

Status: `DRAFT FOR ADVERSARIAL CRITIQUE / NO CODE AUTHORITY`

Date: `2026-08-20`

Predecessor: `K2_SELF_FORMED_UNCERTAINTY_V5_R7J_RESULT_2026-08-20.md`

## 1. Exact Scope

R7K closes only the DevelopmentRehearsal process-control and classified-cleanup
component boundary:

```text
real invalid process input
-> exact existing owner boundary
-> named expected rejection
-> process outcome receipt
-> independent R7J control evaluation
-> Development terminal receipt
-> complete classified path manifest
-> cleanup authorization
-> bounded deletion
-> independent retained/residue verification
-> Development completion receipt
```

R7K does not claim an authorization slot, obtain a CSPRNG nonce, run Confirm,
execute a sealed attempt, publish a scientific verdict, deploy, read natural
traffic, mutate K1 or phase memory, or modify production services.

```text
sealed attempts                  0 / 1
real authorization claims        0
real Confirm nonce               absent
scientific verdict               absent
Natural K2 authority             false
deployment authority             false
```

R7K component PASS unlocks only R8B. R9B, R10B and R11B remain locked.

## 2. Frozen Inputs

R7K binds these immutable predecessors:

```text
V2-V5 contract aggregate root
R7J source commit and tree
R7J preflight root
R7J oracle/baseline executable SHA-256
R7J control-evaluator executable SHA-256
R7J terminal-evaluator executable SHA-256
Development generator seed and historical behavior roots
R7H attempt-journal schemas
R7I public/private process schemas
```

The R7J component result is authority-denied evidence. R7K may consume its
closed schemas and executable roots, not reinterpret its in-memory fixtures as
real process outcomes.

## 3. Executable Ownership

R7K adds four pairwise-distinct self-hash-bound executables:

```text
nando-k2-self-formed-cleanup-authorizer
nando-k2-self-formed-cleanup-owner
nando-k2-self-formed-cleanup-verifier
nando-k2-self-formed-result-publisher
```

The DevelopmentRehearsal integration harness may launch processes and record
their OS outcomes. It has no control, terminal, cleanup or result authority.
The R7J control evaluator remains the only owner that decides whether the
twelve process outcomes match the frozen expected dispositions. The terminal
evaluator remains the only owner of the Development terminal disposition.

```text
confirm/development supervisor  transports roots and owns the path census
target executable               emits the named rejection
integration harness             records exit/stdout/stderr and source/log roots
control evaluator               validates exact K1-K12 process evidence
terminal evaluator              emits DevelopmentRehearsalPass or named failure
cleanup authorizer              validates terminal plus classification policy
cleanup owner                   deletes only authorized disposable paths
cleanup verifier                proves retained parity and zero residue
result publisher                joins terminal and cleanup receipts
```

No executable may occupy two authority roles. Thin wrappers contain no domain
decision logic.

## 4. Development Control Process Evidence

Every K1-K12 control is executed against an isolated scratch copy of the exact
R7K successor artifacts. The harness records
`K2UncertaintyControlProcessOutcomeV1` from the actual child process:

```text
scope                           DevelopmentRehearsalV5
experiment root                exact rehearsal root
freeze root                    exact successor freeze root
attempt root                   absent
runner executable SHA-256      actual target process
test executable SHA-256        actual harness
control request root           exact invalid input
normal exit / exit code        measured
stdout bytes/root              measured and decoded
stderr root                    measured
timeout / panic                measured false
source artifact root           exact successor source set
log artifact root              immutable per-control log
authority                      all false
```

An outcome counts only when the target process exits normally with code zero
and emits the exact two-field `K2UncertaintyControlStdoutV1` expected by R7J.
A parse error, panic, signal, timeout, missing log, hand-constructed stdout or
test assertion alone is not a PASS.

The harness cannot call `K2UncertaintyControlProcessOutcomeV1::seal` until the
child has exited and its stdout, stderr, executable and request bytes have been
hashed. Each control runs in a fresh directory under the rehearsal root.

## 5. Exact K1-K12 Scenarios

Each scenario invokes the real owner that protects the named boundary:

| ID | Invalid operation | Required target disposition |
|---|---|---|
| K1 | Confirm generator request reuses the frozen Development commitment | `reused_development_commitment_rejected` |
| K2 | confirm-owner descriptor omits authorization or binds a foreign authorization root | `missing_or_foreign_authorization_rejected` |
| K3 | fixed rehearsal canary appears in argv, environment, path or persisted generator request | `nonce_transport_rejected` |
| K4 | fixed private canary appears in a public artifact tree | `private_public_leakage_rejected` |
| K5 | resolver request is launched before `ALL_CASES_PRECOMMITTED` | `early_private_resolver_rejected` |
| K6 | final verifier receives truth before the complete observation vector | `early_final_truth_rejected` |
| K7 | public coordinator executable is absent from or mismatched with the manifest | `coordinator_manifest_mismatch_rejected` |
| K8 | a fixture slot ledger receives a second claim or a second attempt/nonce identity | `duplicate_slot_attempt_or_nonce_rejected` |
| K9 | Development terminal request omits one case or one required conjunct | `partial_terminal_denominator_rejected` |
| K10 | a two-probe case receives a substituted one-probe oracle receipt | `one_probe_oracle_substitution_rejected` |
| K11 | oracle/baseline receipt omits one case or one frozen policy denominator | `baseline_denominator_omission_rejected` |
| K12 | cleanup request deletes retained evidence or omits disposable residue | `cleanup_retention_or_residue_violation_rejected` |

K3 and K8 use deterministic Development fixture bytes and fixture ledgers.
They create no authorization receipt, real slot claim or CSPRNG nonce. K12 uses
a disposable clone and cannot mutate the canonical rehearsal tree.

All twelve run exactly once per R7K acceptance run. Their order is K1 through
K12 and their denominator is exactly 12. Rehearsal and future SealedAttemptV5
receipts remain schema-distinct and cannot substitute for one another.

## 6. Classified Path Manifest

After a durable Development terminal receipt, the supervisor freezes one
complete manifest for every regular file, directory and symlink below the
rehearsal root. Paths are canonical UTF-8 relative paths with no empty,
absolute, parent, duplicate or platform-alias components.

Each row binds:

```text
relative path
file kind
artifact kind
retention class
pre-cleanup SHA-256 for regular files
mode
size
producer executable root
producing journal event root
```

Allowed retention classes are frozen V5 values:

```text
RETAIN_ALWAYS
RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
DELETE_AFTER_TERMINAL_AND_OBSERVER_FSYNC
SUPERSEDED_NEVER_USE
```

The artifact-kind-to-retention mapping is a closed code table owned by the
cleanup authorizer. Caller-provided classification cannot override it.
Unrecognized kinds, missing paths and symlinks are fail-closed.

## 7. Cleanup Authorization

`K2UncertaintyCleanupAuthorizationRequestV1` contains only:

```text
experiment and rehearsal roots
Development terminal receipt root
classified manifest root
attempt-journal projection root
cleanup-authorizer executable SHA-256
authority all false
```

Authorization requires:

```text
DevelopmentRehearsalPass
sealed attempts == 0
authorization claims == 0
Confirm nonce absent
terminal and manifest roots belong to one rehearsal
complete artifact-kind classification
observer and terminal events fsynced
no path or mode ambiguity
```

The authorizer emits exact relative paths permitted for deletion plus their
pre-cleanup identities. It cannot delete or publish a result.

## 8. Cleanup Transaction

The cleanup owner opens only the rehearsal root and authorization receipt. It
rejects symlinks and path traversal, revalidates each disposable identity, then
deletes entries deepest-first. Retained and superseded evidence is never
deleted.

For every deletion it durably appends:

```text
sequence
previous event root
relative path
expected pre-cleanup identity
delete result
directory fsync result
owner executable SHA-256
```

Restart resumes from the exact prefix under the same authorization root. An
identity mismatch or missing unrecorded path is terminal and cannot be healed
by broad recursive deletion. The cleanup owner emits only an execution receipt;
it cannot claim cleanup completeness.

## 9. Independent Cleanup Verification

The cleanup verifier receives the before manifest, cleanup journal and a fresh
read-only after census. It independently requires:

```text
every RETAIN_ALWAYS path present with exact bytes and mode
every RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW path present exactly
every SUPERSEDED_NEVER_USE path present exactly
every disposable path absent
every deletion represented once in the journal
zero unclassified paths
zero unexpected residue
zero symlinks
same rehearsal and terminal roots throughout
```

Only then may it emit `K2UncertaintyCleanupReceiptV1` with disposition
`CleanupFrozen`. The receipt has denied authority and cannot alter the terminal
verdict.

## 10. Development Result Publication

The result publisher receives only the Development terminal receipt and
CleanupFrozen receipt. It emits `DevelopmentRehearsalComplete`, never the
scientific capability string. It rejects a sealed terminal, missing cleanup,
foreign root, cleanup failure or any authority promotion.

The scientific result-publisher path is implemented only as closed-schema
validation and substitution negatives. R7K does not create a scientific result.

## 11. Failure And Restart Matrix

Mandatory fault points:

```text
before classified-manifest publication
after manifest file fsync before rename
after manifest rename before directory fsync
before cleanup authorization publication
after authorization file fsync before rename
after each disposable deletion before journal append
after journal append before directory fsync
after all deletions before verifier invocation
after CleanupFrozen file fsync before rename
after CleanupFrozen rename before directory fsync
before result publication
```

Every prefix has one deterministic projection. Failure retains all evidence;
no cleanup failure changes a terminal verdict or allows a second rehearsal.

## 12. Size And Resource Budgets

```text
new mixed-authority modules                 0
new owner-specific modules                <= 5
new thin wrappers                           4
target module size                      <= 700 lines
protocol payload                      < 1 MiB
child RSS                            <= 512 MiB
case wall time                       <= 60 seconds
batch wall time                      <= 20 minutes
network callsites                            0
production/K1/dashboard mutations            0
```

The cleanup census may stream. It must not load complete artifact contents or
all oracle rows into one protocol object.

## 13. Acceptance Evidence

R7K component PASS requires:

```text
R7J oracle/baseline regression                   PASS
R7J evaluator regression                         PASS
real Development K1-K12 process outcomes      12 / 12
R7J control-evaluator receipt                  12 / 12
Development terminal                         PASS
classified manifest completeness                PASS
cleanup authorization                           PASS
cleanup owner restart matrix                    PASS
independent retained-byte/mode parity            PASS
disposable paths absent                          PASS
unclassified or unexpected residue                  0
Development result publication                  PASS
cross-mode and cross-root substitutions          PASS
false accepts                                       0
sealed attempts                                   0/1
real slot claims                                    0
real Confirm nonce                             absent
production/network/K1 effects                       0
strict Clippy, fmt and diff check                PASS
post-implementation structural routes            PASS
```

Heavy tests run only on the mini-PC with twenty Cargo jobs. R7K has no
deployment step.

## 14. Stop Boundary

R7K stops after DevelopmentRehearsal control, terminal, cleanup and result
receipts are durably verified. It cannot execute R8B work in the same commit and
cannot unlock a sealed attempt. The next stage remains separately reviewed R8B.
