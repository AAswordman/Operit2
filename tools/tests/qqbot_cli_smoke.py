#!/usr/bin/env python3
"""Exercise QQBot and the native WebSocket tool through an isolated real CLI daemon."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import uuid
import zipfile


class EchoHandler(BaseHTTPRequestHandler):
    """Accept a local WebSocket handshake and require real UTF-8 text frames."""

    protocol_version = "HTTP/1.1"

    def log_message(self, *_args):
        """Keep the smoke-test output limited to assertion results."""

    def do_GET(self):
        """Echo a client text frame and acknowledge its close frame."""
        assert self.path == "/echo"
        assert self.headers["Upgrade"].lower() == "websocket"
        key = self.headers["Sec-WebSocket-Key"]
        accept = base64.b64encode(hashlib.sha1(
            (key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")
        ).digest()).decode("ascii")
        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()
        self.close_connection = True
        while True:
            head = self.rfile.read(2)
            if head == b"":
                return
            assert len(head) == 2
            opcode = head[0] & 15
            assert head[0] & 128
            assert head[1] & 128
            length = head[1] & 127
            if length == 126:
                length = struct.unpack("!H", self.rfile.read(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self.rfile.read(8))[0]
            assert length < 65536
            mask = self.rfile.read(4)
            encoded = self.rfile.read(length)
            assert len(encoded) == length
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(encoded))
            if opcode == 8:
                self.wfile.write(bytes([0x88, len(payload)]) + payload)
                self.wfile.flush()
                self.server.close_received.set()
                return
            assert opcode == 1, f"Expected a text frame, received opcode {opcode}"
            text = payload.decode("utf-8")
            reply = ("echo:" + text).encode("utf-8")
            assert len(reply) < 126
            self.wfile.write(bytes([0x81, len(reply)]) + reply)
            self.wfile.flush()


def pack_socket_fixture(path: Path):
    """Package a test-only authenticated plugin that calls the public socket SDK."""
    manifest = {
        "schema_version": 1, "toolpkg_id": "com.operit.qqbot_cli_socket_test",
        "version": "1.0.0", "api_version": "2.0.0", "main": "main.js",
        "display_name": "QQBot CLI socket fixture", "description": "Local native socket check",
        "enabled_by_default": True,
        "subpackages": [{"id": "socket_fixture", "entry": "tools.js"}], "resources": [],
    }
    metadata = {
        "name": "socket_fixture", "description": "Local native socket check",
        "tools": [{"name": "roundtrip", "description": "Check native text frames",
                   "parameters": [{"name": "url", "type": "string", "required": True,
                                   "description": "Loopback fixture endpoint"}]}],
        "enabled_by_default": True,
    }
    script = "/* METADATA\n" + json.dumps(metadata) + "\n*/\n" + r'''
/** Reads ordered native events until the expected event arrives. */
async function waitEvent(handle, expected) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const result = await Tools.Net.webSocket({ op: "read", handle });
    if (result.op !== "read") throw new Error("Unexpected read operation");
    for (const event of result.events) {
      if (event.type === "error") throw new Error(event.message);
      if (event.type === expected) return event;
      if (event.type === "closed") throw new Error("Socket closed before expected event");
    }
    await new Promise(resolve => setTimeout(resolve, 20));
  }
  throw new Error("Native socket event deadline exceeded: " + expected);
}
/** Tests open, ordered reads, UTF-8 text send, text receive and explicit release. */
exports.roundtrip = async function roundtrip(params) {
  const opened = await Tools.Net.webSocket({ op: "open", url: params.url, connect_timeout_seconds: 10 });
  if (opened.op !== "open") throw new Error("Unexpected open operation");
  const handle = opened.handle;
  try {
    await waitEvent(handle, "opened");
    await Tools.Net.webSocket({ op: "send", handle, text: "QQBOT_CLI_原生文本" });
    const message = await waitEvent(handle, "message");
    if (message.text !== "echo:QQBOT_CLI_原生文本") throw new Error("Text frame roundtrip mismatch");
    return { success: true, text: message.text };
  } finally {
    await Tools.Net.webSocket({ op: "close", handle });
  }
};
'''
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", json.dumps(manifest))
        archive.writestr("main.js", "/** Registers the local test package. */\nexports.registerToolPkg = function registerToolPkg() { return true; };\n")
        archive.writestr("tools.js", script)


def main():
    """Run real package import, configuration persistence, errors and native socket checks."""
    repo = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--waifu-package", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--profile", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    package = args.package.resolve()
    root = args.profile.resolve() if args.profile is not None else Path(tempfile.mkdtemp(prefix="operit-qqbot-cli-"))
    config = root / "config"
    config.mkdir(parents=True, exist_ok=False)
    identity = str(uuid.uuid4())
    (config / "storage.json").write_text(json.dumps({
        "runtimeRoot": str(root / "runtime"), "workspaceRoot": str(root / "workspace"),
        "activeIdentityId": identity,
        "identities": [{"id": identity, "name": "QQBot CLI regression", "createdAt": 1}],
    }), encoding="utf-8")
    environment = dict(os.environ, OPERIT_CLI_CONFIG_DIR=str(config))
    print("Isolated profile:", root, flush=True)
    calls = []

    def run(*command, expected_success=True, json_output=True):
        """Execute one real command and retain its machine-readable result for review."""
        suffix = ["--json"] if json_output else []
        stdout_path = root / f"command-{len(calls):02d}.stdout"
        stderr_path = root / f"command-{len(calls):02d}.stderr"
        with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
            result = subprocess.run([str(binary), *command, *suffix], env=environment,
                                    stdout=stdout_file, stderr=stderr_file, timeout=180)
        stdout = stdout_path.read_text(encoding="utf-8")
        stderr = stderr_path.read_text(encoding="utf-8")
        calls.append({"command": list(command), "exitCode": result.returncode,
                      "stdout": stdout, "stderr": stderr})
        (root / "results.json").write_text(json.dumps(calls, ensure_ascii=False, indent=2), encoding="utf-8")
        if expected_success:
            assert result.returncode == 0, f"{command}: {stdout}\n{stderr}"
        else:
            assert result.returncode != 0, f"{command}: unexpectedly succeeded"
        print("PASS", " ".join(command[:4]), flush=True)
        return json.loads(stdout) if json_output else None

    def tool(name, params=None, expected_success=True):
        """Unwrap an actual ToolResult and its JSON StringResultData payload."""
        result = run("cli", "tool", "exec", name, json.dumps({} if params is None else params),
                     expected_success=expected_success)
        if not expected_success:
            return result
        assert result["success"], result
        assert result["result"]["__type"] == "StringResultData", result
        return json.loads(result["result"]["value"])

    started = False
    server = None
    try:
        run("cli", "plugin", "import", str(args.waifu_package.resolve()))
        run("cli", "plugin", "enable", "com.operit.waifu")
        run("cli", "plugin", "import", str(package))
        run("cli", "plugin", "enable", "com.operit.qqbot_bundle")
        info = run("cli", "plugin", "show", "com.operit.qqbot_bundle")
        assert info["enabled"] is True
        assert info["counts"]["uiModules"] == 1, info
        assert info["counts"]["appLifecycleHooks"] == 3, info
        catalog = run("cli", "tool", "list", "all")
        assert info["subpackages"][0]["toolCount"] == 18, info
        assert any(item["name"] == "web_socket" for item in catalog["tools"]), catalog
        print("PASS all 18 QQBot tools, Compose settings and lifecycle hooks registered", flush=True)

        fixture = root / "socket_fixture.toolpkg"
        pack_socket_fixture(fixture)
        run("cli", "plugin", "import", str(fixture))
        run("cli", "plugin", "enable", "com.operit.qqbot_cli_socket_test")
        run("daemon", "start", "--no-listen", json_output=False)
        started = True
        status = tool("qqbot:qqbot_status")["status"]
        assert status["configured"] is False
        assert status["service"]["status"] == "stopped"
        assert "appSecret" not in json.dumps(status)
        assert tool("qqbot:qqbot_receive_events")["events"] == []
        assert tool("qqbot:qqbot_clear_events")["clearedCount"] == 0
        assert isinstance(tool("qqbot:usage_advice")["advice"], str)
        missing = tool("qqbot:qqbot_service_start", expected_success=False)
        assert "AppID must not be blank" in json.dumps(missing), missing
        print("PASS missing credentials rejected explicitly", flush=True)

        params = {"app_id": "cli-test-app", "app_secret": "cli-test-secret", "use_sandbox": True,
                  "listener_enabled": False}
        assert tool("qqbot:qqbot_configure", params)["status"]["configured"] is True
        configured = tool("qqbot:qqbot_dashboard_status")["status"]
        assert configured["appId"] == "cli-test-app"
        assert configured["useSandbox"] is True
        assert "cli-test-secret" not in json.dumps(configured)
        auto = tool("qqbot:qqbot_auto_reply_configure", {
            "enabled": False, "start_now": False, "chat_group": "QQBot CLI smoke", "ai_timeout_ms": 30000,
        })["status"]["autoReply"]
        assert auto["config"]["chatGroup"] == "QQBot CLI smoke"
        assert auto["config"]["aiTimeoutMs"] == 30000
        tool("qqbot:qqbot_auto_reply_stop")
        tool("qqbot:qqbot_service_stop")

        server = ThreadingHTTPServer(("127.0.0.1", 0), EchoHandler)
        server.daemon_threads = True
        server.close_received = threading.Event()
        threading.Thread(target=server.serve_forever, daemon=True).start()
        echoed = tool("socket_fixture:roundtrip", {"url": f"ws://127.0.0.1:{server.server_port}/echo"})
        assert echoed == {"success": True, "text": "echo:QQBOT_CLI_原生文本"}
        assert server.close_received.wait(5), "Native Host did not transmit a WebSocket close frame"
        print("PASS real Host WebSocket open/read/text-send/receive/close", flush=True)
        run("daemon", "stop")
        started = False
        run("daemon", "start", "--no-listen", json_output=False)
        started = True
        restored = tool("qqbot:qqbot_status")["status"]
        assert restored["appId"] == "cli-test-app"
        assert restored["useSandbox"] is True
        assert restored["autoReply"]["config"]["chatGroup"] == "QQBot CLI smoke"
        assert restored["service"]["status"] == "stopped"
        tool("qqbot:qqbot_configure", {"app_id": "", "app_secret": "", "use_sandbox": False})
        print("PASS QQBot configuration survives daemon restart", flush=True)
        print("PASS CLI smoke complete; live Tencent authentication and messaging require real credentials", flush=True)
    finally:
        if started:
            run("daemon", "stop")
        if server is not None:
            server.shutdown()
            server.server_close()
        print("Diagnostics:", root / "results.json", flush=True)


if __name__ == "__main__":
    main()
