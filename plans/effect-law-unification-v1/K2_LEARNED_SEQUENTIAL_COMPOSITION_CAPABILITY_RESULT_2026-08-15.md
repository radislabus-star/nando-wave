# K2 Learned Sequential Composition Capability Result

Status: `K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS`

Date: `2026-08-15`

Authority: `FALSE`

## 1. Result

The preregistered bounded generated experiment passed its exact contract:

```text
observe isolated before/after transitions
-> induce six hidden action effects
-> freeze the learned law sets
-> reveal independent targets and exact goals
-> enumerate the complete bounded program denominator
-> select an unprovided depth-three semantic class
-> verify the complete planner output independently
-> execute the selected composition in a separate sandbox
-> observe the real filesystem independently
-> verify exact goal satisfaction in a separate oracle process
```

This establishes a concrete capability that had not been proved by the prior
one-step Law Lab result:

```text
learn effects -> compose learned effects -> act -> attain an exact goal
```

The selected programs were not supplied as a DAG, expected sequence, private
mapping, fixture label, or planner hint. The learner saw opaque action IDs and
before/after observations. The planner received only the frozen learned laws,
target state, exact goal, and bounded search contract.

## 2. Exact Denominators

```text
disjoint fixture routes                         2 / 2
opaque actions                                 6
real isolated support executions              18 / 18
uniquely induced action laws                    6 / 6
candidate programs                             30 / 30
independently reconstructed candidates         30 / 30
positive sequential target executions           2 / 2
negative target executions                      1 / 1
separate exact-oracle processes                  3 / 3
negative controls                              18 / 18
journal events per route                       29 / 29
restart parity                                 every prefix
residual generated paths                         0
```

Planner accounting:

```text
main route          8 valid + 7 inapplicable = 15
topology route      3 valid + 12 inapplicable = 15
minimum satisfying depth                         3
satisfying strict prefixes                       0
main satisfying semantic-class members           3
topology satisfying semantic-class members       1
```

The different topology route prevents the result from being only one prepared
copy-chain shape. Equal schedules collapse only under the frozen exact semantic
quotient: equal depth, action multiplicity, and terminal manifest.

## 3. Release Evidence

The decisive release run emitted:

```text
capability root        a3e4526dd300a8ccbbe3fa3ed533e3dc49b6f3b18f00ee6c5538f76172685857
main plan root         5ae9f483520be581ba31759112dfa4a640d0ecd4c94536a5f816088f8f4ba2af
topology plan root     98d5a2040624e1c1f4f713ce3c51e7aee343119b2bc07578d1b00f53b7cc195d
main journal root      72bc17fcaca57c6744b118f655f2891ec8482964cf01df79bcdbc371d0fb57be
topology journal root  8318264d4416d1224f691f2041c2982fe6eb120421a4485953722a9913b72e5d
ablation root          f75a063a17266e62ecccdc0a99573d13494c70d71556a91659079fd1a172f84f
```

Final release executable roots:

```text
effect learner     a11bfbe334096359ecc67b88e7d8f07c17816169cd787e8907931da3b1059441
planner            d14533031e819437b7d990383a3b4d971a1756e0b559c24c3960035c1eb1e81d
sequential worker  6dde0f6d26b01245ab265cf5b071c2d1872ea3105dc667d9b61f7a62ceec6782
exact oracle       d4f7ce5d55a2356542dcd0d763d19dd7ee70ef7595c6b91529045e63920b6602
```

Machine receipt:

```text
evidence/K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_V1/capability-receipt.json
SHA-256 95baf02f6a20a5b6bf884f8a47a0c00b5830ce0f775770273285e266ecb4ebb0
```

## 4. Verification

All Rust builds and tests ran on `e-MEGA-MINI-M1-13th` with 20 build jobs and
the dedicated target directory
`/home/e/.cache/nando-wave-k2-composition-target`.

```text
cargo check --all-targets                       PASS
cargo fmt --all -- --check                      PASS
cargo clippy --all-targets -- -D warnings       PASS
nonignored package tests                        449 PASS
ignored real-process regressions                  6 PASS
release decisive composition test                 1 PASS
prior frozen source baselines                    17 PASS
allowed additive sibling registration             1 exact two-line change
NANDA structural packets                          4 PASS, authority false
observed-source route markers                     42 / 42 PASS
planner/verifier implementation dependency         0
journal injected fault points                      2 / 2 PASS
same-identity retry after durable dispatch         FORBIDDEN
```

The observed-source route receipt has 21 nodes, 21 edges, and 15 separated
execution, authority, observation, and proof routes. It reports no issues or
warnings. The independent verifier does not import or call the planner
transition implementation.

Every preflight-frozen Law Lab V1, K2 V1, learned-capability, and prior test
file retained its exact SHA-256. The only old source edit is the preregistered
additive `learned_composition` sibling registration. `graphify-out/` was not
updated or modified by this work.

## 5. What The PASS Means

Within a finite, resettable filesystem language, Nanda can now:

1. infer action laws from real isolated interventions;
2. use those learned laws as a world model;
3. discover a multi-step causal program that was never supplied;
4. execute it in the real sandbox state rather than only simulate it;
5. have separate code reconstruct the planner denominator and exact outcome.

This is the first evidence-backed baseline for a verified causal action agent
in this project. It is stronger than a rule table because the hidden effects
are induced from observations and then composed on an unseen target. It is
stronger than plan-only search because the selected composition is executed
and its state transition is observed independently.

## 6. What The PASS Does Not Mean

The result does not establish any of the following:

```text
natural K2                                              NOT PROVED
self-chosen causal inquiry                              NOT PROVED
self-created state predicates or action language       NOT PROVED
hidden representation superior to explicit search      NOT PROVED
open-ended planning                                     NOT PROVED
production execution authority                         FALSE
LawCertificate or K1 registry membership               FALSE
Wave-caused whole-circuit grokking for this capability NOT PROVED
general intelligence                                    NOT CLAIMED
```

Humans still supplied the finite observable language and safe operation family.
The planner uses complete bounded enumeration, which will not scale by itself.
The fixtures are generated and resettable, not natural ordinary LLM traffic.

## 7. Next Scientific Step

Do not widen this PASS into production or natural authority. The next useful
experiment is the preregistered successor suggested by the critique:

```text
same learned L1 law vocabulary
-> generated train split
-> explicit complete planner baseline
vs
-> bounded hidden composition representation
-> sealed confirmatory split
-> exact goal and counterexample comparison
```

That test asks whether a learned hidden representation can preserve exactness
while avoiding exhaustive enumeration. Only after that should the project move
to self-chosen safe probes and then learned state-language growth.

The broader AI direction and its claim boundaries are recorded in
`VERIFIED_CAUSAL_AI_DIRECTION_V1.md`.
