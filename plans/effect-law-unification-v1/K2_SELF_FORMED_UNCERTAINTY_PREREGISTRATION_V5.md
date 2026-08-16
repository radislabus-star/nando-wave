# K2 Self-Formed Uncertainty Preregistration V5

Status: `REVISED AFTER CRITIQUE / PENDING STRUCTURAL GATES AND PREFLIGHT`

Date: `2026-08-16`

## 1. Canonical Relationship

V2, V3, V4 and V5 together define the experiment. V5 supersedes only:

```text
V2 generator clauses that hard-code Development for every split
V2 one-probe oracle clause after V4 introduced two-probe closure
V2 single-probe durable transitions superseded by V4 ordered plans
R9 declarative confirm-read capability
```

All other V2-V4 grammar, support, model, quotient, complete-frontier,
predecessor-scorer, safety, resource, control and claim boundaries remain.

The failed pre-attempt audit is retained in
`K2_SELF_FORMED_UNCERTAINTY_R10_PREATTEMPT_DISCREPANCY_2026-08-16.md`.
The confirm execution addendum V1 is rejected by its critique and has no code
authority. The second V5 critique records the final contradiction pass and the
repairs incorporated into this revision.

## 2. Frozen Development Oracle

Before repair, frozen commit `8e416d1d` and generator executable
`929c51bf374ddc55a4a109977bbc987e3443ee1bf4317f873fb3aed21568652b`
produced this exact Development response:

```text
generator request root        c285343b9aa9c5146a1f512cf3c2f412a0e538dd803665429b42035e81430588
response root                 10264f4a25e3ad22156a30c49dc1f53aee1de5754d6dd0f1b77c54929b3531cf
public batch root             9fbdd35627b2b5265b8e35274412e7dc2a0cce576066022d99b8c67f13b8ad8a
private batch root            5ed5436d0c78e5e62f58bbb4efa34cda73a50af296fdf03dfab16014f1c274e5
private denominator root      011ea03be71e80d520101ce2e7b8897be2a2b88e4285d0c8ca905dbb715026dc
canonical response SHA-256    26e509ae09fd68b97323e9e9d0ee1bce9bfee7b0dbd75be5aca2015675027c6e
case count                    16
```

The historical canonical response SHA-256 binds every public and private case
root. The repaired generator necessarily has a new executable SHA-256, so its
Development request root and outer response root must be mechanically rebound.
They are not required to equal the historical identity-bound roots.

Behavior parity is exact:

```text
public batch root             unchanged
private batch root            unchanged
private denominator root      unchanged
all public/private case roots unchanged through their bound batch roots
```

The successor request and outer response must equal an independent
reconstruction that substitutes only the successor generator executable hash.
Any other byte or root difference is `DEVELOPMENT_ORACLE_DRIFT` and blocks R9B.

## 3. Exact Authorization Receipt

After a successor R9 freeze, but before attempt-directory creation, a distinct
authorization-slot owner freezes `K2UncertaintyR10AuthorizationReceiptV1`:

```text
schema
exact user authorization text
Codex session ID
authorized_at
experiment ID
successor freeze root
contract aggregate root for V2-V5
executable manifest root
maximum attempts = 1
maximum slot claims = 1
authority = all false
receipt root
```

The exact required user text must name R10, one sealed scientific attempt, the
successor freeze root and the V2-V5 contract. A generic `continue`, environment
boolean or old-freeze authorization is invalid.

This receipt proves only local procedural authorization. It is not external
attestation or independent custody.

The same owner maintains an append-only slot ledger outside every attempt
directory. The globally unique slot key is:

```text
experiment ID
+ successor freeze root
+ contract aggregate root
```

`SLOT_CLAIMED` binds that key and the authorization receipt root and is fsynced
before attempt-directory creation. The owner rejects both a reused receipt root
and any second claim for the same slot key, even when a new authorization
receipt is supplied. Neither attempt cleanup, directory deletion, a new user
message nor process restart can free a claimed slot. Thus a second receipt
cannot manufacture a second nonce for one frozen experiment.

## 4. Split-Specific Generator

The existing `K2UncertaintyGeneratorRequestV1` and
`K2UncertaintyGeneratorResponseV1` remain byte-exact Development schemas. V5
adds separate `K2UncertaintyConfirmGeneratorRequestV1` and
`K2UncertaintyConfirmGeneratorResponseV1` schemas. No optional Confirm field is
added to a Development wire object.

The generator process reads one canonical JSON object and dispatches by exact
schema string before typed decoding. It accepts only these two closed schemas:

