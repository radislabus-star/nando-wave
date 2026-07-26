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

The same endpoint independently watches the next operational edge by exact
`request_event_id`:

```text
post-watermark topology without terminal receipt
+ age below 300 seconds
-> IN_FLIGHT

post-watermark topology without terminal receipt
+ age at least 300 seconds
-> RECEIPT_STALLED
```

`RECEIPT_STALLED` preserves the frozen denominator, watermark and deadline. It
permits repair of topology-to-terminal association only. An uncovered row is
never converted into negative evidence, and the scientific report remains
unchanged.

## Verification

```text
nando-operator-learning                 281 / 281 PASS
nando-transition-serving                127 / 127 PASS, 1 perf ignored
provider capture index                     7 / 7 PASS
provider capture persistence               5 / 5 PASS
capture health + acquisition               7 / 7 PASS
Clippy -D warnings                            PASS
rustfmt / diff check                          PASS
NANDA composite                               PASS
```

Deployment:

```text
source commit             a579628
deployed binary SHA-256   b463e0fc11fe0ee0364ece99a4428e7179f4cdf7a04e1fac228ddaf78789fb31
cold invocation           d6b258e9dfa74ce084220768b956f796
hot invocation            aec3a81260c648ed84538066131715c6
ACTIVE packages           2
false accepts             0
runtime parity failures   0
```

Live post-repair receipt:

```text
provider capture phase       ready_hash_only
provider capture records     16,384
captured / censored          2 / 0
persistence failures         0
topology rows                1,832 -> 1,834
terminal receipts            2
capture health               CAPTURE_PROGRESS
scientific verdict           collecting
linked frames                0
authority / phase mutation   false / false
```

Read-only live endpoint:

```text
http://127.0.0.1:18790/v2/multi-source/ms3-linked-frame-acquisition
```
