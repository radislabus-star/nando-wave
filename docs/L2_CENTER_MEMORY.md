# L2 Center Memory

L2 Center Memory is the second layer of the layered center architecture.

It does not read raw text directly.
It consumes L1 center-id sequences and learns reusable sequence motifs.

It is still not semantic memory.

## Stored Form

Input:

```text
word -> L1 surface_center_id sequence
```

Storage:

```text
L2 center records   = reusable motifs over L1 center-id sequences
token refs          = per-word motif ids or tagged residual L1 center ids
word records        = source hash + token span + coverage metadata
```

Important distinction:

```text
storage reconstruction uses motif tokens + residual tokens
proof metrics use motif centers only
```

Residual positions stay as zero-amplitude gaps in the proof wave. They do not
disappear from the Fourier shape, and they do not make the center proof
trivially exact.

## Heavy Gate

Dataset:

```text
data/corpus/russian_words_300k.txt
train:   240,000 words
heldout:  60,000 words
```

Result:

```text
verdict: L2CenterMemoryVerdict::Proven
l1_center_count: 27,258
l2_center_count: 105,143
train_l1_refs: 3,997,324
train_l2_token_refs: 1,430,494
train_residual_l1_refs: 574,884
heldout_l1_refs: 1,130,781
heldout_covered_l1_refs: 959,604
heldout_ref_coverage: 0.84862053
heldout_word_coverage: 0.7791333
average_sequence_similarity: 0.9088977
average_fourier_similarity: 0.87673235
average_ablated_fourier_similarity: 0.5886245
fourier_ablation_drop: 0.28810868
corrupt_ref_coverage: 0.44084197
real_vs_corrupt_coverage_gap: 0.40777856
exact_lookup_heldout_hits: 0
model_hot_bytes: 14,608,840
naive_total_l1_sequence_bytes: 25,312,420
model_to_naive_total_ratio: 0.57714117
promotion_ready_for_l3: true
```

Run:

```bash
cargo test -p nando-core --test russian_l2_center_memory \
  russian_l2_center_memory_proves_240k_60k_sequence_motifs_heavy \
  -- --ignored --nocapture
```

The heavy gate currently finishes in about 14 seconds on this host.

## Meaning

This proves:

```text
L2 can learn reusable motifs over L1 center sequences.
Heldout words reuse those motifs without exact word lookup.
The motif-only wave preserves most sequence/Fourier shape.
Fourier ablation damages the heldout signal.
Reversed corrupt words have much lower motif coverage.
The L2 motif layer beats naive direct L1 sequence storage.
```

It does not prove:

```text
word meaning
semantic atoms
query understanding
general chat
L3 semantic grokking
```

## Promotion Output

L2 promotes this kind of atom to L3:

```text
L2 motif id sequence
L1 residual center refs
motif coverage
center-only sequence/Fourier similarity
corrupt rejection gap
Fourier ablation support
```

Next layer:

```text
L3 = L2 motifs -> role/frame/binding/grounded semantic centers
```
