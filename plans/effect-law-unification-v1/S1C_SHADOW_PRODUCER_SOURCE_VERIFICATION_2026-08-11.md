# S1C Shadow Producer Source Verification 2026-08-11

Status: `S1C2_SOURCE_PASS / SOURCE ONLY / NO DEPLOYMENT`

Frozen preregistration:
`S1C_SHADOW_PRODUCER_PREREGISTRATION_V1.md`

Paper verification:
`S1C_SHADOW_PRODUCER_VERIFICATION_2026-08-11.md`

Source branch: `k1-topology-quotient-v2-20260810`

Parent commit: `af776d44dbd1d1d212b3f1b67e74a440b09014c1`

## Verdict

The bounded S1C-2 source implementation satisfies the frozen source, test,
durability, parity, resource, and structural contracts. It creates no runtime,
scientific, deployment, admission, certification, training, or phase authority.

```text
S1C-2 source implementation             PASS
terminal verdict                        S1C2_SOURCE_PASS
capture activation                      false
deployment                              forbidden
authority_ready                         false
model training                          false
phase mutation                          false
natural decision surface                NOT OBSERVED
K2                                      NOT PROVED
next paper-first stage                  S1C-3
```

## Implemented Route

```text
exact content-addressed goal ingress
-> provider capture and topology binding
-> atomic executor plus K1 index authority snapshot
-> one prepared evaluator
-> synced decision precommit
-> unchanged exact-Wave precommit
-> execution of the same prepared object
-> synced selected-action binding
-> independent exact-goal verification
-> synced satisfaction receipt
```

The route is false by default. Missing goals, missing K1 authority, malformed
projections, quota failures, verifier failures, and persistence failures are
named censors and preserve the parent serving result.

## Final Audit Repairs

Manual source review found and repaired two issues before acceptance:

1. `ComposeCollection` with a non-direct renderer now projects
   `rendered_sequence`, not `collection`.
2. The atomic response-authority fingerprint now hashes registry and admission
   contents as well as metadata, certification journal, anchor, public key, and
   runtime contract. A same-length registry mutation changes the fingerprint.

Both repairs have regression coverage. No threshold, authority boundary, or
paper contract changed after observing the results.

## Source Identity

The source-only diff contains six permitted files and no other tracked source:

```text
dd69bd665ae54d915e2d3e7a01b928ad531b4c849ca0b8bc235186264f60622c  crates/nando-operator-learning/src/grounded_decision/pre_action.rs
bb9d24c66212157139080710c26283b116123d42632a7803496e5502c8f01643  crates/nando-operator-learning/src/grounded_decision/pre_action_tests.rs
c4eab1584dec4489fb9d4604e70de3a6fbbae79c6e9ef5c797880698a9a8afe4  crates/nando-response-actor/src/lib.rs
8533a9538f3edbf021e4d4d6461ee820503ab44815aa6e524e0c9b7149ddc90a  crates/nando-response-actor/src/package.rs
fc18c4ed4416fa1881b825fc6b2643e772e84a3905baf33837a506c6aeb79ac3  crates/nando-transition-serving/src/grounded_decision_capture.rs
30e53b09f1fe690bf78e460b65e577603282fc046d578dd73c30eb27f0c1b505  crates/nando-transition-serving/src/lib.rs
```

Tracked six-file binary diff SHA-256:
`66f3531112b615188b30a63354fa4d8083a640a3eca978ed5fbc315fab7d4bcd`

Diff size: `2463 insertions / 138 deletions`.

`graphify-out/` remained pre-existing, untracked, and untouched.

## Functional Verification

Final sequential command:

```text
cargo test -p nando-operator-learning -p nando-response-actor \
  -p nando-transition-serving -- --test-threads=1
```

Primary library results on the candidate source:

```text
nando-operator-learning                 414 PASS / 0 failed
nando-response-actor                    385 PASS / 0 failed / 2 ignored
nando-transition-serving                304 PASS / 0 failed / 9 ignored
all binary, integration, and doc suites  0 failed
```

