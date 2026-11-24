"""Exercise CLA API requests over HTTP without contacting GitHub."""

import importlib.util
import json
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from threading import Thread
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "cla_check", Path(__file__).resolve().parents[1] / "cla-check.py"
)
cla_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cla_check)


class ApiRequestTests(unittest.TestCase):
    def test_comment_and_status_payloads_reach_server(self):
        received = []

        class Handler(BaseHTTPRequestHandler):
            def handle_api(self):
                raw = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                received.append(
                    (self.command, self.path, json.loads(raw) if raw else None,
                     self.headers.get("Content-Type"), self.headers.get("Authorization"))
                )
                response = b'{"ok": true}'
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(response)))
                self.end_headers()
                self.wfile.write(response)

            do_GET = do_POST = do_PATCH = handle_api

            def log_message(self, *_args):
                pass

        cases = [
            ("POST", "/repos/owner/repo/issues/1/comments", {"body": "CLA 已签署"}),
            ("PATCH", "/repos/owner/repo/issues/comments/2", {"body": "更新签署状态"}),
            ("POST", "/repos/owner/repo/statuses/abc", {"state": "success", "context": "CLA"}),
            ("GET", "/repos/owner/repo/issues", None),
        ]
        with HTTPServer(("127.0.0.1", 0), Handler) as server:
            worker = Thread(target=server.serve_forever, daemon=True)
            worker.start()
            try:
                with patch.object(cla_check, "API", f"http://127.0.0.1:{server.server_port}"), \
                     patch.object(cla_check, "TOKEN", "test-token"):
                    for method, path, payload in cases:
                        with self.subTest(method=method, path=path):
                            result = cla_check.api(method, path, data=payload, params={"page": 1})
                            self.assertEqual(result, {"ok": True})
                            actual_method, actual_path, body, content_type, auth = received[-1]
                            self.assertEqual(actual_method, method)
                            self.assertEqual(actual_path, path + "?page=1")
                            self.assertEqual(body, payload)
                            self.assertEqual(auth, "Bearer test-token")
                            if payload is not None:
                                self.assertEqual(content_type, "application/json")
            finally:
                server.shutdown()
                worker.join(timeout=5)


if __name__ == "__main__":
    unittest.main()