```text
Development
  split == Development
  seed length == 32
  commitment == frozen development commitment
  existing V2/V3 roots and wire bytes unchanged

Confirm
  split == Confirm
  nonce length == 32
  commitment == SHA-256(nonce)
  commitment differs from development and superseded V1 commitments
  successor freeze root present
  exact R10 authorization receipt root present
```

Confirm request roots bind split, nonce bytes, commitment, V2-V5 roots,
successor freeze root where applicable, authorization root where applicable,
generator executable SHA-256 and denied authority.

Confirm uses disjoint experiment and case domain separators. Public vocabulary,
support, cases and batch preserve the split and dynamic commitment. No downstream
validator may coerce Confirm to Development.

## 5. One-Shot Nonce Transport

The authorization-slot owner first publishes `SLOT_CLAIMED`. The confirm owner
then:

1. validates the descriptor, authorization, claimed slot, current executable
   hashes and empty attempt path;
2. creates the attempt directory exclusively with mode `0700`;
3. publishes `ARTIFACTS_FROZEN` durably;
4. obtains exactly 32 bytes from the operating-system CSPRNG;
5. writes the retained nonce artifact with mode `0400`;
6. publishes `NONCE_COMMITTED` containing SHA-256 only;
7. creates the canonical Confirm generator request in memory;
8. publishes `GENERATOR_DISPATCHED`, binding the request root and child
   executable root, before writing any request byte;
9. sends the request once through the generator child's anonymous stdin pipe;
10. closes the pipe and zeroizes the request and in-memory nonce buffers;
11. never invokes the generator again for the claimed slot key.

Nonce bytes are forbidden from argv, environment, filenames, persisted request
files, logs, public receipts and public process requests. The generator child
runs with a cleared environment and no network.

`SLOT_CLAIMED` consumes the authorization before attempt-directory mutation.
Any later failure is an immutable terminal. A crash after nonce-file publication
but before `NONCE_COMMITTED` projects as `NONCE_CREATED_UNCOMMITTED` and becomes
`INDETERMINATE`; the retained nonce may be hashed for diagnosis but is never
dispatched. No replacement nonce or directory is allowed.

A crash at or after `GENERATOR_DISPATCHED` but before complete split-output
publication projects as `GENERATOR_RESULT_INDETERMINATE`. The generator is not
rerun, even when no request byte reached the child. The combined generator
response exists only in bounded memory; `CASES_GENERATED` is published only
after the public batch, denominator receipt, all resolver tables, all final
truth files and private split receipt are validated, fsynced and rooted.
Partial output files are retained as failure evidence and never promoted.

## 6. Generator Output Split

The Confirm generator response contains three separately rooted payloads:

```text
public batch
  vocabularies, support, public split commitment and case order

resolver tables, one file per case
  one case ID -> opaque action ID -> concrete effect
  no topology family, matched-pair label or true semantic-class label

final private truth, one file per case
  one complete private case, topology family and expected denominator witness
```

The confirm owner atomically publishes them as separate files:

```text
public batch                    0600  RETAIN_ALWAYS
public denominator receipt      0600  RETAIN_ALWAYS
sixteen resolver tables         0400  RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
sixteen final-truth case files  0400  RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
nonce                           0400  RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
private split receipt           0400  RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
```

The private split receipt binds every private artifact and is not a public
selection input. A dedicated public denominator receipt contains only
experiment ID, public batch root, private expected-denominator commitment,
generator executable root and denied authority. The public coordinator receives
only the public batch and this receipt before `ALL_CASES_PRECOMMITTED`.

## 7. Executable Topology

The successor executable manifest contains one unique SHA-256 for every entry:

```text
confirm owner
generator
public coordinator
learner
probe enumerator
predecessor selector
closure planner
baseline owner
selection preverifier
closure verifier
private resolver
safety verifier
worker
observer
final verifier V2
terminal evaluator
cleanup authorizer
cleanup owner
cleanup verifier
result publisher
authorization-slot owner
oracle and baseline evaluator
control evaluator
development freeze owner
```

The authorization-slot owner, confirm owner, coordinator, closure planner,
resolver, oracle and baseline evaluator, control evaluator, terminal evaluator,
cleanup authorizer, cleanup owner, cleanup verifier and result publisher are new
non-production executables. A Cargo test binary cannot be the sealed
coordinator, ranking owner, mutation owner, cleanup authority, result publisher
or control authority. Every process verifies its own executable SHA-256 against
its request or descriptor.

