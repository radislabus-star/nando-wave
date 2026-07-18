import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
from unittest import mock
from email.message import Message
from pathlib import Path


BIN_DIR = Path(__file__).resolve().parents[1] / "bin"
sys.path.insert(0, str(BIN_DIR))
os.environ["NANDO_PHASE_CENTER_ENV"] = "/nonexistent/nando-test.env"

SPEC = importlib.util.spec_from_file_location(
    "nando_provider_bridge",
    BIN_DIR / "nando-provider-bridge.py",
)
BRIDGE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(BRIDGE)


class ProviderBridgeTest(unittest.TestCase):
    def test_stream_relay_preserves_chunks_and_bounds_capture(self) -> None:
        class FakeResponse:
            def __init__(self) -> None:
                self.chunks = [b"event: one\n\n", b"event: two\n\n", b""]

            def read1(self, _size: int) -> bytes:
                return self.chunks.pop(0)

        sink = io.BytesIO()
        captured, total, first_byte_ns = BRIDGE.relay_upstream_stream(
            FakeResponse(), sink.write, capture_limit=10
        )
        self.assertEqual(sink.getvalue(), b"event: one\n\nevent: two\n\n")
        self.assertEqual(captured, b"event: one")
        self.assertEqual(total, len(sink.getvalue()))
        self.assertGreater(first_byte_ns, 0)

    def test_transport_circuit_opens_and_success_resets_it(self) -> None:
        with mock.patch.object(BRIDGE, "TRANSPORT_FAILURE_THRESHOLD", 2), mock.patch.object(
            BRIDGE, "TRANSPORT_FAILURE_WINDOW_S", 60
        ):
            BRIDGE.TRANSPORT_FAILURES.clear()
            BRIDGE.TRANSPORT_LAST_SUCCESS_UNIX = 0
            BRIDGE.TRANSPORT_LAST_ERROR = ""
            BRIDGE.record_transport_result(False, "reset-one")
            self.assertTrue(BRIDGE.transport_status()["transport_ready"])
            BRIDGE.record_transport_result(False, "reset-two")
            self.assertTrue(BRIDGE.transport_status()["transport_circuit_open"])
            BRIDGE.record_transport_result(True)
            self.assertTrue(BRIDGE.transport_status()["transport_ready"])
            self.assertEqual(BRIDGE.transport_status()["transport_recent_failures"], 0)

    def typed_executor_result(self, **overrides: object) -> dict[str, object]:
        after = {"count": 2}
        result: dict[str, object] = {
            "local_accept": True,
            "verifier_ok": True,
            "false_accepts": 0,
            "response": json.dumps({"after": after}),
            "verification_receipt_id": "a" * 64,
            "verified_after_digest": BRIDGE.stable_receipt(after),
            "verifier_schema": "typed_actor_independent_verifier.v1",
        }
        result.update(overrides)
        return result

    def run_typed_executor_result(self, result: dict[str, object]) -> tuple[bool, str]:
        completed = mock.Mock(returncode=0, stdout=json.dumps(result))
        with mock.patch.object(BRIDGE, "local_policy_allows", return_value=True), mock.patch.object(
            BRIDGE.subprocess, "run", return_value=completed
        ):
            accepted, _, reason = BRIDGE.try_typed_transition_executor(
                {"before": {"count": 1}, "action": {"kind": "increment", "amount": 1}}
            )
        return accepted, reason

    def test_typed_executor_requires_actor_verification_receipt(self) -> None:
        accepted, reason = self.run_typed_executor_result(
            self.typed_executor_result(verification_receipt_id=None)
        )
        self.assertFalse(accepted)
        self.assertEqual(reason, "typed_verification_receipt_missing")

    def test_typed_executor_rejects_verified_after_digest_mismatch(self) -> None:
        accepted, reason = self.run_typed_executor_result(
            self.typed_executor_result(verified_after_digest="b" * 64)
        )
        self.assertFalse(accepted)
        self.assertEqual(reason, "typed_verified_after_digest_mismatch")

    def test_typed_executor_accepts_content_bound_actor_receipt(self) -> None:
        accepted, reason = self.run_typed_executor_result(self.typed_executor_result())
        self.assertTrue(accepted)
        self.assertEqual(reason, "typed_verifier_bound_local_accept")

    def test_client_intent_identity_is_stable_only_for_supplied_identity(self) -> None:
        headers = Message()
        headers["Idempotency-Key"] = "intent-42"
        first = BRIDGE.client_intent_identity(headers, {})
        second = BRIDGE.client_intent_identity(headers, {})
        self.assertEqual(first, second)
        self.assertEqual(first[1:], ("idempotency_key", True))

        generated_first = BRIDGE.client_intent_identity(Message(), {})
        generated_second = BRIDGE.client_intent_identity(Message(), {})
        self.assertNotEqual(generated_first[0], generated_second[0])
        self.assertEqual(generated_first[1:], ("bridge_generated", False))

    def test_metadata_client_intent_identity_precedes_headers(self) -> None:
        headers = Message()
        headers["Idempotency-Key"] = "header-intent"
        identity = BRIDGE.client_intent_identity(
            headers,
            {"metadata": {"nando_client_intent_id": "metadata-intent"}},
        )
        expected = BRIDGE.client_intent_identity(
            Message(),
            {"metadata": {"nando_client_intent_id": "metadata-intent"}},
        )
        self.assertEqual(identity, expected)
        self.assertEqual(identity[1:], ("metadata", True))

    def test_codex_turn_and_body_deduplicate_retries_not_continuations(self) -> None:
        headers = Message()
        headers["X-Codex-Turn-Metadata"] = "opaque-turn"
        first = BRIDGE.client_intent_identity(headers, {}, "body-a")
        retry = BRIDGE.client_intent_identity(headers, {}, "body-a")
        continuation = BRIDGE.client_intent_identity(headers, {}, "body-b")
        self.assertEqual(first, retry)
        self.assertNotEqual(first[0], continuation[0])
        self.assertEqual(first[1:], ("codex_turn_body", True))

    def test_economics_row_cannot_claim_unverified_avoid(self) -> None:
        base = {
            "client_intent_id": "intent-test",
            "intent_id_source": "metadata",
            "intent_dedupe_eligible": True,
            "provider_attempt_id": None,
            "request_hash": "request-test",
            "endpoint": "responses",
            "route": "local_actor",
            "terminal_state": "delivered",
            "input_tokens": 100,
            "upstream_socket_opened": False,
            "avoided_call": True,
            "status_code": 200,
        }
        unverified = BRIDGE.economics_terminal_row(
            **base,
            verification_status="verified",
        )
        self.assertFalse(unverified["avoided_call"])

        verified = BRIDGE.economics_terminal_row(
            **base,
            verification_status="verified",
            verification_receipt_id="a" * 64,
            projector_receipt_id="b" * 64,
        )
        self.assertTrue(verified["avoided_call"])

        upstream = BRIDGE.economics_terminal_row(
            **{
                **base,
                "provider_attempt_id": "attempt-test",
                "upstream_socket_opened": True,
            },
            verification_status="verified",
            verification_receipt_id="a" * 64,
            projector_receipt_id="b" * 64,
        )
        self.assertFalse(upstream["avoided_call"])

    def test_provider_attempt_ids_are_unique(self) -> None:
        self.assertNotEqual(
            BRIDGE.provider_attempt_identity(),
            BRIDGE.provider_attempt_identity(),
        )

    def test_extracts_latest_user_message_from_responses_input(self) -> None:
        payload = {
            "input": [
                {
                    "type": "message",
                    "role": "developer",
                    "content": [{"type": "input_text", "text": "rules"}],
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "old"}],
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "nando readiness"}],
                },
            ]
        }
        self.assertEqual(BRIDGE.extract_request_text(payload), "nando readiness")

    def test_extracts_latest_user_message_from_chat(self) -> None:
        payload = {
            "messages": [
                {"role": "user", "content": "old"},
                {"role": "assistant", "content": "reply"},
                {"role": "user", "content": "new"},
            ]
        }
        self.assertEqual(BRIDGE.extract_request_text(payload), "new")

    def test_local_responses_stream_completes(self) -> None:
        response = BRIDGE.responses_response("OK", "test", "route", "v2")
        body = BRIDGE.responses_sse_body(response).decode("utf-8")
        events = [
            json.loads(line[6:])
            for line in body.splitlines()
            if line.startswith("data: {")
        ]
        self.assertEqual(events[-1]["type"], "response.completed")
        self.assertEqual(events[-1]["response"]["status"], "completed")
        self.assertIn("response.output_text.delta", [event["type"] for event in events])

    def test_extracts_structured_transition_envelope_only(self) -> None:
        payload = {
            "metadata": {
                "nando_transition": {
                    "before": {"rows": []},
                    "action": {"kind": "append"},
                    "ignored": "not-runtime-authority",
                }
            }
        }
        self.assertEqual(
            BRIDGE.transition_envelope_from_payload(payload),
            {"before": {"rows": []}, "action": {"kind": "append"}},
        )
        self.assertIsNone(BRIDGE.transition_envelope_from_payload({"metadata": {}}))

    def test_extracts_observed_after_and_provider_usage(self) -> None:
        upstream = json.dumps(
            {
                "choices": [
                    {"message": {"content": json.dumps({"after": {"count": 4}})}}
                ],
                "usage": {"prompt_tokens": 10, "completion_tokens": 3, "total_tokens": 13},
            }
        ).encode()
        after, usage = BRIDGE.observed_after_from_upstream(upstream)
        self.assertEqual(after, {"count": 4})
        self.assertEqual(usage["total_tokens"], 13)

    def test_grounded_evidence_receipt_is_stable_and_content_bound(self) -> None:
        receipt = BRIDGE.grounded_evidence_receipt(
            {"count": 1},
            {"kind": "increment", "amount": 2},
            {"count": 3},
            "application_state",
            "state_store_commit",
        )
        self.assertEqual(len(receipt), 64)
        self.assertEqual(
            receipt,
            BRIDGE.grounded_evidence_receipt(
                {"count": 1},
                {"amount": 2, "kind": "increment"},
                {"count": 3},
                "application_state",
                "state_store_commit",
            ),
        )
        self.assertNotEqual(
            receipt,
            BRIDGE.grounded_evidence_receipt(
                {"count": 1},
                {"kind": "increment", "amount": 2},
                {"count": 4},
                "application_state",
                "state_store_commit",
            ),
        )

    def test_v2_grounded_receipt_binds_time_and_provenance(self) -> None:
        provenance = {
            "source_session_id_sha256": "a" * 64,
            "source_event_id_sha256": "b" * 64,
            "call_input_sha256": "c" * 64,
            "call_output_sha256": "d" * 64,
        }
        base = BRIDGE.grounded_evidence_receipt(
            {"count": 1},
            {"kind": "increment"},
            {"count": 2},
            "tool_result",
            "codex_session_call_output_pair_v1",
            "nando.grounded-transition-receipt.v2",
            "2026-07-11T01:00:00+03:00",
            provenance,
        )
        self.assertNotEqual(
            base,
            BRIDGE.grounded_evidence_receipt(
                {"count": 1},
                {"kind": "increment"},
                {"count": 2},
                "tool_result",
                "codex_session_call_output_pair_v1",
                "nando.grounded-transition-receipt.v2",
                "2026-07-11T01:00:01+03:00",
                provenance,
            ),
        )
        self.assertNotEqual(
            base,
            BRIDGE.grounded_evidence_receipt(
                {"count": 1},
                {"kind": "increment"},
                {"count": 2},
                "tool_result",
                "codex_session_call_output_pair_v1",
                "nando.grounded-transition-receipt.v2",
                "2026-07-11T01:00:00+03:00",
                {**provenance, "call_output_sha256": "e" * 64},
            ),
        )

    def test_transition_append_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "transitions.jsonl"
            with mock.patch.object(BRIDGE, "TRANSITION_TRACE_JSONL", path):
                BRIDGE.TRANSITION_TRACE_IDS = None
                row = {"trace_id": "trace-1", "before": {}, "action": {}, "after": {}}
                self.assertEqual(BRIDGE.append_transition_once(row), "appended")
                self.assertEqual(BRIDGE.append_transition_once(row), "duplicate")
                self.assertEqual(len(path.read_text(encoding="utf-8").splitlines()), 1)
                BRIDGE.TRANSITION_TRACE_IDS = None

    def test_forwards_codex_auth_headers_only(self) -> None:
        headers = {
            "Authorization": "Bearer test",
            "ChatGPT-Account-Id": "account",
            "Originator": "codex_cli_rs",
            "X-Codex-Turn-Metadata": "metadata",
            "Cookie": "do-not-forward",
            "Host": "127.0.0.1:8787",
        }
        forwarded = {key.lower(): value for key, value in BRIDGE.upstream_headers_for_request(headers).items()}
        self.assertEqual(forwarded["authorization"], "Bearer test")
        self.assertEqual(forwarded["chatgpt-account-id"], "account")
        self.assertEqual(forwarded["originator"], "codex_cli_rs")
        self.assertEqual(forwarded["x-codex-turn-metadata"], "metadata")
        self.assertNotIn("cookie", forwarded)
        self.assertNotIn("host", forwarded)


if __name__ == "__main__":
    unittest.main()
