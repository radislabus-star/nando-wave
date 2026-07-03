# Lexicon Foundation V1

Lexicon Foundation V1 is the canonical surface lexicon base for early Wave L1/L2
work.

It is not a semantic dataset and it is not L3 authority.

## Files

```text
ru_hot_100k.txt
ru_cold_300k.txt
en_hot.txt
manifest.json
build_lexicon_foundation.py
validate_lexicon_foundation.py
```

## Boundary

```text
ru_hot_100k = compact high-priority Russian surface lexicon
ru_cold_300k = wider Russian surface reserve
en_hot = starter English surface lexicon
```

This layer supports:

```text
L1 form coverage
L2 motif mining
Task DSL text rendering
surface shortcut audits
```

This layer does not provide:

```text
semantic atoms
operator truth
domain authority
general language understanding
```

## Build

```bash
python3 data/lexicon_foundation_v1/build_lexicon_foundation.py
```

## Validate

```bash
python3 data/lexicon_foundation_v1/validate_lexicon_foundation.py
```
