from __future__ import annotations

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest


SCRIPT = (
    Path(__file__).resolve().parents[1]
    / "bin"
    / "nando-provider-bridge-upstream-readiness.sh"
)


class _HealthHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path != "/health":
            self.send_error(404)
            return
        body = json.dumps({"ok": True, "upstream_configured": True}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, _format: str, *args: object) -> None:
        del args


class UpstreamReadinessTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), _HealthHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)

    def _run(self, rows: list[dict | str]) -> dict:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            boundary = root / "boundary.jsonl"
            boundary.write_text(
                "\n".join(
                    row if isinstance(row, str) else json.dumps(row) for row in rows
                )
                + "\n",
                encoding="utf-8",
            )
            report = root / "readiness.json"
            env = os.environ.copy()
            env.update(
                {
                    "NANDO_PROVIDER_BRIDGE_BIND": (
                        f"127.0.0.1:{self.server.server_address[1]}"
                    ),
                    "NANDO_PROVIDER_BRIDGE_BOUNDARY_EVENTS_JSONL": str(boundary),
                    "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_REPORT": str(report),
                    "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_ALLOW_REAL_CALL": "0",
                }
            )
            subprocess.run(
                [str(SCRIPT), str(root / "missing.env")],
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            return json.loads(report.read_text(encoding="utf-8"))

    def test_observed_live_2xx_proves_transport_readiness(self) -> None:
        report = self._run(
            [
                "malformed row is ignored",
                {
                    "billing_source": "nando_provider_bridge_observed_upstream_response",
                    "timestamp": "2026-07-10T11:47:20+03:00",
                    "provider": "chatgpt_codex",
                    "path": "/responses",
                    "status_code": 200,
                },
            ]
        )

        self.assertTrue(report["ready_for_broad_provider_traffic"])
        self.assertTrue(report["observed_live_upstream_success"])
        self.assertEqual(report["observed_live_success_count"], 1)
        self.assertEqual(
            report["verdict"],
            "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_PASS_OBSERVED_LIVE_TRAFFIC",
        )

    def test_failed_or_unrelated_rows_do_not_prove_readiness(self) -> None:
        report = self._run(
            [
                {
                    "billing_source": "nando_provider_bridge_observed_upstream_response",
                    "path": "/responses",
                    "status_code": 500,
                },
                {
                    "billing_source": "synthetic_smoke",
                    "path": "/responses",
                    "status_code": 200,
                },
            ]
        )

        self.assertFalse(report["ready_for_broad_provider_traffic"])
        self.assertFalse(report["observed_live_upstream_success"])
        self.assertEqual(
            report["verdict"],
            "NANDO_PROVIDER_BRIDGE_UPSTREAM_READINESS_WATCH_UPSTREAM_CONFIGURED_NOT_PROBED",
        )


if __name__ == "__main__":
    unittest.main()