The coordinator transports immutable messages only. It cannot generate model
sets or probes, rank a plan, resolve a private effect, fabricate observations,
evaluate final truth or decide the terminal verdict.

The public coordinator exits after publishing `ALL_CASES_PRECOMMITTED`. The
confirm owner then acts only as a sealed execution supervisor: it follows the
frozen schedule, launches owner-specific sandboxes and transports path and
receipt roots through the journal. It never parses resolver, nonce or final
truth bytes and never sends a private child result back to the public
coordinator. This is a procedural separation on one local host, not independent
custody.

## 8. Isolation And Mount Matrix

All child owners run through the frozen sandbox launcher:

```text
bwrap --unshare-all --die-with-parent --new-session --cap-drop ALL
cleared environment
no network
read-only /usr, /lib and exact executable
tmpfs /tmp
RSS <= 512 MiB
bounded CPU, process and file size
```

Mount authority:

```text
public coordinator    public batch + public artifacts only
confirm supervisor    descriptor + journal + classified path identities;
                      private artifact contents never parsed
private resolver      resolver table + frozen plan packet only
worker                one disposable workspace read-write
observer              completed workspace read-only
final verifier        one final private case + frozen evidence read-only
oracle evaluator       one final private case + public plans/evidence read-only
control evaluator      after MODELS_UPDATED only: public artifacts plus
                      read-only private artifacts for leak audit and isolated
                      negative-control copies; no learner output channel
terminal evaluator    roots, receipts and counters; no nonce or mapping bytes
cleanup verifier      complete attempt root + before/after classified manifests
                      read-only after cleanup
```

Same-user file modes and bwrap are procedural isolation, not external custody.
The scientific report must state `LOCAL_PROCEDURAL`.

## 9. Global Precommit And Private Reveal Order

The generator's public case order is nonce-derived and enters the batch root.
For all sixteen cases, before private resolver execution or any worker:

```text
public support and vocabulary roots
complete four-model sets
complete 1,792-probe denominators
all first predecessor tournament results
all complete closure censuses
all one-or-two-probe plans
all baseline decisions
all independent preverification receipts
private expected-denominator commitment
execution order
```

These produce one `ALL_CASES_PRECOMMITTED` root. Only then can one case resolver
table be mounted into a fresh private-resolver process. The coordinator never
opens resolver or final-truth files and exits before the private route begins.
The confirm supervisor launches the private route from the immutable precommit
root without changing any plan, case order or denominator.

For each already frozen plan step, one private-resolver invocation receives one
case table and emits only:

```text
case ID
plan root and probe ordinal
selected opaque action ID
resolved effect
resolver executable root
resolver receipt root
authority false
```

It cannot emit topology family, matched-pair label, alternative mappings or
true semantic class. The independently built safety-verifier executable owns
closed resolver-receipt schema validation before its separate safety decision;
it rejects any extra mapping entry or private byte in the receipt.

## 10. Ordered Execution

For each case in the frozen order and each plan step in frozen ordinal order:

```text
resolver receipt
-> independent safety PASS
-> durable case dispatch
-> isolated worker
-> worker exit
-> read-only observer
-> worker/observer exact parity
-> immutable observation vector append
```

No next case begins until the current case observation vector, final-verifier
receipt, model update and case terminal are durable. Outcomes cannot change any
frozen plan or later-case selection.

A durable dispatch without a matching observation is `INDETERMINATE`; the same
workspace identity is never redispatched.

## 11. Bounded One-Or-Two-Probe Oracle

V5 replaces V2's one-probe oracle after V4 closure.

After reveal, the distinct oracle and baseline evaluator enumerates the complete representative set
and every ordered plan of length one or two with distinct probe roots. It uses
actual true-model outcomes only for evaluation, never for learner, selector or
closure-planner input.

For each plan it computes exact sequential residual semantic classes. Valid
plans satisfy all safety and cumulative risk/cost budgets. The oracle rank is:

```text
min actual residual semantic classes
-> min plan length
-> min cumulative risk
-> min cumulative cost
-> lexicographic ordered probe roots
```

PASS requires both model-guided and oracle residual classes to equal one in
every case. Equality is reported as equality, never superiority over oracle.
The oracle evaluator cannot import closure planner, terminal evaluator or final
verifier elimination helpers. It receives one final-truth case file only after
that case's observation vector and final-verifier receipt are durable.

## 12. Rule Baselines

The four frozen non-oracle policies remain one-probe policies receiving the
same adapted public bytes:

```text
passive observation
stable-root order
cheapest first
explicit applicability/dependency/cleanup heuristic
```

