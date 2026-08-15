# K2 Self-Formed Uncertainty R7 Discrepancy

Status: `P0 FOUND IN DEVELOPMENT / CONFIRM NONCE ABSENT`

Date: `2026-08-15`

Authority: `FALSE`

## Finding

The V2/V3 contract requires every four-model case to reach one surviving
semantic class after one selected probe. That requirement is impossible for
the `U2DoubleTwo` and `U4DoubleTwoRisk` families produced by the frozen
factorized learner.

The contradiction was exposed only after R7 expanded the real process route
from one case to all sixteen development cases. The all-case precommit passed,
case 1 completed, and case 2 failed in the independent final verifier with:

```text
k2_composition_invalid:self_formed_final_semantic_elimination_failed
```

A bounded development-only census of that exact public case reported:

```text
case
2743a8395253938e22cc15666bad68cd608046a4cbeb661b9296cfdb42076525

family                         U4DoubleTwoRisk
matched_pair                   1
complete quotient classes     11
partition geometries           [2,2] x 6; [4] x 5
selected partition             [2,2]
true surviving classes         2
```

There was no `[1,1,1,1]` probe in the complete frontier. The predecessor
selector therefore chose the correct minimax winner among the probes that
actually existed.

## Impossibility Proof

For `U2` and `U4`, support leaves two independent ambiguous action slots with
two effects each:

```text
action A survivors     2
action B survivors     2
Cartesian models       2 x 2 = 4
```

Every `K2InquiryProbeV1` contains exactly one action ID and one initial state.
Its observable outcome depends on the effect assigned to that selected action.
Models that differ only in the other ambiguous action therefore produce the
same outcome for every possible single-action probe.

Consequently:

```text
minimum largest outcome partition     2
maximum single-probe elimination       2 of 4
minimum residual semantic classes      2
```

Changing paths, contents, support order, stable hashes, private truth, risk, or
cost cannot remove this lower bound. It follows from the product factorization
and single-action observation boundary.

## Impact

This is a paper-contract defect found before source/test freeze, before confirm
nonce creation, and before any sealed attempt. It is not a scientific FAIL and
does not invalidate completed R0-R6 ownership, induction, complete-frontier,
tournament, dispatch, worker, observer, or independent-verifier work.

The following apparent fixes are forbidden:

```text
rename U2/U4 while secretly generating single-four cases
accept two survivors as one
use private truth to choose a convenient probe
inject a family label or preferred action into public requests
drop the U2/U4 cases from the denominator
claim the selector failed when the frontier lacked a closing probe
```

## Required Repair

The smallest honest repair is a bounded outcome-blind closure plan:

```text
complete public frontier
-> unchanged predecessor single-probe winner
-> if its worst-case partition is one: freeze one probe
-> otherwise combine that winner with every other complete representative
-> derive every joint partition from precommitted public predictions
-> freeze the best closing second probe before any outcome
-> execute each probe from its own immutable initial state
-> independently verify the ordered observation vector
-> require exactly one surviving private semantic class
```

This changes probe-plan cardinality and execution receipts. It does not change
the induced models, raw frontier, predecessor selector source, private answer,
authority boundary, production state, or confirm chronology.

## Disposition

```text
V2 one-probe closure clauses       SUPERSEDED_PENDING_V4
V3 quotient/tournament repair      PRESERVED
R7 full-process result             EXPECTED FAIL, PRESERVED
confirm nonce                      ABSENT
sealed attempts                    0
next permitted action              V4 draft -> critique -> structural gate
                                   -> preflight delta -> R7 repair
```
