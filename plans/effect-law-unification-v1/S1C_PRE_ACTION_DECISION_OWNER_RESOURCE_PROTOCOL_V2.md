# S1C Pre-Action Decision Owner Resource Protocol V2

Status: `PRE-MEASUREMENT FROZEN / CRITIQUE PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Date: `2026-08-11 Europe/Tallinn`

Parent protocol:
`S1C_PRE_ACTION_DECISION_OWNER_PREREGISTRATION_V1.md`

Parent V1 result:
`S1C_PRE_ACTION_DECISION_OWNER_IMPLEMENTATION_VERIFICATION_2026-08-11.md`

## 1. Purpose

V1 remains an immutable `VETO`. It required a fresh absolute inherited F8-D
gate under ordinary server load. The baseline passed only one of three runs and
the candidate passed two of three. The targeted S1C path, serving parity, RSS,
sync latency, idle CPU, tests, and structural routes passed.

V2 does not relabel those measurements. It freezes a new post-change protocol
that separates two claims:

```text
targeted S1C incremental path
-> absolute hot-path and durability budgets
-> may accept S1C-1 code only

inherited full-generation F8-D sentinel
-> paired baseline/candidate regression evidence under ordinary load
-> may veto S1C-1
-> cannot grant deployment or product authority
```

The product absolute gate remains required at S1C-3 before deployment. V2
cannot activate capture, change serving authority, start S1C-2, or authorize a
production rollout by itself.

## 2. Frozen Candidate Identity

```text
base commit
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

No implementation source may change between the protocol commit and the final
measurement receipt. A mismatch is terminal `CANDIDATE_IDENTITY_DRIFT`.

## 3. Frozen Executables And Tests

Mini-PC: `e@192.168.3.94`

Pinned CPU: `4`

Inherited sentinel test:

`performance::full_generation_shadow_latency_stays_within_traffic_budget`

```text
baseline executable
  /home/e/.cache/nando-wave-s1c1-baseline-target/release/deps/
  f7_generation_shadow_v3-257d2fa93e7c240e
baseline executable SHA-256
  ab31fde97776084de499e8d70ff3ade6d20a9d05dba912e69e5d069c777e6656

candidate executable
  /home/e/.cache/nando-wave-s1c1-target/release/deps/
  f7_generation_shadow_v3-257d2fa93e7c240e
candidate executable SHA-256
  99c8b9fe8c8e192c418aa1057bec0380c568f666166d40674685aa2132982277
```

Targeted S1C hot-path test:

`package::tests::capture_disabled_compatibility_latency_stays_within_hot_budget`

```text
candidate executable
  /home/e/.cache/nando-wave-s1c1-target/release/deps/
  nando_response_actor-94c534b357a046f6
candidate executable SHA-256
  dd785c1c96122aa1c6aa33f5f637d92636346b15d55902659cfe067c127a124b
```

Precommitted result verifier:

```text
path
  ops/remote-backend/verify_s1c1_resource_v2.py
SHA-256
  1f1c10cc873bf6f4c12bb499979632798155996be684e49fa48fd88dd454bf5a

verifier tests
  ops/remote-backend/test_verify_s1c1_resource_v2.py
SHA-256
  0b447aa3b199b2dd39a420eae6c1c3c2dfb0ece7c6738633ca7a7d6a5a9995b2
remote mini-PC result
  7 PASS / 0 FAIL
```

The verifier tests use synthetic metric documents only to prove terminal math
and fail-closed parsing. They are not natural evidence, benchmark evidence, or
runtime authority.

Every executable must still list the exact frozen test before measurement.
Rebuilds, replacement binaries, changed hashes, and alternate test names require
V3. Existing binaries are executed directly; Cargo is not invoked during the
measurement set.

## 4. Machine And Runtime Freeze

Measurements start immediately after the paper-only protocol commit and gate.
They do not wait for an idle server and do not stop, throttle, reprioritize,
restart, or reconfigure production.

Before the set, record:

```text
boot ID
kernel release
CPU model
rustc version used by the frozen artifacts, when available
load average
production service MainPID / NRestarts
connector MainPID / NRestarts
```

Before and after each invocation, record wall time, `/proc/loadavg`, and service
survival. These observations describe interference; they do not permit deleting
or replacing a run.

Any production restart, PID change, binary hash change, missing metrics line,
wrong sample count, or test-name mismatch makes the complete set terminal
`INVALID_ENVIRONMENT`. It cannot be rerun under V2.

## 5. Exact Schedule

There are exactly three targeted S1C runs and three inherited A/B pairs. One
process runs at a time. Every invocation uses `taskset -c 4`,
`RUST_TEST_THREADS=1`, `--ignored`, `--exact`, `--nocapture`, and
`--test-threads=1`.

The order is frozen:

```text
T1  targeted S1C candidate
P1  inherited baseline -> inherited candidate

T2  targeted S1C candidate
P2  inherited candidate -> inherited baseline

T3  targeted S1C candidate
P3  inherited baseline -> inherited candidate
```

A fixed two-second gap follows every invocation. The gap is not conditioned on
load. No warmup run, replacement run, fourth run, outlier deletion, or
post-result rebuild is allowed.

The inherited test may exit nonzero because it contains the V1 absolute
assertions. The runner must preserve the exit code and metrics line. V2 derives
its verdict from the frozen rules below; a nonzero exit remains an explicit
absolute failure and is never hidden.