For every policy and case, the oracle and baseline evaluator records selected probe,
actual residual semantic classes, true-class retention, risk and cost. A
baseline with no probe records the pre-probe class count and zero execution.

PASS requires for each policy independently:

```text
sum(model-guided residual classes) < sum(policy residual classes)
strict per-case residual improvement >= 12 / 16
```

Risk and cost break no residual-class tie in the superiority count; they are
reported separately. The smallest-model diagnostic remains correctness-only
and cannot count as a singleton posterior.

## 13. Independent Terminal Evaluation

`K2UncertaintySealedTerminalRequestV1` contains only immutable roots, receipts,
exact counters and resource measurements. It binds a distinct oracle/baseline
receipt, a successor static-control receipt and an attempt-bound V5 control
receipt. The terminal evaluator independently reopens them and emits exactly one:

```text
K2_SELF_FORMED_UNCERTAINTY_CAPABILITY_PASS
SCIENTIFIC_FAIL
INFRASTRUCTURE_FAIL
INDETERMINATE
```

PASS requires:

```text
attempts                                      1 / 1
sealed cases                                 16 / 16
four-model sets                              16 / 16
raw probe dispositions                  28,672 / 28,672
raw predictions                        114,688 / 114,688
all closure plans available                   16 / 16
plan length                                     1 or 2 each
selected executions                        derived exact
independent preverification                 16 / 16
safety receipts                      selected executions
worker/observer matches              selected executions
final verification                         16 / 16
surviving semantic classes                   1 each
private true-class matches                  16 / 16
bounded oracle equality                     16 / 16
four baseline aggregate tests                4 / 4
four baseline per-case thresholds            4 / 4
successor static legacy controls            32 / 32
successor static V3 controls                 4 / 4
successor static V4 controls                16 / 16
V5 controls                                12 / 12
false accepts                                      0
forbidden executions                               0
authority promotions                               0
production/network effects                         0
resource violations                                 0
```

No weighted score, missing-row default, majority vote or narrative override is
allowed.

Development rehearsal uses the same terminal-evaluator executable but a
separate closed `K2UncertaintyDevelopmentRehearsalTerminalRequestV1` schema. It
can emit only `DEVELOPMENT_REHEARSAL_PASS` or a named rehearsal failure. It
requires `sealed_attempts = 0`, no authorization slot and no CSPRNG nonce, and
cannot emit or be interpreted as a scientific verdict.

The sealed scientific verdict is durable before cleanup. It is never rewritten
afterward. A distinct cleanup-authorizer executable receives only the frozen
verdict root and classified-path-manifest root; it may authorize the cleanup
owner but cannot evaluate science or delete a path. A distinct result-publisher
executable receives only the frozen scientific verdict and independent cleanup
receipt. It publishes no overall PASS unless both roots are present and the
cleanup receipt is `CLEANUP_FROZEN`. Cleanup failure therefore leaves the
scientific verdict preserved and the run operationally incomplete.

## 14. V5 Controls

Exactly twelve new controls must return named expected dispositions:

```text
K1  Confirm request with reused development commitment rejected
K2  missing or foreign exact-root authorization rejected
K3  nonce in argv, env, path or persisted generator request rejected
K4  nonce or private bytes in any public artifact rejected
K5  private resolver access before global precommit rejected
K6  final truth access before complete observation vector rejected
K7  unmanifested or hash-mismatched coordinator rejected
K8  second slot claim, attempt directory or nonce for one frozen experiment rejected
K9  partial case set or missing top-level terminal conjunct rejected
K10 one-probe oracle substituted for required two-probe oracle rejected
K11 omitted baseline case or policy denominator rejected
K12 cleanup deletion of retained evidence or residue omission rejected
```

A parse error, panic, timeout or wrong code is not a control PASS.

The `32 + 4 + 16` static controls are rebuilt and run at R8B against the exact
successor commit. The control evaluator verifies their frozen receipt. R8B also
runs rehearsal instances of K1-K12 against DevelopmentRehearsal roots to prove
the evaluator and expected dispositions are executable. These rehearsal
results are readiness evidence only.

After the sealed attempt reaches `MODELS_UPDATED`, K1-K12 run exactly once
against that attempt's roots. The terminal accepts only this attempt-bound V5
receipt, never the rehearsal receipt. Controls that construct invalid inputs do
so in isolated scratch trees and cannot mutate or replace sealed artifacts.
Static, rehearsal and attempt-bound denominators are recorded separately and
never merged.

## 15. Durable Attempt Machine

