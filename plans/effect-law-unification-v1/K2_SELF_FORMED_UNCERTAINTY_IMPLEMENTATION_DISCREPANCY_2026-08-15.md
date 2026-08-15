# K2 Self-Formed Uncertainty Implementation Discrepancy

Status: `P0 FOUND BEFORE R1 CODE / CONFIRM NONCE ABSENT`

Date: `2026-08-15`

Authority: `FALSE`

V2 root:
`7875d8809b9340774170d2468b07302e17e503712728173b6efb699f9b768a95`

## Finding

V2 requires all `1,792` raw probes to be quotiented by the ordered vector of
four exact observable post-manifest roots and requires exactly eight quotient
classes. Those two requirements cannot both hold in the frozen grammar.

For one raw probe, every permitted effect can modify at most one destination
path:

```text
CopyFile(source, target)  reads source and can replace target
RemoveFile(path)          can remove path
```

At least two of the four paths are therefore unchanged by every prediction for
a fixed action. Assigning any of the four public states to those two untouched
paths gives at least `4^2 = 16` distinct exact post-manifest vectors. Because
the exact post-manifest root includes unchanged entries, those vectors cannot
be equal. An eight-class absolute-outcome quotient is impossible before any
fixture or private mapping is chosen.

An exhaustive development-only check over all U1 four-effect sets confirms the
second issue: even after replacing absolute outcome labels with the exact
pairwise prediction partition used by the scorer, the frozen risk, cost, and
baseline bits produce at least eleven classes, not eight.

## Impact

This is a paper-contract defect, not a scientific result and not a code failure.
It was found before R1 source creation, before executable freeze, and before a
V2 confirm nonce existed. R0 remains valid because it is byte-preserving and
independent of probe semantics.

Continuing V2 literally would force one of three invalid shortcuts:

```text
omit raw probes
hide state identity in an undocumented quotient
hand-pick eight prepared probe roles
```

All three satisfy the primary null rather than the claim.

## Required Repair

V3 must:

1. retain all exact raw predictions and post-manifest roots as witnesses;
2. quotient only by the scorer-observable equality partition plus frozen
   eligibility, risk, cost, and baseline bits;
3. retain every resulting representative, with no top-k or sampling;
4. use the byte-identical predecessor scorer as a deterministic tournament over
   exact eight-probe requests;
5. prove the tournament winner equals direct application of the frozen ranking
   tuple to the complete representative set;
6. make all adapted prediction denominators derived and checked rather than
   asserting the invalid fixed `512` count.

No generator mapping, private answer, topology label, outcome, or human probe
role may enter quotient or tournament bytes.

## Disposition

```text
V2 paper                         SUPERSEDED FOR PROBE QUOTIENT/ADAPTER ONLY
V2 confirm nonce                 ABSENT
V2 scientific attempt           NOT STARTED
R0 ownership split              PRESERVED
predecessor selector source      BYTE-IDENTICAL
next permitted action            V3 critique -> preflight delta -> R1
```