After the content-fingerprint audit repair, the complete
`nando-transition-serving` package suite was rerun and retained the same
`304 PASS / 0 failed / 9 ignored` library result; all of its binary,
integration, and doc suites also passed.

Evidence log SHA-256:

```text
1e5dd223aef1c36029a1a877b015304e500386e8c242ad76542c97a98ee595a2  full sequential three-crate suite
3f0212e3fe4bec634c087a777fde457de285a8a4169ef9bf21e78d9715dfd93b  post-audit transition-serving suite
```

Additional checks on the final source:

```text
strict scoped Clippy --all-targets -D warnings  PASS
cargo fmt --all -- --check                    PASS
git diff --check                              PASS
```

## Resource Gates

Isolated release measurements remained inside the frozen budgets:

```text
capture-off matched p99                   341727 ns  <= 1000000 ns
capture-off no-goal p99                    11034 ns  <=  250000 ns
capture-off hard max                      790041 ns  <= 2000000 ns
single-ledger sync p99                   1110281 ns  <= 5000000 ns
single-ledger sync hard max              1363279 ns  <= 20000000 ns
three-ledger sync p99                    4263174 ns  <= 5000000 ns
three-ledger sync hard max               5322918 ns  <= 20000000 ns
idle CPU delta                                  0%   <= 0.25%
```

With capture false, no grounded-decision runtime, K1 index, or journal is
constructed. The source-only default path therefore adds no owning shadow
state and remains inside the inherited `16 MiB` steady-state RSS budget. This
is not a live-process or deployed RSS claim; S1C-3 owns that measurement after
transactional installation.

The durability tests also prove the combined `2 GiB` disk quota, `64 MiB`
segments, `32 KiB` record limit, `256` action limit, partial-tail recovery,
ordered joins, replay rejection, poison behavior, and zero persisted raw
request/session/provider/response payload.

## Structural Verification

The installed NANDA v6.2 checker passed self-check and doctor. The six frozen
source routes all returned formal `PASS` with `authority_ready=false`:

```text
goal ingress                             PASS / repair queue 0
one evaluator                            PASS / repair queue 0
atomic authority snapshot                PASS / repair queue 0
persistence and serving                  PASS / repair queue 0
terminal receipts                        PASS / broad citation repair 1
slice boundary                           PASS / repair queue 0
```

The immutable terminal-receipts packet was not rewritten. Its two broad paper
citations were replaced by an exact-span supplemental transformation under
v6.2; that result is `PASS / repair queue 0 / authority_ready=false`.

Final result hashes:

```text
b1218cf3f100401752cb60b1a8152facf12de9203e1bbafe9121e92c54e68bf8  authority snapshot
3fd999b176cb03fb288994ba2bd671ca6792ca53f788aec98bc8825146a951f0  goal ingress
e31631004c51842632c3ff8133f0b4b120a1e8f6aa08066a11556a369b3ade84  one evaluator
03966f131dbaed011a2ac014b83660f5ec31cf71c8db4be40451c64730950f41  persistence and serving
2963b2d601618cf31e84a911eb3475e13f936cdc032f16bdbdd7c5f0239bce8c  immutable terminal receipts
fa21534fe85add17cd78898a9975773d473e39f7015560d80fdca277ca3725ce  exact-span terminal supplement
1c5bf7ccabe8a24af88d28093de7eb055a5444b23076763ec290ce7bb2d69d1a  slice boundary
```

These receipts establish structural coherence only. They grant no deployment
or runtime authority.

## Production And Provenance Boundary

No binary was installed, no service or connector was restarted, no feature was
activated, no dashboard claim was added, and no production file was mutated.
Entire remained enabled on the branch and preserved the active Codex session.

The only permitted next action is a separately preregistered S1C-3
transactional deployment and restart-parity slice. S1C-4 natural census and
all K2 claims remain blocked until that boundary passes.
