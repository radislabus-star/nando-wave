# NANDA Triad Worksheet

task_id: s1c3h-claim-boundary-implementation-v1
domain: code
query: Does the implementation prove only recorder installation while preserving natural evidence and keeping K2 closed?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t1 | installation receipt | claims | capture installed only | receipt carries capture flag and scientific authority false | 1.0 | operational proof | scoped installation claim | authority | install-claim | proof | independent verifier | final receipt | capture INSTALLED | ops/remote-backend/verify_s1c3h_transaction_v1.py:290 | deployment |
| t2 | natural decision journal | preserves | frozen prefix and ordinary suffix | runtime journal verifier requires append-only prefix preservation | 1.0 | evidence ledger | independent future | evidence | journal | science | journal owner | final verifier | byte-preserved suffix | ops/remote-backend/verify_s1c3h_transaction_v1.py:286 | science |
| t3 | transaction | forbids | generated traffic synthetic fixture and post-hoc goal | no traffic generator exists and paper preflight vetoes remain bound | 1.0 | deployment mechanism | forbidden evidence mutation | authority | evidence-veto | science | evidence owner | orchestrator | no manufactured records | plans/effect-law-unification-v1/S1C3H_IMPLEMENTATION_PREFLIGHT_V1.json:47 | science |
| t4 | first natural record | is_separate_from | installation PASS | receipt reports count but does not require or promote it | 1.0 | ordinary evidence | operational installation | claim | evidence-count | science | census owner | natural journal | S1C-4 collecting | ops/remote-backend/s1c3h_remote_transaction_v1.py:1056 | science |
| t5 | S1C-3H | cannot_grant | K2 grounded meaning Law2 phase mutation or model training | source test rejects all authority true literals | 1.0 | installer | forbidden scientific promotion | authority | claim-boundary | science | claim owner | verifier | K2 CLOSED | ops/remote-backend/test_s1c3h_transaction_v1.py:349 | science |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c1 | installation receipt | claims | capture installed only | candidate c1 | 1.0 | operational proof | scoped installation claim | authority | install-claim | proof | independent verifier | final receipt | capture INSTALLED | candidate_answer:c1 | deployment |
| c2 | natural decision journal | preserves | frozen prefix and ordinary suffix | candidate c2 | 1.0 | evidence ledger | independent future | evidence | journal | science | journal owner | final verifier | byte-preserved suffix | candidate_answer:c2 | science |
| c3 | transaction | forbids | generated traffic synthetic fixture and post-hoc goal | candidate c3 | 1.0 | deployment mechanism | forbidden evidence mutation | authority | evidence-veto | science | evidence owner | orchestrator | no manufactured records | candidate_answer:c3 | science |
| c4 | first natural record | is_separate_from | installation PASS | candidate c4 | 1.0 | ordinary evidence | operational installation | claim | evidence-count | science | census owner | natural journal | S1C-4 collecting | candidate_answer:c4 | science |
| c5 | S1C-3H | cannot_grant | K2 grounded meaning Law2 phase mutation or model training | candidate c5 | 1.0 | installer | forbidden scientific promotion | authority | claim-boundary | science | claim owner | verifier | K2 CLOSED | candidate_answer:c5 | science |
