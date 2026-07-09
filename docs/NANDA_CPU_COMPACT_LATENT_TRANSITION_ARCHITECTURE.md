# NANDA CPU: Compact Latent Transition Architecture

Status: architecture north star, not a product claim by itself.

Unified transferable-operator doctrine:

```text
docs/TRANSFERABLE_OPERATORS_UNIFIED_V1.md
```

Current promotion/admission contract:

```text
docs/HYBRID_SYMBIOTIC_OPERATOR_PROMOTION_CONTRACT.md
```

NANDA CPU is not a cache and not a small LLM.

It is an executable memory of hidden state transitions.

The core does not store answers. The core stores verified transition behavior:

```text
if the current state is close to z_t
and the action is close to a_t
then the next hidden state should be close to z_t+1
```

## Main Formula

```text
surface_event
  -> hidden_state z_t

action / operator a_t
  -> transition center C(a)

NANDA CPU:
  z_t + C(a) -> z_hat_t+1

verifier:
  does z_hat_t+1 match the observable result?
```

The model does not learn the answer as the main object.

It learns compact hidden transitions:

```text
state_t + action -> state_t+1
```

and executes them on CPU only when the verifier boundary allows it.

## Layer Model

Old shorthand:

```text
L1 = n-gram
L2 = meaning
L3 = action
```

Updated architecture:

```text
L1 = surface
L2 = hidden state
L3 = compact transition center
L4 = goal-directed transition selector
```

## Architecture Tree

```text
REAL STREAM
|
+-- L1 Surface Encoder
|   |
|   +-- text
|   +-- tool calls
|   +-- stdout / stderr shape
|   +-- UI events
|   +-- future frame/video atoms
|
+-- L2 Latent State Packer
|   |
|   +-- safe atoms
|   +-- ternary vector: -1 / 0 / +1
|   +-- phase vector
|   +-- hidden_state z_t
|
+-- L3 Phase-Center Operator Memory
|   |
|   +-- action / operator class
|   +-- transition center
|   +-- multi-center suboperators
|   +-- positive evidence
|   +-- negative evidence
|   +-- background evidence
|
+-- L4 Goal Selector / Planner
|   |
|   +-- choose target / goal state
|   +-- choose operator profile
|   +-- choose chain of operators
|   +-- score expected value
|   +-- score false_accept risk
|
+-- Verifier Gate
    |
    +-- CPU accept
    +-- or LLM fallback
```

## How An Operator Is Learned

Not:

```text
memorize answer
```

But:

```text
observe many verified transitions:
  z_t + action -> z_t+1

build center of mass:
  C(action)

test against negatives:
  correct transition closer than wrong transition

shadow replay:
  future rows remain clean

promote only if:
  false_accepts = 0
  verifier binding exists
```

## What Is Stored

The hot runtime should not store raw text as authority.

It stores compact executable transition profiles:

```text
OperatorProfile
|
+-- action_id
+-- center vector
+-- subcenters
+-- negative / background centers
+-- margin threshold
+-- verifier binding
+-- stats:
    |
    +-- accepts
    +-- rejects
    +-- false_accepts
    +-- saved_tokens
    +-- saved_cost
```

The profile is a small CPU-resident action memory, not a full model checkpoint.

## Hot CPU Runtime

The hot path must stay small and source-neutral:

```text
request / event
|
+-- encode to z_t
|
+-- route to profile
|
+-- score:
|   |
|   +-- distance to positive transition center
|   +-- distance to negative / background centers
|   +-- margin
|
+-- if margin is high and verifier exists:
|   |
|   +-- CPU accept
|
+-- else:
    |
    +-- LLM fallback
```

Hot path must not contain:

```text
raw corpus authority
test fixture authority
provider-specific hardcode
agent-name hardcode
manual local_out_t
target_id authority
proof_rule_id authority
local accept without verifier
```

## HTTP Product Surface

The production-compatible local bridge exposes the compact latent transition
surface as:

```text
http://127.0.0.1:8787/v2
```

`/v2` is the default OpenAI-compatible client base URL for NANDA CPU.
`/v1` may remain as a legacy compatibility surface, but new clients should use
`/v2`.

The `/v2` boundary is still the same safety boundary:

```text
verified route -> CPU accept
unverified / broad route -> upstream fallback or upstream_missing
```

## Cold Learning Path

Cold path is allowed to be heavier:

```text
trace stream
|
+-- extract state / action / result
|
+-- build safe atoms
|
+-- infer hidden states
|
+-- mine repeated transitions
|
+-- split unsafe broad classes into safe subcenters
|
+-- build phase centers
|
+-- shadow replay on future rows
|
+-- verifier false_accepts = 0
|
+-- promote to hot CPU profile
```

The cold path may discover and quarantine candidates. It must not grant production authority.

## L4 Responsibility

L4 is not a language layer.

L4 is the control layer that decides which transition memory is worth using:

```text
L4
|
+-- finds repeated operator classes
+-- splits broad unsafe classes
+-- chooses profile under memory/latency budget
+-- ranks value:
|   |
|   +-- new calls saved
|   +-- new tokens saved
|   +-- cost saved
|   +-- cache overlap
|   +-- false_accept risk
|
+-- selects operator chain under goal_state
+-- disables bad profiles
+-- keeps local_accept off until verifier says yes
```

L4 helps L3 by giving it cleaner, smaller transition classes.

## Product Translation

NANDA CPU should be described as:

```text
compact verified latent action runtime
```

or in Russian:

```text
компактный рантайм проверенных скрытых переходов
```

Short product sentence:

```text
NANDA CPU learns repeated verified transitions from real agent streams
and executes safe ones locally on CPU, with fallback when confidence or
verification is insufficient.
```

Short internal sentence:

```text
NANDA CPU = L1 surface -> L2 hidden state -> L3 transition center -> L4 selector -> verifier.
```

## Boundary

This document is an architecture direction.

Operational server/client handoff lives in:

```text
ops/phase-center-test-server/CLIENT_HANDOFF.md
```

It does not by itself claim:

```text
production local_accept is enabled
market money savings are proven
all operator classes are covered
text generation is solved
multi-step reasoning is solved
```

Those require separate gates:

```text
real denominator
exact-cache baseline
future/shadow replay
zero false_accepts
verifier binding
runtime parity
provider billing evidence for money claims
```

## Final Core

NANDA CPU learns not answers and not text.

It learns compact hidden transitions:

```text
state_t + action -> state_t+1
```

and executes them on CPU only when the verifier confirms safety.

This is not a cache.

It is a small processor of verified actions.
