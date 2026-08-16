# K2 Self-Formed Uncertainty Confirm Execution Addendum V1

Status: `DRAFT FOR ADVERSARIAL CRITIQUE / NO CODE AUTHORITY`

Date: `2026-08-16`

## 1. Scope

This addendum repairs only the missing transition from the proven development
route to one executable sealed confirm attempt. It does not change the V4
grammar, four semantic classes, probe denominator, predecessor scorer,
one-or-two-probe closure rule, budgets, controls, PASS conjunction, or claim
boundary.

The old R9 freeze remains historical evidence. A successor freeze must bind the
repair before any nonce exists.

## 2. Required Owners

```text
confirm owner
  creates one 32-byte OS-CSPRNG nonce
  atomically publishes its commitment before generator execution
  transports nonce bytes only to the generator through a one-shot pipe

generator owner
  validates Confirm split and dynamic nonce commitment
  emits one public batch and one private batch

public coordinator
  receives no nonce bytes
  receives public batch only until ALL_CASES_PRECOMMITTED
  invokes learner, probe, selector, baseline, closure verifier and preverifier

private resolver
  opens private batch only after ALL_CASES_PRECOMMITTED
  resolves only the already selected action effects
  emits safety-bound dispatch material and no selection input

worker and observer
  execute and observe isolated disposable workspaces

final verifier
  receives private case only after every observation for that case is frozen
  independently reconstructs induction, frontier, closure and true-class match

terminal owner
  aggregates exact denominators, controls, baselines, resource use and cleanup
  cannot change any prior request, plan, observation or verification receipt
```

Every owner is a frozen executable with a distinct SHA-256 identity. Shared
library code is permitted only where already allowed by V4. The final verifier
keeps its existing import prohibition.

## 3. Confirm Generator Contract

`K2UncertaintyGeneratorRequestV1` gains a confirm constructor. Validation is
split-specific:

```text
Development
  split == Development
  seed length == 32
  seed commitment == frozen development commitment

Confirm
  split == Confirm
  nonce length == 32
  nonce commitment == SHA-256(nonce bytes)
  commitment != development commitment
  commitment != superseded V1 commitment
  R10 authorization receipt root is present and frozen
```

Public vocabulary, public cases and public batch preserve the request split and
dynamic split commitment. Development bytes remain unchanged under the
development constructor. Confirm experiment and case roots use a disjoint
domain separator.

The generator process still verifies its own executable hash. It receives the
confirm request on stdin inherited from a one-shot pipe. The serialized request
is never persisted.

## 4. Attempt Descriptor And State Machine

Before nonce creation, the confirm owner validates an immutable descriptor:

```text
new R9 freeze root
new executable manifest root
contract manifest root
test and gate manifest root
R10 authorization receipt root
all executable paths and SHA-256 values
attempt directory path
resource limits
```

The attempt directory must not exist. Creation is exclusive and mode `0700`.
The durable state machine is:

```text
EMPTY
-> ARTIFACTS_FROZEN
-> NONCE_COMMITTED
-> CASES_GENERATED
-> MODEL_SETS_FROZEN
-> PROBE_SETS_FROZEN
-> SELECTIONS_FROZEN
-> ALL_CASES_PRECOMMITTED
-> PROBES_EXECUTED
-> OBSERVATIONS_FROZEN
-> MODELS_UPDATED
-> CONTROLS_FROZEN
-> PASS | SCIENTIFIC_FAIL | INFRASTRUCTURE_FAIL | INDETERMINATE
-> CLEANUP_FROZEN
```

Every transition uses temp file, file fsync, rename and directory fsync. Every
event binds the prior event root, exact owner executable hash, request root,
payload root, authority false and monotonic sequence.

After `NONCE_COMMITTED`, no state permits generator, case, probe or observation
redispatch. Restart may only project the durable prefix, publish an
`INDETERMINATE` terminal where required, or finish cleanup.

## 5. Private Material Boundary

The confirm owner writes:

```text
nonce bytes                     RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW, 0400
private batch                   RETAIN_SEALED_UNTIL_POST_RESULT_REVIEW, 0400
public batch                    RETAIN_ALWAYS, 0600
nonce commitment receipt       RETAIN_ALWAYS, 0600
generator split receipt        RETAIN_ALWAYS, 0600
```

The public coordinator cannot open the nonce file or private batch before the
all-case precommit is durably frozen. The private resolver receives only the
private batch path and immutable selected-plan packet after that barrier.

No public learner, probe, selector, baseline, closure or preverification
request may contain nonce bytes, nonce path, private mapping bytes, private
mapping root, topology family or true-class label.

## 6. Execution And Verification

The existing V4 process route becomes an application-owned coordinator instead
of a test-owned authority. Development tests call the same coordinator with a
development batch. The sealed binary calls it with a confirm batch.

All sixteen closure plans freeze before any worker starts. For each plan step:

```text
selected public probe
-> private effect resolution after global precommit
-> independent safety PASS
-> durable dispatch
-> isolated worker
-> worker exit
-> read-only observer
-> immutable ordered observation vector
-> independent final verifier
-> case terminal
-> case cleanup
```

The top-level terminal owner requires exactly the V4 PASS conjunction. A panic,
wrong control code, missing denominator, process timeout, resource exhaustion,
partial case set, cleanup residue, or authority bit produces a named terminal
that cannot be converted to PASS.

## 7. Side Effects

Allowed:

```text
read successor frozen source and executable manifests
create one private attempt directory on the mini-PC
create one nonce and generated filesystem cases
spawn only manifest-bound local owner processes
write journals, receipts, observations and disposable workspaces
remove only classified disposable workspaces after observation and terminal
```

Forbidden:

```text
network access
service, connector, dashboard or production checkout mutation
natural traffic access
K1, LawCertificate, package, phase-memory or product authority mutation
second nonce or second scientific attempt
post-nonce source, binary, threshold, formula or contract change
seed shopping, case replacement, truncation or approximation
```

## 8. Pre-Nonce Controls

Non-sealed tests must prove:

```text
Confirm request accepts a fresh 32-byte nonce and rejects development reuse
Development output remains byte-identical
public artifacts contain no nonce or private mapping bytes
coordinator cannot open private material before all-case precommit
every executable in the route is present exactly once in the manifest
missing or foreign executable root fails before nonce
attempt directory collision fails before nonce
nonce write and commitment publication fault paths are closed
generator failure after nonce consumes the attempt and publishes terminal
restart never redispatches
cleanup cannot remove retained evidence
```

The full package tests, strict Clippy, format check, 32 legacy controls, four V3
controls, sixteen V4 controls, sixteen development cases, one/two split,
independent verification, resource bounds and structural routes must pass again
before the successor R9 freeze.

## 9. Freeze And Authorization

The successor R9 freeze adds the confirm owner, public coordinator, private
resolver and terminal owner executable roots. Its capability receipt proves a
complete dry-run with a non-scientific development seed through the exact
process topology, not a boolean assertion.

The dry-run may not use a Confirm split or fresh nonce. It proves transport and
terminal closure only.

After the new freeze:

```text
R10 STOP
-> user explicitly authorizes one attempt against the exact new freeze root
-> confirm owner creates the sole nonce
-> R11 reaches one immutable terminal and cleanup result
```

## 10. Claim Boundary

Repair PASS proves only confirm-route readiness. It is not the sealed scientific
result. The strongest scientific claim remains exactly the V4/V2 bounded
generated-language statement and requires the later one-shot terminal PASS.
