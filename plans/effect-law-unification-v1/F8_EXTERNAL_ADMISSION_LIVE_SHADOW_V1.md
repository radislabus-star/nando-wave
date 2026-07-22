# F8 External Admission And Live Shadow V1

Status: `STOP-F8 PASS / CONTROLLED LIVE SHADOW / AUTHORITY FALSE`

Date: `2026-07-22`

## Objective

Close the first real request path without creating a second owner of semantic
truth:

```text
provider request bytes
-> trusted hash-only capture
-> pinned F7 generation
-> F5 role grounding and actor
-> independent F6 verifier
-> generation-owned shadow receipt
-> external reconstruction
-> admission candidate
-> live structural gate
-> SHADOW only
```

F8 does not grant production authority. A later explicit rollout decision may
consume a passing STOP-F8 receipt, but it is not part of this implementation.

## Non-Negotiable Ownership

```text
nando-transition-serving
  owns provider-boundary ingress and nonblocking capture enqueue

nando-operator-learning
  owns capture commitments and generation evidence partitions

nando-operator-runtime
  owns bounded role grounding, phase selection and actor execution

nando-operator-proof
  owns independent result verification

nando-operator-persistence
  owns atomic capture and receipt storage

nando-operator-admission
  owns independent reconstruction and candidate verdicts

nando-live-transition-gate
  owns the final project-level structural veto
```

Forbidden ownership:

```text
capture owner choosing an operator
actor manufacturing capture provenance
verifier trusting actor-selected roles or expected output
admission compiling a missing law or selector
serving upgrading candidate to authority
control page becoming a status authority
```

## F8-0: Resource Truth

Current evidence:

```text
F5 2048-mode process RSS delta    49.6 MiB
target                            16.0 MiB
verdict                           WATCH
```

The current number is conservative because artifact construction, dispatch
compilation and the hot generation are measured in one unit-test process. F8-0
must separate live retained state from compiler scratch without hiding real
allocator retention.

Required measurements:

```text
system allocator control
production mimalloc control
fresh process before load
fresh process after canonical generation load
after compiler scratch is dropped
after 128 warmup requests
after generation swap while old pins are released
```

Required accounting:

```text
canonical artifact bytes
compiled modes
dispatch masks
generation wrapper
allocator/process residual
```

STOP-F8-0:

```text
measured retained hot RSS <= 16 MiB
or
representation change with byte-identical behavior and a new frozen budget
```

Re-labelling the existing 49.6 MiB result as PASS is forbidden.

F8-0 result: `STOP_F8_0_RESOURCE_TRUTH.json`. The production allocator policy
reduces the measured peak to at most 10,723,328 bytes across twelve resource
observations. Latency remains a separate F8-D WATCH.

## F8-A: Live Provider Capture Owner

The HTTP handler already computes SHA-256 over the exact provider request
bytes. F8-A reuses that digest and does not parse or hash the body a second time
on the request path.

Capture output:

```text
ProviderRequestCaptureReceiptV3 {
    capture_sequence,
    capture_epoch_root,
    lineage_root_sha256,
    event_root_sha256,
    request_root_sha256,
    projection,
    streaming,
    observed_at_unix_ms,
    receipt_sha256,
}
```

The receipt stores no request text, JSON values, provider payload, teacher
output or actor result. Downstream evidence must carry these exact roots; it
may not recompute a convenient replacement.

Ingress behavior:

```text
existing body hash
-> allocate monotonic capture sequence
-> construct commitment
-> try_send to one bounded writer
-> continue provider fallback immediately
```

The request path reports `ENQUEUED`, not `CAPTURED`. `CAPTURED` is a terminal
writer-owned state recorded only after the inactive slot has been published and
read back byte-identically. An enqueued receipt may feed F7 telemetry, but it is
not durable evidence until F8-B joins it against the published capture index.

Outcomes:

```text
CAPTURED
CENSORED_DISABLED
CENSORED_QUEUE_FULL
CENSORED_DISCONNECTED
CENSORED_BUDGET
CENSORED_INVALID_PROVENANCE
```

Censored outcomes never update positive centers, anti-centers or operator
applicability.

Persistence:

```text
bounded canonical index       <= 16,384 records / 8 MiB
writer queue                  <= 48
file mode                     0600
publication                   inactive slot + fsync + rename + dir fsync
raw payload bytes written     0
```

STOP-F8-A:

```text
request-path blocking writes                 0
second request-body hash                     0
raw payload bytes persisted                  0
restart sequence reuse                       0
duplicate event/request roots                BLOCK
queue overload semantic updates              0
```

F8-A controlled implementation result:
`STOP_F8_A_PROVIDER_CAPTURE_OWNER.md`. All six STOP invariants pass in focused,
restart, privacy and structural tests. The feature remains disabled by default;
no live provider capture, deployment, service restart or authority change was
performed. Therefore the implementation boundary is `CONTROLLED_PASS` and the
live boundary remains `NOT_RUN`.

## F8-B: Generation Shadow Receipt Ledger

The F7 worker currently evaluates and counts a receipt. F8-B makes the receipt
durable without giving it authority.

```text
capture receipt
+ pinned CanonicalGenerationId
+ F5 traffic receipt root
+ exact actor action/output roots
+ F6 independent verifier receipt root
+ terminal outcome
-> GenerationShadowReceiptV3
```

