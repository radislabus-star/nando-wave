# S1C Transactional Deployment Preregistration V3

Status: `PAPER FROZEN / EXECUTION FORBIDDEN UNTIL V3 PAPER VERIFICATION PASS`

Date: `2026-08-12 Europe/Tallinn`

Parent authority:

- `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2.md`
- `S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V2_CRITIQUE.md`
- `S1C_TRANSACTIONAL_DEPLOYMENT_V2_ATTEMPT_2026-08-12.md`

## 1. Exact Delta

V3 inherits V2 byte-for-byte except for parity-oracle workspace ownership and
its prebuild verification. It does not change candidate source, production
config, baseline, quiescence, measurement order, thresholds, transaction,
rollback, or claim authority.

```text
V2 root-created oracle directory
-> Cargo as e could not create Cargo.lock

V3 root creates bounded oracle workspace
-> recursively assigns workspace owner/group e:e
-> sets directories 0750 and source/manifest files 0640
-> user e creates, fsyncs, and removes an ownership probe
-> directory fsync
-> only then may Cargo prebuild the oracle
```

## 2. Frozen Identity And Limits

```text
candidate commit
  a3ea27a49af397ef79e5c9ec80089ecf53a41d59

candidate tree
  670d9c4ed170a76f107db13262abcd7cc035578e

candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6

single-ledger p99
  <= 5,000,000 ns PASS 3/3

three-ledger p99
  <= 5,000,000 ns PASS 3/3

hard max
  <= 20,000,000 ns PASS 3/3
```

Every other V2 resource, parity, idle, RSS, false-accept, service, connector,
journal, and rollback threshold remains unchanged.

## 3. Oracle Ownership Receipt

Before each oracle Cargo command, the executor freezes an ownership row:

```text
label                          baseline | candidate
workspace absolute path
workspace uid/gid/mode         e:e / 0750
src uid/gid/mode               e:e / 0750
Cargo.toml uid/gid/mode        e:e / 0640
src/main.rs uid/gid/mode       e:e / 0640
probe writer uid/gid           e:e
probe create/fsync/unlink      PASS
directory fsync                PASS
```

The probe uses an exclusive new file and contains no production, fixture, or
scientific data. It is removed before Cargo. A missing, preexisting, retained,
wrong-owner, wrong-mode, or non-fsyncable probe is terminal preflight failure.

The ownership rows are written to
`nando.s1c3-oracle-ownership-receipt.v3`, rooted, mode `0400`, and bound by the
quiescence receipt executable set. The ownership receipt is frozen before
quiescence and cannot be generated after a metric.

## 4. Fresh-Build Boundary

V3 uses a new transaction id, new upload directory, new detached checkout, new
candidate target, and new oracle targets. No V2 binary, harness, oracle,
Cargo.lock, target directory, or quiet-window evidence may be reused.

All five executables still finish before the 30-second quiescence gate. After
the quiescence receipt, Cargo, rustc, linkers, and build systems remain
forbidden exactly as in V2.

## 5. Test And Fault Boundary

Before the one V3 remote attempt, local tests must prove:

```text
oracle workspace owner mismatch       rejected
oracle directory not writable by e    rejected
probe retained after check             rejected
ownership receipt root mismatch        rejected
ownership receipt not mode 0400        rejected
Cargo/rustc after quiescence            absent
```

The implementation must also run one isolated non-root smoke that creates and
removes the probe through the same production helper. It may not build an
oracle or execute a candidate metric during that local smoke.

## 6. Attempt And Transaction Boundary

V3 authorizes exactly one remote attempt. Any ownership, build, quiescence,
contamination, resource, parity, stale-baseline, transaction, or rollback
result is terminal for V3.

Only after all V3 preflight receipts pass may the unchanged V2 chronology stop
and restart `nando-transition-serving.service`. Every other service and the
connector preserve PID, restart count, and authority.

## 7. Claim Boundary

```text
operational capture installation      only allowed new claim
natural decision episode              not proved
S1C-4                                 blocked until deployment PASS
K2                                    blocked
model training                        false
phase mutation                        false
dashboard scientific claim            forbidden
```

V3 repairs a build-workspace permission defect. It does not create scientific
evidence.

