# K2 Self-Chosen Safe Inquiry True-Choice Boundary Repair V1

Status: `FROZEN BEFORE CONFIRM REVEAL`

Date: `2026-08-15`

Authority: `FALSE`

## 1. Defect Preserved

The V1 preregistration and critique required the private true-model root value
to be bytewise absent from selector and baseline inputs. That requirement is
impossible as written: the public case must contain the roots of all four
candidate models, including the candidate that is privately selected as true.

This is preserved as:

```text
INVALID_DESIGN_ATTEMPT
reason  public candidate identity was confused with the private true-choice relation
scientific attempt consumed  no
confirm preimage revealed    no
```

The original V1 files and Git history remain unchanged.

## 2. Correct Boundary

```text
public candidate identity
  = one of four unlabeled model roots in the public hypothesis set

private true choice
  = the relation that identifies one public candidate as the true model
```

Selector and baseline processes receive every public candidate identity. They
must not receive the private true-choice relation, a field that labels one
candidate as true, the resolved private effect, or any post-outcome bytes.

The private choice may enter only after selection has been precommitted and
independently verified. It is then consumed by the dispatch owner and outcome
verifier, never by selector or baseline processes.

## 3. Replacement Leakage Control

The impossible byte-substring test is replaced by all of these checks:

```text
selector schema contains no private-true-choice field
baseline schema contains no private-true-choice field
unknown private-choice field injection is rejected
unknown post-outcome field injection is rejected
selector request bytes are identical for all four possible private choices
baseline request bytes are identical for all four possible private choices
selection precommit is identical for all four possible private choices
```

Seeing an unlabeled candidate root among the four public hypotheses is not
leakage. Receiving which candidate is true is leakage.

## 4. Frozen Surface Unchanged

This repair changes no confirmatory scientific degree of freedom:

```text
confirm commitment              unchanged
generator schema commitment     unchanged
confirm cases                   8
models per case                 4
probes per case                 8
probe budget                    1
risk and cost limits            unchanged
ranking tuple                   unchanged
baseline definitions            unchanged
PASS thresholds                 unchanged
sealed execution count          1
```

No generated fixture, true choice, outcome, executable, or confirm case was
opened while making this repair.

## 5. Structural Check

The repaired ownership route was checked with `nanda-structural-gate`:

```text
verdict                         PASS
safe_to_edit                    true
weak triads                     0
conflicts                       0
repair queue                    0
authority_ready                 false
```

Worksheet SHA-256:
`4544059344d08eb19467646f997c75864ab9ef0d1f003f560e87a0b06d6b2149`.

Raw structural receipt SHA-256:
`64ad3c055f35b5828f14e30e634183dfced91c02d5c57eb5487e6506ec018029`.

This is coherence evidence only and grants no production or scientific
authority.

## 6. Claim Boundary

The experiment still asks only whether a frozen public hypothesis set can
guide one safe generated probe better than the frozen non-oracle baselines.
It does not prove learned strategy, Natural K2, production authority, or a
LawCertificate.
