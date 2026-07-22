# STOP-F7-C Generation Verifier Receipt

Date: `2026-07-22`

Verdict: `COMPLETE_CONTROLLED_PROOF_PASS`

Authority: `false`

## Result

```text
F6 IndependentVerifierReceiptV3
        |
        +-- exact artifact-set root ----------- generation manifest
        +-- exact canonical receipt bytes
        +-- exact request root
        `-- exact verdict
        |
        v
GenerationVerifierReceiptV3                  PASS
        +-- generation ID                    BOUND
        +-- support/future partition          BOUND
        +-- capture sequence + watermark      BOUND
        +-- exact support freeze              BOUND
        +-- lineage + event roots             COMMITTED
        +-- raw payloads                      0
        `-- execution authority               false
        |
        v
GenerationEvidenceLedgerV3 bridge            PASS
        +-- VERIFIED -> VerifiedPass          PASS
        +-- REJECT -> VerifiedPass            REJECT
        +-- foreign generation                REJECT
        `-- duplicate request/receipt          REJECT
        |
        v
trusted live capture-owner join               F7-E / NOT STARTED
atomic checkpoint IO                          F7-D / UNLOCKED
external admission                            F8 / BLOCKED
```

The proof owner preserves the F6 verdict and binds it to exactly one immutable
generation. The learning owner may classify a non-PASS receipt as an
applicability negative, hard contradiction, or censored outcome, but it cannot
turn that receipt into positive evidence.

Lineage and event roots are committed by the envelope, not independently
attested by it. F7-E must join them to the live capture owner's immutable
commitment. Until that join exists, these receipts are controlled evidence and
have no production caller or authority.

## Budgets

```text
generation receipt bytes  <= 16 KiB
raw payload bytes         = 0
production callers        = 0
authority                 = false
```

## Verification

```text
F7-C receipt tests                   4 / 4 PASS
F7-C receipt-to-ledger bridge        1 / 1 PASS
strengthened F7-B ledger tests       6 / 6 PASS
kernel/learning/proof broad gate   240 PASS / 1 ignored perf gate
kernel/proof/learning Clippy         PASS (-D warnings)
runtime all-target check             PASS
changed-file rustfmt                 PASS
foreign generation replay           BLOCK
reject-to-positive relabelling       BLOCK
production callers                   0
services restarted                   NO
deployment changed                   NO
```

Next boundary: F7-D atomic write-new, file `fsync`, rename, directory `fsync`,
quarantine and previous-generation recovery. F7-E live capture joining and
shadow integration remain separate.