```text
EMPTY
-> SLOT_CLAIMED
-> ARTIFACTS_FROZEN
-> NONCE_CREATED
-> NONCE_COMMITTED
-> GENERATOR_DISPATCHED
-> CASES_GENERATED
-> MODEL_SETS_FROZEN
-> PROBE_SETS_FROZEN
-> SELECTIONS_FROZEN
-> ALL_CASES_PRECOMMITTED
-> case plan steps repeated in frozen order
-> OBSERVATIONS_FROZEN
-> MODELS_UPDATED
-> CONTROLS_FROZEN
-> terminal verdict
-> CLEANUP_FROZEN
```

Every event binds attempt root, sequence, previous event root, owner executable
root, request root, payload root and denied authority. Publication is temp file,
file fsync, rename and directory fsync. Restart exactly projects every legal
prefix. No terminal transitions back to nonterminal.

`NONCE_CREATED` is never a dispatch permit. If `NONCE_COMMITTED` is not the next
durable event, restart publishes `NONCE_CREATED_UNCOMMITTED` and terminal
`INDETERMINATE` without generator execution.

`NONCE_COMMITTED` alone is also not a replay permit. Once
`GENERATOR_DISPATCHED` is durable, every restart forbids a second generator
invocation. Missing or partial generated output becomes
`GENERATOR_RESULT_INDETERMINATE` and retains all observed bytes.

## 16. Cleanup

Before cleanup, the terminal owner freezes a complete classified path manifest:

```text
RETAIN_ALWAYS
RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW
DELETE_AFTER_TERMINAL_AND_OBSERVER_FSYNC
SUPERSEDED_NEVER_USE
```

The cleanup authorizer checks the terminal and classification roots, then
authorizes the cleanup owner. Cleanup removes only the third class. An
independent cleanup verifier compares
before and after manifests, verifies every retained SHA-256 and mode, and
requires zero disposable or unclassified residue. It then publishes
`CLEANUP_FROZEN`.

Failure or indeterminate evidence is retained exactly like PASS evidence.

## 17. Repair Slices

```text
R7F  V5 paper, critique, structural routes and implementation preflight
R7G  split-aware generator and exact Development behavioral parity
R7H  authorization-slot ledger, nonce owner, output split and attempt journal
R7I  public coordinator, private resolver and sandbox mount enforcement
R7J  bounded oracle/baseline and control evaluators, independent terminal evaluator
R7K  cleanup verifier, V5 K1-K12 controls and crash/restart tests
R8B  full non-sealed suites, resource run and owner-bounded gates
R9B  successor source/executable/test freeze with executable dry-run evidence
R10B mandatory stop for fresh exact-root authorization
R11B exactly one sealed attempt, terminal result, cleanup and critique
```

Heavy builds and tests run only on the mini-PC with `-j 20`. No production
service, connector, dashboard, traffic, K1, LawCertificate, package or phase
memory is touched.

## 18. R9B Readiness Rule

R9B cannot set confirm readiness from booleans. It requires:

```text
exact Development behavioral parity with identity-only outer-root rebinding
DevelopmentRehearsal through confirm owner, generator pipe, output split,
coordinator, resolver, terminal and cleanup
all route executables present exactly once in manifest
all executable self-hash checks exercised
all 32 + 4 + 16 static controls PASS
all twelve DevelopmentRehearsal instances of K1-K12 PASS
fault injection for every mutating transition
restart parity for every legal prefix
zero disposable residue
authority false
confirm nonce absent
sealed attempts 0
rehearsal terminal disposition DEVELOPMENT_REHEARSAL_PASS
```

`DevelopmentRehearsal` is a descriptor mode of the exact confirm-owner binary.
It uses only the frozen development seed and unchanged Development wire schema,
creates no CSPRNG nonce, claims no authorization slot and records
`sealed_attempts=0`. It exercises the same anonymous pipe, cleared environment,
public/private split, downstream process topology, terminal and cleanup paths.
It proves process readiness, not the scientific result.

## 19. Stop And Claim Boundary

V5 grants no code authority until its adversarial critique, owner-bounded
structural gates and implementation preflight all pass. It grants no nonce or
sealed attempt authority through R9B.

After R9B, fresh user authorization must name the exact successor freeze root.
Only then may R11B create the sole nonce.

A sealed PASS proves only the bounded generated-filesystem capability stated by
V2-V4, with V5's corrected one-or-two-probe oracle. It does not prove Natural
K2, open-ended strategy learning, natural-traffic transfer, Wave-causal
grokking, K1 admission, product authority, deployment readiness or superiority
over adaptive baselines that receive the same two-probe budget.
