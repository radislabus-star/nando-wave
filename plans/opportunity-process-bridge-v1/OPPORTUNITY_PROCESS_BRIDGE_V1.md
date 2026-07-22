# Opportunity Process Bridge V1

## Purpose

The hot serving process and the cold learner are intentionally separate owners.
The bridge transfers compact opportunity events between them without restoring
the embedded learner in the hot path.

```text
ordinary request in hot serving
-> versioned hash-only event
-> producer-owned staging file
-> atomic rename into shared pending spool
-> bounded background durability sync (10 ms)
-> shared ordered pending spool
-> cold ordered pipeline (up to 256 in flight)
-> cold opportunity ledger append + fsync
-> apply to OpportunityBoard
-> durable ACK
-> remove pending file
```

## Ownership

- `nando-operator-learning::opportunity_bridge` owns the stable event schema.
- `nando-transition-serving::opportunity_bridge` owns transport and recovery.
- `miner_worker` owns lowering into the existing opportunity ledger format.
- `OpportunityBoard` remains the only owner of learned opportunity accounting.
- Admission and runtime authority are not changed by this bridge.

The bridge carries only intent SHA-256, token count, timestamp, reducibility
class, and a bounded diagnostic reason. It never carries prompts, responses,
teacher text, actor output, or an authority capability.

## Delivery Contract

- Files are ordered by a producer sequence, so `Request` precedes later events
  for the same intent.
- A complete file becomes visible in `pending/` through an atomic rename; a
  producer durability worker syncs published files within a bounded 10 ms
  interval, outside the request path.
- The consumer removes a file only after the cold ledger is synced and the
  event has been applied.
- A slow synthesis/report lock does not block ingress: events are submitted in
  sequence to a bounded in-flight pipeline and retain their spool files until
  their individual ACKs arrive.
- A crash before ACK leaves the file pending.
- A crash after apply but before removal can replay the event; OpportunityBoard
  terminal updates are idempotent, so the transport is at-least-once.
- Invalid or tampered files move to `rejected/` and never become evidence.
- Producer staging is separate from consumer-visible pending storage, avoiding
  a startup race over partially written files.
- A process crash does not remove published files. The bounded sync interval is
  the explicit host-power-loss window; once cold ACKs, the event is durable in
  the cold ledger regardless of producer state.

## Deployment Roles

```text
nando-transition-serving :18789
  NANDO_OPPORTUNITY_BRIDGE_PRODUCER_ENABLED=1
  NANDO_OPPORTUNITY_BRIDGE_CONSUMER_ENABLED=0

nando-response-learning :18790
  NANDO_OPPORTUNITY_BRIDGE_PRODUCER_ENABLED=0
  NANDO_OPPORTUNITY_BRIDGE_CONSUMER_ENABLED=1
```

Both use:

```text
/var/lib/nando-wave/transition/opportunity-bridge-v1
```

Each process exposes `opportunity_process_bridge` in `/health` and
`/v2/miner/report`.

## Claim Boundary

The bridge starts a new measurable ingress epoch. Historical traffic that was
not delivered to OpportunityBoard remains historical unaccounted traffic; it
is not synthesized or relabelled as evidence.

Deployment success requires:

```text
producer failures = 0
consumer failures = 0
invalid events = 0
pending returns to 0
consumer inflight returns to 0
producer request events/tokens = consumer request events/tokens
false accepts = 0
parity mismatches = 0
authority unchanged
```

## Live Proof 2026-07-23

Both systemd owners were loaded from the same release binary:

```text
sha256 e2dd7016ba43efc7b649129da7c1c68a19183e53aaf0b827bacd99c864758fac
hot  nando-transition-serving :18789
cold nando-response-learning  :18790
```

A request sent through the ordinary Codex route
`http://127.0.0.1:8787/v2` produced the following common-epoch delta:

```text
producer events delta          2
consumer events delta          2
producer request-token delta   17540
consumer request-token delta   17540
pending after ACK              0
consumer in flight             0
producer/consumer failures     0 / 0
invalid events                 0
hot persistence max            85 us
false accepts                  0
runtime parity mismatches      0
ACTIVE packages                0
```

The earlier loaded producer epoch reached `172 us`, also below the `250 us`
hot no-match target. The consumer drained a live backlog without an ACK timeout.

`OpportunityBoard.ordinary_tokens` is a rolling-window value and can decrease
when the board rolls. It is not the bridge parity ledger. Transport parity is
proved from producer and consumer deltas over the same observation interval.

The composite live gate remained fail-closed: structural and wave-causal
sections passed, while the overall verdict was `VETO` because production local
accept and natural package authority remain disabled. The bridge grants no
authority.
