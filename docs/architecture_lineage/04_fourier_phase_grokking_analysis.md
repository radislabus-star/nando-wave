# Position 4: Fourier / Phase / Grokking Analysis

Anchor:

```text
docs/ARCHITECTURE.md:17
```

## Central Work

- Nanda et al., "Progress measures for grokking via mechanistic
  interpretability", modular addition case study.
- Reference handle: `https://arxiv.org/abs/2301.05217`

Known lesson:

```text
grokking can look sudden at the behavior level,
but progress measures can reveal gradual mechanism formation.
```

In the modular addition case, the analyzed learned algorithm uses Fourier
structure, and the proof includes ablations in Fourier space.

## Nando Wave Mapping

Nando Wave uses this as a methodological lesson, not as a copied architecture.

Current local uses:

```text
L1/L2 Fourier similarity and ablation reports
phase-center toy probes
center_gap / heldout transfer checks
claim boundary: no grokking without mechanism proof
```

2026-07-02 update:

```text
The v4 phase-center operator probe is now the primary mechanism candidate,
not a side toy:

relation waves
-> circular/phase center of mass
-> correct heldout transition closer than same-bag wrong transition

artifact:
  data/rule_logic_operator_battery_v4/diagnostics/PHASE_CENTER_OPERATOR_PROBE_C32.md
  data/rule_logic_operator_battery_v4/diagnostics/phase_center_operator_probe_c32_report.json

result:
  phase cells: 32
  compiled_phase_centers: 380
  heldout_accuracy_milli: 1000
  wrong_wins: 0
  median_margin: 0.7671
  p10_margin: 0.3130
  no_action_accuracy_milli: 782
```

The L1/L2 Fourier logic is currently mostly diagnostic:

```text
actual wave / sequence
vs reconstructed center-only wave / sequence
-> Fourier similarity
-> top-bin ablation
-> ablation drop
```

The phase-center toy probe is useful as a controlled signal, but it is not
production proof.

## Stronger For Our Goal

The key imported discipline is:

```text
do not trust final accuracy alone;
find progress measures and ablations tied to the mechanism.
```

For Nando Wave this becomes:

```text
heldout transfer
gap / p10 gap
ablation drop
shortcut rejection
flat runtime parity
center/action separability
energy proxy
```

The stronger local goal is not:

```text
prove modular addition in a transformer
```

It is:

```text
prove a compact transferable operator in a small wave/cell system.
```

## Weak / Not Proven

Current weak points:

```text
1. Fourier is diagnostic in L1/L2, not a full learned algorithm proof.
2. Phase-center probe is now a strong v4 diagnostic, but not production memory.
3. V3/v4 still need runtime parity for phase-center readout.
4. No unified training-phase trace exists like memorization -> circuit -> cleanup.
5. No full Fourier/phase ablation proof exists for WavePredictor v3.
```

The current danger:

```text
calling any high score "grokking" without mechanism trace
```

This is why `semantic_grokking_claim_allowed` stays false in current
WavePredictor training reports.

## Next Proof / Debt

Take these into work:

```text
progress trace:
  record action separability, gap, energy proxy, and shortcut scores per phase
  compiler step. Epochs are fallback repair only.

phase/action census:
  measure whether rule actions form stable phase/action clusters.

Fourier ablation:
  remove top phase/Fourier components and require predictable degradation.

capacity curve:
  C8 / C16 / C32 / C64 phase cells, with p10 margin and no-action ablation.

cleanup detection:
  detect whether lookup-like components shrink while operator components grow.

toy-to-core boundary:
  compile phase-center records into Rust/Wave runtime before claiming runtime
  proof.
```

## Status

```text
relation to Nanda-style grokking analysis: YES
copied transformer/Fourier algorithm: NO
current use: diagnostic/proof discipline
production phase closure proven: NO
next work: phase-center Rust runtime / capacity curve / ablation proof
```
