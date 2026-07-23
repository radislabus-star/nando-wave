# Learning Evidence Process Bridge V1

## Purpose

Hot serving and cold learning are separate process owners. Ordinary traffic
must reach the learner without embedding the miner in the request path and
without treating the F8 verifier budget as a traffic-acquisition budget.

```text
Codex request through Nginx
-> hot nando-transition-serving :18789
   -> durable OpportunityBridge event for every request
   -> compact LearningRequestStructureV1 for every valid provider request
   -> optional raw provider bytes only inside the F6/F8 budget
-> private Unix socket
-> cold nando-response-learning :18790
   -> RequestLearningIndex
   -> session/turn RelationFrame enrichment
   -> cold miner
   -> optional GenerationShadow verifier
```

The bridge carries learning evidence. It never grants execution authority.

## Two Independent Channels

### Opportunity channel

The existing durable opportunity spool carries the complete denominator:
intent commitment, input-token count, reducibility class and terminal outcome.
It survives process restart and answers how much ordinary traffic the learner
accounted for. It carries no prompt or response text.

### Structural learning channel

The private Unix socket carries a canonical CBOR envelope:

```text
ProviderRequestCaptureReceiptV3
+ LearningRequestStructureV1
+ Option<bounded raw provider payload>
```

`LearningRequestStructureV1` contains only bounded structural data:

- `TurnIntentId` SHA-256;
- up to four session identity SHA-256 roots;
- request phase atoms;
- pre-action context atoms;
- advertised capability atoms;
- token and payload-size accounting;
- whether the turn identity came from provider metadata.

Raw payload bytes are included only when they fit
`F6_MAX_RAW_REQUEST_BYTES_V3`. A larger request still crosses as compact
structure. Therefore the 256 KiB F6/F8 ceiling limits independent raw replay,
not traffic visibility or phase learning.

## Identity Contract

The three identities are deliberately separate:

```text
SessionLineageId  client_metadata.session_id, then thread_id
TurnIntentId      client_metadata.turn_id
EventId           provider capture sequence/event commitment
```

The HTTP path and the Codex session observer hash the same provider `turn_id`.
The Nginx request ID is only a transport fallback and cannot manufacture
independent future evidence. Requests without provider session metadata share
one conservative unattributed lineage.

The cold `RequestLearningIndex` is bounded to 4,096 turn and session
identities. It joins request-phase atoms by exact `TurnIntentId` and capability
atoms by `SessionLineageId`; the session observer then emits them through the
existing `RelationFrame` owner. No second miner or second relation truth is
introduced.

## Delivery And Backpressure

- The hot handler performs only bounded extraction and `try_send` into a queue
  of at most 48 entries.
- Serialization, socket I/O and ACK waiting occur on a background producer.
- The producer retries socket connection for up to one second so a cold-service
  restart does not create an immediate hole.
- A private `0700` directory and `0600` Unix socket delimit the process trust
  boundary.
- No raw payload is persisted by this bridge.
- Queue overflow, invalid envelopes and unavailable consumer remain censored
  evidence and cannot update authority.

ACK meanings are disjoint:

```text
STRUCTURAL_RAW_ENQUEUED  compact structure accepted; raw shadow queued
STRUCTURAL_ONLY         compact structure accepted; raw omitted by budget
STRUCTURAL_RAW_CENSORED compact structure accepted; raw could not be evaluated
INVALID                 envelope or compact structure rejected
```

## Process Ownership

```text
nando-transition-serving :18789
  producer=true
  consumer=false
  embedded miner=false

nando-response-learning :18790
  producer=false
  consumer=true
  embedded miner=true
  generation shadow=true
```

Systemd orders the hot service after the cold learner but keeps `Wants`, not
`Requires`: if learning is unavailable, Codex traffic still fails open through
Nginx while bridge losses remain visible in counters.

## Proof Gates

The implementation is acceptable only when all of the following hold:

```text
producer accepted structures = consumer accepted structures
provider-bound turn count     > 0 on real Codex traffic
session-bound request count   > 0 on real Codex traffic
capability-bound request count > 0 when tools are advertised
oversized request             -> STRUCTURAL_ONLY, not semantic loss
raw payloads persisted        = 0
queue full / invalid          = 0 in the measured live window
false accepts                 = 0
parity mismatches             = 0
execution authority           = false
```

This bridge removes the process boundary as the reason the miner saw only a
small subset of ordinary request traffic. It does not itself create an ACTIVE
operator or increase CPU execution. That next increase must come from
additional verified operator families and external admission, using the now
complete request denominator and joined structural evidence.

## Live Proof 2026-07-23

The same release binary was loaded by both process owners:

```text
sha256 616c505db0bf3d737dfed90b163ba32635a501381366ccb5425b0703499cbaeb
hot  nando-transition-serving :18789
cold nando-response-learning  :18790
```

The measured post-restart live snapshot contained 26 requests, including
bounded direct requests, ordinary Codex traffic and intentionally oversized
raw requests:

```text
hot submitted / accepted structures       26 / 26
cold received / accepted structures       26 / 26
provider-bound TurnIntentId                26 / 26
session-bound requests                     26 / 26
capability-bound requests                  26 / 26
raw accepted                               2
raw omitted by F8 budget                   24
structural requests censored               0
invalid envelopes                          0
queue full / transport failures            0 / 0
hot submit max                             4 us
raw payloads persisted                     0
opportunity producer/consumer sequence      634 / 634
opportunity pending / inflight events       0 / 0
opportunity producer/consumer failures      0 / 0
generation shadow false accepts            0
generation shadow parity mismatches        0
execution authority                        false
```

The oversized request therefore increased the cold structural denominator and
`RequestLearningIndex` while remaining absent from raw F8 replay. This is the
required separation between learning visibility and verifier replay budget.
