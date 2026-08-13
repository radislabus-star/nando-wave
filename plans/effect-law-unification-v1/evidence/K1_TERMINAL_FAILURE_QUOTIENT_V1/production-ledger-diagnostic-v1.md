# K1 Terminal Failure Production Ledger Diagnostic V1

Scope: read-only projection of the production-copy scheduler ledger captured before implementation. This receipt has no selection, deployment, certificate, or Law #2 authority.

Source ledger: `/tmp/k1-terminal-failure-quotient-v1-baseline-B2zz2fwF/k1-epistemic-scheduler-ledger-v1.json`; bytes `2246130`; file SHA-256 `00e44ee2c9127c71231bb2b413500fbe1a4693e1c834c5cf061f60c8df8cd362`.

Source anchor: `/tmp/k1-terminal-failure-quotient-v1-baseline-B2zz2fwF/k1-epistemic-scheduler-anchor-v1.json`; bytes `576`; file SHA-256 `882fb350544622c7f2e616eb5157e442379d7549bc286cbdf58c09fd55e4c197`.

Validated ledger projection: schema `nando.k1-natural-scheduler-ledger.v1`; revision `1174`; ledger root `365f4eda97c38d1d7b817896971ae8789a83c0566a8401b862e192d9f5cd3acc`; `586` candidate freezes; `585` terminal verdicts; see this line for the compact signed-pair evidence span.

Exact signed V6 `candidate_freeze -> terminal_verdict` pairs with `ACQUISITION_FAIL + motif_program_candidates_empty`: `211` pairs, `211` generations, `175` distinct candidate structural roots.

Current cost-8 family evidence: capture `2cee19cd0032342e8faca74380c3a9018134358f9322a3e2d5f9b4f505d997dc` / scalar has `15` generations and `15` distinct motif roots; capture `12554dab3c559ca0615ce88ead725d024764aa11b7bf88a5851976f56da2d79a` / scalar has `6` and `6`; capture `b914ccb75722af1de5305789bad9e1cfb5c934bb40d88cd3e5514fec38a93d33` / collection has `7` and `7`.

Projection key: Epistemic Registry root, fixture exclusion root, capture generation root, consequence type, semantic novelty signature root, generator schema, discovery basis root, and bounded discovery cost units. Candidate root and evidence manifest root are not grouping inputs.

The receipt records observed repetition only. The implementation must replay and validate the signed ledger itself; this checked-in diagnostic must never become runtime authority.
