# STOP-MS3 Linked-Frame Acquisition

Date: 2026-07-26 Europe/Tallinn.

## Live Status

```text
bounded acquisition implementation       PASS
durable contract                         PASS
contract restart byte identity           PASS
topology watermark                       1832
new topology rows                         0 / 256
terminal receipts                         0
relevant verified frames                  0
immutable linked receipts                 0
A/B/C/D verdicts                          0
verdict                                  COLLECTING
authority_ready                          false
phase_update_allowed                     false
```

This experiment answers one question only:

```text
Can fresh ordinary topology rows reach a verified completed RelationFrame
through the existing transport join?
```

It does not change the classifier, Wave, DSL, phase memory, admission or any
ACTIVE package.

## Frozen Contract

```text
schema                   nando.ms3-linked-frame-acquisition-contract.v1
contract root            df7604216002cb4b66372e5158b04bc194559d37c30d7eba1349fde6cf3c44d2
topology prefix root     9d5602c169be6e4f62b5af268830ebdeec8b34d1157bd14c107290b0e23d5cae
topology watermark       1832
opened                   2026-07-26T19:44:23+03:00
deadline                 2026-07-27T19:44:23+03:00
maximum new topologies   256
classifier version       nando.representation-gap-classifier.v1
```

The experiment terminates when either:

```text
first immutable linked-frame receipt exists

or

256 evaluated topology rows all have terminal receipts and linked rows = 0

or

the 24-hour deadline expires and linked rows = 0
```

An in-flight topology without a terminal receipt does not prematurely exhaust
the row budget. A frame observed after the frozen deadline cannot rescue the
experiment.

## Immutable Receipt

Every successful link seals:

```text
acquisition contract root
topology commitment root
completed frame root
terminal receipt root
transport binding root
session lineage
session identity
turn intent identity
request event identity
action event identity
classifier version
optional A/B/C/D adjudication root
```

No representation-gap result is forced when the linked frame is directly
expressible by the existing source-neutral language. Such a receipt is
explicitly counted as `no_representation_gap`.

## Failure Meaning

If the bounded experiment ends with no linked receipt:

```text
MS3_LINKED_FRAME_ACQUISITION_FAIL
```

The permitted repair owner is then capture/join reachability. The verdict does
not authorize changes to the classifier, Wave, DSL, applicability rules or
anti-centers.

Terminal receipt without a relevant verified frame is censored acquisition
evidence, not negative operator evidence.

## Operational Capture Guard

The immutable acquisition verdict is separate from transport health. A
read-only operational monitor compares post-open ordinary opportunity rows with
the append-only pre-action topology archive:

```text
ordinary traffic observed
+ topology delta = 0
+ first ordinary row older than 300 seconds
-> CAPTURE_STALLED
```

`CAPTURE_STALLED` is not
`MS3_LINKED_FRAME_ACQUISITION_FAIL`. It permits repair of capture, bridge or
join reachability only. It cannot change the acquisition contract, classifier,
row budget, deadline, phase memory or authority.

Live inspection found the first real operational fault:

```text
ordinary opportunity events continued
provider capture records              16,384 / 16,384
provider capture phase                blocked_fail_closed
provider capture error                provider_capture_append:BudgetExhausted
new topology rows                     0
```

The provider capture object was documented as a bounded rolling index, but the
implementation never evicted its oldest prefix. The repair keeps the same
`16,384`-record and `8 MiB` budgets, preserves monotonic sequence leases, and
allows only byte-identical suffix retention plus newly appended sequences.
Middle deletion, record rebinding and sequence reuse remain rejected.

Operational endpoint:

```text
http://127.0.0.1:18790/v2/multi-source/ms3-capture-health
```

## Verification

```text
nando-operator-learning                 279 / 279 PASS
nando-transition-serving                122 / 122 PASS, 1 perf ignored
F7 generation shadow                      7 / 7 PASS, 1 perf ignored
acquisition causal tests                  3 / 3 PASS
acquisition persistence tests             2 / 2 PASS
Clippy -D warnings                            PASS
rustfmt / diff check                          PASS
NANDA composite                               PASS
```

Deployment:

```text
source commit             2e718a4
cold binary SHA-256       e12c06a79e1f4cdd3891725fcd948e830e462de64b7afe902ec7677388c4d8a4
cold invocation           d8292912724441c5aa7e056ff439bb38
cold NRestarts            0
hot serving PID           3061210
hot serving restarted     no
ACTIVE packages           2
false accepts             0
runtime parity failures   0
```

Read-only live endpoint:

```text
http://127.0.0.1:18790/v2/multi-source/ms3-linked-frame-acquisition
```
