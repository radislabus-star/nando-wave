#!/usr/bin/env python3
"""Build Lexicon Foundation v1 from local corpus files.

This is a deterministic surface-lexicon builder for L1/L2 work. It does not
create semantic atoms and must not be used as L3 authority.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CORPUS_DIR = ROOT / "corpus"
OUT_DIR = Path(__file__).resolve().parent

RU_HOT_LIMIT = 100_000
RU_COLD_LIMIT = 300_000

RU_SOURCES = [
    "russian_words_300k.txt",
    "russian_words_full.txt",
    "russian_words_danakt_full.txt",
]
EN_SOURCE = "english_words_system_full.txt"

RU_LETTERS = set("абвгдеёжзийклмнопрстуфхцчшщъыьэюя")
RU_ALLOWED = RU_LETTERS | set("-'’")
EN_ALLOWED = set("abcdefghijklmnopqrstuvwxyz-'’")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def normalize_line(line: str) -> str:
    return line.strip().lower()


def is_strict_ru_word(word: str) -> bool:
    if not word:
        return False
    if word[0] in "-'’" or word[-1] in "-'’":
        return False
    if not any(character in RU_LETTERS for character in word):
        return False
    return all(character in RU_ALLOWED for character in word)


def is_strict_en_word(word: str) -> bool:
    if not word:
        return False
    if word[0] in "-'’" or word[-1] in "-'’":
        return False
    if not any("a" <= character <= "z" for character in word):
        return False
    return all(character in EN_ALLOWED for character in word)


def collect_unique(
    source_names: list[str],
    predicate,
    limit: int | None = None,
) -> tuple[list[str], list[dict[str, int]]]:
    seen: set[str] = set()
    accepted: list[str] = []
    audit: list[dict[str, int]] = []

    for source_name in source_names:
        path = CORPUS_DIR / source_name
        lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
        source_seen_before = len(seen)
        source_accepted_before = len(accepted)
        rejected = 0
        duplicates = 0

        for line in lines:
            word = normalize_line(line)
            if not predicate(word):
                rejected += 1
                continue
            if word in seen:
                duplicates += 1
                continue
            seen.add(word)
            accepted.append(word)
            if limit is not None and len(accepted) >= limit:
                break

        audit.append(
            {
                "source": source_name,
                "lines": len(lines),
                "accepted_added": len(accepted) - source_accepted_before,
                "rejected_by_filter": rejected,
                "duplicate_against_previous": duplicates,
                "unique_seen_after_source": len(seen),
                "unique_seen_before_source": source_seen_before,
            }
        )

        if limit is not None and len(accepted) >= limit:
            break

    return accepted, audit


def write_lines(path: Path, lines: list[str]) -> None:
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def build() -> None:
    ru_words, ru_audit = collect_unique(RU_SOURCES, is_strict_ru_word, RU_COLD_LIMIT)
    en_words, en_audit = collect_unique([EN_SOURCE], is_strict_en_word)

    if len(ru_words) < RU_COLD_LIMIT:
        raise RuntimeError(f"not enough strict Russian words: {len(ru_words)}")

    ru_hot = ru_words[:RU_HOT_LIMIT]
    ru_cold = ru_words[:RU_COLD_LIMIT]

    outputs = {
        "ru_hot_100k.txt": ru_hot,
        "ru_cold_300k.txt": ru_cold,
        "en_hot.txt": en_words,
    }
    for filename, lines in outputs.items():
        write_lines(OUT_DIR / filename, lines)

    manifest = {
        "version": "lexicon_foundation_v1",
        "generated_by": "data/lexicon_foundation_v1/build_lexicon_foundation.py",
        "purpose": "L1/L2 surface lexicon foundation, not semantic authority.",
        "claim_boundary": {
            "l1_surface_foundation": True,
            "l2_motif_support": True,
            "semantic_understanding": False,
            "operator_memory": False,
            "general_llm_ready": False,
        },
        "sources": {
            "ru": RU_SOURCES,
            "en": [EN_SOURCE],
        },
        "audit": {
            "ru": ru_audit,
            "en": en_audit,
        },
        "outputs": {
            filename: {
                "entries": len(lines),
                "sha256": sha256(OUT_DIR / filename),
            }
            for filename, lines in outputs.items()
        },
        "rules": {
            "ru": "strict lowercase Russian letters plus internal hyphen/apostrophe; extended Cyrillic letters are rejected for v1 hot/cold.",
            "en": "strict lowercase Latin letters plus internal hyphen/apostrophe.",
            "dedupe": "preserve first occurrence across sources.",
            "hot": "first 100k accepted Russian entries.",
            "cold": "first 300k accepted Russian entries.",
        },
    }
    (OUT_DIR / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    build()
