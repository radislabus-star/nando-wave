# S1C Pre-Action Decision Owner Implementation Verification

Status: `S1C-1 CANDIDATE IMPLEMENTED / VERIFICATION VETO / UNCOMMITTED / NO DEPLOYMENT`

Date: `2026-08-11 Europe/Tallinn`

Frozen preregistration:
`S1C_PRE_ACTION_DECISION_OWNER_PREREGISTRATION_V1.md`

Source branch: `k1-topology-quotient-v2-20260810`

Source and remote base:
`ac98ec02da9e6b8584bba0cd48aa6b54d457bb53`

## Verdict

The permitted S1C-1 code slice is implemented and its targeted contract,
authority, persistence, serving-parity, resource, and structural checks pass.
It is not accepted because the mandatory fresh three-run inherited F8-D
absolute latency gate did not pass every candidate run under ordinary server
load.

```text
S1C targeted resource gates             PASS
relative baseline/candidate regression  PASS
inherited absolute F8-D                  VETO
structural WATCH or VETO                 none
S1C-1 acceptance                         VETO
commit                                   forbidden
push                                     forbidden
deployment                               forbidden by slice
authority_ready                          false
```

This classification follows the frozen contract exactly:

- release baseline/candidate latency requires three pinned runs;
- no-match p99 must be at most 250 us;
- matched p99 must be at most 1 ms;
- a resource-budget breach is a terminal S1C stop condition;
- a budget or denominator cannot be widened after observing evidence.

The better relative candidate result and the passing targeted S1C hot path do
not convert the failed absolute inherited gate into PASS.

## Implemented Slice

```text
typed goal predicate artifact
-> exact pre-action goal binder
-> K1 action contract projection
-> authority-owned opaque execution binding
-> immutable DecisionAuthoritySnapshotV1
-> one prepared evaluator
-> durable DecisionContractPrecommitV1
-> deterministic durability receipt
-> SelectedActionBindingReceiptV1
-> framed CBOR journal
```

Principal implementation points:

```text
grounded_decision/pre_action.rs:35       typed goal predicate artifact
grounded_decision/pre_action.rs:110      K1 action contract projection
grounded_decision/pre_action.rs:267      immutable authority snapshot
grounded_decision/pre_action.rs:365      exact pre-action goal binder
grounded_decision/pre_action.rs:415      decision precommit
grounded_decision/pre_action.rs:680      selected-action binding receipt
response-actor/package.rs:784            authority-owned K1 index builder
response-actor/package.rs:1085           prepared evaluator
response-actor/package.rs:1521           prepared execution
transition-serving/grounded_decision_capture.rs:1
                                         framed durable journal
```

Raw K1 action-index constructors are not public. The authority-owned builder
checks registry and admission identity, certification ledger revision and root,
K1 gate root, execution payload, admitted package binding, latest
`k1_unit_eligible` certification, semantic law, topology projection, and current
runtime contract. The negative forged-execution-payload test passes.

Disabled capture does not hash the request or serialize provider material.
Quota or evidence overflow disables new S1C evidence and leaves serving
unchanged. No raw request, session, provider, tool, actor, or upstream payload is
persisted.

## Functional Verification

All builds and tests ran on `e@192.168.3.94` in
`/tmp/nando-s1c1-build`, using
`/home/e/.cache/nando-wave-s1c1-target`.

```text
nando-operator-learning tests            412 passed
nando-response-actor tests               383 passed / 2 ignored
nando-transition-serving tests           299 passed / 8 ignored
failures                                 0
strict scoped Clippy -D warnings         PASS
cargo fmt --all -- --check               PASS
git diff --check                         PASS
```

The ignored cases are explicit isolated resource/provisioning gates; the S1C
durability, idle CPU, latency, and parity receipts were executed separately in
release mode where required.

## Serving Parity

The baseline and candidate oracle used the same live registry and admission
artifacts and covered eight authorized and shadow route cases, producing 16
rows each.

