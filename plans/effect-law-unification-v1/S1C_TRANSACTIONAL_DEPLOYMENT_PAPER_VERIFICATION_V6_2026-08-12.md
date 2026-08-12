# S1C Transactional Deployment Paper Verification V6 2026-08-12

Status: `PASS / ONE S1C-3 V6 ATTEMPT AUTHORIZED / PRODUCTION UNCHANGED`

## Verdict

V5 is terminal and is not retried. V6 repairs only parity-oracle dependency
closure before quiescence.

```text
V5 candidate and test builds              PASS
V5 baseline parity oracle                 PASS
V5 candidate parity oracle                PREFLIGHT FAILURE
V5 blocker                                online registry DNS resolution
V5 quiescence                             not reached
V5 production mutation                    none
V6 candidate                              unchanged
V6 config                                 unchanged
V6 resource thresholds                    unchanged
V6 remote attempts                        exactly one
```

## Frozen Offline Contract

```text
oracle package                            s1c3-parity-oracle 0.1.0
oracle source SHA-256
  bc5a2255de62a05b44be677ba67331cfbf47b884557f8d8a0d3ac06e46c64b26
oracle Cargo.lock SHA-256
  498855d2a867ba80dba55ad1609bf937852aa61e9de97203d26f067a619da32b
Cargo flags                               --offline --locked
Cargo environment                         CARGO_NET_OFFLINE=true
baseline and candidate targets            fresh and disjoint
post-build lock identity                   exact frozen hash
network fallback                          forbidden
missing cache closure                     terminal non-authority
```

The only manifest difference between the two oracle crates is the checkout
path of `nando-response-actor`. The package identity, oracle source, dependency
lock, Cargo flags, and network policy are identical.

## Diagnostic Verification

A disposable mini-PC diagnostic proved that the proposed lock resolves and
builds both source trees offline:

```text
baseline oracle offline locked build      PASS
candidate oracle offline locked build     PASS
baseline lock after build                 unchanged
candidate lock after build                unchanged
baseline diagnostic binary SHA-256
  4d03399f00646f80d5b1ce305ccb4c3a46403e818aefb516b5a14078a22e2ec5
candidate diagnostic binary SHA-256
  6b80c6b971c5306d9e4f0beb552bab64e667bec28f25e5fca66490dd51dc97f5
```

Those binaries are not V6 evidence and cannot be reused. Their only authority
is that the frozen offline mechanism is feasible before implementation.

## Candidate And Resource Identity

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

```text
representative CPU pool                    [4,6]
physical sibling rows                      4:[4,5], 6:[6,7]
single-ledger p99                          <= 5,000,000 ns
precommit p99                              <= 5,000,000 ns
settlement p99                             <= 5,000,000 ns
each durability hard max                   <= 20,000,000 ns
aggregate episode hard max                 <= 20,000,000 ns
```

No runtime source, durability chronology, measurement denominator, latency
threshold, quiescence condition, or production affinity changes in V6.

## Structural Verification

```text
NANDA self-check                           PASS
NANDA doctor                               healthy
structural verdict                         PASS
authority_ready                            false
weak details                               0
owner conflicts                            0
foreign pull                               0
repair queue                               0
safe_to_edit                               true
```

This proves route coherence only. Deployment authority remains exclusively in
the independent V6 verifier and terminal receipts.

## Production Baseline

```text
transition-serving          PID 165670   restarts 0   active
response-learning           PID 369456   restarts 0   active
gateway-control            PID 1035203  restarts 0   active
certification authority     PID 164668   restarts 0   active
transport / Nginx           PID 682430   restarts 0   active
local connector             PID 2919     restarts 0   active
route receipt failures      0
grounded-decision journal   ABSENT
```

```text
installed binary SHA-256
  6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58
installed config SHA-256
  cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5
```

## Paper Identity

```text
preregistration commit
  a359ef55131a6d2a8ac1f36cbd3f9597f93e2d64
preregistration tree
  6f194539886cb65ea1d11ec47a2216918d426313

V5 terminal report SHA-256
  31ae7641997f4b932602bd0e61b55a04518e088bf400d6697cc7891bddfc8c07
V6 preregistration SHA-256
  7f2935d6c9f59dbb92aaf171f8e2fc788014e1447a0da6f7e72037d526c1e3f9
V6 critique SHA-256
  3389ddcbbcd23ed1b9ef071b522cfc8678b1e2034ec5c4df17afb57323ad6537
V6 structural result SHA-256
  445ca0099539965633b379f3ca194154d2d7b930a9aabd35ad0ffda7ca07053c
```

Exactly one V6 transaction is authorized after this document and the manifest
are committed and the executor/verifier fault-injection gates pass. A complete
S1C-3 deployment installs operational capture only. It does not prove a
natural decision episode, grounded meaning, S1C-4, or K2.
