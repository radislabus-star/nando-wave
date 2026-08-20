# R8B V6 Restart And Cleanup

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| t1 | restart suite | launches | real Development owner process in P01-P07 | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V5.md:504-518 | 1.0 | process test owner | Development owner | execution | process-restart |
| t2 | traced first owner | holds | exact lab-root flock during contender run | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6_CRITIQUE_V1.md:26-26 | 1.0 | lock owner | lab-root inode | concurrency | p07 |
| t3 | started unfinished child | yields | indeterminate without automatic replay | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6_CRITIQUE_V1.md:29-29 | 1.0 | durable process ledger | fail-closed terminal | recovery | child-journal |
| t4 | cleanup authorizer | precedes | cleanup mutation owner | k2_self_formed_uncertainty_confirm_r7k_v1.rs:456-500 | 1.0 | authorization owner | mutation owner | authority | cleanup-order |
| t5 | cleanup verifier | independently checks | retained parity deletion and residue | k2_self_formed_uncertainty_confirm_r7k_v1.rs:511-540 | 1.0 | proof owner | post-cleanup tree | proof | cleanup-proof |
| t6 | Development result publisher | follows | cleanup verification | k2_self_formed_uncertainty_confirm_r7k_v1.rs:542-559 | 1.0 | completion publisher | verified cleanup | chronology | completion |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group |
|---|---|---|---|---|---:|---|---|---|---|
| c1 | restart suite | launches | real Development owner process in P01-P07 | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:177-227 | 1.0 | process test owner | Development owner | execution | process-restart |
| c2 | traced first owner | holds | exact lab-root flock during contender run | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:210-227 | 1.0 | lock owner | lab-root inode | concurrency | p07 |
| c3 | started unfinished child | yields | indeterminate without automatic replay | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:304-307 | 1.0 | durable process ledger | fail-closed terminal | recovery | child-journal |
| c4 | cleanup authorizer | precedes | cleanup mutation owner | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:311-330 | 1.0 | authorization owner | mutation owner | authority | cleanup-order |
| c5 | cleanup verifier | independently checks | retained parity deletion and residue | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:311-346 | 1.0 | proof owner | post-cleanup tree | proof | cleanup-proof |
| c6 | Development result publisher | follows | cleanup verification | K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V6.md:311-346 | 1.0 | completion publisher | verified cleanup | chronology | completion |

## notes

- Pure publication faults remain separate from process restart evidence.
