# S1C-3F Magic-Only Ledger Critique V1

Status: `ADVERSARIAL REVIEW PASS / STRUCTURAL 4 OF 4 PASS / AUTHORITY FALSE`

Reviewed artifact: `S1C3F_MAGIC_ONLY_LEDGER_PREREGISTRATION_V1.md`

| Priority | Finding | Risk | Applied repair |
|---|---|---|---|
| P0 | Four format bytes were called a scientific row. | A correct runtime open was rejected. | Define emptiness as exact `NTF1` plus zero framed records, not zero bytes. |
| P0 | Accepting any four-byte file would be too weak. | Forged magic or truncated data could pass. | Require exact bytes, SHA-256, owner/mode, three filenames, bounded framing, and zero trailing bytes. |
| P0 | Reusing S1C-3E would rewrite a terminal result. | Evidence history would be corrupted. | Preserve S1C-3E and create a new paper/source/transaction identity. |
| P0 | Removing the preserved files before S1C-3F would erase the runtime-open witness. | The repair would manufacture a cleaner baseline. | Bind the existing journal root and forbid all journal mutation by the transaction. |
| P0 | Requiring magic-only files throughout survival could censor a natural future. | A valid post-cursor episode would force rollback. | Freeze cursor immediately; later valid suffix frames are allowed and prefix-preserved. |
| P0 | A preexisting frame could be silently admitted into S1C-4. | Retroactive evidence would enter the denominator. | Require zero records at opening; any pre-cursor frame is a veto. |
| P1 | Repeating latency/parity runs could select a better sample. | Frozen WATCH could be laundered. | Inherit exact S1C-3D roots and candidate artifacts without rerunning measurements. |
| P1 | Installation could be presented as grounded meaning. | Infrastructure would impersonate science. | Keep scientific, K2, training, and phase authority false; open only `COLLECTING`. |

Rejected: stripping `NTF1`, truncating segments, generating a probe row,
raising thresholds, changing the Rust format, or rerunning S1C-3E.

Verdict: implementation may proceed only with independent record parsing,
immutable parent roots, journal non-mutation, natural-suffix preservation, and
the one-attempt rule.
