# S1C Shadow Producer Paper Verification 2026-08-11

Status: `PASS / S1C-2 SOURCE IMPLEMENTATION ALLOWED / NO DEPLOYMENT`

Verified preregistration: `S1C_SHADOW_PRODUCER_PREREGISTRATION_V1.md`

Adversarial review: `S1C_SHADOW_PRODUCER_CRITIQUE_V1.md`

Parent commit: `d43fc8cd4fcb73e6fb15bcde143a68960272425e`

Parent receipt SHA-256:
`fa29cc86e5610a844080b97a35d3816c73597c45e443a0a80651bbf54050c455`

Frozen evidence manifest:
`evidence/S1C_SHADOW_PRODUCER_PREREGISTRATION_V1/SHA256SUMS`

## 1. Verdict

The S1C-2 paper contract is accepted for one source-only implementation slice.
It freezes the missing runtime joins without manufacturing a natural goal or
granting runtime authority.

```text
S1C-1 pure contracts                    PASS / SOURCE ONLY
S1C-2 preregistration                   PASS
S1C-2 source implementation             NEXT
capture activation                      false
deployment                              forbidden
model training                          false
phase mutation                          false
authority_ready                         false
natural decision surface                NOT OBSERVED
S1C-3                                   BLOCKED BY S1C-2 SOURCE PASS
S1C-4                                   BLOCKED BY S1C-3
S2-S6                                   BLOCKED
```

The exact scientific risk remains visible: ordinary traffic may contain no
eligible exact typed goal. S1C-2 must report `MISSING_EXACT_GOAL`; only S1C-4 may
later conclude `EMPTY_GOAL_SURFACE` from a finite append-cursor census.

## 2. Source Inspection

The review confirmed the parent source at the frozen commit:

```text
ResponseExecutorCache                  executor only; no K1 index
off-path response refresh              existing fingerprint and cache owner
evaluate_pre_action                    implemented
PreparedResponseEvaluation             implemented
execute_prepared                       implemented and consuming
K1ActionIndexV1                         implemented
DecisionAuthoritySnapshotV1            implemented
DecisionContractPrecommitV1             implemented
SelectedActionBindingReceiptV1          implemented, no runtime producer
GoalSatisfactionReceiptV1               implemented, no runtime producer
precommit append + sync + recovery       implemented
selected/satisfaction persistence        missing
natural exact-goal ingress               not proved present
```

The paper therefore repairs four exact integration blockers rather than adding
a new VM, learner, scheduler, or authority.

## 3. Adversarial Review

The critique found and repaired these decisive failure routes before paper
acceptance:

- a hash of request text could masquerade as typed goal evidence;
- selected package, rank, actor output, or future result could create a
  post-hoc goal;
- evidence could prepare once while serving evaluated a second time;
- executor and K1 index could come from torn authority epochs;
- append without sync could be reported as durable;
- precommit could exist without durable selected-action and satisfaction joins;
- HTTP or actor success could be mislabeled as goal satisfaction;
- shadow failure could change serving or invoke a second evaluator;
- a default goal could hide an empty natural goal surface;
- source PASS could be promoted into deployment or K2 authority.

The final preregistration includes every accepted P0/P1 repair. No threshold,
budget, natural-evidence criterion, S1C terminal verdict, or S2 entry criterion
was weakened.

## 4. Structural Gate

The gate ran read-only on the mini-PC:

```text
NANDA version                          6.1.0
core                                   sparse-triad-v6.1-trusted-proof
binary SHA-256                         1309c0d2...27f489c
self-check                             PASS
doctor                                 healthy=true
```

The first pass correctly returned six VETO verdicts because candidate evidence
used different role names from the contract and several groups appeared to have
multiple owners. Those six inputs and results are retained as
`*.initial-veto.*`.

After subject/relation normalization, three routes passed and three remained
VETO due exact role/object mismatches. Those intermediate failures are retained
as `*.repair1-veto.*`. One final authority packet remained VETO after a packet
edit accidentally assigned the publication owner the identity role; that pair
is retained as `authority-snapshot.repair2-veto.*`.

No VETO was accepted. No scientific contract text changed during packet repair.
The final route set is:

| Route | Verdict | Complexity | Stable | Conflicts | Weak | Repairs | Safe to edit | Authority |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Goal ingress | PASS | 20 | 4 | 0 | 0 | 0 | true | false |
| One evaluator | PASS | 18 | 4 | 0 | 0 | 0 | true | false |
| Atomic authority snapshot | PASS | 18 | 4 | 0 | 0 | 0 | true | false |
| Persistence and serving | PASS | 22 | 5 | 0 | 0 | 0 | true | false |
| Terminal receipts | PASS | 20 | 4 | 0 | 0 | 0 | true | false |
| Slice boundary | PASS | 24 | 5 | 0 | 0 | 0 | true | false |

Final packet roots:

```text
goal-ingress input                     c18d64eb...a77342f
goal-ingress result                    c765de78...b346f6d
one-evaluator input                    5cbd9cb0...560b6f7a
one-evaluator result                   792af93a...210c7d72
authority-snapshot input               a68b8b81...4a29895d
authority-snapshot result              2379a87f...4cdfe5fd
persistence-serving input              0af0802d...15a31612
persistence-serving result             17bdb14f...d9ff5ef2
terminal-receipts input                3bed977c...5361439e
terminal-receipts result               946e9d6d...9704c98bf
slice-boundary input                   ab70629d...2b36a6c5
slice-boundary result                  c8a0a1e2...3d119e0f
```

Every final packet has:

```text
verdict                                PASS
WATCH                                  0
conflicts                              0
foreign pull                           0
owner conflicts                        0
negative hits                          0
repair queue                           0
authority_ready                        false
```

These are coherence-only receipts. They grant no real-world, scientific,
runtime, deployment, or execution authority.

## 5. Runtime Non-Mutation Check

No build, install, service command, feature activation, journal creation, or
deployment was performed. The read-only post-check found the same process
identities as the parent S1C-1 receipt:

```text
remote transport PID / restarts        682430 / 0
transition-serving PID / restarts      165670 / 0
response-learning PID / restarts       369456 / 0
gateway-control PID / restarts         1035203 / 0
certification authority PID / restarts 164668 / 0
all five services                      active / running

local connector PID / restarts         2919 / 0
local connector                        active / running
route receipt failures                 0

remote CPU mode                        CPU
remote admission                       PASS
response active profiles               2
false accepts                          0
```

The current `/cpu-health` schema does not expose a runtime-parity-failure field,
so this paper check makes no new live parity assertion. The parent byte-parity
receipt remains the source oracle for the later S1C-2 candidate tests.

`graphify-out/` remained pre-existing, untracked, and untouched.

## 6. Accepted Next Boundary

The only newly allowed action is:

```text
S1C-2 source implementation
-> false-by-default feature
-> exact goal allowlist and denylist
-> atomic executor plus K1 index snapshot
-> one prepared evaluator
-> synced precommit
-> same prepared execution
-> synced selected-action and satisfaction receipts
-> source tests, parity, resources, and structural gates
```

Still forbidden:

```text
deployment
capture activation
natural census
model training
phase mutation
LawCertificate or K2 claim
admission or certification change
dashboard claim
```

The next paper-first stage after `S1C2_SOURCE_PASS` is S1C-3 transactional
deployment and restart parity. S1C-4 natural census remains separate.
