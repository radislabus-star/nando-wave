# S1C-4 Natural Census Critique V1

Status: `ADVERSARIAL REVIEW APPLIED BEFORE IMPLEMENTATION`

| Priority | Finding | Failure if ignored | Frozen repair |
|---|---|---|---|
| P0 | Existing censor counts live only in process memory. | Restart erases the denominator explanation and an empty journal is falsely called `EMPTY_GOAL_SURFACE`. | Persist one bounded classification for every post-cursor durable request and derive the verdict only from that ledger. |
| P0 | Counting classification rows alone can hide requests lost by a full queue or crash. | Missing evidence silently shrinks the denominator. | Freeze the independent opportunity request counter and require exact one-to-one count, identity, and sequence binding; every gap is `VETO`. |
| P0 | A classification can be written twice from different return paths. | One duplicate can numerically mask another missing request. | Enforce unique request identity and opportunity sequence in the writer and census; duplicates are `VETO`, never deduplicated into PASS. |
| P0 | The request event and final classification become durable at different times. | A terminal scan races the background writer and invents a gap. | Freeze the end first, then apply a fixed 60-second quiescence allowance; after it expires, remaining gaps are terminal `VETO`. |
| P0 | Existing S1A/S1B census does not read the three S1C journals. | Dynamics-only rows are mistaken for grounded decisions. | Implement a separate S1C-4 append-cursor census and join precommit, selected action, and satisfaction exactly. |
| P0 | A `DECISION_RECORDED` label could be trusted without its receipts. | Runtime self-report becomes scientific authority. | Require exact validated journal roots, temporal order, K1 authority binding, independent verifier, and satisfaction receipt. |
| P0 | Retrospective classification of requests since S1C-3H looks attractive. | Post-hoc reasoning contaminates independent evidence. | Open a new cursor only after classification deployment; earlier requests remain outside S1C-4. |
| P0 | A free-text or inferred goal can create an apparently rich surface. | The model manufactures the property being tested. | Accept only the frozen exact pre-action goal envelope and preserve `MISSING_EXACT_GOAL` as the ordinary negative. |
| P1 | `EMPTY_GOAL_SURFACE` based on a majority of missing goals is too weak. | A few malformed or unresolved rows are hidden. | Require every non-VETO row to be exactly `MISSING_EXACT_GOAL`; heterogeneous unresolved surfaces fail closed. |
| P1 | One busy session can create many decision rows. | Repetition is mistaken for independent meaning. | Bind a source-neutral session-lineage root before action and require two distinct lineages for PASS. |
| P1 | Synchronous fsync for every no-goal request would perturb serving. | The measurement changes the product path and can cause fallback. | Use a bounded in-memory queue and one background append/sync owner; expose overflow as VETO instead of blocking ordinary traffic. |
| P1 | Background durability without a bounded disk contract can grow forever. | A research ledger becomes a production resource leak. | Close after 1024 requests or 24 hours, cap row bytes and total bytes, and stop appending after immutable terminal closure. |
| P1 | Deployment or rollback could truncate a naturally arriving suffix. | Independent future evidence is destroyed. | Freeze prefixes only; preserve all valid suffixes during rollback and seal a failed candidate window as VETO. |
| P1 | Dashboard text can promote census PASS into K2. | Evidence availability becomes a false law claim. | Render recorder, finite census verdict, K2, training, and phase authority as separate fields; K2 remains CLOSED. |
| P2 | Wall-clock changes can move the deadline. | Window selection becomes mutable. | Bind opening unix time and monotonic process observation; use the persisted earliest terminal condition and never extend the deadline. |
| P2 | Opportunity bridge aggregates can drift while the census reads them. | Start/end roots refer to different source states. | Read stable projections before and after scan and reject changed frozen prefixes or non-durable end sequences. |

## Post-Implementation Conformance Review

The frozen preregistration above was not rewritten after coding. Review of the
implemented bytes found and closed five additional P0/P1 conformance hazards:

| Priority | Finding | Implemented repair |
|---|---|---|
| P0 | A controller poll after the 24-hour deadline could include requests that arrived after the exact deadline. | A request-linearized, fsynced window-boundary receipt freezes the pre-deadline opportunity sequence, ordinal, and token count before any post-deadline request is published. Restart reloads the same rooted boundary. |
| P0 | The 1024-request limit could also be observed only after later requests arrived. | The same boundary receipt is sealed under the opportunity persist lock immediately after the exact 1024th eligible request; subsequent requests are marked census-ineligible. |
| P0 | Opening the cursor and enabling request classification in separate critical sections could lose the first post-freeze request. | Cursor projection and both recorder predicates are configured atomically under the opportunity persist lock before the immutable cursor file is published. Failed publication disables both predicates. |
| P1 | Adding `available_action_count` to the V1 precommit struct changed old CBOR decode/root semantics. | New precommits use `nando.decision-contract-precommit.v2`; V1 decodes with a default zero count and validates against the original V1 digest without the new field. |
| P0 | The first implementation placed new cursor/report files in the root-owned `grounded-meaning-v1` directory, while the serving process runs as `e`. | All S1C-4 mutable files now live under a dedicated child of the existing service-owned durable decision journal. The root-owned parent remains read-only to the service. |

The rooted boundary validates its closure reason: `REQUEST_LIMIT` must equal the
exact frozen maximum ordinal, while `TIME_LIMIT` must equal the immutable
deadline and remain below that ordinal. Neither path grants S2 or K2 authority.

## Accepted Route

```text
S1C-3H recorder installed
-> durable classification instrument
-> new immutable post-instrumentation cursor
-> 1024 ordinary requests or 24 hours
-> exact opportunity/classification/journal join
-> PASS | EMPTY_GOAL_SURFACE | EMPTY_ALTERNATIVE_SURFACE
   | INSUFFICIENT_LINEAGES | VETO
-> stop; no automatic S2 or K2 promotion
```

The critique authorizes implementation of the bounded instrument and census
only. It does not authorize synthetic traffic, a goal injector, model training,
phase mutation, package activation, or a grounded-meaning claim.