## 6. Frozen Measurements

Each targeted S1C run must emit exactly 4,096 matched and 4,096 no-goal samples:

```text
matched_p99_ns
no_goal_p99_ns
hard_max_ns
samples
exit_code
```

Each inherited invocation must emit exactly 4,096 matched and 4,096 no-match
samples:

```text
matched_p99_ns
no_match_p99_ns
hard_max_ns
samples
exit_code
```

All integers must be positive except an exit code. Missing, duplicate, malformed,
or conflicting metrics make the set `INVALID_ENVIRONMENT`.

## 7. Targeted Absolute Gate

The original budgets are unchanged:

```text
matched p99                          <= 1,000,000 ns
no-goal p99                          <=   250,000 ns
hard max                             <= 2,000,000 ns
required result                      PASS 3/3
```

Any targeted run above any budget is `VETO`. The inherited sentinel cannot
override a targeted failure.

The previously passing sync, RSS, idle CPU, serving-parity, false-accept,
runtime-parity, test, Clippy, format, and structural receipts remain bound to
the same frozen source manifest. They must be rechecked for identity and
survival, but V2 does not manufacture replacement observations for them.

## 8. Inherited Paired Regression Gate

The V1 absolute classification is retained for every invocation:

```text
absolute PASS =
  matched_p99_ns <= 1,000,000
  and no_match_p99_ns <= 250,000
  and hard_max_ns <= 2,000,000
```

For each pair and each metric compute the exact rational candidate/baseline
ratio without rounding. Define the pair regression factor as the maximum of:

```text
candidate matched p99 / baseline matched p99
candidate no-match p99 / baseline no-match p99
candidate hard max / baseline hard max
```

The inherited regression gate is PASS only when all conditions hold:

```text
candidate absolute PASS count >= baseline absolute PASS count
median of three pair regression factors <= 1.10
every individual pair regression factor <= 2.00
candidate median matched p99 <= 1.10 * baseline median matched p99
candidate median no-match p99 <= 1.10 * baseline median no-match p99
candidate median hard max <= 1.10 * baseline median hard max
```

Cross multiplication with unsigned integers is used for every `<= 1.10`
comparison:

`candidate_value * 100 <= baseline_value * 110`

The 10% margin is a frozen ordinary-load non-regression budget, not a new
product latency budget. If baseline passes all three absolute runs and candidate
fails any absolute run, the result is `VETO` regardless of ratios.

If both sides miss an absolute inherited budget but the paired regression gate
passes, report:

`INHERITED_ABSOLUTE_ENVIRONMENT_FAIL / RELATIVE_NON_REGRESSION_PASS`

That state can accept the pure S1C-1 code only when the targeted absolute gate
passes 3/3. It cannot support deployment, live capture, or a latency claim about
production.

## 9. Result Matrix

```text
identity drift or malformed set
-> INVALID_ENVIRONMENT
-> no rerun under V2

targeted absolute gate fails
-> VETO
-> no commit of implementation

inherited paired regression gate fails
-> VETO
-> no commit of implementation

all targeted and paired gates pass
-> S1C-1 RESOURCE PASS
-> precommitted verifier returns PASS
-> rerun strict tests, Clippy, format, parity, structural boundary
-> commit and push S1C-1 implementation plus final receipt
-> no deployment
```

S1C-2 remains a separate shadow-producer slice. S1C-3 remains the only
transactional deployment slice and must satisfy a fresh product absolute gate.

## 10. Stop Rules

Stop before measurement unless the adversarial critique and split NANDA routes
for protocol identity, evidence chronology, metric math, and slice authority
all pass without `WATCH` or `VETO`.

Stop after measurement on any of:

```text
source or executable identity drift
run count or order drift
service restart or PID drift
missing or duplicate metrics
sample count other than 4096
targeted absolute failure
paired regression failure
false accepts > 0
runtime parity failures > 0
serving parity mismatch
structural WATCH or VETO
```

No threshold, ratio, run count, order, metric, denominator, binary, test, or
terminal rule may change after the protocol commit. A required repair creates
V3 and a new post-change watermark.

## 11. Pre-Measurement Gate Receipt

No benchmark invocation was executed while drafting, critiquing, repairing, or
structurally checking V2. Binary inspection was limited to SHA-256 and test-name
listing.

The initial over-separated worksheets correctly returned `VETO` because source
and candidate groups appeared as duplicate owners. They were repaired by
expressing each route as one contract relation with distinct evidence links,
not by combining routes or changing the protocol.

```text
NANDA self-check                         PASS
NANDA doctor healthy                    true

identity route                          PASS
  gate SHA-256
  61ab2dce8de914f509f4063ec76b4f0b2e8752421eda6124ba7e2370f296b4b0

chronology route                        PASS
  gate SHA-256
  a30a0a274bdde7cfffeaf7a246cfe60ccfeda54c3f778964f7fa7cbbff008915

metric and terminal math route          PASS
  gate SHA-256
  97789d7881a0ce2d250c5433e2bb48c3ab03a363ea55543b1998f3709a5fdc77

slice authority route                   PASS
  gate SHA-256
  a04cb1c5dfe1211c337e8101f590104c61edaf38f01404af9abd423545eaccf2

weak triads                              none
conflicts                                none
foreign pull                             none
owner conflicts                          none
negative hits                            none
repair queue                             empty
authority_ready                          false
measurement started                      no
```
