# K0 Discovery Curriculum And Basis Contract V1

Status: preregistered architecture contract. No Law #2 claim.

Date: 2026-08-10.

## Decision

Nando receives a small manually authored language for discovering laws, not
handwritten laws.

```text
K0 Discovery Curriculum
  source-neutral types
  universal typed primitives
  verifier semantics
  bounded circuit-search limits
        |
        v
versioned DiscoveryBasis
        |
        v
operator-blind natural cohort freeze
        |
        v
Raw Phase + bounded circuit synthesis
        |
        v
OperatorIdentificationMachineV1
        |
        v
independent post-freeze future
        |
        v
LawCertificate -> K1
```

Allowed K0 examples are typed role selection, comparison, filtering, counting,
transformation, branching, and rendering. K0 teaches what computations exist.
It does not tell Nando which computation explains a candidate.

Forbidden before identification:

```text
candidate root -> program mapping
source or product family hints
active package names
handwritten FILTER/COUNT/BRANCH choice
manual law credited as natural discovery
```

## Current Signal Path And Blocker

The live root-complete partition before this change is:

```text
37 readiness-PASS cohorts
  29 exact candidate roots already terminal
   7 suppressed by old identity-level duplicate verdicts
   1 mature retained cohort downgraded by global-watermark staleness
   0 schedulable
```

The seven duplicate verdicts were produced before the current Raw Phase
executable frontier existed. Their persisted identity commits the cohort and
active protocol set but not the hypothesis/executable basis. The remaining
cohort has 10 settled and verified rows, 2 independent lineages, and 1,433,011
input tokens, but unrelated traffic moved the global sequence beyond its
recency formula.

Therefore `waiting_for_evidence` is not an evidence-shortage verdict. It is the
composition of two contract defects:

```text
new search language treated as old search language
+ unrelated traffic treated as negative cohort evidence
```

Touched boundaries: discovery freeze, scheduler duplicate disposition, and
pre-freeze readiness. Capture, Wave scoring, actor, verifier, admission,
runtime execution, and economics authority do not change in this repair.

## DiscoveryBasisV1

The basis root must commit at least:

```text
basis schema
natural candidate generator schema
Raw Phase hypothesis generator schema
executable blueprint builder schema
K0 primitive/type algebra schema
verifier-semantics schema
bounded synthesis configuration schema
```

`K1NaturalCandidateFreezeV3` persists that root. Authority independently checks
the current basis before appending a new freeze. Historical V1/V2 freezes remain
byte-decodable and root-stable and are assigned only their explicit legacy
unversioned basis for duplicate comparison. They are never reinterpreted as the
current basis.

An active V3 generation must execute under its exact frozen basis. If the
installed runtime cannot dispatch that basis, it fails closed; deployment may
not silently replace it.

## Ownership

The route has one owner for each decision:

```text
Architecture canon
  defines allowed K0/K1 boundaries; no runtime authority

Operator learning
  constructs and validates DiscoveryBasisV1
  generates hypotheses and executable circuits after freeze
  cannot append a freeze or grant a certificate

K1 scheduler
  ranks operator-blind natural cohorts
  computes basis-aware duplicate dispositions
  cannot select a program or append its own freeze

Certification authority
  independently reconstructs the current basis root
  performs registry and active-protocol CAS
  atomically appends the immutable freeze

OperatorIdentificationMachineV1
  consumes the exact frozen basis and evidence domain
  alone selects, rejects, or probes semantic program classes

Scheduler runtime
  replays the ledger and dispatches the exact frozen basis
  cannot reinterpret an active generation after upgrade
```

The dashboard and reports expose these roots and dispositions but own none of
the decisions.

## Duplicate Semantics

A duplicate blocker is valid only for:

```text
(cohort structural identity, active protocol-mode set, discovery basis)
```

Required behavior:

```text
same identity + same active set + same basis -> excluded
same identity + newer basis                 -> reopened once
terminal under newer basis                  -> excluded again
active-set CAS mismatch                     -> fail closed
```

Changing evidence volume alone does not reopen an identity. Changing an
unrelated registry revision does not reopen it either.

## Cohort-Local Readiness

Natural support is immutable evidence, not a cache entry. It remains freeze
eligible when all of these stay valid:

```text
capture generation
candidate structural identity
evidence manifest
settled and verified minima
independent lineage minimum
fixture and safety exclusion
registry/catalog CAS at freeze
```

Unrelated global traffic cannot turn `readiness PASS` into `readiness rank 0`.
Whether the cohort still recurs is answered prospectively:

```text
freeze at contract watermark
-> accept only sequence >= future_min_sequence
-> natural match or bounded terminal deadline
```

An insufficient cohort remains blocked by its readiness receipt. A mature but
low-frequency cohort may consume one bounded generation and then terminate with
an honest acquisition failure; it may not be erased before the experiment.

## Live Baseline

Read-only snapshot immediately before implementation:

```text
K1 state                         waiting_for_evidence
next generation                  78
scheduler ledger revision        157
readiness-PASS cohorts            37
schedulable cohorts                0
current-epoch ordinary tokens     1,487,535,556
verified avoided tokens             117,403,069
global current-epoch share                  7.8%
verified local accepts                      487
verification coverage                      100%
active false accepts                           0
runtime parity mismatches                      0
missing evidence receipts                      0
hot PID                                  3901227
cold learner PID                         3163368
Nginx PID                                 682430
service restarts                               0
cold learner RSS                   1,978,200,064 bytes
transition state size             33,337,488,712 bytes
local health-call latency                 6.9-9.1 ms
```

## Acceptance Contract

Implementation is complete only when focused and restart tests prove:

1. V1/V2 freeze decoding and historical roots remain stable.
2. V3 freeze binds a valid discovery basis root.
3. Same-basis duplicate suppression remains fail-closed.
4. A changed basis reopens the same cohort exactly once.
5. Unrelated global traffic cannot stale a readiness-PASS cohort.
6. Insufficient evidence still cannot freeze.
7. Registry and active-protocol CAS checks remain authoritative.
8. Installed runtime survives restart with ledger/root parity.
9. Generation 78 freezes automatically or every ready root receives a
   byte-exact terminal veto.

Generation 78 entering identification is an implementation result, not Law #2.
Law #2 still requires exact version-space collapse, independent future,
cleanup, certification, ordinary verified CPU execution, and zero false
accepts.
