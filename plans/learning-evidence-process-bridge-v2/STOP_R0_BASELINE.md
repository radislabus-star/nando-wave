# STOP-R0 Baseline

Captured at `2026-07-23T18:04:06+03:00` from the current dirty working tree.
No service was restarted and no authority was changed.

```text
hot process start                 2026-07-23 03:49:53 EEST
cold process start                2026-07-23 13:41:03 EEST
gateway process start             2026-07-23 17:01:26 EEST

hot structural submitted/enqueued 2612 / 2612
hot structural accepted           2611
hot structural failures/censored  1 / 1
cold structural received/accepted 717 / 717
observed lifetime counter offset  1894

opportunity producer sequence     5806
opportunity consumer sequence     5806
opportunity pending/inflight      0 / 0

cold raw accepted/evaluated       66 / 66
cold raw runtime abstains         66
cold raw verified                 0
turn graphs finalized             230

false accepts                     0
parity mismatches                 0
execution authority               false
```

The constant structural counter offset is consistent with different process
epochs and cannot be called loss. The compact structural transport did record
one real `Connection refused` failure and has no durable replay.

The cold `/health` response is about 389 KiB; about 381 KiB belongs to
`online_collection_miner`. A one-second gateway timeout therefore sometimes
renders the cold side unavailable even while the learner is live.
