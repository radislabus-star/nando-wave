#!/usr/bin/env python3
"""Validate Lexicon Foundation v1 files."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "manifest.json"
RU_LETTERS = set("абвгдеёжзийклмнопрстуфхцчшщъыьэюя")
RU_ALLOWED = RU_LETTERS | set("-'’")
EN_ALLOWED = set("abcdefghijklmnopqrstuvwxyz-'’")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8").splitlines()


def is_strict_ru_word(word: str) -> bool:
    if not word or word != word.lower():
        return False
    if word[0] in "-'’" or word[-1] in "-'’":
        return False
    if not any(character in RU_LETTERS for character in word):
        return False
    return all(character in RU_ALLOWED for character in word)


def is_strict_en_word(word: str) -> bool:
    if not word or word != word.lower():
        return False
    if word[0] in "-'’" or word[-1] in "-'’":
        return False
    if not any("a" <= character <= "z" for character in word):
        return False
    return all(character in EN_ALLOWED for character in word)


def validate_file(
    filename: str,
    expected_entries: int,
    expected_sha256: str,
    predicate,
) -> list[str]:
    path = ROOT / filename
    errors: list[str] = []
    if not path.exists():
        return [f"{filename}: missing"]

    lines = load_lines(path)
    if len(lines) != expected_entries:
        errors.append(f"{filename}: entries {len(lines)} != {expected_entries}")
    if len(set(lines)) != len(lines):
        errors.append(f"{filename}: duplicate entries present")
    if sha256(path) != expected_sha256:
        errors.append(f"{filename}: sha256 mismatch")

    bad = [(index, line) for index, line in enumerate(lines, 1) if not predicate(line)]
    if bad:
        preview = ", ".join(f"{index}:{line!r}" for index, line in bad[:5])
        errors.append(f"{filename}: invalid entries: {preview}")

    return errors


def main() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    outputs = manifest["outputs"]
    errors: list[str] = []

    errors.extend(
        validate_file(
            "ru_hot_100k.txt",
            100_000,
            outputs["ru_hot_100k.txt"]["sha256"],
            is_strict_ru_word,
        )
    )
    errors.extend(
        validate_file(
            "ru_cold_300k.txt",
            300_000,
            outputs["ru_cold_300k.txt"]["sha256"],
            is_strict_ru_word,
        )
    )
    errors.extend(
        validate_file(
            "en_hot.txt",
            outputs["en_hot.txt"]["entries"],
            outputs["en_hot.txt"]["sha256"],
            is_strict_en_word,
        )
    )

    if errors:
        print("lexicon_foundation_v1 validation FAILED")
        for error in errors:
            print(error)
        return 1

    print("lexicon_foundation_v1 validation OK")
    print(f"ru_hot_100k entries={outputs['ru_hot_100k.txt']['entries']}")
    print(f"ru_cold_300k entries={outputs['ru_cold_300k.txt']['entries']}")
    print(f"en_hot entries={outputs['en_hot.txt']['entries']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
