# K2 Self-Formed Uncertainty V5 R8B V6 Implementation Discrepancy

Status: `P0 / V6 READY_TO_IMPLEMENT REVOKED / NO R8B EXECUTION`

Date: `2026-08-21`

Observed paper commit:
`0263e6ef18ec348379af73bd4793b975f4b74870`

Observed implementation branch:
`research/k2-self-formed-uncertainty-r8b-v6-implementation-20260821`

No R8B attempt, deployment, production mutation, dashboard update or push
occurred before this discrepancy was found.

## 1. Verdict

The V6 process topology is not implementable under its frozen owner map. The
problem is not a failing experiment. It is a paper/preflight defect discovered
before execution.

The existing partial implementation remains useful and is preserved. Its
component checks do not grant authority to bypass the three defects below.

## 2. P0 Defects

### D1: M24 cannot journal spawns owned by M01 and M10

V6 assigns every `ChildStarted` and `ChildFinished` row to M24. The preserved
source has two nested spawn owners:

```text
M01 Development owner -> M02 generator
M10 public coordinator -> M03..M09 public workers
```

M01 calls `dispatch_self_formed_generator_once_v1`, which performs the actual
`Command::spawn`. M10 calls `run_self_formed_confirm_sandbox_v1` from its
private `invoke_owner_v1`, which performs each M03-M09 spawn.

M24 has no callback, descriptor, socket, prepared-output protocol or other
pre-spawn interposition point in either route. It therefore cannot truthfully
fsync a request-bound `ChildStarted` before those spawns or validate and fsync
their typed `ChildFinished` rows afterward. Reconstructing rows from final
artifacts would be post-hoc synthetic evidence and is forbidden.

### D2: suite receipt bytes have no canonical producer channel

V6 names five Rust libtest binaries as suite producers. M25 requires each
non-parent packet entry to match the producing process event byte-for-byte:

```text
event stdout length/hash == packet receipt length/hash
decoded stdout receipt root == packet semantic root
```

Libtest owns stdout framing. Even when a test prints canonical JSON, the
process stdout also contains harness framing and summaries. V6 defines no
separate canonical receipt descriptor, pipe, socket or immutable sidecar.

M24 cannot construct a receipt afterward while claiming that S01-S05 produced
it. That would repeat the synthetic-positive defect V6 was intended to close.

### D3: the aggregate packet cannot contain finished future owners

V6 says the process ledger covers every invocation, including the active M24
parent, M25 and M26. The packet must be closed before M25 starts:

```text
closed packet -> M25 authorization -> M26 publication
```

Consequently the packet cannot already contain:

```text
M24 parent process exit
M25 process exit
M26 process exit
```

Putting those future outcomes into the packet is a causal cycle. Omitting them
while claiming an all-process denominator is a false denominator.

## 3. Consequence

The following V6 conclusions are revoked:

```text
implementation preflight READY_TO_IMPLEMENT   REVOKED
safe_to_implement=true                        REVOKED
22-path source scope                          SUPERSEDED
V6 positive linked route                      FORBIDDEN
R8B execution                                 FORBIDDEN
```

The V6 structural receipts remain historical evidence about the paper packet.
They do not answer the source-level spawn and causal-order defects above.

## 4. Required Repair

The successor contract must:

1. assign intent-first process rows to the process that actually owns each
   spawn, while retaining one validated append-only route ledger;
2. give suite binaries a distinct canonical receipt channel and preserve real
   stdout/stderr as separate evidence;
3. close the aggregate over pre-authorization processes only;
4. keep M25 and M26 outcomes in a post-authorization audit chain that cannot
   become an input to its own authorization;
5. add `confirm_public_coordinator.rs` to the exact modified source scope;
6. rerun critique, structural routes, code-route design and implementation
   preflight before implementation continues.

The repaired paper is
`K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V7.md`.
