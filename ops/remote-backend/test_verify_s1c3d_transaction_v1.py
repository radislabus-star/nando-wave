#!/usr/bin/env python3
"""Negative authority tests for the S1C-3D verifier."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import s1c3d_remote_transaction_v1 as executor
import verify_s1c3d_transaction_v1 as verifier


def rooted(value: dict[str, object], field: str) -> dict[str, object]:
    value[field] = verifier.digest(verifier.canonical_bytes(value))
    return value


class SnapshotVerificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        evidence = self.root / "evidence"
        evidence.mkdir()
        probe = {
            "directory": {
                name: {"denied": True, "errno": 13}
                for name in ("chmod", "rename")
            },
            "files": {},
            "all_denied": True,
        }
        bindings = {}
        fixture = f"{executor.SNAPSHOT_PARENT}/fixture"
        for label, source, filename, sha in (
            ("registry", str(executor.base.legacy.RESPONSE_REGISTRY), "response-registry.json", "a" * 64),
            ("admission", str(executor.base.legacy.ADMISSION), "admission.json", "b" * 64),
        ):
            bindings[label] = {
                "source": {"path": source, "sha256": sha, "size_bytes": 10, "read_stable": True},
                "snapshot": {
                    "path": f"{fixture}/{filename}",
                    "sha256": sha,
                    "size_bytes": 10,
                    "uid": 0,
                    "gid": 1000,
                    "mode_octal": "0440",
                },
            }
            probe["files"][filename] = {
                "read_sha256": sha,
                "denials": {
                    name: {"denied": True, "errno": 13}
                    for name in ("chmod", "write", "unlink", "rename")
                },
            }
        payload = (json.dumps(probe, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (evidence / "parity-snapshot-permissions.log").write_bytes(payload)
        self.snapshot = rooted(
            {
                "schema": executor.SNAPSHOT_SCHEMA,
                "directory": {"path": fixture, "uid": 0, "gid": 1000, "mode_octal": "0550"},
                "parent": {"path": str(executor.SNAPSHOT_PARENT), "uid": 0, "gid": 0, "mode_octal": "0711"},
                "bindings": bindings,
                "permission_probe": {
                    "returncode": 0,
                    "output_sha256": verifier.digest(payload),
                    "result": probe,
                },
            },
            "snapshot_root_sha256",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def verify(self) -> None:
        verifier._verify_snapshot(self.snapshot, self.root)

    def reroot(self) -> None:
        self.snapshot["snapshot_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.snapshot, "snapshot_root_sha256")
        )

    def test_valid_snapshot_contract_passes(self) -> None:
        self.verify()

    def test_source_snapshot_hash_mismatch_is_rejected(self) -> None:
        self.snapshot["bindings"]["registry"]["snapshot"]["sha256"] = "c" * 64
        self.reroot()
        with self.assertRaisesRegex(verifier.InvalidReceipt, "snapshot_registry_sha"):
            self.verify()

    def test_registry_admission_swap_is_rejected(self) -> None:
        left = self.snapshot["bindings"]["registry"]["snapshot"]["path"]
        self.snapshot["bindings"]["registry"]["snapshot"]["path"] = self.snapshot["bindings"]["admission"]["snapshot"]["path"]
        self.snapshot["bindings"]["admission"]["snapshot"]["path"] = left
        self.reroot()
        with self.assertRaisesRegex(verifier.InvalidReceipt, "snapshot_registry_path"):
            self.verify()

    def test_writable_snapshot_is_rejected(self) -> None:
        self.snapshot["bindings"]["registry"]["snapshot"]["mode_octal"] = "0640"
        self.reroot()
        with self.assertRaisesRegex(verifier.InvalidReceipt, "snapshot_registry_mode"):
            self.verify()

    def test_permission_probe_failure_is_rejected(self) -> None:
        self.snapshot["permission_probe"]["result"]["files"]["response-registry.json"]["denials"]["unlink"]["denied"] = False
        probe = self.snapshot["permission_probe"]["result"]
        payload = (json.dumps(probe, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (self.root / "evidence" / "parity-snapshot-permissions.log").write_bytes(payload)
        self.snapshot["permission_probe"]["output_sha256"] = verifier.digest(payload)
        self.reroot()
        with self.assertRaisesRegex(verifier.InvalidReceipt, "snapshot_probe_registry_unlink"):
            self.verify()


class StaticAuthorityBoundaryTests(unittest.TestCase):
    def test_snapshot_verifier_does_not_reread_mutable_live_sources(self) -> None:
        source = Path(verifier.__file__).read_text()
        snapshot_verifier = source.split("def _verify_snapshot(", 1)[1].split(
            "def _verify_parity(", 1
        )[0]
        self.assertIn('source.get("read_stable")', snapshot_verifier)
        self.assertNotIn("live_source_sha", snapshot_verifier)
        self.assertNotIn("file_digest(live_path)", snapshot_verifier)

    def test_old_executors_are_dependencies_not_launchers(self) -> None:
        for name in (
            "s1c3d_remote_transaction_v1.py",
            "verify_s1c3d_transaction_v1.py",
            "s1c3d_transaction_v1.py",
        ):
            source = (Path(__file__).resolve().parent / name).read_text()
            self.assertNotIn("run_s1c3b_transaction_v1.sh", source)
            self.assertNotIn("run_s1c3c_transaction_v1.sh", source)

    def test_installation_never_grants_scientific_authority(self) -> None:
        source = Path(verifier.__file__).read_text()
        self.assertNotIn('"scientific_authority": True', source)
        self.assertNotIn('"model_training": True', source)
        self.assertNotIn('"phase_mutation": True', source)


if __name__ == "__main__":
    unittest.main()
