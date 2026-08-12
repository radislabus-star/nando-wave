# S1C-3H Authority Pair Installation Preregistration V1

Status: `FROZEN BEFORE IMPLEMENTATION / ENGINEERING REPAIR ONLY`

Date: `2026-08-12`

## 1. Plain-Language Purpose

S1C is the decision recorder for Nanda. It must durably record this sequence:

```text
goal before action
-> available K1 actions
-> selected action
-> independently verified result
```

Without these records Nanda observes state changes but cannot later test why an
action was right for a goal. S1C-3H installs that recorder. It does not prove a
meaning law, K2, Law #2, or model quality.

## 2. Exact Blocker

S1C-3G replaced only `nando-transition-serving`. The runtime contract changed
from `f8d955...` to `8e10d1...`, while `nando-response-admission` continued to
issue authority bound to `f8d955...`. The candidate runtime correctly rejected
that authority with:

```text
response_authority_runtime_build_mismatch
```

The failed transaction therefore never exercised the decision recorder. This
is a deployment compatibility defect, not a negative scientific result.

## 3. Frozen Repair

The production compatibility unit is:

```text
nando-transition-serving binary
+ transition-serving environment
+ nando-response-admission binary
+ response authority sidecar generation
+ composite admission.json
```

The two binaries must report the same exact 64-byte runtime contract digest.
The candidate pair is built from source commit
`03e3dd00c90206e2f705371318c50dd50537d6d8`. No digest is forged, excluded, or
rewritten.

The candidate authority is prepared off-path before mutation:

```text
copy current immutable registry and authority inputs into transaction staging
-> run candidate response-admission as user e against staged outputs
-> run the composite gate against a staged profile and staged outputs
-> require PASS, two ACTIVE packages, and candidate runtime digest
-> freeze candidate diagnostic packet
```

Only then may the production transaction run:

```text
pause response-admission and composite-gate path/timer triggers
-> stop transition-serving
-> install the candidate compatibility unit
-> start transition-serving
-> require cache READY and exact candidate digest
-> resume triggers
-> observe an authority renewal and 15-second survival
```

Nginx, the connector, learning, certification, gateway control, and ordinary
traffic generators are not restarted or rewritten.

## 4. Failure And Rollback Contract

Before mutation, the transaction stores and fsyncs exact rollback bytes for all
replaced files. It also records service states, binary hashes, runtime digests,
health, economics, journal prefixes, and authority sidecar hashes.

Before any rollback, it persists:

```text
failure stage and exception
candidate binary hashes and runtime digests
candidate authority and composite-gate outputs
candidate health projection if reachable
candidate startup journal
transaction state
```

Rollback restores the complete old compatibility unit, starts the old runtime,
resumes the previously active triggers, and requires the old pair digest,
cache READY, two ACTIVE packages, zero false accepts, and zero parity failures.
The decision journals are append-only: frozen prefixes and any naturally
arriving valid suffix are preserved.

An interrupted host may temporarily be fail-closed, but cannot gain execution
authority from a mixed pair. Recovery uses the durable transaction state and
rollback bytes. No partial pair is accepted as PASS.

## 5. Attempt And Evidence Boundary

S1C-3G remains terminal and is never relabelled or rerun. S1C-3H uses new
immutable transaction identities.

Engineering repair is not a one-shot scientific observation. A failed S1C-3H
installation may be repaired only after its transaction is terminally sealed,
the defect is recorded, and changed implementation bytes receive a new commit,
preflight, and transaction identity. This does not permit retrying, deleting,
or manufacturing natural decision evidence.

After installation, the natural append cursor is immutable. Generated traffic,
synthetic goals, post-hoc goals, selected-action leakage, phase mutation, and
manual K2 promotion remain forbidden.

## 6. Acceptance Contract

Immediate installation PASS requires all of:

```text
candidate transition contract == candidate authority contract
candidate composite admission runtime contract == candidate pair contract
response executor cache READY
response active profiles 2
capture environment enabled at the frozen journal path
all three NTF1 journals valid and prefix-preserved
false accepts 0
runtime parity failures 0
Nginx PID unchanged
connector PID and restart count unchanged
response-admission and live-gate triggers restored
15-second runtime survival
```

The first naturally produced journal record is reported separately. Its absence
during the bounded installation window does not undo installation PASS and does
not produce scientific PASS.

## 7. Claim Boundary

```text
S1C-3H installation PASS       proves recorder is installed and fail-closed
first natural journal record   proves one ordinary decision was captured
S1C-4 census                   measures natural decision evidence
grounded meaning / K2          remains unproved until later frozen tests
Law #2                         unaffected
```
