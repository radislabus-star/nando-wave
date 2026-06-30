# NANDA Structural Gate Roadmap

The goal is to turn the current placeholder into a real local verifier that an
agent can call before finalizing relation-heavy answers.

## Phase 0 - Bookmark

Status: done.

- Skill triggers on role/route/obligation/evidence-chain tasks.
- Local CLI exists.
- Agent is forbidden from claiming broad NANDA superiority.

## Phase 1 - SPARSE-TRIAD-0 Eval

Status: started with PASS, role-swap VETO, route-splice VETO, and
evidence-conflict VETO examples.

Build a benchmark eval in `/home/ubu/projects/nando-wave` or this project:

- Follow `/home/ubu/projects/nanda-structural-gate-skill/ARCHITECTURE.md`.
- Generate correct triads and corrupted role-swap triads.
- Generate route-splice cases where exact candidate facts come from different
  source groups.
- Track exact-baseline false accepts.
- Keep a code-flow template so agents can check source/runtime/CLI/plugin
  structures during implementation.
- Encode role/entity/relation as compact wave/VSA vectors.
- Score local bindings and composite triad modes.
- Compare against:
  - exact symbolic rule;
  - token overlap;
  - cosine/vector baseline;
  - simple graph consistency rule.
- Pass only if NANDA catches structural errors that cheap baselines miss.

## Phase 2 - CLI

Replace the stub with:

```bash
nanda-check --triads task.json --format text
nanda-check --triads task.json --format json
```

Required output:

- `verdict`;
- stable triads;
- weak triads;
- conflicts;
- evidence gaps;
- baseline comparison summary.

## Phase 3 - Agent Gate

Make agent usage cheap:

- accept JSON, YAML, or simple pipe table;
- provide short reports by default;
- keep full traces in local files;
- never require the agent to hand-write a giant packet.

## Phase 4 - Real Work Tests

Use domains where LLMs often confuse routes:

- customs/import chain;
- contract party responsibility;
- certification holder/manufacturer/applicant;
- CRM deal stage and document status;
- code dependency and ownership flow.

The project is useful only if it produces fewer structural mistakes than an LLM
alone and gives a clear `WATCH`/`VETO` when evidence is insufficient.

## Phase 29 - Resonance Learner

Status: implemented.

- `nanda-feedback` records field-shape memory for accept/reject/WATCH
  decisions.
- `nanda-index` merges duplicate `resonance_memory` forms and accumulates
  observations.
- `nanda-search` softly replays accepted forms as reinforcement and rejected
  forms as suppression.
- Search output reports which form was replayed and keeps the replay separate
  from the final trust gate.

## Phase 30 - Pattern Store + Wave Decoder

Status: started.

- `nanda-decode` runs the interference field and decodes ranked
  `next_structural_pattern` candidates.
- The decoder emits structural continuations rather than prose:
  `subject -> relation -> object`, route, role, polarity, continuity, and
  support score.
- This is the first LLMWave bridge: field peak -> next pattern distribution.
- `--steps N` runs recurrent wave decode: selected pattern -> query context ->
  next field. The decoder stops honestly with `PATTERN_SATURATED` when no new
  structural continuation is available.
- `nanda-decode-eval` runs continuation regressions for expected top pattern,
  decoder state, recurrent final state, and minimum completed steps.
- `nanda-encode` is the first token/pattern encoder: raw text -> deterministic
  token waves -> 1024-dimensional pattern signature -> preview query triads.
- `nanda-feedback` accepts decoder output and emits `continuation_memory`.
- `nanda-index` merges continuation memory, and `nanda-decode` applies it as a
  local training signal before recurrent selection.
- v35 compact pattern store: each continuation memory record can be represented
  as a 32-byte packed pattern signature.
- v36 pattern replay: decode applies the compact store before recurrent
  selection.
- v37 capacity: `nanda-pattern-capacity` shows 1k, 4k, 16k and 64k learned
  pattern pressure.
- v38 negative continuation lanes: rejects suppress only the local false
  continuation signature, not the whole route.
- v39 mini-loop: `nanda-llmwave` runs raw text -> encode -> decode ->
  feedback preview.
- v40 NANDA-6M pattern runtime: budget/pack reports include the compact pattern
  arena contract.
- v41 learning eval: `nanda-pattern-eval` measures baseline decode against
  trained decode and proves whether continuation feedback moved or reinforced
  the next-pattern field.
- v42 beam decode: `--beam-width N` keeps multiple structural continuations in
  superposition and ranks trajectory chains.
- v43 trajectory eval: `nanda-decode-eval` checks beam route, length,
  saturation, and forbidden trajectory patterns.
- v44 early replay report: decode exposes pre-ranking pattern replay under
  `early_pattern_replay`.
- v45 adaptive pattern scoring: `--adaptive-scoring` reports field-state-aware
  decode weights while preserving static scoring by default.
- v46 pattern bank: `nanda-pattern-bank` builds and inspects the learned
  continuation-memory bank as standalone 32-byte pattern records.
- v47 HRR binding sandbox: `nanda-llmwave` reports role/filler bind-unbind
  probes for packet triads.
- v48 cleanup memory: `nanda-pattern-bank` becomes the cleanup dictionary for
  noisy decoded structural patterns.
- v49 attractor trace: recurrent decode reports an energy-style route-basin
  trace.
- v50 capacity curve: `nanda-llmwave` reports active pattern load and estimated
  crosstalk instead of treating superposition as a slogan.
- v51 anti-wave audit: rejected continuation lanes are reported as
  shortcut-specific suppressions.
- v52 read/write/retrieve loop: `nanda-llmwave` now combines raw text encoding,
  HRR binding probe, structural decode, cleanup, energy trace, capacity,
  anti-wave audit, and feedback preview.
- v53 proof suite: `nanda-llmwave-eval` checks full LLMWave proof packets.
- v54 packed HRR lanes: HRR binding probes expose fixed hot-lane estimates.
- v55 cleanup dictionary thresholds: cleanup reports exact/near/ambiguous
  states explicitly.
- v56 anti-wave locality: reject lanes must suppress a shortcut while keeping
  decode alive.
- v57 capacity baseline: packed-wave estimates are compared to direct-table
  storage.
- v58 packed hot-cycle bridge: report states collapse into
  `LLMWAVE_HOT_READY` or `LLMWAVE_HOT_WATCH`.
- v59 proof summary: LLMWave exposes one answer-readiness contract.
- v60 public demo packet: the demo surface includes safe claims and proof
  blockers instead of hiding uncertainty.
- v61 demo weak-spot surface: `nanda-demo` compresses v60 proof JSON into a
  short human/agent dashboard and `examples/demo-corpus.json` checks ready,
  anti-wave, and review cases.
- Next milestone is moving v60/v61 proof signals deeper into the NANDA-6M hot
  loop.
