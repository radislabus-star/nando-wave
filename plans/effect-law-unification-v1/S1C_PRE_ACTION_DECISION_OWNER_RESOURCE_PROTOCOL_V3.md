# S1C Pre-Action Decision Owner Resource Protocol V3

Status: `PRE-MEASUREMENT FROZEN / CRITIQUE PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Date: `2026-08-11 Europe/Tallinn`

Parent documents:

- `S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V2.md`
- `S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_V2_TERMINAL_RECEIPT.json`

## 1. Purpose And Claim Boundary

V1 remains immutable `VETO`. V2 remains immutable
`INVALID_ENVIRONMENT`: its precommitted verifier required a 64-hex root in
`protocol_commit`, while the protocol and runner correctly recorded a
40-hex Git SHA-1.

V3 repairs that identity contract before collecting a new observation. It does
not reinterpret or reuse V2 measurements.

```text
targeted S1C compatibility path
-> absolute resource gate

inherited full-generation sentinel
-> paired baseline/candidate non-regression gate

raw runner directory
-> exact manifest + parsed metrics + service chronology
-> precommitted V3 verifier
-> S1C-1 RESOURCE PASS | VETO | INVALID_ENVIRONMENT
```

A V3 PASS may authorize committing the already frozen pure S1C-1
implementation only after strict tests, parity, format, and structural checks
pass again. It cannot start S1C-2, activate capture, deploy a binary, change
serving authority, or alter natural evidence.

## 2. Non-Circular Protocol Identity

The V3 paper commit cannot contain its own Git hash. V3 therefore uses three
separate identities instead of a circular self-hash:

```text
protocol parent commit
  335696e903e58c3710e7f813ed79805fec5b26cc

protocol epoch root
  2a21bc5d99a0dd8181ec105a2bdb449f66715674ffb109e3d8941a0bf9a47590

protocol commit
  exactly 40 lowercase hex
  direct child of the frozen parent
  HEAD of the pushed working branch before measurement
```

The epoch root is SHA-256 over the fixed domain string containing the parent,
candidate source-manifest root, V3 schema, and exact schedule. The runner checks
the direct-parent and pushed-HEAD relations. The verifier checks the exact epoch
root, exact parent, 40-hex commit shape, candidate source-manifest root, and
evidence-directory prefix derived from the first eight commit characters.

No arbitrary 64-hex surrogate may be substituted for the Git commit.

## 3. Frozen Candidate Identity

```text
candidate base commit
  ac98ec02da9e6b8584bba0cd48aa6b54d457bb53

tracked implementation diff SHA-256
  283d566c531b87f16dde62f77f97a752fd1ccdabefa425c4453f396a47ea24f1

pre_action.rs SHA-256
  3a22c7e2f7ba679f0294cc19fab460d28113f8dce5b5ec05fa8c88df2dfff3e9

pre_action_tests.rs SHA-256
  879336edfaf0f837c503351a9184ff768b06c31f4e3b4069e180117f635b2615

grounded_decision_capture.rs SHA-256
  10aaf8ba40e0152ea205934729521adc76384b7a890acd2a8fc1c0f1e3f50486

candidate source-manifest root SHA-256
  aa046add5048987c744ca25db89d1510d5f99105305d72bcfc4bed7be805b6b2
```

Any candidate-source edit after the paper commit is terminal
`CANDIDATE_IDENTITY_DRIFT`. V3 protocol, runner, verifier, tests, critique,
and receipts are proof-plane files and are excluded from the frozen
implementation diff.

## 4. Frozen Executables And Tests

Mini-PC: `e@192.168.3.94`

Pinned CPU: `4`

Inherited sentinel:

`performance::full_generation_shadow_latency_stays_within_traffic_budget`

```text
baseline executable SHA-256
  ab31fde97776084de499e8d70ff3ade6d20a9d05dba912e69e5d069c777e6656

candidate executable SHA-256
  99c8b9fe8c8e192c418aa1057bec0380c568f666166d40674685aa2132982277
```

Targeted S1C test:

`package::tests::capture_disabled_compatibility_latency_stays_within_hot_budget`

```text
candidate executable SHA-256
  dd785c1c96122aa1c6aa33f5f637d92636346b15d55902659cfe067c127a124b
```

Frozen proof tools:

```text
runner
  ops/remote-backend/run_s1c1_resource_v3.sh
  SHA-256 a871f0f148a9fa2f85a7f848f36473d02f8d77e8cf745c234bbfa16f421b7d4b

verifier
  ops/remote-backend/verify_s1c1_resource_v3.py
  SHA-256 10c4371ed0dd3b87853e24c711a53a1eb83b97813e593905dff7c818f460ee8c

verifier tests
  ops/remote-backend/test_verify_s1c1_resource_v3.py
  SHA-256 929e47ed7de47e87db0e38cdddd2ead4c05db5f3f046ecbb7525cd9f83469037
  13 PASS / 0 FAIL
