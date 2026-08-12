# S1C Transactional Deployment Preregistration V4 Critique

Status: `ADVERSARIAL REVIEW / ACCEPTED FOR TEST-ONLY CANDIDATE / NO DEPLOYMENT`

Date: `2026-08-12 Europe/Tallinn`

## Findings

| Severity | Finding | Failure mode | Accepted repair |
|---|---|---|---|
| P0 | V3 summed pre-action durability and post-action settlement across a response execution boundary. | A synthetic sum was labelled as one production latency and rejected an otherwise bounded path. | Measure the two synchronous production stages separately with the same 5 ms p99 and 20 ms hard maximum. |
| P0 | Splitting stages could hide total storage work. | Three expensive fsyncs might pass individually while aggregate cost disappears from evidence. | Retain episode p99 as diagnostic and require aggregate hard max <= 20 ms in every run. |
| P0 | A test-only claim could conceal runtime optimization or weaker durability. | Candidate might skip fsync, batch records, or change append order. | Freeze the only crates diff inside cfg(test), bind the unchanged production projection hash, and compare release size/ELF runtime section dimensions. |
| P1 | Whole-file release hashes differ after a test-only Rust source change. | An impossible byte-identity gate would reject crate fingerprint and ELF Build ID metadata rather than runtime logic. | Freeze the exact V4 binary after build, but use source projection plus runtime parity to prove behavior preservation. |
| P0 | Reusing the V3 harness would make the corrected test cheap. | V4 would inherit artifacts from a terminal attempt. | Fresh checkout, target, harnesses, oracles, ownership receipt, and quiescence evidence. |
| P1 | Warm-up or retry could discard slow first writes. | Reported p99 would use a selected denominator. | No warm-up, exactly 256 records, exactly three preregistered runs, no retry. |
| P1 | Individual ledger timing would not match the post-action lock scope. | Selected and satisfaction writes could be presented as two independent paths. | Settlement timing spans both append calls exactly as production does. |
| P1 | A passing benchmark could be promoted into meaning evidence. | Operational installation would be confused with a natural decision episode. | Keep S1C-4 and K2 closed until ordinary goal/action/outcome evidence exists. |

## Rejected Alternatives

```text
raise p99 to 6 ms
  rejected: changes the frozen performance budget

ignore the 5.767585 ms V3 result
  rejected: violates the terminal V3 contract

parallelize or batch the three fsyncs
  rejected: changes production durability chronology

run a fourth selected measurement
  rejected: adaptive retry

reuse the V3 candidate target
  rejected: stale attempt state
```

## Verdict

```text
5 ms p99 retained for each real critical stage    yes
20 ms aggregate hard ceiling retained             yes
runtime durability semantics changed              no
candidate diff allowed outside cfg(test)           no
whole-file binary identity required                no, Rust metadata differs
exact built V4 binary frozen before metrics        yes
fresh V4 evidence required                         yes
V4 remote attempts                                 one after final freeze
scientific authority                               false
ready for bounded candidate implementation         yes
```
