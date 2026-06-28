# L1 Center Memory

L1 Center Memory is the first finished layer of the layered center architecture.

It is not a language model and not semantic memory.

## Stored Form

Input:

```text
Russian words -> UTF-8 byte 4-grams + position code
```

Storage:

```text
center records       = reusable 4-gram/position wave contributions
sequence refs        = per-word center-id sequence
residual n-grams     = rare pieces not promoted into centers
```

It does not store full word waves.

Naive word-wave storage:

```text
300k words * 8192 bytes = 2.45 GB
```

## Heavy Gate

Dataset:

```text
data/corpus/russian_words_300k.txt
train:   240,000 words
heldout:  60,000 words
```

Result:

```text
verdict: L1CenterMemoryVerdict::Proven
center_count: 27,258
train_sequence_refs: 3,997,324
heldout_ngram_coverage: 0.9972845
heldout_word_coverage: 0.9981
average_reconstruction_similarity: 0.9989514
average_fourier_similarity: 0.99819964
fourier_ablation_drop: 0.34658888
corrupt_ngram_coverage: 0.83449304
real_vs_corrupt_coverage_gap: 0.16279143
exact_lookup_heldout_hits: 0
model_hot_bytes: 20,760,224
naive_total_wave_bytes: 2,457,600,000
model_to_naive_total_ratio: 0.0084473565
promotion_ready_for_l2: true
```

Run:

```bash
cargo test -p nando-core --test russian_l1_center_memory \
  russian_l1_center_memory_proves_240k_60k_surface_centers_heavy \
  -- --ignored --nocapture
```

## Meaning

This proves:

```text
L1 raw 4-gram surface pieces can be stored as centers + residual refs.
Heldout words reconstruct through centers without exact word lookup.
Fourier ablation damages the heldout signal.
The compressed model beats naive full-wave storage.
```

It does not prove:

```text
word meaning
semantic atoms
general chat
L2/L3 grokking
```

## Promotion Output

L1 promotes this kind of atom to L2:

```text
surface_center_id sequence
residual_ngram_count
reconstruction/coherence scores
Fourier signature support
```

Next layer:

```text
L2 = L1 center sequences -> word/motif centers
```