The durable ledger is hash-only. It distinguishes:

```text
VERIFIED_PASS
RUNTIME_ABSTAIN
RUNTIME_REJECT
VERIFIER_ABSTAIN
VERIFIER_REJECT
CENSORED
```

Only `VERIFIED_PASS` may become positive future evidence. A reject can become
negative evidence only after the outcome owner classifies it as an
applicability error or hard contradiction. Timeout, overload, missing capture
and unavailable environment remain censored.

STOP-F8-B:

```text
generation/request/capture join        EXACT
receipt append after restart           MONOTONIC
duplicate receipt                      BLOCK
foreign generation                     BLOCK
unverified PASS relabel                 BLOCK
raw payload bytes persisted            0
execution authority                    false
```

F8-B result: `STOP_F8_B_GENERATION_SHADOW_LEDGER.md`. The exact provider
capture, F5 traffic and complete hash-only F6 receipt now enter a separate
generation-owned atomic ledger. Restart is byte-identical and no receipt grants
authority.

## F8-C: External Admission Reconstructor

`nando-operator-admission` receives immutable bytes, not trusted Rust objects:

```text
generation checkpoint bytes
capture index bytes
shadow receipt ledger bytes
phase-control receipt bytes
resource receipt bytes
```

It independently performs:

```text
decode canonical generation
recompute generation and artifact-set roots
rebuild structural dispatch index
recompute support/future partition roots
join every F6 receipt to capture provenance
join every live shadow receipt to one pinned generation
recompute actor/verifier/action-equivalence roots
validate resource and phase-control gates
```

Output:

```text
ExternalGenerationAdmissionCandidateV3 {
    generation_id,
    reconstructed_roots,
    support_denominator,
    future_denominator,
    negative_denominator,
    phase_control_roots,
    resource_receipt_root,
    verdict,
    authority: false,
}
```

The constructor must remain opaque. JSON, CBOR or a caller-provided Boolean
cannot forge a verified candidate.

STOP-F8-C:

```text
submitted vs reconstructed commitments   BYTE IDENTICAL
missing independent input                ABSTAIN
commitment drift                         BLOCK
unknown schema                           BLOCK
candidate authority                      false
production callers                       0
```

F8-C result: `STOP_F8_C_EXTERNAL_RECONSTRUCTION.md`. The reconstructor joins
the frozen generation and live provider capture domains independently and
binds phase controls to the exact live traffic receipt set. The candidate and
verified submission remain opaque and authority-free.

## F8-D: Causal Controls

Every candidate is evaluated under identical traffic and budgets:

```text
full phase
no phase
shuffled phase
magnitude only
matched random center
no Wave routing
```

Admission requires:

```text
false accepts                          0
wrong actions                          0
runtime parity mismatches              0
restart mismatches                     0
censored semantic updates              0
support/future lineage overlap         0
full phase search or applicability gain > controls
```

Equal results produce `WATCH_NO_CAUSAL_GAIN`, not PASS.

F8-D result: `STOP_F8_D_CAUSAL_CONTROLS.md`. Runtime-owned observations are
now committed to the generation shadow ledger and independently aggregated by
admission. The controlled live set produced three full-phase selections and
zero selections under every ablation. This is an applicability gain; search
gain remains zero and is not claimed.

## F8-E: Live Shadow

F8-E may be enabled only after STOP-F8-0 through STOP-F8-D pass. Initial
deployment remains SHADOW:

```text
provider request
-> capture try_send
-> generation shadow try_send
-> provider fallback continues
-> off-path actor and verifier
-> external admission candidate
-> nando-live-transition-gate
-> no local accept
```

Traffic budgets:

```text
structural candidates             <= 32
modes per set                     <= 32
mappings per mode                 <= 64
total mapping evaluations         <= 2,048
runtime JSON nodes                <= 4,096
advertised capabilities           <= 64
capture queue                     <= 48
generation shadow queue           <= 48
no-match p99                      <= 250 us
matched shadow p99                <= 1 ms
hard ceiling                      <= 2 ms
hot RSS delta for 2,048 operators <= 16 MiB or accepted frozen replacement
```

The HTTP handler may perform bounded allocation and `try_send`; it may not wait
for disk, generation restore, actor, verifier, admission or Graphify.

F8-E result: `STOP_F8_E_LIVE_SHADOW.md`. Three verified receipts crossed the
actual HTTP boundary, survived restart-monotonic durable append and reconstructed
one `SHADOW_READY` candidate from immutable bytes. The seed is explicitly
controlled, local accept remained false and no ACTIVE package changed.

## STOP-F8

```text
real provider-boundary capture owner        PASS
hash-only capture restart                   PASS
generation-owned live receipt restart       PASS
external reconstruction                     PASS
full causal controls                         PASS
resource budget                              PASS
request-path latency                         PASS
false accepts                                0
wrong actions                                0
parity mismatches                            0
raw persisted payload bytes                  0
nando-live-transition-gate                   PASS
eligible_for_local_accept                    false
execution authority                          false
ACTIVE change                                0
```

Only after STOP-F8 may an explicit, separately reviewed rollout change be
proposed. That change must have its own kill switch, rollback proof and signed
authority lease.

Canonical final receipt: `STOP_F8.md` and `STOP_F8.json`.
