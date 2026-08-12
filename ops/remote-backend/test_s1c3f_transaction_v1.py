#!/usr/bin/env python3
"""Focused framing and authority tests for S1C-3F."""

from __future__ import annotations

import hashlib
import json
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import s1c3f_remote_transaction_v1 as executor
import verify_s1c3f_transaction_v1 as verifier


def frame(payload: bytes) -> bytes:
    digest = int.from_bytes(hashlib.sha256(payload).digest()[:8], "little")
    return b"NTF1" + struct.pack("<IQ", len(payload), digest) + payload


class FramedParserTests(unittest.TestCase):
    def test_magic_only_is_zero_records(self) -> None:
        self.assertEqual(
            verifier.parse_segment_bytes(b"NTF1", "segment"),
            {"format_bytes": 4, "frame_bytes": 0, "record_count": 0, "tail_bytes": 0},
        )

    def test_one_valid_frame_is_one_record(self) -> None:
        result = verifier.parse_segment_bytes(frame(b"payload"), "segment")
        self.assertEqual(result["record_count"], 1)
        self.assertEqual(result["frame_bytes"], 19)

    def test_wrong_magic_is_rejected(self) -> None:
        with self.assertRaisesRegex(verifier.InvalidReceipt, "magic"):
            verifier.parse_segment_bytes(b"NOPE", "segment")

    def test_partial_header_is_rejected(self) -> None:
        with self.assertRaisesRegex(verifier.InvalidReceipt, "partial_header"):
            verifier.parse_segment_bytes(b"NTF1\x01", "segment")

    def test_payload_budget_is_rejected(self) -> None:
        payload = b"NTF1" + struct.pack("<IQ", verifier.MAX_PAYLOAD + 1, 0)
        with self.assertRaisesRegex(verifier.InvalidReceipt, "payload_budget"):
            verifier.parse_segment_bytes(payload, "segment")

    def test_partial_payload_is_rejected(self) -> None:
        payload = b"NTF1" + struct.pack("<IQ", 3, 0) + b"x"
        with self.assertRaisesRegex(verifier.InvalidReceipt, "partial_payload"):
            verifier.parse_segment_bytes(payload, "segment")

    def test_digest_mismatch_is_rejected(self) -> None:
        payload = b"NTF1" + struct.pack("<IQ", 1, 0) + b"x"
        with self.assertRaisesRegex(verifier.InvalidReceipt, "digest"):
            verifier.parse_segment_bytes(payload, "segment")

    def test_executor_and_verifier_agree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "segment.cbor"
            path.write_bytes(frame(b"payload"))
            self.assertEqual(executor.parse_framed_segment(path), verifier.parse_segment_file(path))

    def test_natural_suffix_preserves_frozen_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "segment.cbor"
            opening = frame(b"opening")
            path.write_bytes(opening + frame(b"natural-future")[4:])
            parsed = executor.parse_framed_segment(
                path,
                {"size_bytes": len(opening), "sha256": hashlib.sha256(opening).hexdigest()},
            )
            self.assertTrue(parsed["prefix_preserved"])
            self.assertEqual(parsed["record_count"], 2)

    def test_changed_existing_frame_is_rejected_as_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "segment.cbor"
            opening = frame(b"opening")
            path.write_bytes(frame(b"changed"))
            parsed = executor.parse_framed_segment(
                path,
                {"size_bytes": len(opening), "sha256": hashlib.sha256(opening).hexdigest()},
            )
            with self.assertRaisesRegex(executor.base.GateFailure, "prefix_changed"):
                executor.require_prefix_preserved(
                    {"entries": [{"path": path.name, "size_bytes": len(opening), "sha256": hashlib.sha256(opening).hexdigest()}]},
                    {"entries": [{"path": path.name, **parsed}]},
                )


