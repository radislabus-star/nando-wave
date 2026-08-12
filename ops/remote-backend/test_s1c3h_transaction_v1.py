#!/usr/bin/env python3
"""Fault, ownership and verifier tests for S1C-3H."""

from __future__ import annotations

import json
import hashlib
import os
import re
import stat
import tempfile
import unittest
from contextlib import ExitStack
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import s1c3h_remote_transaction_v1 as executor
import verify_s1c3h_transaction_v1 as verifier


CONTRACT = "a" * 64


def write(path: Path, payload: bytes = b"{}", mode: int = 0o600) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    path.chmod(mode)


def compatibility_fixture(root: Path) -> str:
    generation_root = "b" * 64
    for name in executor.COMPATIBILITY_FILES:
        payload = (
            json.dumps({"generation_root_sha256": generation_root}).encode()
            if name == "response-authority-sidecar-current-v2.json"
            else json.dumps({"name": name}).encode()
        )
        write(root / name, payload, 0o600)
    generation = root / "response-authority-sidecar-generations-v2" / generation_root
    write(generation / "manifest.json", b'{"generation":"old"}', 0o600)
    write(generation / "registry.json", b'{"registry":"old"}', 0o640)
    generation.chmod(0o700)
    return generation_root


class StagingTests(unittest.TestCase):
    def test_trigger_baseline_waits_for_natural_oneshot_completion(self) -> None:
        busy = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "activating"} for unit in executor.ONESHOT_UNITS},
        }
        quiet = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in executor.ONESHOT_UNITS},
        }
        with (
            mock.patch.object(executor, "trigger_snapshot", side_effect=[busy, quiet]),
            mock.patch.object(executor.time, "sleep"),
            mock.patch.object(executor.time, "monotonic", side_effect=[0.0, 0.0, 0.1]),
        ):
            self.assertEqual(executor.stable_trigger_baseline(timeout=1.0), quiet)

    def test_execution_staging_is_not_below_root_only_e_can_traverse(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root = base / "deployments" / "20260812T120000Z-aaaaaaaaaaaa-s1c3h-v1"
            root.mkdir(parents=True, mode=0o700)
            write(root / "candidate-nando-response-admission", b"binary", 0o500)
            state = base / "state"
            for name in (
                "response-registry.json",
                "response-admission-controller.json",
                "response-authority-candidate.json",
                "response-admission-controller.marker.json",
            ):
                write(state / name)
            user = SimpleNamespace(pw_uid=os.getuid(), pw_gid=os.getgid())
            with (
                mock.patch.object(executor, "STATE_DIR", state),
                mock.patch.object(executor, "EXECUTION_STAGING_PARENT", state / ".staging"),
                mock.patch.object(executor.pwd, "getpwnam", return_value=user),
            ):
                staging = executor.reset_staging(root)
            self.assertEqual(staging.parent, state / ".staging")
            self.assertFalse(staging.is_relative_to(root))
            self.assertEqual(stat.S_IMODE(staging.stat().st_mode), 0o700)
            self.assertEqual(
                stat.S_IMODE((staging / "nando-response-admission").stat().st_mode),
                0o500,
            )

    def test_controller_failure_bytes_are_persisted_before_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "evidence").mkdir()
            staging = root / "staging"
            staging.mkdir()
            write(staging / "nando-response-admission", b"binary", 0o500)
            completed = SimpleNamespace(returncode=17, stdout=b"out", stderr=b"real blocker")
            with (
                mock.patch.object(executor, "reset_staging", return_value=staging),
                mock.patch.object(executor, "run_as_e", return_value=completed),
                self.assertRaisesRegex(executor.GateFailure, "staged_controller_exit:17"),
            ):
                executor.stage_candidate_authority(root, "test")
            self.assertEqual((root / "evidence" / "candidate-controller.stderr").read_bytes(), b"real blocker")


