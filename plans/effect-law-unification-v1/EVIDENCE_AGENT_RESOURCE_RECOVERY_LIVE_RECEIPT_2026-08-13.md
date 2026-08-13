# Evidence Agent Resource Recovery Live Receipt

Date: 2026-08-13

Status: `RECOVERY_PASS`

## Exact Repair

The local evidence agent no longer retains complete outbox frames or complete
route receipts in memory. The outbox is represented by framed-CBOR disk
coordinates, pending batches are materialized under an 8 MiB bound, and the
route-receipt index stores fixed-size identities plus JSONL offsets. A selected
receipt is reread and fully validated before use.

The systemd resource boundary is:

```text
MemoryMax=128M
MemorySwapMax=0
CPUWeight=10
IOWeight=10
IOSchedulingClass=idle
```

Source commit: `81c468c`

## Resource Gate

The release binary opened the complete real state and route-receipt ledger
under the production limits without network mutation:

```text
cold resource run peak       99.3 MiB
repeat resource run peak     65,441,792 bytes
live recovery peak           119,656,448 bytes
live current memory          approximately 100 MiB
MemorySwapPeak               0
OOM                           0
major faults, 4 minute gate  0 in every sample
```

The old unit had previously reached the 128 MiB memory limit plus a
1,019,076,608-byte swap peak. The repaired unit produced no swap or page-fault
storm.

## Epoch Recovery

The server rollback had left the old client branches divergent:

```text
remote accepted head         sequence 5476
local accepted head          sequence 5477
missing remote batch         5477
```

No receipt was synthesized and neither branch was rewritten. The old local
branch, its pending batch, the full outbox, the old key, and all 5,476 remote
accepted batch receipts were retained as an immutable orphan epoch.

```text
old client
93bde8a498ad119e5b2f2c46551c268223eab7a6a115877105dd9de8cf5a2edb

new client
683fca2f6454e7d6f4cb8e2d43a4b0c19ec8b4f9ce928a548578fef7e6b78a64

orphan archive
/home/ubu/.local/state/nando-evidence-agent-orphaned-epochs/20260813T104811Z-93bde8a498ad119e5b2f2c46551c268223eab7a6a115877105dd9de8cf5a2edb

archive files               5,502
archive bytes               352,210,122
final manifest SHA-256      49ac521804981a9b8fb73468d6def8d4d13d9fe6cf30e0973eecd833f09acc76
```

The five preregistered frozen local artifacts retained their exact SHA-256
values. Both old and new remote client keys remain present; the cold learner,
hot serving, transport gateway, and control plane were not restarted.

## Live Delivery

The new client epoch drained the complete queued natural suffix:

```text
accepted batches            197
accepted frames             6,279
remote route-bound delta    6,279
local outbox after ACK       0
local pending after ACK      0
sequence conflicts           0 after rollover
auth failures                0
route receipt refresh fail   0
```

Stable runtime projection:

```text
local evidence agent PID    3990539, active, restarts 0
local connector PID         2919, unchanged
cold learner PID            369456, unchanged, restarts 0
hot serving PID             1816591, unchanged, restarts 0
Nginx gateway PID           682430, unchanged, restarts 0
control plane PID           2264042, unchanged, restarts 0
gateway health              PASS
CPU admission               PASS
false accepts               0
```

## Claim Boundary

This receipt proves resource recovery, authenticated transport recovery,
durable delivery, and service/PID parity. It does not grant learning authority,
phase mutation authority, a new LawCertificate, or a grounded-meaning result.
The cold learner remains fail-closed with
`NO_ROUTE_BOUND_TOPOLOGY_FRAME_TERMINAL_LINK`.
