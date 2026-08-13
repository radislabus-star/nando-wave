# K1 Restart Recovery Bounded Authority Wire Preregistration V1

Status: `FROZEN BEFORE IMPLEMENTATION`

Date: `2026-08-13`

## Plain-Language Problem

The K1 scheduler has completed 555 epistemic generations and must select
generation 556. It repeatedly fails before the authority receives a request:

```text
epistemic scheduler ledger       revision 1113
completed generations            555
next generation                  556
active candidate                 none
cold retry interval              about 101 seconds
client error                     k1_scheduler_authority_request_budget
authority observation            empty connection
authority / phase mutation       false / false
```

`send_authority_request()` connects to the Unix socket, serializes the request,
then rejects any payload larger than 4 MiB. The dropped connection therefore
appears to the authority as an empty request. This is a transport failure after
candidate selection, not absence of natural evidence and not a scientific
verdict about Law #2.

## Frozen Measurement

Before changing the wire, reproduce one K1 tick against a read-only or isolated
copy of the production evidence and scheduler state. Record, for every request
that the tick attempts:

```text
request schema
uncompressed canonical JSON bytes
wire bytes
configured wire limit
ledger revision and root before the attempt
projection root before the attempt
selected candidate root, if any
```

The measurement must identify the exact oversized schema. A size estimate from
`structural-frontier-census-v2/latest.json` is insufficient because that report
is published only after a successful runtime tick and can lag the failing
in-memory frontier.

## Frozen Change

Only the oversized K1 authority request may gain a V2 bounded wire. All other
authority request schemas and scientific state machines remain unchanged.

The V2 wire must carry the same logical data that V1 validated, but large
frontier material may be represented as a bounded compressed canonical blob.
The authority, not the caller, must:

1. enforce the compressed and decompressed byte ceilings;
2. decode the complete logical request with `deny_unknown_fields`;
3. validate catalog, deficit snapshot, queue, candidate and freeze;
4. recompute queue derivation from the decoded catalog and current exclusions;
5. restore the current certification ledger and perform registry CAS;
6. restore the scheduler ledger and perform generation/projection CAS;
7. reseal the candidate freeze and require byte-equivalent logical content;
8. append only the existing `CandidateFreeze` event after every check passes.

The compression envelope is transport only. It is not evidence, authority,
admission, a new root of truth, or a change to candidate ranking.

## Frozen Budgets

```text
outer JSON wire                          <= 4 MiB
compressed frontier blob                 <= 3 MiB
decompressed logical freeze request      <= 16 MiB
catalog candidates                       existing catalog model limit
queue rows                               existing queue model limit
decompression                            exact bounded allocation
unknown fields                           reject
trailing bytes                           reject
checksum or logical root mismatch        reject
```

The implementation must not raise the existing 4 MiB transport limit. If the
measured logical request does not fit the frozen 16 MiB decompressed ceiling,
implementation stops and a new paper contract is required.

## Restart And Idempotency Contract

The production ledger copy is the restart oracle:

```text
before candidate append
  completed_generations        555
  next_generation_sequence     556
  active_candidate             false
  authority_ready              false
  phase_mutation_allowed       false

after one valid V2 append
  exactly one generation-556 CandidateFreeze event
  active candidate root equals the independently measured selection
  authority_ready              false
  phase_mutation_allowed       false

same request repeated
  no second event
  identical ledger revision, ledger root and projection root

restart
  identical ledger revision, ledger root and projection root
  identical active generation-556 freeze
```

Crash before journal append leaves the old `555 / 556` projection. Crash after
the signed journal append is recovered by the existing journal/anchor protocol.
No new sidecar may become a competing recovery authority.

## Required Negative Tests

```text
oversized outer wire                     reject before decode
oversized compressed blob                reject
decompressed output above 16 MiB          reject
truncated compressed blob                reject
trailing compressed bytes                reject
unknown envelope or logical field        reject
catalog root mismatch                    reject
queue derivation mismatch                reject
candidate not selected by queue          reject
registry revision/root stale             reject
scheduler revision/root stale            reject
active protocol mode root stale          reject
freeze reseal mismatch                   reject
generation other than 556 in oracle      reject
same valid request twice                 idempotent
restart after valid append               byte-identical projection
```

## Deployment Scope

The candidate release may replace and restart only:

```text
nando-operator-certification-authority.service
nando-response-learning.service
nando-gateway-control.service
```

It must not restart or modify hot serving, Nginx, the local connector, S1C-4,
natural evidence archives, phase memory, active packages, or economics.

Deployment readiness has three separate timeouts:

```text
base cold health              up to 120 seconds
K1 scheduler summary          up to 240 seconds
control dashboard v21         up to 60 seconds after K1 summary
```

The first failed deployment proved that a 20-second cold timeout is invalid for
the current restart workload; it did not prove the release unhealthy.

## Live Acceptance

```text
K1 summary HTTP                         200
completed generations                  555 or later natural terminal progress
next generation                        556 or later consistent progress
runtime_pending                        absent
authority request budget errors        zero after cutover
authority empty-request errors         zero after cutover
authority_ready / phase mutation       false / false
false accepts / parity failures        0 / 0
S1C-4 report SHA-256                    unchanged
hot PID                                unchanged
Nginx PID                              unchanged
connector PID and restart count        unchanged
dashboard                              control-v21, desktop/mobile no overflow
```

Natural traffic may move append-only counters during deployment. It may also
settle generation 556 after a valid freeze. Such progress is observed, not
rolled back, provided every state transition validates against the ledger.

## Claim Boundary

This slice can prove that K1 restart recovery crosses its bounded authority
wire and resumes the already preregistered scheduler. It cannot prove Law #2,
grokking, answer quality, Wave causality, K2 meaning, or commercial savings.