class AuthorityBoundaryTests(unittest.TestCase):
    def test_transaction_has_no_journal_write_primitive(self) -> None:
        source = Path(executor.__file__).read_text()
        for marker in ("write_bytes", "write_text", "truncate", "JOURNAL.unlink", "JOURNAL.rmdir"):
            self.assertNotIn(marker, source)

    def test_existing_journal_is_checked_not_created(self) -> None:
        source = Path(executor.__file__).read_text()
        body = source.split("def verify_existing_directory", 1)[1].split("def preserve_journal", 1)[0]
        self.assertIn("journal_snapshot()", body)
        self.assertNotIn("mkdir", body)
        self.assertNotIn("open(", body)

    def test_runtime_shape_uses_original_owner_without_alias_recursion(self) -> None:
        self.assertIsNot(executor.ORIGINAL_REQUIRE_SURVIVAL, executor.require_valid_survival)
        with mock.patch.object(executor, "ORIGINAL_REQUIRE_SURVIVAL") as original:
            executor.require_valid_survival(
                {"entries": [{"path": "segment", "format_bytes": 4, "tail_bytes": 0}]}
            )
        original.assert_called_once()

    def test_route_receipt_is_read_only_health(self) -> None:
        with mock.patch.object(
            executor.base,
            "health_snapshot",
            return_value={"hot": {"url": "http://health", "semantic": {"ok": True}}},
        ):
            self.assertEqual(
                executor.read_only_route_receipt(),
                {"hot": {"url": "http://health", "semantic": {"ok": True}}},
            )
        source = Path(executor.__file__).read_text().split(
            "def read_only_route_receipt", 1
        )[1].split("def verify_predeployment", 1)[0]
        self.assertNotIn("/v1/responses", source)
        self.assertNotIn("POST", source)

    def test_parent_copy_failure_is_terminal_before_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "transaction"
            parent = Path(directory) / "parent"
            root.mkdir()
            parent.mkdir()
            for name in executor.PARENT_FILE_SHA256:
                (parent / name).write_text("evidence")
            args = SimpleNamespace(
                transaction_directory=str(root), transaction_id="test-transaction"
            )

            def prepared(_: object) -> int:
                executor.base.write_json(
                    root / "transaction-state.json",
                    {"schema": executor.STATE_SCHEMA, "state": "PREPARED"},
                    0o600,
                )
                return 0

            with (
                mock.patch.object(executor.base, "prepare", side_effect=prepared),
                mock.patch.object(executor, "PARENT_DIRECTORY", parent),
                mock.patch.object(executor.shutil, "copy2", side_effect=OSError("copy failed")),
                self.assertRaisesRegex(OSError, "copy failed"),
            ):
                executor.prepare(args)
            state = json.loads((root / "transaction-state.json").read_text())
            self.assertEqual(state["state"], "PREFLIGHT_FAILURE")

    def test_state_namespace_promotion_removes_s1c3e_filename(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.base.write_json(
                root / "s1c3e-state.json",
                {
                    "schema": "nando.s1c3e-state.v1",
                    "verdict": "S1C3E_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH",
                    "state_root_sha256": "old",
                },
                0o400,
            )
            executor._promote_state_namespace(root)
            self.assertFalse((root / "s1c3e-state.json").exists())
            state = json.loads((root / "s1c3f-state.json").read_text())
            self.assertEqual(state["schema"], executor.STATE_SCHEMA)
            self.assertEqual(
                state["verdict"], "S1C3F_DEPLOYMENT_PASS_WITH_OPTIMIZATION_WATCH"
            )

    def test_installation_never_grants_scientific_authority(self) -> None:
        source = Path(executor.__file__).read_text() + Path(verifier.__file__).read_text()
        self.assertNotIn('"scientific_authority": True', source)
        self.assertNotIn('"model_training": True', source)
        self.assertNotIn('"phase_mutation": True', source)

    def test_freeze_binds_exact_files(self) -> None:
        value = verifier.create_freeze(
            "a" * 40, "b" * 40, Path(verifier.__file__).resolve().parent
        )
        self.assertEqual([row["path"] for row in value["files"]], list(verifier.IMPLEMENTATION_FILES))
        self.assertIn("s1c3e_remote_transaction_v1.py", verifier.IMPLEMENTATION_FILES)
        self.assertIn("s1c3_remote_transaction_v7.py", verifier.IMPLEMENTATION_FILES)


if __name__ == "__main__":
    unittest.main()