```

Rebuilds, replacement binaries, changed hashes, alternate test names, or
changes to these proof tools require V4. Cargo is not invoked during the
measurement set.

## 5. Evidence Contract

The runner creates exactly one directory:

`s1c1-v3-<first eight protocol commit characters>`

It must not already exist locally or remotely. The set contains exactly 40
files:

```text
9 *.log
9 *.exit
18 before/after remote snapshots
environment.txt
final.snapshot
local_connector.before
local_connector.after
```

The verifier computes a canonical manifest from sorted
`SHA-256 + two spaces + filename + newline` records. It rejects missing,
extra, duplicate, renamed, or modified files.

For every run it independently parses the exact metrics line, test name, exit
code, before/after timestamps, five production service states, and health
record. It requires:

- raw metrics equal the measurements JSON;
- exactly 4,096 samples;
- exit code agrees with the absolute budget classification;
- every service remains active with identical `MainPID / NRestarts`;
- every inter-run and final gap is at least 1.9 seconds, representing the
  frozen two-second sleep with timestamp tolerance;
- local connector remains active with identical `MainPID / NRestarts`;
- route receipt failures remain zero;
- false accepts remain zero.

The measurements JSON is not authority without this raw-evidence gate.

## 6. Exact Schedule

One process runs at a time. Every invocation uses `taskset -c 4`,
`RUST_TEST_THREADS=1`, `--ignored`, `--exact`, `--nocapture`, and
`--test-threads=1`.

```text
T1  targeted
P1  baseline -> candidate

T2  targeted
P2  candidate -> baseline

T3  targeted
P3  baseline -> candidate
```

A fixed two-second sleep follows every invocation, including the last. There
is no warmup, replacement, fourth run, outlier deletion, idle-conditioned
start, or post-result rebuild.

Nonzero inherited test exits are preserved because the inherited test retains
its V1 absolute assertions. The runner temporarily disables shell fail-fast
only around the test process, writes its exit code, then restores fail-fast.

## 7. Frozen Math

Targeted absolute budgets:

```text
matched p99      <= 1,000,000 ns
no-goal p99      <=   250,000 ns
hard max         <= 2,000,000 ns
required         PASS 3/3
```

Inherited absolute classification uses the same three limits with
`no-match p99`. The paired non-regression gate passes only when:

```text
candidate absolute PASS count >= baseline absolute PASS count
median pair regression factor <= 1.10
every individual pair regression factor <= 2.00
candidate median for each metric <= 1.10 * baseline median
baseline 3/3 absolute PASS cannot become candidate <3/3
```

All comparisons use exact rational arithmetic. The paired rule is an S1C-1
regression sentinel, not a product-latency claim. V1 absolute VETO remains
unchanged.

## 8. Result Matrix

```text
identity, evidence, chronology, service, or schema drift
-> INVALID_ENVIRONMENT
-> no rerun under V3

targeted absolute gate fails
-> VETO
-> no implementation commit

inherited paired gate fails
-> VETO
-> no implementation commit

safety or structural gate fails
-> VETO
-> no implementation commit

all V3 gates pass
-> S1C-1 RESOURCE PASS
-> rerun strict tests, Clippy, format, parity, structural boundary
-> commit frozen S1C-1 implementation plus final receipt
-> no deployment
```

Any required protocol repair creates V4 and a new post-change watermark.

## 9. Post-PASS Checks

The exact frozen source manifest must survive:

- implementation crate tests;
- strict scoped Clippy `-D warnings`;
- `cargo fmt --all -- --check`;
- current/candidate serving parity;
- durability and restart parity;
- split structural routes with no `WATCH` or `VETO`;
- final production and connector survival;
- `git diff --check` for implementation and receipt files.

Passing carried V1 receipts may orient these checks, but the final V3
implementation commit relies on fresh post-measurement checks.

## 10. Production Boundary

V3 is paper plus laboratory resource evidence. It changes no installed binary,
service, registry, admission file, dashboard, natural evidence, model state,
phase state, or ACTIVE package.

```text
S1C-1 PASS
-> source commit only

S1C-2
-> separate shadow-producer protocol

S1C-3
-> separate transactional deployment
-> fresh absolute product gate
```

## 11. Pre-Measurement Gate

```text
verifier tests                          PASS 13/13
runner bash syntax                     PASS
runner ShellCheck                      PASS
candidate identity                     PASS
NANDA identity route                   PASS
  9e534efdd58ebfe8bd6ede4972f54181edab6df0d2191e2d156310438bf3405a
NANDA evidence route                   PASS
  88189487407c7c0944ff09fe3816059e7f70d3d27b33ef152cf034506b470862
NANDA chronology route                 PASS
  0d97776183a6fa5e8131b05e450154ac51128382f348913645f9ab5c21095de7
NANDA authority route                  PASS
  6c401b80c4d0a1094ffda6bd83671c1f581cb4b2329221c23e2070ea9e7b0205
structural receipt manifest            PASS
  bb57d80e16cbf0aca9ccf910fa57b3d524841970769d0746e097218dd6edfc05
weak triads                            none
conflicts                              none
foreign pull                           none
owner conflicts                        none
negative hits                          none
repair queue                           empty
measurement started                    NO
authority_ready                        false
```

Measurements remain forbidden until the separate V3 critique is complete, all
four structural routes pass without `WATCH` or `VETO`, and the paper-only
V3 commit is pushed.
