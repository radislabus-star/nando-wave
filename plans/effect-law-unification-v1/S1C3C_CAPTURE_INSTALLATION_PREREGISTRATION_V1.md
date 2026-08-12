# S1C-3C Capture Installation Preregistration V1

Status: `PROSPECTIVE SUCCESSOR / PAPER ONLY / IMPLEMENTATION AUTHORITY FALSE`

Date: `2026-08-12 Europe/Tallinn`

Parent source state: `b9e68da940bdf3f140a918c066126f52573f95a2`

## 1. Exact Question

Can the already implemented, false-by-default S1C pre-action decision capture
be installed under ordinary mini-PC load while preserving the frozen product,
resource, rollback, connector, and authority boundaries?

S1C-3C is a new prospective protocol. It does not reopen, retry, relabel, or
repair the historical verdict of S1C-3B.

```text
S1C-3B attempt                  consumed
S1C-3B verdict                  PREFLIGHT_FAILURE
S1C-3B resource verdict         null
S1C-3B deployment verdict       null
S1C-3B production mutation      false
S1C-3B capture installed        false
```

## 2. Scoped Claim

The maximum positive claim is:

> S1C capture was transactionally installed from the frozen candidate after
> the unchanged absolute resource gate passed, and production safety survived
> restart and rollback verification.

The transaction cannot prove a natural goal, an alternative, a decision
episode, grounded meaning, K2, model quality, grokking, a new K1 law, or CPU
savings.

## 3. Prospective Authority Boundary

```text
committed local schema preflight
-> no-side-effect dry fixture PASS
-> one S1C-3C transaction ID
-> frozen ordinary-load measurements
-> independent receipt verification
-> resource VETO
   or transactional capture install + attributed transition restart
-> rollback on any post-arm failure
-> operational deployment receipt
```

The local schema preflight must finish before all of:

- timestamp or transaction ID creation;
- local evidence-directory creation;
- SSH or SCP;
- remote attempt enumeration;
- remote lock acquisition;
- build, measurement, service restart, or production mutation.

A local preflight failure consumes no S1C-3C production attempt because no
remote transaction exists. Once a remote transaction directory is created, the
sole S1C-3C attempt is consumed regardless of its terminal verdict.

## 4. Frozen Candidate And Mechanism

S1C-3C reuses the frozen S1C-3B measurement/deployment mechanism as a
non-authoritative implementation dependency. It does not execute the old
S1C-3B launcher.

```text
candidate commit
  03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree
  06a9df51797dffc127fec41672bddae29c38bb92
production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
oracle source SHA-256
  bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26
oracle Cargo.lock SHA-256
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
```

The implementation freeze must additionally bind exact SHA-256 identities for
the reused executor, reused independent verifier, S1C-3C local preflight,
S1C-3C authority wrapper, S1C-3C launcher, tests, paper, critique, structural
receipts, candidate config, and source bundle commit/tree.

## 5. Local Schema Contract

`S1C3CSchemaPreflightV1` is pure and fail-closed. It must validate every metric
family before remote work:

```text
metric family
-> exact observed fixture line
-> regex full contract match
-> capture group count == declared field count
-> typed parse has exact declared key set
-> complete dry evaluator accepts the row shape
-> one mutation per declared field is observed by shape or verdict validation
```

Declared tuples remain:

```text
hot
  p99_ns, no_goal_p99_ns, hard_max_ns, samples
single sync
  p99_ns, hard_max_ns, samples, segments
three-ledger sync
  precommit_p99_ns, precommit_hard_max_ns,
  settlement_p99_ns, settlement_hard_max_ns,
  episode_p99_ns, episode_hard_max_ns, samples
idle
  elapsed_ticks, ticks_per_second, percent_of_one_core
```

Diagnostic fields must be typed and retained even when they do not alter a
threshold verdict. No undeclared, missing, renamed, or unconsumed receipt field
may pass the preflight. The exact observed S1C-3B idle log is a mandatory
regression fixture.

## 6. Unchanged Resource And Denominator Contract

```text
measurement CPU                         fixed logical CPU 4
rounds                                  exactly 3
warmup, retry, CPU shopping             forbidden
hot matched p99                         <= 1,000,000 ns, PASS 3/3
hot no-goal p99                         <=   250,000 ns, PASS 3/3
hot hard max                            <= 2,000,000 ns, PASS 3/3
single-ledger p99                       <= 5,000,000 ns, PASS 3/3
precommit p99                           <= 5,000,000 ns, PASS 3/3
settlement p99                          <= 5,000,000 ns, PASS 3/3
each durability hard max                <= 20,000,000 ns
aggregate episode hard max              <= 20,000,000 ns
capture-disabled idle CPU               <= 0.25% core
capture-on minus capture-off RSS        <= 16 MiB
ordinary output parity                  byte-identical, 16/16
false accepts / runtime parity          0 / 0
```

Filesystem-floor observations remain diagnostic. Previous S1C-3B logs may be
used only as parser regression fixtures, never as resource evidence or to
adapt thresholds.

## 7. Transaction And Rollback Contract

```text
S1C-3C remote attempts                  exactly one
automatic S1C-3D                       forbidden
production traffic intervention        forbidden
synthetic or targeted requests         forbidden
transition-serving restart             allowed once, attributed
gateway-control restart                forbidden by this transaction
learning/certification restart          forbidden
transport gateway/Nginx restart        forbidden
local connector restart                forbidden
```

Rollback is armed only after local and remote independent predeployment
verification agree. Any post-arm error restores the prior transition binary,
config, journal prefix, and service state. A connector PID/restart/receipt
failure change forces rollback and VETO. A resource failure before mutation is
terminal `RESOURCE_VETO` and performs no deployment.

## 8. Terminal Outcomes

```text
LOCAL_SCHEMA_VETO
  no remote attempt exists; production unchanged

PREFLIGHT_FAILURE
  remote attempt exists but resource authority was not established;
  production unchanged

RESOURCE_VETO
  complete frozen measurements fail an unchanged bound;
  production unchanged

ROLLBACK_PASS
  mutation began and prior production was restored exactly

VETO
  safety or connector invariant failed; rollback required

DEPLOYMENT_PASS
  capture installation only
```

No terminal outcome automatically starts another deployment protocol.

## 9. Post-Deployment Scientific Boundary

Only `DEPLOYMENT_PASS` may open S1C-4 as `COLLECTING` with a new immutable
cursor. The census is natural and read-only, with no targeted request creation.
It terminates at the earliest of:

```text
10,000 ordinary decision-boundary surfaces
72 hours from the frozen opening cursor
safety VETO
```

Allowed S1C-4 verdicts are `EMPTY_GOAL_SURFACE`,
`EMPTY_ALTERNATIVE_SURFACE`, `INSUFFICIENT_LINEAGES`, `PASS`, or `VETO`.
S2 remains blocked unless S1C-4 is `PASS`, at least two independent decision
lineages exist, and at least two independently realized K1 laws are available
for any composition claim.

## 10. Implementation Entry Gate

Implementation authority remains false until all are present and committed:

- this preregistration;
- independent adversarial critique with accepted repairs;
- owner-scoped NANDA structural routes with no repair queues;
- paper verification with exact roots;
- tests proving no remote side effect precedes local schema PASS;
- tests proving one S1C-3C attempt and no old S1C-3B launcher invocation.

Structural coherence never grants deployment authority. The independent
transaction verifier owns operational authority, and scientific authority
remains false.
