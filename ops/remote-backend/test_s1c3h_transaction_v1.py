#!/usr/bin/env python3
"""Fault, ownership and verifier tests for S1C-3H."""

from __future__ import annotations

import json
import hashlib
import os
import re
import stat
import subprocess
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
    def test_staged_profile_copies_bound_structural_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            staging = root / "staging"
            staging.mkdir()
            profile = root / "profile.json"
            profile_value = {
                "response_runtime": {"registry": "old"},
                "runtime": {"admission_status": "old"},
                "deployment": {"response_runtime_build": "old"},
            }
            write(profile, json.dumps(profile_value).encode())
            structural = root / "STRUCTURAL_GATE_V2.json"
            structural_value = {
                "verdict": "PASS",
                "pass_count": 4,
                "route_count": 4,
                "blocked_routes": [],
            }
            write(structural, json.dumps(structural_value, sort_keys=True).encode())
            structural_sha = hashlib.sha256(structural.read_bytes()).hexdigest()
            user = SimpleNamespace(pw_uid=os.getuid(), pw_gid=os.getgid())
            with (
                mock.patch.object(executor, "GATE_PROFILE", profile),
                mock.patch.object(executor, "STRUCTURAL_RECEIPT", structural),
                mock.patch.object(executor, "STRUCTURAL_RECEIPT_SHA256", structural_sha),
                mock.patch.object(executor.pwd, "getpwnam", return_value=user),
            ):
                destination = executor.staged_profile(staging, staging / "authority")
            self.assertTrue(destination.is_file())
            self.assertEqual(
                (staging / "receipts" / "STRUCTURAL_GATE_V2.json").read_bytes(),
                structural.read_bytes(),
            )

    def test_clean_interpreter_import_exposes_every_bound_dependency(self) -> None:
        directory = Path(executor.__file__).resolve().parent
        script = """
import s1c3h_remote_transaction_v1 as module
required = (
    'canonical_bytes', 'economics_snapshot', 'fsync_directory',
    'health_snapshot', 'journal_snapshot', 'process_environment',
    'service_snapshot', 'sha256_file', 'systemctl', 'write_json',
)
missing = [name for name in required if not callable(getattr(module, name, None))]
assert not missing, missing
assert callable(module.http_json)
"""
        subprocess.run(
            [__import__("sys").executable, "-c", script],
            check=True,
            cwd=directory,
            capture_output=True,
            timeout=10,
        )

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

    def test_pause_clears_only_intentional_oneshot_stop_failures(self) -> None:
        calls: list[tuple[str, str]] = []

        def control(action: str, unit: str, check: bool = True) -> None:
            del check
            calls.append((action, unit))

        def state(unit: str) -> dict[str, object]:
            self.assertIn(unit, (*executor.TRIGGER_UNITS, *executor.ONESHOT_UNITS))
            return {
                "active_state": "inactive",
                "result": "success",
                "exec_main_status": 0,
            }

        with (
            mock.patch.object(executor, "systemctl", side_effect=control),
            mock.patch.object(executor, "unit_state", side_effect=state),
        ):
            executor.pause_authority_triggers()
        reset_calls = [call for call in calls if call[0] == "reset-failed"]
        self.assertEqual(
            reset_calls,
            [("reset-failed", unit) for unit in executor.ONESHOT_UNITS],
        )
        first_reset = calls.index(reset_calls[0])
        self.assertTrue(all(action == "stop" for action, _ in calls[:first_reset]))

    def test_restored_trigger_uses_settled_snapshot_without_second_read(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in executor.ONESHOT_UNITS},
        }
        settled = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{
                unit: {
                    "active_state": "inactive",
                    "result": "success",
                    "exec_main_status": 0,
                }
                for unit in executor.ONESHOT_UNITS
            },
        }
        retriggered = {
            **settled,
            executor.ONESHOT_UNITS[1]: {
                "active_state": "activating",
                "result": "success",
                "exec_main_status": 0,
            },
        }
        with mock.patch.object(
            executor, "trigger_snapshot", side_effect=[settled, retriggered]
        ) as snapshot:
            observed = executor.wait_for_oneshots()
            executor.require_trigger_state_restored(before, observed)
        snapshot.assert_called_once_with()

    def test_restored_trigger_rejects_failed_oneshot(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in executor.ONESHOT_UNITS},
        }
        after = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{
                unit: {
                    "active_state": "inactive",
                    "result": "success",
                    "exec_main_status": 0,
                }
                for unit in executor.ONESHOT_UNITS
            },
        }
        after[executor.ONESHOT_UNITS[0]] = {
            "active_state": "failed",
            "result": "exit-code",
            "exec_main_status": 1,
        }
        with (
            self.assertRaisesRegex(executor.GateFailure, "s1c3h_oneshot_final_state"),
        ):
            executor.require_trigger_state_restored(before, after)

    def test_authority_renews_before_background_triggers_resume(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in executor.ONESHOT_UNITS},
        }
        completed = {
            "active_state": "inactive",
            "result": "success",
            "exec_main_status": 0,
        }
        calls: list[tuple[str, str]] = []

        def control(action: str, unit: str, check: bool = True) -> None:
            del check
            calls.append((action, unit))

        def state(unit: str) -> dict[str, object]:
            if unit in executor.TRIGGER_UNITS:
                return {"active_state": "active"}
            return completed

        with (
            mock.patch.object(executor, "systemctl", side_effect=control),
            mock.patch.object(executor, "unit_state", side_effect=state),
        ):
            after = executor.renew_authority_and_restore_triggers(before)
        self.assertEqual(
            calls[: len(executor.ONESHOT_UNITS)],
            [("start", unit) for unit in executor.ONESHOT_UNITS],
        )
        self.assertEqual(
            calls[len(executor.ONESHOT_UNITS) :],
            [("start", unit) for unit in executor.TRIGGER_UNITS],
        )
        self.assertEqual(after[executor.ONESHOT_UNITS[0]], completed)

    def test_failed_explicit_authority_renewal_does_not_restore_triggers(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in executor.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in executor.ONESHOT_UNITS},
        }
        failed = {
            "active_state": "failed",
            "result": "exit-code",
            "exec_main_status": 1,
        }
        calls: list[tuple[str, str]] = []

        def control(action: str, unit: str, check: bool = True) -> None:
            del check
            calls.append((action, unit))

        with (
            mock.patch.object(executor, "systemctl", side_effect=control),
            mock.patch.object(executor, "unit_state", return_value=failed),
            self.assertRaisesRegex(executor.GateFailure, "oneshot_renewal_failed"),
        ):
            executor.renew_authority_and_restore_triggers(before)
        self.assertEqual(calls, [("start", executor.ONESHOT_UNITS[0])])

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
    def test_interrupted_recovery_requires_primary_diagnostic(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.write_json(
                root / "transaction-state.json", {"state": "RUNTIME_INSTALLED"}, 0o600
            )
            executor.write_json(
                root / "preparation.json",
                {
                    "transaction_id": "test",
                    "services_before": {},
                    "triggers_before": {},
                    "nginx_pid_before": 1,
                    "preparation_root_sha256": "a" * 64,
                },
                0o400,
            )
            args = SimpleNamespace(
                transaction_directory=str(root),
                repair_source_commit="b" * 40,
                repair_source_tree="c" * 40,
                diagnostic_scope="PRIMARY_FAILURE_PRESERVED",
            )
            production = {"pair": {"pair_contract_equal": True}, "journal": {}}
            with (
                mock.patch.object(executor, "verify_current_production", return_value=production),
                mock.patch.object(executor, "service_snapshot", return_value={}),
                mock.patch.object(executor, "stable_health", return_value={}),
                mock.patch.object(executor, "health_contract"),
                mock.patch.object(executor, "stable_trigger_baseline", return_value={}),
                mock.patch.object(
                    executor,
                    "economics_snapshot",
                    return_value={"false_accepts": 0, "runtime_parity_mismatches": 0},
                ),
                mock.patch.object(executor, "nginx_pid", return_value=1),
                mock.patch.object(executor, "existing_candidate_diagnostic", return_value=None),
                self.assertRaisesRegex(executor.GateFailure, "diagnostic_missing"),
            ):
                executor.recover_interrupted(args)

    def test_interrupted_recovery_seal_requires_independent_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor.write_json(
                root / "transaction-state.json",
                {"state": "RECOVERY_VERIFICATION_PENDING"},
                0o600,
            )
            receipt = executor.rooted(
                {
                    "transaction_id": "test",
                    "verdict": "S1C3H_INTERRUPTED_RECOVERY_PASS",
                    "connector_survival": "UNKNOWN_MISSING_ORIGINAL_BEFORE_ARTIFACT",
                },
                "recovery_receipt_root_sha256",
            )
            executor.write_json(root / "interrupted-recovery-receipt.json", receipt, 0o400)
            verification = root / "verification.json"
            executor.write_json(
                verification,
                {
                    "schema": executor.RECOVERY_VERIFICATION_SCHEMA,
                    "valid": True,
                    "authority": True,
                    "verdict": receipt["verdict"],
                    "recovery_receipt_root_sha256": "d" * 64,
                    "recovery_verification_root_sha256": "e" * 64,
                },
                0o400,
            )
            with self.assertRaisesRegex(
                executor.GateFailure, "recovery_verification_root"
            ):
                executor.seal_interrupted_recovery(
                    SimpleNamespace(
                        transaction_directory=str(root),
                        recovery_verification=str(verification),
                    )
                )

    def test_preflight_failure_seals_with_verified_unchanged_production(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            failure = executor.rooted(
                {
                    "schema": "nando.s1c3h-preflight-failure.v1",
                    "transaction_id": "test",
                    "error": "clean-import-blocker",
                    "production_mutation": False,
                    "observed_at_unix": 1,
                },
                "preflight_failure_root_sha256",
            )
            executor.write_json(root / "preflight-failure.json", failure, 0o400)
            executor.write_json(root / "transaction-state.json", {"state": "PREFLIGHT_FAILURE"}, 0o600)
            production = {"pair": {"pair_contract_equal": True}, "journal": {"record_count": 0}}
            with (
                mock.patch.object(executor, "verify_current_production", return_value=production),
                mock.patch.object(executor, "execution_staging", return_value=root / "missing"),
            ):
                executor.abort_predeployment(
                    SimpleNamespace(transaction_directory=str(root), reason="ignored")
                )
            terminal = json.loads((root / "s1c3h-state.json").read_text())
            self.assertEqual(terminal["state"], "COMPLETE")
            self.assertEqual(terminal["verdict"], "S1C3H_PREFLIGHT_FAILURE")
            self.assertEqual(
                terminal["preflight_failure_root_sha256"],
                failure["preflight_failure_root_sha256"],
            )
            self.assertFalse(terminal["production_mutation"])

    def test_legacy_preflight_failure_preserves_original_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = b'{"error":"old-import-error","schema":"nando.s1c3h-preflight-failure.v1"}\n'
            write(root / "preflight-failure.json", original, 0o400)
            executor.write_json(
                root / "transaction-state.json",
                {"state": "PREFLIGHT_FAILURE", "transaction_id": "legacy-test"},
                0o600,
            )
            production = {"pair": {"pair_contract_equal": True}, "journal": {"record_count": 0}}
            with (
                mock.patch.object(executor, "verify_current_production", return_value=production),
                mock.patch.object(executor, "execution_staging", return_value=root / "missing"),
            ):
                executor.abort_predeployment(
                    SimpleNamespace(transaction_directory=str(root), reason="ignored")
                )
            self.assertEqual(
                (root / "preflight-failure.unrooted.json").read_bytes(), original
            )
            rooted_failure = json.loads((root / "preflight-failure.json").read_text())
            self.assertEqual(rooted_failure["transaction_id"], "legacy-test")
            self.assertEqual(
                rooted_failure["legacy_unrooted_sha256"], hashlib.sha256(original).hexdigest()
            )

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

    def test_first_rooted_diagnostic_survives_repeated_rollback(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = executor.rooted(
                {
                    "schema": executor.DIAGNOSTIC_SCHEMA,
                    "stage": "triggers_restored",
                    "error": "original_failure",
                    "observed_at_unix": 1,
                },
                "diagnostic_root_sha256",
            )
            executor.write_json(root / "candidate-diagnostic.json", original, 0o400)
            original_bytes = (root / "candidate-diagnostic.json").read_bytes()
            with (
                mock.patch.object(executor, "stable_health") as health,
                mock.patch.object(executor, "journal_snapshot") as journal,
            ):
                observed = executor.persist_candidate_diagnostic(
                    root, "manual_rollback:RUNTIME_INSTALLED", "later_failure"
                )
            self.assertEqual(observed, original)
            self.assertEqual(
                (root / "candidate-diagnostic.json").read_bytes(), original_bytes
            )
            health.assert_not_called()
            journal.assert_not_called()

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
                mock.patch.object(
                    executor, "renew_authority_and_restore_triggers", return_value={}
                ),
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
                    mock.patch.object(
                        executor,
                        "renew_authority_and_restore_triggers",
                        return_value={},
                    ),
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
    def interrupted_recovery_fixture(self, root: Path) -> tuple[str, str]:
        source_commit = "a" * 40
        source_tree = "b" * 40
        preparation = {
            "schema": "nando.s1c3h-preparation.v1",
            "transaction_id": "20260812T211329Z-570609bdef03-s1c3h-v1",
            "services_before": {},
            "triggers_before": {
                **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
                **{unit: {"active_state": "inactive"} for unit in verifier.ONESHOT_UNITS},
            },
            "baseline": {"journal": {}},
            "nginx_pid_before": 1,
        }
        preparation["preparation_root_sha256"] = verifier.digest_bytes(
            verifier.canonical_bytes(preparation)
        )
        executor.write_json(root / "preparation.json", preparation, 0o400)
        diagnostic = {
            "schema": executor.DIAGNOSTIC_SCHEMA,
            "stage": "manual_rollback:RUNTIME_INSTALLED",
        }
        diagnostic["diagnostic_root_sha256"] = verifier.digest_bytes(
            verifier.canonical_bytes(diagnostic)
        )
        executor.write_json(root / "candidate-diagnostic.json", diagnostic, 0o400)
        pair = {
            "transition_sha256": verifier.BASELINE_TRANSITION_SHA256,
            "authority_sha256": verifier.BASELINE_AUTHORITY_SHA256,
            "transition_runtime_contract_sha256": verifier.BASELINE_RUNTIME_CONTRACT,
            "authority_runtime_contract_sha256": verifier.BASELINE_RUNTIME_CONTRACT,
            "pair_contract_equal": True,
        }
        triggers = {
            **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
            **{
                unit: {
                    "active_state": "inactive",
                    "result": "success",
                    "exec_main_status": 0,
                }
                for unit in verifier.ONESHOT_UNITS
            },
        }
        health = {
            label: {
                "stable": {
                    "ok": True,
                    "mode": "CPU",
                    "admission_verdict": "PASS",
                    "response_active_profiles": 2,
                    "response_executor_cache_ready": True,
                }
            }
            for label in ("hot", "cpu")
        }
        receipt = {
            "schema": verifier.RECOVERY_RECEIPT_SCHEMA,
            "transaction_id": preparation["transaction_id"],
            "verdict": "S1C3H_INTERRUPTED_RECOVERY_PASS",
            "repair_source": {"commit": source_commit, "tree": source_tree},
            "preparation_root_sha256": preparation["preparation_root_sha256"],
            "recovered_from_state": "RUNTIME_INSTALLED",
            "production_pair": pair,
            "production_config_sha256": verifier.BASELINE_CONFIG_SHA256,
            "services": {},
            "health": health,
            "triggers": triggers,
            "journal": {},
            "economics": {"false_accepts": 0, "runtime_parity_mismatches": 0},
            "nginx_pid_before": 1,
            "nginx_pid_after": 1,
            "connector_survival": "UNKNOWN_MISSING_ORIGINAL_BEFORE_ARTIFACT",
            "diagnostic_root_sha256": diagnostic["diagnostic_root_sha256"],
            "diagnostic_scope": "RECOVERY_DIAGNOSTIC_ONLY_PRIMARY_FAILURE_WAS_OVERWRITTEN",
            "capture_installed": False,
            "scientific_authority": False,
        }
        receipt["recovery_receipt_root_sha256"] = verifier.digest_bytes(
            verifier.canonical_bytes(receipt)
        )
        executor.write_json(root / "interrupted-recovery-receipt.json", receipt, 0o400)
        return source_commit, source_tree

    def test_interrupted_recovery_verifier_preserves_unknown_connector_scope(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commit, tree = self.interrupted_recovery_fixture(root)
            with mock.patch.object(verifier.parent.parent, "verify_runtime_journal"):
                result = verifier.verify_interrupted_recovery(root, commit, tree)
            self.assertEqual(result["verdict"], "S1C3H_INTERRUPTED_RECOVERY_PASS")
            self.assertFalse(result["capture_installed"])

    def test_interrupted_recovery_verifier_rejects_connector_backfill(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commit, tree = self.interrupted_recovery_fixture(root)
            receipt = json.loads(
                (root / "interrupted-recovery-receipt.json").read_text()
            )
            receipt["connector_survival"] = "PASS"
            receipt["recovery_receipt_root_sha256"] = verifier.digest_bytes(
                verifier.canonical_bytes(receipt, "recovery_receipt_root_sha256")
            )
            executor.write_json(root / "interrupted-recovery-receipt.json", receipt, 0o400)
            with (
                mock.patch.object(verifier.parent.parent, "verify_runtime_journal"),
                self.assertRaisesRegex(verifier.InvalidReceipt, "recovery_connector_scope"),
            ):
                verifier.verify_interrupted_recovery(root, commit, tree)

    def test_interrupted_recovery_verifier_rejects_unproved_journal_change(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commit, tree = self.interrupted_recovery_fixture(root)
            receipt = json.loads(
                (root / "interrupted-recovery-receipt.json").read_text()
            )
            receipt["journal"] = {"changed": True}
            receipt["recovery_receipt_root_sha256"] = verifier.digest_bytes(
                verifier.canonical_bytes(receipt, "recovery_receipt_root_sha256")
            )
            executor.write_json(root / "interrupted-recovery-receipt.json", receipt, 0o400)
            with (
                mock.patch.object(
                    verifier.parent.parent,
                    "verify_runtime_journal",
                    side_effect=verifier.InvalidReceipt("missing_prefix_proof"),
                ),
                self.assertRaisesRegex(verifier.InvalidReceipt, "missing_prefix_proof"),
            ):
                verifier.verify_interrupted_recovery(root, commit, tree)

    def test_interrupted_recovery_verifier_rejects_nonterminal_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            commit, tree = self.interrupted_recovery_fixture(root)
            terminal = {
                "state": "RECOVERY_VERIFICATION_PENDING",
                "verdict": "S1C3H_INTERRUPTED_RECOVERY_PASS",
            }
            terminal["state_root_sha256"] = verifier.digest_bytes(
                verifier.canonical_bytes(terminal)
            )
            executor.write_json(root / "s1c3h-state.json", terminal, 0o400)
            with (
                mock.patch.object(verifier.parent.parent, "verify_runtime_journal"),
                self.assertRaisesRegex(verifier.InvalidReceipt, "recovery_terminal_state"),
            ):
                verifier.verify_interrupted_recovery(root, commit, tree)

    def test_trigger_verifier_accepts_settled_oneshot_snapshot(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in verifier.ONESHOT_UNITS},
        }
        after = {
            **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
            verifier.ONESHOT_UNITS[0]: {
                "active_state": "inactive",
                "result": "success",
                "exec_main_status": 0,
            },
            verifier.ONESHOT_UNITS[1]: {
                "active_state": "inactive",
                "result": "success",
                "exec_main_status": 0,
            },
        }
        verifier.verify_triggers(before, after)

    def test_trigger_verifier_rejects_failed_oneshot(self) -> None:
        before = {
            **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
            **{unit: {"active_state": "inactive"} for unit in verifier.ONESHOT_UNITS},
        }
        after = {
            **{unit: {"active_state": "active"} for unit in verifier.TRIGGER_UNITS},
            **{
                unit: {
                    "active_state": "inactive",
                    "result": "success",
                    "exec_main_status": 0,
                }
                for unit in verifier.ONESHOT_UNITS
            },
        }
        after[verifier.ONESHOT_UNITS[0]] = {
            "active_state": "failed",
            "result": "exit-code",
            "exec_main_status": 1,
        }
        with self.assertRaisesRegex(verifier.InvalidReceipt, "oneshot_state"):
            verifier.verify_triggers(before, after)

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
