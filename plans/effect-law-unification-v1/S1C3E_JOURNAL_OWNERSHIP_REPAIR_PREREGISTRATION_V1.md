# S1C-3E Journal Ownership Repair Preregistration V1

Status: `PAPER FROZEN / CRITIQUE PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Date: `2026-08-12 Europe/Tallinn`

Immutable parent evidence:

- S1C-3D attempt `20260812T145640Z-c3eaddc55dfc-s1c3d-v1`;
- S1C-3D state root
  `6ec0baf716a12467b9f7ca6e18bc6e6bf4543f1c95432dc65daf7b3ce5685ffb`;
- S1C-3D resource root
  `c917e62a85d2776e3a20d3efd72b16230a0689c73975b786d6ab8687c1176038`;
- S1C-3D parity root
  `55ae110ce15f198e0741890e856e5822170e1ba479870ea9c03ac4bd34ad3ea9`;
- exact candidate binary SHA-256
  `360498a0908739cad6f1ac21cf4053b7421daaf8b1d9a6502b72132a94a692df`;
- exact candidate config SHA-256
  `1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6`.

## 1. Exact Question

Can the exact S1C-3D candidate be installed by repairing the production
journal-directory ownership boundary, without rerunning resource selection,
parity measurement, or creating a scientific row?

S1C-3D remains terminal and immutable:

```text
S1C-3D resource/parity result       PASS / OPTIMIZATION_WATCH
S1C-3D installation result          ROLLBACK_PASS
S1C-3E ownership repair             new paper, code, root, transaction identity
S1C-4                               CLOSED until S1C-3E installation PASS
```

## 2. Observed Blocker

The candidate process reached `GroundedDecisionShadowRuntimeV1::open`, then
failed inside `FramedCborLedger::open_with_limits`:

```text
nando-grounded-decision shadow unavailable:
framed_ledger_dir:/var/lib/nando-wave/transition/grounded-meaning-v1/
decision-contract-precommits-v1:Permission denied (os error 13)
```

The parent directory is `root:root 0755`; transition-serving runs as `e:e`.
The runtime correctly calls `create_dir_all`, but cannot create the final child.
The S1C-3D transaction then observed no journal and rolled back exactly.

This is not a lazy-journal state and not a durability-latency failure. The
ledger opens its segment files at startup. S1C-3E repairs only the missing
directory owner.

## 3. Frozen Repair

After rollback is armed and before the candidate process starts:

```text
root transaction
-> require final journal path absent
-> mkdir final journal path
-> chown e:e
-> chmod 0700
-> fsync parent
-> require directory empty
-> install exact S1C-3D candidate binary and config
-> start transition-serving as e:e
-> runtime opens exactly three framed-ledger segment files
```

Expected empty runtime-owned files:

```text
decision-precommit-00000000000000000000.cbor
selected-action-binding-00000000000000000000.cbor
goal-satisfaction-00000000000000000000.cbor
```

At installation PASS each file must be a regular `e:e 0600` file with zero
payload bytes. The directory must remain `e:e 0700`. Any other entry, symlink,
nonzero row bytes, ownership drift, or mode drift is a hard veto.

No transaction helper may create any segment file or append any frame. Segment
creation must be attributable to the candidate runtime startup.

## 4. Inherited Evidence

S1C-3E does not repeat the expensive S1C-3D resource and parity experiment.
The independent verifier must bind all of these before mutation:

```text
parent terminal verdict              S1C3D_ROLLBACK_PASS
parent state root                     exact
parent resource root                  exact
parent parity root                    exact
parent hard-gate status               PASS
parent correctness                    PASS
parent operational safety             PASS
parent optimization                   OPTIMIZATION_WATCH
candidate artifact roots              exact
current production binary/config      exact restored baseline
current journal path                   absent
```

The inherited `5 ms` p99 target remains `OPTIMIZATION_WATCH`. The `20 ms` hard
maximum remains the safety boundary. S1C-3E neither reruns nor relabels those
measurements.

## 5. Transaction Boundary

Allowed mutations:

- create the final empty journal directory with frozen owner and mode;
- install the exact inherited candidate binary and config;
- intentionally restart transition-serving once;
- write append-only S1C-3E deployment, rollback, verifier, and cursor receipts;
- after PASS, project S1C-4 as `COLLECTING`.

Forbidden mutations:

- any synthetic, targeted, fixture, or manually authored decision row;
- any precommit, selected-action, or satisfaction append by the transaction;
- Nginx or connector restart;
- response package, admission, K1, phase-memory, or model mutation;
- retroactive S1C-4 rows;
- relabelling or deleting S1C-3D evidence;
- granting scientific, K2, training, or action authority.

## 6. Forward Gates

Installation PASS requires:

1. Local and remote independent predeployment verification are byte-identical.
2. Rollback state is durable before the first production mutation.
3. Candidate binary/config match the inherited roots exactly.
4. Process environment contains the exact capture enable flag and journal path.
5. Boot-scoped logs contain no grounded-decision startup failure.
6. The runtime creates exactly the three empty ledger files with frozen owner
   and mode.
7. Hot, CPU, gateway, and control health semantics remain valid.
8. Active packages, false accepts, runtime parity, and economics authority do
   not regress.
9. Transition-serving survives 15 seconds with one intentional PID change and
   no `NRestarts` increase.
10. Nginx and connector PIDs remain unchanged and route receipt failures remain
    zero.

## 7. Rollback

Any forward failure stops transition-serving, restores the exact prior binary
and config, and starts the prior service. The newly created journal directory
is removed only if it still contains exactly the three expected zero-byte
segment files and no other entry. If a natural append occurred, the journal is
preserved and reported; it is never erased to make rollback look clean.

Rollback PASS requires exact prior binary/config, service health, untouched
Nginx/connector identities, and an append-only reason receipt.

## 8. S1C-4 Cursor

On installation PASS, the verifier freezes:

```text
S1C4AppendCursorV1
├─ S1C-3E deployment receipt root
├─ exact empty journal manifest root
├─ opened_at
├─ retroactive_rows_allowed       false
├─ scientific_authority           false
└─ state                          COLLECTING
```

Only natural post-install appends after this cursor belong to S1C-4. Capture
installation alone proves no decision episode, grounded meaning, K2 law,
training authority, or phase mutation.

## 9. Identity And Attempt Discipline

The paper is committed and pushed before implementation constants are frozen.
The implementation is then committed and pushed before one production attempt.
One source identity permits at most one production mutation attempt. Any defect
requires a new paper/source/root/transaction identity and preserves this result.

## 10. Structural Gate

Four owner-local routes are checked independently:

```text
historical identity and inheritance       PASS
journal ownership and runtime attribution PASS
rollback and natural-row preservation     PASS
installation versus science authority     PASS
WATCH                                     none
authority_ready                           false
```

Structural PASS authorizes implementation within this paper only. Production
authority comes from the separately implemented independent verifier and exact
predeployment roots.
