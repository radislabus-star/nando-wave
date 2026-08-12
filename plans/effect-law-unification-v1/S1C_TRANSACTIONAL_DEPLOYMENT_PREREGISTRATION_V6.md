# S1C Transactional Deployment Preregistration V6

Status: `DESIGN FROZEN / OFFLINE DIAGNOSTIC PASS / NO DEPLOYMENT`

Date: `2026-08-12 Europe/Tallinn`

## 1. Exact Blocker

The single V5 transaction ended before quiescence because the generated
candidate parity-oracle crate had no frozen lockfile. Cargo tried to refresh
the crates.io registry and DNS resolution failed. Production was unchanged.

V6 repairs only parity-oracle dependency closure. It does not change runtime,
measurements, thresholds, quiescence, deployment chronology, or scientific
claims.

## 2. Immutable Candidate Boundary

```text
candidate commit
  03e3dd00c90206e2f705371318c50dd50537d6d8
candidate tree
  06a9df51797dffc127fec41672bddae29c38bb92
production projection SHA-256
  10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316
Cargo.lock SHA-256
  0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1
candidate config SHA-256
  1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6
```

There is no Rust candidate change. The exact V5 candidate and config are
reused. Runtime append order, three fsyncs, ledger format, failure censors,
service unit, phase memory, authority config, and process affinity are
immutable.

## 3. Frozen Offline Oracle Contract

Both fresh parity workspaces use one package identity:

```text
package name                 s1c3-parity-oracle
package version              0.1.0
edition                      2024
oracle source SHA-256
  bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26
oracle Cargo.lock SHA-256
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
```

The only manifest difference is the absolute path of
`nando-response-actor`: baseline checkout versus candidate checkout. Before
either build, the exact frozen lockfile is copied into both workspaces.

Both commands must contain all of:

```text
cargo build
--release
--quiet
--offline
--locked
--manifest-path <fresh workspace>/Cargo.toml
CARGO_NET_OFFLINE=true
fresh CARGO_TARGET_DIR
```

After each build, the executor hashes the workspace lockfile again and rejects
any drift. The ownership row records `cargo_lock` identity. The ownership
receipt records one exact `oracle_build_contract`, including package name,
source hash, lock hash, offline/locked booleans, and both manifest hashes. The
preparation and deployment receipts bind that contract through the ownership
root and executable-set root.

Missing cached dependencies are terminal `OFFLINE_DEPENDENCY_CLOSURE_MISSING`.
They cannot trigger network access, fallback to an online build, reuse a V5
oracle binary, or authorize production mutation.

## 4. Diagnostic Before Freeze

A read-only disposable diagnostic on the mini-PC already built both oracle
variants with the proposed common lock and no network:

```text
baseline offline locked build          PASS
candidate offline locked build         PASS
lock before
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
baseline lock after                    identical
candidate lock after                   identical
baseline oracle binary SHA-256
  4d03399f00646f80d5b1ce305ccb4c3a46403e818aefb516b5a14078a22e2ec5
candidate oracle binary SHA-256
  6b80c6b971c5306d9e4f0beb552bab64e667bec28f25e5fca66490dd51dc97f5
```

These binaries are diagnostic only and cannot be reused by V6. V6 creates new
workspaces, targets, ownership receipts, executable identities, and parity
outputs.

## 5. Inherited V5 Gates

V6 inherits the V5 multi-core quiescence selector and every V4 resource and
deployment bound unchanged:

```text
measurement representatives            [4,6]
physical siblings                       4:[4,5], 6:[6,7]
selection                               simultaneous environment-only windows
window                                  30 intervals
per-logical-CPU busy                    <= 20%
per-logical-CPU window mean             <= 5%
single-ledger p99                       <= 5 ms, PASS 3/3
precommit p99                           <= 5 ms, PASS 3/3
settlement p99                          <= 5 ms, PASS 3/3
each durability hard max                <= 20 ms
aggregate episode hard max              <= 20 ms
false accepts                           0
runtime parity failures                 0
```

A small latency deviation is classified by these frozen limits. It is not a
reason to ask the user for a new threshold or adapt the attempt.

The complete timeout census, selected-CPU binding, independent predeployment
receipt, rollback chronology, service survival, connector identity, journal
prefix preservation, and exact installed hashes remain mandatory.

## 6. Attempt And Claim Boundary

After final paper verification, V6 authorizes exactly one transaction. V5 is
terminal and is never retried.

```text
S1C-3 operational capture installed     only after verified DEPLOYMENT_PASS
natural decision episode                not proved by V6
grounded meaning                        not proved by V6
S1C-4                                   blocked until deployment PASS
K2                                      blocked
model training                          false
phase mutation                          false
dashboard scientific claim              forbidden
```

V6 removes accidental internet dependence from a proof build. It does not
manufacture ordinary evidence or turn operational capture into meaning.
