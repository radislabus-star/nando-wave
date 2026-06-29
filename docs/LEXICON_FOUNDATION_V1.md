# Lexicon Foundation V1

This is the lexical base for the next Wave layers.

It is deliberately L1/L2 material, not semantic authority.

## Sources

| language | layer | file | entries |
| --- | --- | --- | ---: |
| ru | hot | `data/corpus/russian_words_300k.txt` | 300,000 |
| ru | cold | `data/corpus/russian_words_danakt_full.txt` | 1,528,731 |
| en | hot | `data/corpus/english_words_system_full.txt` | 75,119 |

Machine manifest:

```text
data/corpus/lexicon_foundation_v1.json
```

## Boundary

This foundation gives L1 and L2 a serious bilingual surface base:

```text
words -> 4-gram surface waves -> centers -> motifs
```

It does not prove:

```text
semantic understanding
general LLM readiness
English corpus completeness
free-form text generation
```

## Why This Matters

L3 compositional operators should not be trained over a toy surface base.

The intended direction is:

```text
L1: ru/en lexical surface
L2: reusable motifs across ru/en forms
L3: self-induced centers and compositional operators
L4: answer plan
```

The Russian side is already large enough for serious L1/L2 tests.
The English side is a starter local wordlist and must be expanded before making
broad bilingual claims.

## Acceptance

The repo must keep fast tests that prove:

```text
Russian and English corpora exist
Russian full corpus has >1.5M entries
English starter corpus has >70k entries
L1 SurfaceWave compiles both languages
short toy corpora are not silently substituted
```
