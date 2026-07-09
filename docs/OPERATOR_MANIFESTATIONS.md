# Operator Manifestations

## Current Manifestation In Product Terms

Authoritative contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

На сегодня оператор проявился не как "таблица ответов", а как:

```text
повторяемый проверяемый переход состояния
  -> упакованный L4 в route/profile/bucket
  -> считанный L3 как phase-center margin
  -> оставленный в CPU shadow, если verifier подтверждает
```

Best current proof:

```text
compatible shadow frontier:
  unique CPU accepts over exact cache: 6_644
  calls saved: 22.3177%
  tokens saved: 72.0541%
  false_accepts: 0
  local_accept_enabled: false
```

Главная идея:

```text
LLM handles novel/unverified work.
CPU phase-center operators handle repeated verified transitions.
L4 decides which repeated transitions are worth turning into hot profiles.
```

## Current Concrete Manifestation: Agent-Loop Planning Operator

The most concrete current manifestation is no longer abstract. It is:

```text
agent-loop planning_update
-> L4 event/router/packer direction
-> L3 phase-center operator
-> verifier-bounded CPU shadow accept or fallback
```

Current proof snapshot:

```text
planning_update safe cache-miss ceiling: 5939
planning_update phase-center accepts:    2990
planning_update class coverage:          50.35%

compatible denominator:
  rows: 35_829
  CPU accepts over exact cache: 4160
  calls saved: 11.6107%
  tokens saved: 11.5390%
  false_accepts: 0
```

This shows the operator idea in a concrete agent-loop form:

```text
not "answer memorization"
but "verified repeated state transition"
```

Important split:

```text
L3 phase-center:
  already scores a planning_update transition in shadow.

L4 streaming router/packer:
  still open as product path because planning_update rows currently have
  rows_with_shadow_request = 0 in the live packing metric.
```

So the current research/product direction is:

```text
make repeated agent actions visible to phase centers,
then let compact CPU operators handle verified transitions,
while the LLM handles novel or unverifiable work.
```

This note records an R&D direction: try to mathematize the Nando Wave
architecture through the idea of an operator as a transferable form of state
change.

Core frame:

```text
state_t
+ action/operator
-> state_t+1
```

The operator is not only a command. It can be viewed as a reusable shape of
transformation that appears wherever there is state, action, repetition, and a
way to verify the result.

## Unusual Places Where Operators May Appear

### Music

```text
motif -> variation
```

Possible operators:

```text
transfer rhythm
invert theme
transpose phrase
resolve chord
repeat with variation
```

### Video

```text
scene state -> edit action -> next scene
```

Possible operators:

```text
detect repeated motion
cut silence/pause
synchronize frame
track object
compress repeated camera action
```

### Biology

```text
cell state -> regulation action -> next state
```

Possible operators:

```text
activate gene
suppress signal
switch cell mode
stabilize feedback loop
trigger differentiation path
```

### Chemistry

```text
molecule state -> reaction operator -> product
```

Possible operators:

```text
substitute group
split bond
form bond
transfer charge
fold reaction family over new molecules
```

### Robotics

```text
sensor state -> motor primitive -> next state
```

Possible operators:

```text
grasp
rotate
avoid obstacle
stabilize balance
repeat learned motion on a new object
```

### Finance

```text
market/account state -> action -> new risk state
```

Possible operators:

```text
rebalance
hedge
detect anomaly
update risk
close repeated arbitrage pattern
```

### Medicine

```text
patient state -> intervention -> next clinical state
```

Possible operators:

```text
triage
adjust dosage
update risk
route to specialist
flag contraindication
```

### Law And Documents

```text
document state -> legal/document action -> new document state
```

Possible operators:

```text
extract field
compare clauses
normalize party name
flag missing document
check date/amount consistency
```

### UI And Workflow Automation

```text
screen/workflow state -> user/system action -> next state
```

Possible operators:

```text
repeat form action
route ticket
update status
copy verified field
perform safe local workflow step
```

### Thinking And Reasoning

```text
belief/problem state -> inference action -> new belief/problem state
```

Possible operators:

```text
compare
generalize
specialize
contradict
prove
decompose
compose
choose next subgoal
```

## Product Interpretation

Nando should not only be viewed as a text model. It can be viewed as a runtime
for transferable operators:

```text
observe repeated state transitions
extract the reusable action shape
verify that the result is safe
compile the action into a local CPU profile
fallback when verification is not possible
```

This generalizes the current LLM/agent-loop direction:

```text
LLM traffic
-> repeated action centers
-> verified CPU profiles
```

into a broader operator view:

```text
any domain with state_t, action, state_t+1, repetition, and verification
may contain learnable transferable operators.
```

## How Nando Learns An Action

The current working mechanism can be described as a layered path:

```text
surface trace
-> role/slot/motif structure
-> transferable action pressure
-> verified next state
```

In Nando Wave terms:

```text
L1:
  reads the surface
  encodes n-grams / local traces into sparse lanes

L2:
  groups surface evidence into roles, slots, motifs, and action hints

L3:
  learns the operator as a state transition:
    state_t + action -> state_t+1

Runtime:
  applies the learned profile locally
  accepts only when score/admission/verifier allow it
  otherwise falls back

L4:
  routes live events to the correct phase-center profile
  packs state/request/result atoms into a scoreable form
  keeps unsafe or unverifiable events on fallback
```

The important point is that the operator is not supposed to memorize one
answer. It must transfer the same action shape to new fillers, slots, tokens,
or states.

Current proof discipline:

```text
no exact lookup
no target_id authority
no proof_rule_id authority
no concrete_x_lookup
no manual local_out_t
same-bag negatives for order tasks
heldout transfer
ablation collapse when binding/action/role is removed
field/flat runtime parity
false_accepts = 0 before product accept
```

For the sequence/order work, the important lesson was:

```text
token bag is not enough
the operator must preserve position/order
wrong examples use the same tokens in the wrong order
```

For the agent-loop work, the analogous lesson is:

```text
route classification is not enough
the operator must safely replace a real LLM action
wrong accepts must be rejected by verifier/admission
```

## Offline And Online Operator Learning

Offline profile building:

```text
collect examples
build payloads
train/compile profile
run shadow
check verifier
promote only if false_accepts = 0
```

Online operator discovery:

```text
observe live trace
extract state/action/result
cluster repeated transitions by action center
build candidate operator profile
shadow test it
promote only after verifier + false_accepts = 0
```

This gives the current Nando split:

```text
Nando Core:
  runtime for already learned transferable operators

Nando JIT / Online Compiler:
  discovers repeated actions in traffic
  builds candidate profiles
  verifies and promotes safe ones
```

The online compiler may discover candidates automatically, but it must not
auto-accept without verification:

```text
online discovery: yes
online local accept without verifier: no
```

## Boundary

This is a research direction, not a product claim.

Do not claim that Nando already solves these domains. The current proven path
is still the LLM/agent-loop profile runtime. This note only records where the
operator idea may be mathematically and commercially extended.
