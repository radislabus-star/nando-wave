# K2 Self-Formed Uncertainty V5 R7K Contract V4

Status: `REVISED AFTER PREFLIGHT DRIFT / PENDING GATES / NO RESULT AUTHORITY`

Date: `2026-08-20`

Supersedes: `K2_SELF_FORMED_UNCERTAINTY_V5_R7K_CONTRACT_V3.md` only where
this document changes predecessor transport and preservation. All other V3
scope, owner, cleanup, fault, budget and claim boundaries remain frozen.

Discrepancy:
`K2_SELF_FORMED_UNCERTAINTY_V5_R7K_PREFLIGHT_DRIFT_2026-08-20.md`

## 1. Exact Boundary

R7K still emits only:

```text
DEVELOPMENT_REHEARSAL_COMPLETE
```

It creates no sealed attempt, slot claim, Confirm nonce, scientific verdict,
deployment, network, K1, dashboard or production effect. R7K PASS unlocks only
R8B and cannot execute R8B in the same commit.

## 2. R7J Ownership Preservation

These R7J owners remain exact predecessor owners:

```text
control evaluator
terminal evaluator
oracle/baseline evaluator binaries
public/private execution binaries
```

R7K may not modify their decision code, schemas, thresholds, dispositions,
denominators or executable ownership.

The R7J integration harness may expose one explicit Development-only export
mode. This is test evidence transport, not a new R7J decision owner.

## 3. Export Mode

Export mode is enabled only by:

```text
NANDO_K2_R7J_PERSIST_FIXTURE_ROOT=<fresh absolute test-owned path>
```

Requirements:

```text
root did not exist before the test
root mode                                      0700
normal R7J evaluation reaches PASS first
exported regular files                        0400
unknown exported files                           0
missing manifested files                         0
hash mismatches                                  0
symlinks or special files                        0
overwrite of an existing packet                  0
```

Without the variable, R7J has its prior ephemeral behavior and leaves no
persistent packet.

## 4. Closed Packet

The packet contains exactly:

```text
oracle-batch.json
routes.json
resources.json
resolver-request.json
final-request.json
one-probe-descriptor.json
two-probe-descriptor.json
fixture-manifest.json
```

`fixture-manifest.json` maps the seven payload paths to SHA-256. It is itself
read-only. Before decoding any payload, R7K independently requires:

```text
exact path set
exact file count
regular non-symlink files
mode 0400
manifested SHA-256 equals observed bytes
no path alias, absolute path or parent component
```

Any mismatch fails before terminal construction or cleanup authorization.

## 5. Evidence Separation

The exported packet may provide only validated predecessor inputs for the R7J
Development terminal:

```text
oracle batch
route receipts
resource receipt
typed negative-control fixture requests
```

It cannot provide or substitute:

```text
R7K K1-K12 process outcomes
R7K process logs
R7K control-evaluator receipt
R7K cleanup census
R7K cleanup authorization
R7K CleanupFrozen
R7K Development result
```

K1-K12 still require twelve newly launched R7K child processes and newly
measured stdout, stderr, exit, source and log roots.

## 6. Freshness And Identity

The export and consuming R7K test run in one command chain against one Cargo
target. The terminal independently validates all embedded receipt roots and
current evaluator identities. A packet from another build, changed executable
set or previous run must fail validation or be rejected by the fresh-root rule.

## 7. Required Verification

V3 requirements remain mandatory. V4 adds:

```text
R7J default no-export regression                    PASS
R7J explicit export run                             PASS
closed packet path/mode/hash validation             PASS
manifest omission/substitution negative controls    PASS
R7J evaluator and terminal source parity            PASS
K1-K12 independent from exported packet             PASS
V4 implementation preflight                         READY_TO_IMPLEMENT
post-implementation observed-source route gate      PASS
```

## 8. Claim Boundary

V4 repairs predecessor evidence transport only. It does not strengthen the R7K
claim beyond V3 and proves nothing about the self-formed-uncertainty scientific
hypothesis, Natural K2, natural traffic, product value or deployment readiness.
