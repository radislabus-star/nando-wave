# Natural Vocabulary Census V1

## Result

The first source-rooted census of the ordinary multi-source archives ended:

```text
verdict                         NO_READY_FORM
selected missing form          NONE
authority                      false
phase mutation                 false
report root                    94c3767d4a9d7e442db6a25dd5cb37942f2f4de0253f2f3214cf942a39f6c5a9
source root                    6c4e223333ceba5bb1bcf977fc88823145120ff186f673186f64a5535ca27a16
```

This is a terminal census snapshot, not a natural-generation verdict. It does
not write the scheduler ledger, freeze a candidate, issue a certificate, or
change serving authority.

## Why The Census Exists

The deployed Law Lab can distinguish an already ambiguous frozen version
space, but the authoritative K1 ledger contains no such target. Building an
active probe controller at this point would therefore create an idle runtime.

The census asks the narrower prior question:

```text
ordinary completed traces
-> physical executable forms
-> exact self replay
-> independent verifier compilation
-> source-neutral binding coverage
-> current identifier vocabulary coverage
-> one ready missing generic form or NO_READY_FORM
```

It never maps traffic to a desired hand-written program family.

## Frozen Readiness Boundary

A missing form is only ready when Capture V2 evidence independently satisfies:

```text
settled rows       >= 8
verified rows      >= 2
lineages           >= 2
exact replay       PASS
verifier compile   PASS
```

The census also keeps two failures separate:

```text
missing vocabulary
  current physical form has no identifier protocol representation

binding gap
  physical form is already represented, but source-neutral role binding fails
```

Conflating them would incorrectly promote a capture/representation repair into
a new action language.

## Live Snapshot

```text
topology rows                  45 766
relation frames               10 033
joined / accepted              8 847 / 8 847
rows without physical form        16

advance_plan
  historical rows                  7
  historical lineages              2
  exact replay / verifier           7 / 7
  Capture V2                        0
  source-neutral                    0
  current protocol representation  0
  verdict                           NOT READY

function_call_from_roles
  rows                          8 014
  Capture V2 readiness          1 382 / 1 382 / 14 lineages
  source-neutral                6 264
  current protocol rows         7 975
  verdict                       EXISTING VOCABULARY

custom_tool_call_from_roles
  rows                            810
  Capture V2 readiness            377 / 377 / 7 lineages
  source-neutral                    0
  current protocol rows             4
  verdict                       EXISTING VOCABULARY / BINDING GAP
```

Four invalid legacy string-argument programs are counted under their form and
censored diagnostically. They do not abort the archive or enter readiness.

## Hard Stop

`advance_plan` is the only observed physical form missing from the current
identifier vocabulary. It has only seven historical rows and no Capture V2
evidence. Enabling it now would use pre-contract evidence and violate the
preregistered readiness boundary.

Therefore this checkpoint does not:

- extend `OperatorIdentificationMachineV1`;
- open a K1 generation;
- start a probe controller;
- create synthetic traffic;
- change a service threshold.

## Next Natural Trigger

```text
ordinary Capture V2 advance_plan traces
-> 8 settled / 2 verified / 2 lineages
-> repeat source-rooted census
-> one frozen natural generation
-> existing identifier support / replay / semantic quotient
-> unique law or Law Lab distinguishing probe
-> independent natural holdout
-> LawCertificate #2
```

After Law #2 and Law #3:

```text
K1 OPEN
-> natural L2 composition
-> strategy laws
-> learning operators
-> self-growing verified action language
```

## Implementation

- `crates/nando-operator-learning/src/multi_source/natural_vocabulary_census.rs`
- `crates/nando-operator-learning/examples/natural_vocabulary_census_v1.rs`
- `crates/nando-gateway-control/src/live_dashboard.rs`

All builds, tests, and archive calculations are run on the mini-PC. The local
workstation performs source editing only.
