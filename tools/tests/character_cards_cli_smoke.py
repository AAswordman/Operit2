#!/usr/bin/env python3
"""Run the real CLI character plugin in a disposable profile; optionally use a loopback-only mock AI."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import shutil
import uuid
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[2] / "apps/cli/target/debug/operit2")
    parser.add_argument("--mock-ai", action="store_true", help="Verify the real send pipeline against a local HTTP protocol fixture, not a live provider")
    args = parser.parse_args()
    binary = args.binary.resolve()
    root = Path(tempfile.mkdtemp(prefix="operit-character-cli-"))
    print("Isolated profile:", root, flush=True)
    server = None
    passed = False
    try:
        config = root / "config"
        config.mkdir()
        identity = str(uuid.uuid4())
        (config / "storage.json").write_text(json.dumps({
            "runtimeRoot": str(root / "runtime"), "workspaceRoot": str(root / "workspace"),
            "activeIdentityId": identity,
            "identities": [{"id": identity, "name": "Character CLI regression", "createdAt": 1}],
        }), encoding="utf-8")
        environment = dict(os.environ, OPERIT_CLI_CONFIG_DIR=str(config))

        def run(*command):
            # A separate process for every invocation verifies real durable state, not module fixtures.
            result = subprocess.run([str(binary), "cli", *command, "--json"], env=environment,
                                    capture_output=True, text=True, timeout=180)
            if result.returncode:
                raise AssertionError(f"{command}: exit={result.returncode}\n{result.stdout}\n{result.stderr}")
            try:
                value = json.loads(result.stdout)
            except ValueError as error:
                raise AssertionError(f"{command}: invalid JSON stdout\n{result.stdout}\n{result.stderr}") from error
            if isinstance(value, dict) and value.get("ok") is False:
                raise AssertionError(f"{command}: plugin failure {value}")
            print("PASS", " ".join(command[:2]), flush=True)
            return value

        cards = run("character", "list")
        assert any(card["isDefault"] for card in cards)
        card = run("character", "create", "CLI中文回归角色", "你是一名谨慎的测试助手。CLI_ROLE_CARD_SMOKE")
        identifier = card["id"]
        assert run("character", "show", identifier)["characterSetting"] == card["characterSetting"]
        run("character", "update", identifier, "description", "跨进程持久化验证")
        assert run("character", "show", identifier)["description"] == "跨进程持久化验证"
        run("character", "set-active", identifier)
        assert run("active-prompt", "show") == {"CharacterCard": {"id": identifier}}
        combined = run("character", "combine", identifier)
        assert card["characterSetting"] in combined["prompt"]

        run("memory", "character", identifier, "user", "write", "用户偏好：中文回答。")
        assert run("memory", "character", identifier, "user", "show")["content"] == "用户偏好：中文回答。"
        # Writing an empty document must work, not fail generic content validation.
        run("memory", "character", identifier, "user", "write", "")
        assert run("memory", "character", identifier, "user", "show")["content"] == ""
        memory = run("memory", "character", identifier, "item", "create", "测试记忆", "记住回归测试。")
        assert run("memory", "character", identifier, "item", "show", "测试记忆")["item"]["content"] == "记住回归测试。"
        run("memory", "character", identifier, "item", "delete", str(memory["item"]["id"]))

        chat = run("chat", "new")
        record = run("chat", "show", chat["chatId"])["chat"]
        marker = record["pluginExtensions"]["com.operit.character_cards"]
        assert marker["selection"] == "card:" + identifier
        explicit = json.dumps({"com.operit.character_cards": {"version": 1, "selection": "card:default"}})
        other = run("chat", "new", "--input", explicit)
        assert run("chat", "show", other["chatId"])["chat"]["pluginExtensions"]["com.operit.character_cards"]["selection"] == "card:default"
        copied = run("chat", "new", "--source", chat["chatId"])
        assert run("chat", "show", copied["chatId"])["chat"]["pluginExtensions"]["com.operit.character_cards"]["selection"] == "card:" + identifier
        if args.mock_ai:
            requests = []
            reply = "本地协议夹具回复：角色提示已接收。"

            class Handler(BaseHTTPRequestHandler):
                """Supplies only deterministic local protocol responses, never a business-service result."""
                def log_message(self, *_args):
                    pass

                def do_POST(self):
                    request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                    requests.append(request)
                    (root / "mock-requests.json").write_text(json.dumps(requests, ensure_ascii=False, indent=2), encoding="utf-8")
                    if request.get("stream"):
                        chunks = [
                            {"id": "local-fixture", "object": "chat.completion.chunk", "model": request["model"], "choices": [
                                {"index": 0, "delta": {"role": "assistant", "content": reply}, "finish_reason": None}]},
                            {"id": "local-fixture", "object": "chat.completion.chunk", "model": request["model"], "choices": [
                                {"index": 0, "delta": {}, "finish_reason": "stop"}]},
                        ]
                        payload = "".join("data: " + json.dumps(chunk, ensure_ascii=False) + "\n\n" for chunk in chunks) + "data: [DONE]\n\n"
                        content_type = "text/event-stream"
                    else:
                        payload = json.dumps({"id": "local-fixture", "object": "chat.completion", "model": request["model"], "choices": [
                            {"index": 0, "message": {"role": "assistant", "content": reply}, "finish_reason": "stop"}],
                            "usage": {"prompt_tokens": 12, "completion_tokens": 8, "total_tokens": 20}}, ensure_ascii=False)
                        content_type = "application/json"
                    body = payload.encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", content_type)
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)

            server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
            threading.Thread(target=server.serve_forever, daemon=True).start()
            endpoint = f"http://127.0.0.1:{server.server_port}/v1/chat/completions"
            provider = run("model", "provider-create", "Local protocol fixture", "OPENAI_GENERIC", endpoint)["providerId"]
            run("model", "provider-set-key", provider, "fixture-not-a-real-key")
            run("model", "provider-model-create", provider, "fixture-chat")
            # Bind every functional model, including title generation and memory jobs, to loopback.
            for function in run("model", "function-list")["bindings"]:
                run("model", "function-set", function["functionType"], provider, "fixture-chat")
            receipt = run("chat", "send", "--chat", chat["chatId"], "请验证角色提示。")
            assert receipt["outcome"]["type"] == "committed" and receipt["outcome"]["status"] == "completed", receipt
            assert receipt["aiResponse"] == reply, receipt
            assert any("CLI_ROLE_CARD_SMOKE" in json.dumps(request, ensure_ascii=False) for request in requests), requests
            history = run("chat", "show", chat["chatId"])
            assert any(message["sender"] == "ai" and any(reply in part["content"] for part in message["parts"]) for message in history["messages"]), history
            print("PASS actual plugin role prompt -> loopback HTTP/SSE -> committed receipt and persisted AI message; NOT a live provider test", flush=True)

        for created in [chat, other, copied]:
            run("chat", "delete", created["chatId"])
        run("character", "set-active", "default")
        assert run("character", "delete", identifier)["deleted"]
        assert all(card["id"] != identifier for card in run("character", "list"))
        print("PASS real CLI character/memory persistence and generic chat lifecycle; no live AI call", flush=True)
    except Exception:
        print("FAIL diagnostics retained at", root, flush=True)
        raise
    else:
        passed = True
    finally:
        if server is not None:
            server.shutdown()
            server.server_close()
        if passed:
            shutil.rmtree(root)


if __name__ == "__main__":
    main()
