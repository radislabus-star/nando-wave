#!/usr/bin/env python3
"""Focused contract tests for the separately rooted S1C-3C transaction."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from types import SimpleNamespace
from pathlib import Path
from unittest import mock

import s1c3c_schema_preflight_v1 as schema_gate
import s1c3c_transaction_v1 as wrapper
import verify_s1c3c_transaction_v1 as authority


ROOT = Path(__file__).resolve().parents[2]
VALID_ATTEMPT = "20260812T120000Z-2a1505055ce9-s1c3c-v1"


class SchemaPreflightTests(unittest.TestCase):
    def test_complete_preflight_is_pure_and_rooted(self) -> None:
        receipt = schema_gate.run_preflight()
        self.assertTrue(receipt["valid"])
        self.assertFalse(receipt["authority"])
        self.assertFalse(receipt["side_effects"])
        self.assertFalse(receipt["remote_attempt_created"])
        self.assertEqual(
            receipt["schema_preflight_root_sha256"],
            schema_gate.digest(
                schema_gate.canonical_bytes(receipt, "schema_preflight_root_sha256")
            ),
        )
        self.assertEqual(
            [row["name"] for row in receipt["metric_families"]],
            ["hot", "single_sync", "three_sync", "idle"],
        )

    def test_exact_observed_idle_log_contract(self) -> None:
        idle = next(spec for spec in schema_gate.SPECS if spec.name == "idle")
        self.assertEqual(
            schema_gate.parse_metric(idle, idle.fixture),
            {
                "elapsed_ticks": 0,
                "ticks_per_second": 100,
                "percent_of_one_core": 0.0,
            },
        )

    def test_group_count_mismatch_is_local_veto(self) -> None:
        hot = next(spec for spec in schema_gate.SPECS if spec.name == "hot")
        broken = schema_gate.MetricSpec(
            hot.name,
            hot.pattern,
            hot.fields[:-1],
            hot.kinds[:-1],
            hot.fixture,
        )
        with self.assertRaisesRegex(schema_gate.SchemaVeto, "regex_field_count"):
            schema_gate.validate_spec(broken)

    def test_every_declared_field_is_retained(self) -> None:
        for spec in schema_gate.SPECS:
            row = schema_gate.validate_spec(spec)
            self.assertEqual(set(row["mutation_roots"]), set(spec.fields))
            self.assertEqual(len(set(row["mutation_roots"].values())), len(spec.fields))

    def test_frozen_thresholds_have_no_adaptation(self) -> None:
        self.assertIn("matched_p99", schema_gate.evaluate_hot({
            "p99_ns": 1_000_001,
            "no_goal_p99_ns": 0,
            "hard_max_ns": 0,
            "samples": 4096,
        })["resource_failures"])
        self.assertIn("p99", schema_gate.evaluate_single_sync({
            "p99_ns": 5_000_001,
            "hard_max_ns": 0,
            "samples": 1024,
            "segments": 2,
        })["resource_failures"])
        self.assertIn("precommit_p99_ns", schema_gate.evaluate_three_sync({
            "precommit_p99_ns": 5_000_001,
            "precommit_hard_max_ns": 0,
            "settlement_p99_ns": 0,
            "settlement_hard_max_ns": 0,
            "episode_p99_ns": 0,
            "episode_hard_max_ns": 0,
            "samples": 256,
        })["resource_failures"])
        self.assertIn("percent_of_one_core", schema_gate.evaluate_idle({
            "elapsed_ticks": 1,
            "ticks_per_second": 100,
            "percent_of_one_core": 0.250001,
        })["resource_failures"])


class AuthorityEnvelopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.schema_path = self.root / "schema.json"
        self.schema_path.write_text(json.dumps(schema_gate.run_preflight()))
        (self.root / "transaction-state.json").write_text(
            json.dumps({"transaction_id": VALID_ATTEMPT, "state": "PREPARED"})
        )
        freeze = {
            "implementation_freeze_root_sha256": "f" * 64,
            "source_commit": "c" * 40,
            "source_tree": "d" * 40,
            "source_bundle_sha256": "e" * 64,
        }
        (self.root / "implementation-freeze.json").write_text(json.dumps(freeze))

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def mechanism(verdict: str, authority_value: bool) -> dict:
        return {
            "schema": "nando.s1c3b-verification.v1",
            "valid": True,
            "authority": authority_value,
            "verdict": verdict,
            "root": "a" * 64,
        }

    def envelope(self, verdict: str, authority_value: bool = True) -> dict:
        with mock.patch.object(authority, "verify_dependencies", return_value={"pinned": "a" * 64}), \
             mock.patch.object(authority, "verify_implementation_freeze", return_value={
                 "implementation_freeze_root_sha256": "f" * 64,
                 "source_commit": "c" * 40,
                 "source_tree": "d" * 40,
                 "source_bundle_sha256": "e" * 64,
             }), \
             mock.patch.object(
                 authority,
                 "mechanism_result",
                 return_value=self.mechanism(verdict, authority_value),
             ):
            return authority.build_envelope(
                self.root,
                self.schema_path,
                predeployment=verdict == "S1C3B_PREPARATION_PASS",
                allow_terminal=verdict != "S1C3B_PREPARATION_PASS",
            )

    def source_bundle(self) -> tuple[Path, str, str]:
        source = self.root / "source"
        source.mkdir()
        subprocess.run(["git", "init", "--quiet"], cwd=source, check=True)
        subprocess.run(["git", "config", "user.email", "s1c3c@test.invalid"], cwd=source, check=True)
        subprocess.run(["git", "config", "user.name", "S1C3C Test"], cwd=source, check=True)
        for name in authority.FROZEN_SOURCE_FILES:
            destination = source / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((ROOT / name).read_bytes())
        subprocess.run(["git", "add", "."], cwd=source, check=True)
        subprocess.run(["git", "commit", "--quiet", "-m", "fixture"], cwd=source, check=True)
        commit = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=source, check=True, capture_output=True, text=True
        ).stdout.strip()
        tree = subprocess.run(
            ["git", "rev-parse", "HEAD^{tree}"], cwd=source, check=True, capture_output=True, text=True
        ).stdout.strip()
        bundle = self.root / "source.bundle"
        subprocess.run(["git", "bundle", "create", str(bundle), "HEAD"], cwd=source, check=True)
        return bundle, commit, tree

    def test_preparation_pass_has_operational_but_no_scientific_authority(self) -> None:
        envelope = self.envelope("S1C3B_PREPARATION_PASS")
        self.assertEqual(envelope["verdict"], "S1C3C_PREPARATION_PASS")
        self.assertTrue(envelope["authority"])
        self.assertFalse(envelope["scientific_authority"])
        self.assertFalse(envelope["capture_installed"])

    def test_deployment_pass_is_capture_only(self) -> None:
        envelope = self.envelope("S1C3B_DEPLOYMENT_PASS")
        self.assertEqual(envelope["verdict"], "S1C3C_DEPLOYMENT_PASS")
        self.assertTrue(envelope["capture_installed"])
        self.assertEqual(envelope["s1c4_state"], "CLOSED")
        self.assertEqual(envelope["s2_state"], "BLOCKED")
        self.assertFalse(envelope["model_training_allowed"])
        self.assertFalse(envelope["phase_mutation_allowed"])

    def test_resource_veto_has_no_mutation_or_authority(self) -> None:
        envelope = self.envelope("S1C3B_RESOURCE_VETO", False)
        self.assertEqual(envelope["verdict"], "S1C3C_RESOURCE_VETO")
        self.assertFalse(envelope["authority"])
        self.assertFalse(envelope["production_mutation"])
        self.assertFalse(envelope["capture_installed"])

    def test_unknown_mechanism_verdict_is_rejected(self) -> None:
        with mock.patch.object(authority, "verify_dependencies", return_value={}), \
             mock.patch.object(authority, "verify_implementation_freeze", return_value={
                 "implementation_freeze_root_sha256": "f" * 64,
                 "source_commit": "c" * 40,
                 "source_tree": "d" * 40,
                 "source_bundle_sha256": "e" * 64,
             }), \
             mock.patch.object(
                 authority,
                 "mechanism_result",
                 return_value=self.mechanism("S1C3B_UNKNOWN", False),
             ):
            with self.assertRaisesRegex(authority.InvalidReceipt, "mechanism_verdict"):
                authority.build_envelope(
                    self.root, self.schema_path, allow_terminal=True
                )

    def test_schema_receipt_is_recomputed_not_trusted(self) -> None:
        value = json.loads(self.schema_path.read_text())
        value["valid"] = False
        self.schema_path.write_text(json.dumps(value))
        with self.assertRaisesRegex(authority.InvalidReceipt, "schema_preflight_receipt"):
            authority.verify_schema_receipt(self.schema_path)

    def test_dependency_hashes_cover_all_mechanism_modules(self) -> None:
        self.assertEqual(
            set(authority.verify_dependencies()),
            {
                "s1c3b_remote_transaction_v1.py",
                "verify_s1c3b_transaction_v1.py",
                "s1c3_remote_transaction_v7.py",
                "verify_s1c3_transaction_v7.py",
            },
        )

    def test_implementation_freeze_binds_commit_tree_bundle_and_files(self) -> None:
        bundle, commit, tree = self.source_bundle()
        implementation = ROOT / "ops/remote-backend"
        freeze = authority.create_implementation_freeze(
            commit, tree, bundle, implementation
        )
        freeze_path = self.root / "freeze.json"
        freeze_path.write_text(json.dumps(freeze))
        self.assertEqual(
            authority.verify_implementation_freeze(
                freeze_path, implementation, bundle
            ),
            freeze,
        )
        self.assertEqual(set(freeze["implementation_files"]), set(authority.IMPLEMENTATION_FILES))
        self.assertEqual(set(freeze["source_files"]), set(authority.FROZEN_SOURCE_FILES))

    def test_implementation_freeze_rejects_bundle_commit_tree_mismatch(self) -> None:
        bundle, commit, _ = self.source_bundle()
        with self.assertRaisesRegex(authority.InvalidReceipt, "source_bundle_tree_mismatch"):
            authority.create_implementation_freeze(
                commit, "d" * 40, bundle, ROOT / "ops/remote-backend"
            )

    def test_implementation_freeze_rejects_file_tamper(self) -> None:
        bundle, commit, tree = self.source_bundle()
        implementation = ROOT / "ops/remote-backend"
        freeze = authority.create_implementation_freeze(
            commit, tree, bundle, implementation
        )
        freeze["implementation_files"]["s1c3c_transaction_v1.py"] = "0" * 64
        freeze["implementation_freeze_root_sha256"] = authority.digest(
            authority.canonical_bytes(freeze, "implementation_freeze_root_sha256")
        )
        freeze_path = self.root / "freeze.json"
        freeze_path.write_text(json.dumps(freeze))
        with self.assertRaisesRegex(authority.InvalidReceipt, "implementation_file_drift"):
            authority.verify_implementation_freeze(freeze_path, implementation, bundle)


class WrapperContractTests(unittest.TestCase):
    def test_attempt_namespace_rejects_old_s1c3b_id(self) -> None:
        with self.assertRaisesRegex(wrapper.mechanism.GateFailure, "attempt_id"):
            wrapper.require_attempt(
                "20260812T093629Z-36ffc0cbf56b-s1c3b-v1"
            )
        wrapper.require_attempt(VALID_ATTEMPT)

    def test_wrapper_requires_both_predeployment_receipts(self) -> None:
        source = (ROOT / "ops/remote-backend/s1c3c_transaction_v1.py").read_text()
        execute = source.split("def execute(", 1)[1].split("def rollback(", 1)[0]
        self.assertIn("mechanism_predeployment_verification", execute)
        self.assertIn("authority_predeployment_envelope", execute)
        self.assertLess(execute.index("verify_predeployment("), execute.index("mechanism.execute("))

    def test_predeployment_abort_is_terminal_without_mutation(self) -> None:
        source = (ROOT / "ops/remote-backend/s1c3c_transaction_v1.py").read_text()
        abort = source.split("def abort_predeployment(", 1)[1].split("def seal(", 1)[0]
        self.assertIn('"S1C3C_PREFLIGHT_FAILURE"', abort)
        self.assertIn('"production_mutation": False', abort)
        self.assertIn('"capture_installed": False', abort)
        self.assertIn('"s1c4_state": "CLOSED"', abort)
        self.assertIn("mechanism.verify_current_production()", abort)

    def test_predeployment_abort_writes_rooted_terminal_receipts(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            schema_path = root / "schema.json"
            schema_path.write_text(json.dumps(schema_gate.run_preflight()))
            freeze = {
                "implementation_freeze_root_sha256": "f" * 64,
                "source_commit": "c" * 40,
                "source_tree": "d" * 40,
                "source_bundle_sha256": "e" * 64,
            }
            (root / "implementation-freeze.json").write_text(json.dumps(freeze))
            (root / "transaction-state.json").write_text(
                json.dumps({"transaction_id": VALID_ATTEMPT, "state": "PREFLIGHT_FAILURE"})
            )
            args = SimpleNamespace(
                transaction_directory=str(root),
                schema_preflight=str(schema_path),
                reason="fixture_failure",
            )
            with mock.patch.object(authority, "verify_dependencies", return_value={}), \
                 mock.patch.object(authority, "verify_implementation_freeze", return_value=freeze), \
                 mock.patch.object(wrapper.mechanism, "verify_current_production"), \
                 mock.patch.object(wrapper.os, "geteuid", return_value=0):
                self.assertEqual(wrapper.abort_predeployment(args), 0)
            receipt = json.loads((root / "s1c3c-preflight-failure.json").read_text())
            state = json.loads((root / "s1c3c-state.json").read_text())
            self.assertEqual(receipt["verdict"], "S1C3C_PREFLIGHT_FAILURE")
            self.assertFalse(receipt["production_mutation"])
            self.assertEqual(state["state"], "PREFLIGHT_FAILURE")
            self.assertEqual(
                json.loads((root / "transaction-state.json").read_text())["state"],
                "PREFLIGHT_FAILURE",
            )
            self.assertEqual(
                state["state_root_sha256"],
                authority.digest(authority.canonical_bytes(state, "state_root_sha256")),
            )

    def test_resource_veto_seal_is_terminal_without_mechanism_complete(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            schema_path = root / "schema.json"
            schema_path.write_text(json.dumps(schema_gate.run_preflight()))
            (root / "transaction-state.json").write_text(
                json.dumps({"transaction_id": VALID_ATTEMPT, "state": "RESOURCE_VETO"})
            )
            mechanism_result = {
                "schema": "nando.s1c3b-verification.v1",
                "valid": True,
                "authority": False,
                "verdict": "S1C3B_RESOURCE_VETO",
                "resource_root_sha256": "a" * 64,
            }
            freeze = {
                "implementation_freeze_root_sha256": "f" * 64,
                "source_commit": "c" * 40,
                "source_tree": "d" * 40,
                "source_bundle_sha256": "e" * 64,
            }
            with mock.patch.object(authority, "verify_dependencies", return_value={}), \
                 mock.patch.object(authority, "verify_implementation_freeze", return_value=freeze), \
                 mock.patch.object(authority, "mechanism_result", return_value=mechanism_result):
                envelope = authority.build_envelope(
                    root, schema_path, allow_terminal=True
                )
                envelope_path = root / "envelope.json"
                envelope_path.write_text(json.dumps(envelope))
                with mock.patch.object(authority.os, "geteuid", return_value=0):
                    state = authority.seal(root, schema_path, envelope_path)
            self.assertEqual(state["verdict"], "S1C3C_RESOURCE_VETO")
            self.assertTrue((root / "s1c3c-authority-envelope.json").is_file())
            self.assertEqual(
                json.loads((root / "s1c3c-authority-envelope.json").read_text())["production_mutation"],
                False,
            )

    def test_success_envelope_is_verified_before_mechanism_seal(self) -> None:
        source = (ROOT / "ops/remote-backend/s1c3c_transaction_v1.py").read_text()
        seal = source.split("def seal(", 1)[1].split("def locked(", 1)[0]
        self.assertLess(seal.index("authority.terminal_state("), seal.index("mechanism.seal("))
        self.assertLess(seal.index("mechanism.seal("), seal.index("authority.atomic_write("))
        self.assertIn("recorded_mechanism_path=Path(args.mechanism_final_verification)", seal)

    def test_old_launcher_is_never_invoked(self) -> None:
        for name in (
            "s1c3c_schema_preflight_v1.py",
            "verify_s1c3c_transaction_v1.py",
            "s1c3c_transaction_v1.py",
        ):
            source = (ROOT / "ops/remote-backend" / name).read_text()
            self.assertNotIn("run_s1c3b_transaction_v1.sh", source)

    def test_modules_compile(self) -> None:
        completed = subprocess.run(
            [
                "python3",
                "-m",
                "py_compile",
                "ops/remote-backend/s1c3c_schema_preflight_v1.py",
                "ops/remote-backend/verify_s1c3c_transaction_v1.py",
                "ops/remote-backend/s1c3c_transaction_v1.py",
            ],
            cwd=ROOT,
            check=False,
        )
        self.assertEqual(completed.returncode, 0)

    def test_launcher_schema_gate_precedes_every_attempt_side_effect(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        schema_gate = source.index("schema_preflight=$(PYTHONPATH=")
        schema_assertion = source.index("<<<\"$schema_preflight\" >/dev/null")
        for marker in (
            "git ls-remote origin",
            "timestamp=$(date",
            "install -d -m 0700 \"$local_dir\"",
            "prior_attempts=$(ssh",
            "connector_snapshot before",
            "git bundle create",
            "ssh \"$remote\"",
            "scp -q",
        ):
            self.assertLess(schema_gate, source.index(marker), marker)
            self.assertLess(schema_assertion, source.index(marker), marker)

    def test_launcher_uploads_receipts_before_remote_verification(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        upload = source.index(
            '"$remote:$remote_upload/mechanism-predeployment.json"'
        )
        remote_verify = source.index(
            "'$remote_upload/mechanism-predeployment.json' --pre-deployment"
        )
        self.assertLess(upload, remote_verify)

    def test_launcher_has_one_attempt_and_no_automatic_successor(self) -> None:
        launcher = ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh"
        source = launcher.read_text()
        self.assertIn("s1c3c_attempt_already_exists", source)
        self.assertIn("-s1c3c-v1", source)
        self.assertNotIn("s1c3d", source.lower())
        self.assertNotIn("retry", source.lower())
        self.assertNotEqual(launcher.stat().st_mode & 0o111, 0)

    def test_launcher_freezes_and_uploads_implementation_before_prepare(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        freeze = source.index("verify_s1c3c_transaction_v1.py freeze")
        upload = source.index('"$local_dir/implementation-freeze.json"')
        prepare = source.index("s1c3c_transaction_v1.py' prepare")
        self.assertLess(freeze, upload)
        self.assertLess(upload, prepare)
        self.assertIn("--implementation-freeze '$remote_upload/implementation-freeze.json'", source)

    def test_launcher_arms_rollback_only_after_dual_predeployment_pass(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        mechanism_compare = source.index(
            'cmp "$local_dir/mechanism-predeployment.local.json"'
        )
        authority_compare = source.index(
            'cmp "$local_dir/s1c3c-predeployment.local.json"'
        )
        rollback_arm = source.index("rollback_armed=true")
        execute = source.index("s1c3c_transaction_v1.py' execute")
        self.assertLess(mechanism_compare, authority_compare)
        self.assertLess(authority_compare, rollback_arm)
        self.assertLess(rollback_arm, execute)

    def test_launcher_closes_prearm_and_postarm_failures(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        prearm = source.split("prearm_failure() {", 1)[1].split(
            "rollback_armed=true", 1
        )[0]
        postarm = source.split("emergency_rollback() {", 1)[1].split(
            "trap emergency_rollback EXIT INT TERM HUP", 1
        )[0]
        self.assertIn("abort-predeployment", prearm)
        self.assertIn("S1C3C_PREFLIGHT_FAILURE", prearm)
        self.assertIn("production_mutation=no", prearm)
        self.assertIn("emergency-mechanism-final.json", postarm)
        self.assertIn("emergency-s1c3c-envelope.json", postarm)
        self.assertIn("s1c3c_transaction_v1.py' seal", postarm)
        self.assertIn("if [[ $state == COMPLETE ]]", postarm)
        self.assertIn("emergency-complete-seal.json", postarm)
        self.assertIn("if [[ $state == PREPARED ]]", postarm)
        self.assertIn("emergency-predeployment-abort.json", postarm)
        prepare_failure = source.split(
            "if [[ $state == PREFLIGHT_FAILURE ]]", 1
        )[1].split("fi", 1)[0]
        self.assertIn("abort-predeployment", prepare_failure)
        self.assertIn("S1C3C_PREFLIGHT_FAILURE", prepare_failure)
        self.assertIn("production_mutation=no", prepare_failure)

    def test_execute_stale_before_mutation_is_terminal(self) -> None:
        source = (ROOT / "ops/remote-backend/run_s1c3c_transaction_v1.sh").read_text()
        execute_result = source.split(
            'state=$(ssh "$remote" "sudo -n jq -r .state', 1
        )[1].split("if [[ $state == ROLLBACK_ARMED ]]", 1)[0]
        self.assertIn("if [[ $state == PREPARED ]]", execute_result)
        self.assertIn("abort-predeployment", execute_result)
        self.assertIn("S1C3C_PREFLIGHT_FAILURE", execute_result)
        self.assertIn("production_mutation=no", execute_result)


if __name__ == "__main__":
    unittest.main()
