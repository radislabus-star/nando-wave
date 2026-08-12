#!/usr/bin/env python3
"""Focused negative tests for S1C-3E."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import s1c3e_remote_transaction_v1 as executor
import verify_s1c3e_transaction_v1 as verifier


def empty_journal() -> dict[str, object]:
    return {
        "present": True,
        "directory": {"path": verifier.JOURNAL_PATH, "uid": 1000, "gid": 1000, "mode_octal": "0700"},
        "entries": [
            {
                "path": name,
                "size_bytes": 0,
                "sha256": verifier.EMPTY_SHA256,
                "uid": 1000,
                "gid": 1000,
                "mode_octal": "0600",
            }
            for name in verifier.EXPECTED_SEGMENTS
        ],
        "total_bytes": 0,
        "manifest_root_sha256": "a" * 64,
    }


class JournalVerifierTests(unittest.TestCase):
    def test_exact_empty_runtime_journal_passes(self) -> None:
        verifier.verify_empty_journal(empty_journal(), "journal")

    def test_forged_segment_is_rejected(self) -> None:
        value = empty_journal()
        value["entries"][0]["path"] = "fixture.cbor"
        with self.assertRaisesRegex(verifier.InvalidReceipt, "segment_set"):
            verifier.verify_empty_journal(value, "journal")

    def test_nonzero_row_is_rejected(self) -> None:
        value = empty_journal()
        value["entries"][0]["size_bytes"] = 1
        with self.assertRaisesRegex(verifier.InvalidReceipt, "size"):
            verifier.verify_empty_journal(value, "journal")

    def test_writable_group_mode_is_rejected(self) -> None:
        value = empty_journal()
        value["entries"][0]["mode_octal"] = "0660"
        with self.assertRaisesRegex(verifier.InvalidReceipt, "mode"):
            verifier.verify_empty_journal(value, "journal")

    def test_foreign_owner_is_rejected(self) -> None:
        value = empty_journal()
        value["entries"][0]["uid"] = 0
        with self.assertRaisesRegex(verifier.InvalidReceipt, "uid"):
            verifier.verify_empty_journal(value, "journal")


class ExecutorBoundaryTests(unittest.TestCase):
    def test_cleanup_preserves_nonempty_natural_journal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            journal = Path(directory) / "journal"
            journal.mkdir()
            (journal / "natural-row.cbor").write_bytes(b"natural")
            with mock.patch.object(executor, "JOURNAL", journal), mock.patch.object(
                executor,
                "require_exact_empty_runtime_journal",
                side_effect=executor.GateFailure("nonempty"),
            ):
                self.assertFalse(executor.cleanup_operational_empty_journal())
                self.assertTrue((journal / "natural-row.cbor").is_file())

    def test_provisioner_does_not_create_segment_files(self) -> None:
        source = Path(executor.__file__).read_text()
        body = source.split("def provision_empty_directory", 1)[1].split(
            "def cleanup_operational_empty_journal", 1
        )[0]
        self.assertIn("JOURNAL.mkdir", body)
        self.assertNotIn("write_bytes", body)
        self.assertNotIn("open(", body)
        self.assertNotIn("EXPECTED_SEGMENTS", body)

    def test_rollback_is_armed_before_directory_provisioning(self) -> None:
        source = Path(executor.__file__).read_text()
        body = source.split("def execute", 1)[1].split("def connector_failure_reasons", 1)[0]
        self.assertLess(body.index('"ROLLBACK_ARMED"'), body.index("provision_empty_directory()"))

    def test_predeployment_abort_is_non_mutating_and_terminal(self) -> None:
        source = Path(executor.__file__).read_text()
        body = source.split("def abort_predeployment", 1)[1].split(
            "def connector_failure_reasons", 1
        )[0]
        for marker in (
            '"production_mutation": False',
            '"capture_installed": False',
            '"s1c4_state": "CLOSED"',
            '"scientific_authority": False',
        ):
            self.assertIn(marker, body)

    def test_no_scientific_authority_can_be_enabled(self) -> None:
        source = Path(executor.__file__).read_text() + Path(verifier.__file__).read_text()
        self.assertNotIn('"scientific_authority": True', source)
        self.assertNotIn('"model_training": True', source)
        self.assertNotIn('"phase_mutation": True', source)


class FreezeTests(unittest.TestCase):
    def test_freeze_binds_exact_implementation_files(self) -> None:
        root = Path(__file__).resolve().parent
        value = verifier.create_implementation_freeze("a" * 40, "b" * 40, root)
        self.assertEqual([row["path"] for row in value["files"]], list(verifier.IMPLEMENTATION_FILES))
        self.assertEqual(
            value["implementation_freeze_root_sha256"],
            verifier.digest(verifier.canonical_bytes(value, "implementation_freeze_root_sha256")),
        )


if __name__ == "__main__":
    unittest.main()