```text
baseline SHA-256
  dfb558103efebe5fb66ba35bcb7b4775053aefcb7aa345aa6d887d905f229526
candidate SHA-256
  dfb558103efebe5fb66ba35bcb7b4775053aefcb7aa345aa6d887d905f229526
diff                                     empty
ordinary output parity                   byte-identical PASS
```

The compatibility `execute()` route and
`evaluate_pre_action -> execute_prepared` use the same evaluator.

## Targeted S1C Resource Gates

Three release runs of the isolated S1C compatibility hot path:

```text
matched p99 ns             20662 / 21354 / 21165
no-goal p99 ns              1104 /  1105 /  1472
hard max ns                34024 / 37531 / 32198
verdict                                  PASS 3/3
```

Three release runs of 1,024 synced precommits with exact three-segment
rotation:

```text
sync p99 ns              3037355 / 2699582 / 2590130
hard max ns              5599190 / 5220283 / 5244087
verdict                                  PASS 3/3
```

Other frozen resource gates:

```text
60-second idle CPU ticks                  0
idle CPU percent of one core              0.000000%
idle CPU verdict                          PASS

baseline RSS KiB               6560 / 6880 / 6880
candidate RSS KiB              6880 / 6720 / 6880
median RSS delta KiB                      0
worst paired RSS delta KiB              +320
budget KiB                             +16384
RSS verdict                               PASS
```

## Mandatory Inherited F8-D Gate

The preregistered fresh A/B run was interleaved and pinned to CPU 4 under
ordinary production load. No extra runs were cherry-picked.

```text
baseline matched p99 ns     1024719 /  658974 / 1024736
baseline no-match p99 ns     199921 /  201126 /  372524
baseline absolute result                         PASS 1/3

candidate matched p99 ns      650929 /  651322 / 1056756
candidate no-match p99 ns     197700 /  196373 /  333313
candidate absolute result                        PASS 2/3

matched p99 budget ns                         1000000
no-match p99 budget ns                         250000
hard ceiling ns                               2000000
```

The candidate is better by median and does not show a relative regression, but
its third run exceeds both frozen p99 budgets. The absolute mandatory verdict
is therefore `VETO`.

## Structural Verification

The fail-closed NANDA binary used for the split packets had SHA-256:

`1309c0d25397b6bd5dc63ec44b3bd3750030b5f9f0fd78b20f5b6b35027f489c`

```text
self-check                               PASS
doctor healthy                          true
authority binding route                 PASS
runtime and persistence route           PASS
slice boundary route                    PASS
WATCH                                   none
conflicts                               none
foreign pull                            none
owner conflicts                         none
negative hits                           none
repair queue                            empty
authority_ready                         false, expected
```

The authority packet required three separate binding evidences. The earlier
over-grouped representation was not accepted.

## Production Boundary

S1C-1 changed no installed binary, service, registry, admission file, feature
flag, dashboard, natural evidence, model state, or phase state. It performed no
deployment and no service restart. Production continued serving and collecting
statistics throughout verification.

Final read-only survival check:

```text
nando-transport-gateway             PID 682430  active  restarts 0
nando-transition-serving            PID 165670  active  restarts 0
nando-response-learning             PID 369456  active  restarts 0
nando-gateway-control              PID 1035203  active  restarts 0
nando-operator-certification        PID 164668  active  restarts 0
local nando-client-connector          PID 2919  active  restarts 0
gateway health                                  PASS
CPU mode / admission                            CPU / PASS
active response profiles                        2
false accepts                                   0
route receipt failures                          0
```

The user-owned untracked `graphify-out/` directory was not modified.

## Terminal State

The implementation remains an uncommitted candidate for inspection. Under the
frozen V1 contract the next action is not S1C-2 and not deployment. Resuming the
same candidate requires either a preregistered V2 resource protocol with a new
post-change watermark, or a user decision to discard the candidate. This V1
receipt must remain `VETO`.
