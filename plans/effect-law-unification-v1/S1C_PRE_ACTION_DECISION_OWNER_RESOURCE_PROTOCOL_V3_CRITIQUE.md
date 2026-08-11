# S1C Pre-Action Decision Owner Resource Protocol V3 Critique

Status: `ADVERSARIAL REVIEW PASS / STRUCTURAL 4 OF 4 PASS / MEASUREMENTS NOT STARTED`

Date: `2026-08-11 Europe/Tallinn`

Reviewed artifact:
`S1C_PRE_ACTION_DECISION_OWNER_RESOURCE_PROTOCOL_V3.md`

## 1. Review Scope

This review asks whether V3 repairs V2 without laundering old measurements,
weakening the resource contract, or creating deployment authority.

It does not reopen S1C semantics, K1 scientific authority, natural evidence,
or the V1 F8-D verdict.

## 2. Adversarial Findings And Repairs

| Priority | Finding | Risk | Repair in V3 |
|---|---|---|---|
| P0 | V2 used one 64-hex validator for both SHA-256 roots and a 40-hex Git SHA-1. | Every honest measurement document was rejected, while an invented 64-hex surrogate could pass shape validation. | V3 has separate `require_root` and `require_commit` functions and tests both directions explicitly. |
| P0 | A protocol document cannot contain the hash of the Git commit that contains that document. | Attempting an exact self-hash creates a circular identity or encourages a placeholder patched after measurement. | V3 freezes an exact parent commit and independent epoch root before commit. The runner requires the eventual paper commit to be the pushed direct child; the verifier checks the epoch, parent, 40-hex commit, and commit-derived directory name. |
| P0 | V2 verifier trusted a manually transcribed measurements JSON. | Correct raw logs could be attached to altered metrics or vice versa. | V3 verifier reads the raw evidence directory, recomputes its canonical manifest, parses each metric and exit, and requires exact equality with JSON. |
| P0 | The V2 handoff searched a seven-character directory while the runner created an eight-character directory. | A complete evidence set was temporarily misclassified as missing. | The V3 runner and verifier derive the same exact directory name from `protocol_commit[:8]`; the verifier rejects any path/name mismatch. |
| P0 | V2 local connector evidence was captured outside the remote evidence directory. | Connector survival depended on recovering a session transcript after the fact. | V3 stores connector before/after snapshots inside the exact 40-file evidence set and binds them into the manifest. |
| P0 | Generic shell fail-fast can erase expected nonzero inherited test results. | A legitimate absolute failure could terminate the runner before its metric and exit code are recorded. | V3 disables fail-fast only around each test process, records the exit, restores fail-fast immediately, and continues the fixed schedule. |
| P1 | A service could restart between runs while final health still looks good. | Final-only health would hide measurement interference. | V3 compares five service `MainPID / NRestarts / ActiveState` tuples across all 19 remote snapshots. |
| P1 | A claimed two-second schedule could differ from the actual chronology. | Manual labels could conceal reordered or back-to-back runs. | V3 parses every before/after timestamp in the frozen label order and requires each post-run gap to be at least 1.9 seconds. |
| P1 | Reusing V2 raw observations after repairing the verifier would be post-result protocol selection. | V3 could appear successful without a post-change future. | V3 explicitly rejects V2 evidence and requires one fresh directory whose name is derived from the pushed V3 commit. |
| P1 | The live CPU health field currently reports response parity as nullable. | Null could be silently coerced into zero parity failures. | The raw evidence gate uses health only for service liveness and false accepts. Fresh post-PASS parity tests remain a separate mandatory gate. |
| P1 | A structural coherence PASS could be reported as implementation or deployment authority. | NANDA could accidentally promote the slice. | All structural checks remain coherence-only with `authority_ready=false`; V3 always emits `deployment_allowed=false`. |
| P1 | Committing protocol files could accidentally stage the dirty candidate. | The candidate might enter Git before the resource verdict. | The paper commit stages only V3 runner, verifier, tests, protocol, critique, and structural receipts. Candidate identity is rechecked immediately before and after. |

## 3. Identity Argument

The identity chain is deliberately non-circular:

```text
frozen parent
+ frozen epoch root
+ frozen candidate source manifest
+ frozen runner/verifier hashes
-> paper-only commit
-> pushed branch HEAD
-> fresh evidence directory named from that commit
-> canonical raw-evidence manifest
-> measurements JSON
-> verifier verdict
```

The epoch root is not claimed to be the Git commit hash. The Git commit is not
claimed to be SHA-256. Each identifier has one type and one owner.

## 4. Remaining Limitations

Three pairs do not estimate a population latency distribution. They are a
bounded regression sentinel under ordinary mini-PC load.

The binaries are content-addressed measurement identities, not
reproducible-build proofs. Source acceptance additionally requires the frozen
source manifest and fresh strict checks.

The canonical manifest proves byte identity of the captured files, not that
the operating system clock or scheduler is scientifically controlled.
Alternating order, a pinned CPU, service survival, and fixed run count bound
that uncertainty without pretending to remove it.

The evidence verifier does not grant runtime authority. S1C-3 still requires a
fresh product absolute gate and transactional deployment protocol.

## 5. Rejected Alternatives

```text
change V2 verifier in place
  rejected: destroys the immutable V2 receipt and hash

pad the Git SHA-1 to 64 characters
  rejected: changes the identifier's type and manufactures a passing shape

hash the Git SHA-1 and put that in protocol_commit
  rejected: field would no longer contain the protocol commit

reuse V2 measurements with V3 verifier
  rejected: verifier repair would be selected after observing the data

omit raw logs after transcribing JSON
  rejected: removes the independent transcription check

rerun after malformed or missing V3 evidence
  rejected: optional stopping; a repair requires V4

deploy to obtain a more realistic latency sample
  rejected: deployment belongs to S1C-3
```

## 6. Review Verdict

The V3 design repairs the actual V2 identity defect and the evidence-binding
weakness exposed during recovery. Its thresholds, schedule, candidate, and
authority boundary remain unchanged.

```text
protocol identity route                 READY FOR GATE
raw evidence binding route              READY FOR GATE
measurement chronology route            READY FOR GATE
slice authority route                   READY FOR GATE
measurements started                    NO
implementation changed after freeze     NO
deployment authority                    false
```

## 7. Structural Result

```text
identity                                PASS
  9e534efdd58ebfe8bd6ede4972f54181edab6df0d2191e2d156310438bf3405a
evidence binding                        PASS
  88189487407c7c0944ff09fe3816059e7f70d3d27b33ef152cf034506b470862
chronology                              PASS
  0d97776183a6fa5e8131b05e450154ac51128382f348913645f9ab5c21095de7
slice authority                         PASS
  6c401b80c4d0a1094ffda6bd83671c1f581cb4b2329221c23e2070ea9e7b0205
WATCH                                   none
conflicts                               none
foreign pull                            none
owner conflicts                         none
negative hits                           none
repair queue                            empty
authority_ready                         false
```

The first worksheets returned `VETO`: broad evidence spans and mixed runner /
verifier owners made the structural claims ambiguous. The repaired worksheets
assign one decision owner per route and compare the same contract relation
against independent document and code evidence paths. No threshold, schedule,
candidate byte, or authority rule changed during that repair.

All four repaired packets pass without `WATCH` or `VETO`. This coherence may
authorize the paper commit and one frozen measurement set, nothing else.