class CompatibilitySnapshotTests(unittest.TestCase):
    def test_atomic_generation_copy_preserves_mode_and_ownership(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            destination = root / "destination"
            write(source / "private.json", b"private", 0o600)
            source.chmod(0o700)
            executor.copy_tree_atomic(source, destination)
            self.assertEqual(
                (destination.stat().st_uid, destination.stat().st_gid),
                (source.stat().st_uid, source.stat().st_gid),
            )
            self.assertEqual(
                (
                    (destination / "private.json").stat().st_uid,
                    (destination / "private.json").stat().st_gid,
                    stat.S_IMODE((destination / "private.json").stat().st_mode),
                ),
                (
                    (source / "private.json").stat().st_uid,
                    (source / "private.json").stat().st_gid,
                    0o600,
                ),
            )

    def test_snapshot_binds_complete_generation_and_preserves_modes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            state = base / "state"
            generation_root = compatibility_fixture(state)
            destination = base / "snapshot"
            with mock.patch.object(executor, "STATE_DIR", state):
                executor.snapshot_compatibility(destination)
                value = executor.verify_compatibility_snapshot(destination)
            self.assertEqual(value["generation_root_sha256"], generation_root)
            self.assertEqual(
                stat.S_IMODE((destination / "generation" / "registry.json").stat().st_mode),
                0o640,
            )

    def test_snapshot_tamper_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            state = base / "state"
            compatibility_fixture(state)
            destination = base / "snapshot"
            with mock.patch.object(executor, "STATE_DIR", state):
                executor.snapshot_compatibility(destination)
            destination.chmod(0o700)
            (destination / "admission.json").chmod(0o600)
            write(destination / "admission.json", b"tampered", 0o600)
            with self.assertRaisesRegex(executor.GateFailure, "snapshot_files"):
                executor.verify_compatibility_snapshot(destination)

    def test_candidate_generation_is_published_before_pointer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staging = root / "staging"
            state = root / "state"
            generation_root = compatibility_fixture(staging)
            calls: list[str] = []

            def tree(source: Path, destination: Path) -> None:
                self.assertEqual(source.name, generation_root)
                calls.append("generation")

            def install(
                source: Path,
                destination: Path,
                mode: int,
                ownership: tuple[int, int] | None = None,
            ) -> None:
                del source, mode, ownership
                calls.append(destination.name)

            with (
                mock.patch.object(executor, "STATE_DIR", state),
                mock.patch.object(executor, "execution_staging", return_value=staging),
                mock.patch.object(executor, "copy_tree_atomic", side_effect=tree),
                mock.patch.object(executor, "atomic_install", side_effect=install),
            ):
                executor.install_staged_authority(root)
            self.assertEqual(calls[0], "generation")
            self.assertLess(calls.index("response-authority-sidecar-current-v2.json"), calls.index("admission.json"))


class TransactionStateTests(unittest.TestCase):
    def test_diagnostic_precedes_rollback_on_execute_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.write_json(root / "transaction-state.json", {"state": "PREPARED"}, 0o600)
            executor.write_json(root / "preparation.json", {"transaction_id": "test"}, 0o600)
            order: list[str] = []

            def diagnostic(*_: object) -> dict[str, str]:
                order.append("diagnostic")
                return {"diagnostic_root_sha256": "c" * 64}

            def rollback(*_: object) -> None:
                order.append("rollback")

            args = SimpleNamespace(
                transaction_directory=str(root), predeployment_verification=str(root / "preflight")
            )
            with (
                mock.patch.object(executor, "verify_predeployment"),
                mock.patch.object(executor, "pause_authority_triggers", side_effect=RuntimeError("stop")),
                mock.patch.object(executor, "persist_candidate_diagnostic", side_effect=diagnostic),
                mock.patch.object(executor, "rollback", side_effect=rollback),
                self.assertRaisesRegex(executor.GateFailure, "S1C3H_ROLLBACK_PASS"),
            ):
                executor.execute(args)
            self.assertEqual(order, ["diagnostic", "rollback"])

    def test_pre_mutation_rollback_does_not_rewrite_production(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.write_json(root / "transaction-state.json", {"state": "ROLLBACK_ARMED"}, 0o600)
            executor.write_json(
                root / "preparation.json",
                {
                    "transaction_id": "test",
                    "triggers_before": {},
                    "baseline": {"journal": {"record_count": 0}},
                },
                0o600,
            )
            pair = {
                "transition_sha256": executor.BASELINE_TRANSITION_SHA256,
                "authority_sha256": executor.BASELINE_AUTHORITY_SHA256,
                "transition_runtime_contract_sha256": executor.BASELINE_RUNTIME_CONTRACT,
                "authority_runtime_contract_sha256": executor.BASELINE_RUNTIME_CONTRACT,
                "pair_contract_equal": True,
            }
            journal = {"record_count": 0}
            with (
                mock.patch.object(executor, "persist_candidate_diagnostic", return_value={"diagnostic_root_sha256": "d" * 64}),
                mock.patch.object(executor, "pause_authority_triggers"),
                mock.patch.object(executor, "pair_identity", return_value=pair),
                mock.patch.object(executor, "wait_for_runtime", return_value=({}, {})),
                mock.patch.object(executor, "restore_authority_triggers"),
                mock.patch.object(executor, "wait_for_oneshots"),
                mock.patch.object(executor, "require_trigger_state_restored"),
                mock.patch.object(executor, "journal_snapshot", return_value=journal),
                mock.patch.object(executor, "require_prefix_preserved"),
                mock.patch.object(executor, "economics_snapshot", return_value={"false_accepts": 0, "runtime_parity_mismatches": 0}),
                mock.patch.object(executor, "nginx_pid", return_value=1),
                mock.patch.object(executor, "sha256_file", return_value=executor.BASELINE_CONFIG_SHA256),
                mock.patch.object(executor, "atomic_install") as install,
                mock.patch.object(executor, "restore_compatibility_files") as restore,
            ):
                executor.rollback(root, "test")
            install.assert_not_called()
            restore.assert_not_called()

    def test_rollback_fault_matrix_restores_every_old_byte(self) -> None:
        cases = {
            "MUTATION_STARTED": ("authority",),
            "AUTHORITY_INSTALLED": ("authority", "compatibility"),
            "RUNTIME_INSTALLED": ("authority", "compatibility", "config", "transition"),
        }
        for state_name, mutations in cases.items():
            with self.subTest(state=state_name), tempfile.TemporaryDirectory() as directory:
                base = Path(directory)
                state_dir = base / "state"
                compatibility_fixture(state_dir)
                transition = base / "bin" / "transition"
                authority = base / "bin" / "authority"
                config = base / "etc" / "transition.env"
                old = {
                    transition: b"old-transition",
                    authority: b"old-authority",
                    config: b"old-config",
                }
                for path, payload in old.items():
                    write(path, payload, 0o755 if path != config else 0o644)
                old_compatibility = {
                    name: (state_dir / name).read_bytes()
                    for name in executor.COMPATIBILITY_FILES
                }
                root = base / "transaction"
                (root / "rollback").mkdir(parents=True)
                write(root / "rollback" / "nando-transition-serving", old[transition], 0o500)
                write(root / "rollback" / "nando-response-admission", old[authority], 0o500)
                write(root / "rollback" / "transition-serving.env", old[config], 0o400)
                with mock.patch.object(executor, "STATE_DIR", state_dir):
                    executor.snapshot_compatibility(root / "rollback" / "compatibility-frozen")
                executor.write_json(root / "transaction-state.json", {"state": state_name}, 0o600)
                executor.write_json(
                    root / "preparation.json",
                    {
                        "transaction_id": "test",
                        "triggers_before": {},
                        "baseline": {"journal": {"record_count": 0}},
                    },
                    0o600,
                )
                if "authority" in mutations:
                    authority.write_bytes(b"new-authority")
                if "compatibility" in mutations:
                    for name in executor.COMPATIBILITY_FILES:
                        (state_dir / name).write_bytes(b'{"candidate":true}')
                if "config" in mutations:
                    config.write_bytes(b"new-config")
                if "transition" in mutations:
                    transition.write_bytes(b"new-transition")
                transition_hash = hashlib.sha256(old[transition]).hexdigest()
                authority_hash = hashlib.sha256(old[authority]).hexdigest()
                config_hash = hashlib.sha256(old[config]).hexdigest()
                journal = {"record_count": 0}
                patches = (
                    mock.patch.object(executor, "TRANSITION_BINARY", transition),
                    mock.patch.object(executor, "AUTHORITY_BINARY", authority),
                    mock.patch.object(executor, "TRANSITION_CONFIG", config),
                    mock.patch.object(executor, "STATE_DIR", state_dir),
                    mock.patch.object(executor, "BASELINE_TRANSITION_SHA256", transition_hash),
                    mock.patch.object(executor, "BASELINE_AUTHORITY_SHA256", authority_hash),
                    mock.patch.object(executor, "BASELINE_CONFIG_SHA256", config_hash),
                    mock.patch.object(executor, "runtime_contract", return_value=executor.BASELINE_RUNTIME_CONTRACT),
                    mock.patch.object(executor, "pause_authority_triggers"),
                    mock.patch.object(executor, "systemctl"),
                    mock.patch.object(executor, "wait_for_runtime", return_value=({}, {})),
                    mock.patch.object(executor, "restore_authority_triggers"),
                    mock.patch.object(executor, "wait_for_oneshots"),
                    mock.patch.object(executor, "require_trigger_state_restored"),
                    mock.patch.object(executor, "journal_snapshot", return_value=journal),
                    mock.patch.object(executor, "require_prefix_preserved"),
                    mock.patch.object(executor, "economics_snapshot", return_value={"false_accepts": 0, "runtime_parity_mismatches": 0}),
                    mock.patch.object(executor, "nginx_pid", return_value=1),
                )
                with ExitStack() as stack:
                    for patcher in patches:
                        stack.enter_context(patcher)
                    executor.rollback(
                        root,
                        "injected",
                        {"diagnostic_root_sha256": "d" * 64},
                    )
                for path, payload in old.items():
                    self.assertEqual(path.read_bytes(), payload)
                for name, payload in old_compatibility.items():
                    self.assertEqual((state_dir / name).read_bytes(), payload)

    def test_fault_matrix_names_every_mutating_boundary(self) -> None:
        source = Path(executor.__file__).read_text(encoding="utf-8")
        for stage in (
            "runtime_stopped",
            "authority_binary_installed",
            "authority_installed",
            "config_installed",
            "transition_binary_installed",
            "runtime_installed",
            "runtime_started",
            "triggers_restored",
        ):
            self.assertRegex(
                source,
                rf'stage = "{re.escape(stage)}"\s+fault_after\(stage\)',
            )

    def test_installation_source_cannot_grant_scientific_authority(self) -> None:
        source = Path(executor.__file__).read_text() + Path(verifier.__file__).read_text()
        self.assertNotIn('"scientific_authority": True', source)
        self.assertNotIn('"phase_mutation": True', source)
        self.assertNotIn('"model_training": True', source)


class VerifierTests(unittest.TestCase):
    def test_snapshot_verifier_ignores_transport_owner_rewrite_but_binds_record(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            state = base / "state"
            compatibility_fixture(state)
            snapshot = base / "snapshot"
            with mock.patch.object(executor, "STATE_DIR", state):
                executor.snapshot_compatibility(snapshot)
            value = verifier.verify_compatibility_snapshot(snapshot)
            self.assertIn("uid", value["compatibility_files"]["admission.json"])
            self.assertNotIn(
                "uid",
                verifier.transport_projection(value["compatibility_files"])["admission.json"],
            )

    def test_executor_accepts_verifier_freeze_schema(self) -> None:
        directory = Path(verifier.__file__).resolve().parent
        value = verifier.create_freeze("a" * 40, "b" * 40, directory)
        executor.verify_implementation_freeze(value)

    def test_freeze_binds_all_new_and_inherited_files(self) -> None:
        directory = Path(verifier.__file__).resolve().parent
        for name in verifier.IMPLEMENTATION_FILES[:3]:
            self.assertTrue((directory / name).is_file())

    def test_mixed_pair_build_receipt_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            transition = root / "transition"
            authority = root / "authority"
            config = root / "config"
            write(transition, b"transition", 0o500)
            write(authority, b"authority", 0o500)
            write(config, b"config")
            with (
                mock.patch.object(verifier, "runtime_contract", side_effect=["a" * 64, "b" * 64]),
                self.assertRaisesRegex(verifier.InvalidReceipt, "candidate_pair_contract"),
            ):
                verifier.create_build_receipt(transition, authority, config)


if __name__ == "__main__":
    unittest.main()
