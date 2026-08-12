#!/usr/bin/env python3

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import verify_s1c3c_postmortem_v1 as verifier


class PostmortemVerifierTests(unittest.TestCase):
    def test_canonical_root_is_order_independent(self) -> None:
        left = verifier.digest(verifier.canonical_bytes({"b": 2, "a": 1}))
        right = verifier.digest(verifier.canonical_bytes({"a": 1, "b": 2}))
        self.assertEqual(left, right)

    def test_manifest_binds_path_size_and_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a").write_bytes(b"one")
            first = verifier.manifest(root)
            (root / "a").write_bytes(b"two")
            self.assertNotEqual(first, verifier.manifest(root))
            (root / "b").write_bytes(b"")
            self.assertNotEqual(first, verifier.manifest(root))

    def test_normalized_manifest_rejects_invalid_rows(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.txt"
            path.write_text("not-a-manifest\n", encoding="utf-8")
            with self.assertRaises(verifier.InvalidEvidence):
                verifier.verify_normalized_manifest(path)

    def test_terminal_status_remains_authority_free(self) -> None:
        report = {"postmortem_root_sha256": "a" * 64}
        status = verifier.terminal_status(report)
        self.assertEqual(status["verdict"], "RESOURCE_VETO")
        self.assertEqual(status["authority_envelope"], "UNSEALED")
        self.assertFalse(status["authority_ready"])
        self.assertFalse(status["scientific_authority"])
        self.assertFalse(status["rerun_allowed"])
        self.assertFalse(status["capture_installed"])
        self.assertFalse(status["production_mutation"])


if __name__ == "__main__":
    unittest.main()
