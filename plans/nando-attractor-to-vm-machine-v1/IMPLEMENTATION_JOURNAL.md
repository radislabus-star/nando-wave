# Nando Attractor-to-VM Implementation Journal

This is the resumable execution log for the canonical roadmap. Each entry
records the exact boundary, evidence, modification, checks, remaining blocker,
and measured command time. A code change is not complete until its entry has a
check result.

## 2026-07-19 15:37 EEST - Stage 8 Runtime Role Grounding, Pass 1

Starting state:

```text
HEAD ca80c15
production services untouched
graphify query wall time 2.98 s
focused source inspection commands < 0.2 s total
```

Confirmed defects:

```text
observed_multi_role_runtime_surface
  derived output role, relation planes, program atoms, and role order from the
  sealed operator TransformProgram instead of raw pre-action observation

independently_bind_verifier
  ignored RoleGraph and RelationProgram and verified a caller-bound selector
```

Implemented in the current worktree:

```text
raw observation bundle now contains only context + observed source roles
raw observation bundle contains no virtual output and no program atoms
RuntimeRoleBinder maps this partial observed graph into the sealed RoleGraph
verifier re-extracts ordinal roles from raw request/output
verifier independently reruns circuit-constrained CSP binding
```

Current check state:

```text
local stable rustfmt: ENVIRONMENT_UNAVAILABLE in 0.18 s (missing rustc driver)
Rust 1.97 fmt check: one formatting delta in 4.42 s
Rust 1.97 formatting applied: 2.95 s
remote compile: PASS in 40.54 s, linker peak RSS 1,823,016 KB
pre-action observation test: PASS 1/1 in 0.08 s
64-row rich role/admission proof: PASS 1/1 in 34.18 s
scalar crystallization rerun: FAIL in 33 s, MissingRuntimeAnchor
scalar crystallization after source-neutral signature repair: PASS 1/1 in 26.22 s
rich regression after scalar repair: PASS 1/1 in 25.86 s
rich reversed-request ordinal proof: PASS 1/1 in 33.67 s (60 s with relink)
remote cargo check --lib: PASS in 15.99 s
remote crate-wide Clippy -D warnings: ENVIRONMENT/BASELINE BLOCK in 36 s
  11 pre-existing warnings outside crystallized_operator.rs
  no warning reported in the changed module
```

Scalar failure diagnosis:

```text
historical scalar blueprint source role constraint mask = 2
runtime selector-specific role mask = 2 | selector class
single unique scalar has no observable selector-class law
repair: keep concrete selector only as ephemeral anchor and expose a
source-neutral scalar role signature to the circuit binder
```

Independent verifier audit:

```text
wrong: retain only mappings whose response already equals actor response
right: compute every independently grounded response class, require exactly
       one class, then compare that class with the actor response
multiple mappings with one action remain legal; multiple actions ABSTAIN
```

Stage 8 pass boundary:

```text
raw multi-role surface excludes output/program atoms                 PASS
raw scalar surface excludes output/program atoms                     PASS
circuit-constrained partial-role CSP                                 PASS
independent raw re-extraction and response-class verification         PASS
equal support values -> diverging future role proof                   PASS
renamed fields and reversed request ordinals                          PASS
restart + external laboratory admission                              PASS
production deployment                                                 NOT RUN
remote graphify update                                                 ENVIRONMENT_UNAVAILABLE
  wrapper shebang points to missing /home/ubu Python environment
  no replacement Python route installed or enabled
```

Next boundary: Stage 9 admission must recompute authority from sealed receipts
instead of trusting externally deserialized candidate booleans or counters.

Next action:

```text
remove the same operator-derived virtual output/program atom from scalar runtime
preserve independent scalar verification
rerun focused scalar and rich proofs
```

## 2026-07-19 - Stage 7 Minimal Operator VM

Baseline inspection:

```text
memory lookup                                      0.30 s
Graphify execution-route query                     1.81 s
scoped source and AST inspection                   1.50 s total
HEAD before change                                 2cd7e11
worktree                                           clean except graphify-out/
```

Confirmed execution gap:

```text
OperatorPage32 contained TransformOp8 bytecode
BoundCrystallizedOperator executed the side-registry ResponseProgram
therefore page bytecode was proof payload, not the cause of execution
```

Implemented MVP boundary:

```text
OperatorPage32 TransformOp8  -> opcode, source order, output, format
BoundRoleEnvironment         -> runtime selector operands
sealed actor renderer        -> response shape only
Operator VM                  -> computed response
legacy actor                 -> temporary parity oracle
independent verifier         -> final truth check
```

Fail-closed limits:

```text
only PROJECT_UNIQUE_SCALAR is currently executable
unknown opcode or transform flags                 ABSTAIN
missing or duplicate source roles                 ABSTAIN
RequestTemplate renderer                          ABSTAIN
ambiguous UniqueConsensus response                ABSTAIN
VM/reference actor mismatch                       ABSTAIN
independent verifier mismatch                     ABSTAIN
```

This stage deliberately does not add count/filter/compose opcodes. First the
existing crystallized relation circuit must become the actual execution cause.

Implementation and focused verification:

```text
Rust 1.97 fmt check                               PASS 3.06 s
remote source sync                                0.54 s
remote cargo check --lib                          PASS 5.85 s
  compiler peak RSS                               832,912 KB
initial test relink                                28.95 s
  exact-name filter mistake                       0 tests (not counted)
scalar crystallized VM proof                      PASS 1/1 0.13 s
rich multi-role VM proof                          PASS 1/1 25.53 s
VM causal tests first run                         1/2 PASS 25.06 s
  fixture used root JSON instead of tool output; ProjectionFailed
VM causal tests with production payload shape     PASS 2/2 17.05 s
rich integration after VM causal fixture          PASS 1/1 25.55 s
  test-process peak RSS                           38,300 KB
```

Stage 7 result:

```text
page transform bytecode drives scalar execution                  PASS
page transform order drives rich multi-role rendering            PASS
unknown opcode fails closed                                      PASS
role-grounded operands, not actor selectors, feed VM             PASS
legacy actor parity guard                                        PASS
independent verifier                                             PASS
new count/filter/compose opcodes                                  NOT STARTED
production deployment                                             NOT RUN
```

Graph maintenance:

```text
graphify update .                                  PASS 26.08 s
  graph                                             22,914 nodes / 51,107 edges
  one-shot indexer peak RSS                         536,476 KB
graph query OperatorPage32 -> VM -> verifier        PASS 1.84 s
```

Next architectural boundary: finish Stage 9 capture-owned admission
provenance, then expose this VM operator in generic scalar live shadow before
adding broader opcodes.

## 2026-07-19 - Stage 9 Capture-Owned Admission Provenance

Confirmed boundary:

```text
external admission already resynthesized candidate programs and proof counters
but TeacherTransition rows were not committed to the capture-owned hash chain
evidence_ref_sha256 was an observation-output digest, not a ledger commitment
```

Implemented streaming proof route:

```text
StreamingEvidenceLedger record
-> bounded per-turn ordered record commitments
-> CaptureEvidenceReceipt in RuntimeParityCase
-> bounded capture-commitment-index.cbor
-> external admission reads index independently
-> every crystallized support/future receipt must be indexed
-> candidate resynthesis and ordinary proof gates continue unchanged
```

Budgets and behavior:

```text
turn receipt maximum                               512 records
capture index maximum                              16,384 records
index persistence                                  existing 64-event / 5-second checkpoint
ordinary startup history scan                      none
missing, stale, tampered, or unindexed receipt     BLOCK
old historical rows                                support only; cannot obtain new authority
```

Focused verification:

```text
fmt application                                    2.97 s
remote source sync                                 0.47 s
combined package check                             BASELINE BLOCK 14.25 s
  unrelated nando-response-miner has 3 old non-exhaustive selector matches
actor lib + external admission check               PASS 12.72 s
serving lib check                                  PASS 0.12 s
capture receipt/index tests                        PASS 2/2 13.39 s
streaming ledger restart/index test                PASS 1/1 17.27 s
live-shaped two-turn capture receipt join          PASS 1/1 2.84 s
rich candidate/resynthesis regression              PASS 1/1 10.94 s
```

Stage 9 result:

```text
capture-owned bounded commitment index             PASS
turn evidence receipt                              PASS
external admission provenance check                PASS
tamper/missing receipt fail closed                 PASS
candidate resynthesis retained                     PASS
production deployment                              NOT RUN
```

Graph maintenance:

```text
graphify update .                                  PASS 24.74 s
  graph                                             22,938 nodes / 51,194 edges
graphify capture-to-admission path                 PASS 1.74 s
```
