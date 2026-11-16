#!/usr/bin/env python3
"""Exercise the real group plugin, native continuation sends and persistence with loopback AI."""
import argparse
import json
from pathlib import Path
import queue
import sqlite3
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from subagents_goal_cli_smoke import CliSession


class GroupModelFixture:
    """Supply only model protocol responses; the production plugin owns the whole group sequence."""

    def __init__(self, root, rounds):
        """Record requests and fail explicitly when a role sees incorrect prior conversation context."""
        self.root = root
        self.rounds = rounds
        self.members = []
        self.requests = []
        self.replies = []
        self.failures = queue.Queue()
        self.lock = threading.Lock()
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            """Serve real OpenAI completion and streaming protocols on a local HTTP socket."""

            def log_message(self, *_args):
                """Keep HTTP noise out of the CLI transcript."""
                pass

            def do_POST(self):
                """Stream a sizeable reply in multiple chunks through the real provider transport."""
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                try:
                    with fixture.lock:
                        fixture.requests.append(request)
                        text = fixture.respond(request)
                        (root / "model-requests.json").write_text(
                            json.dumps(fixture.requests, ensure_ascii=False, indent=2), encoding="utf-8")
                    if request.get("stream"):
                        chunks = []
                        for start in range(0, len(text), 96):
                            chunks.append({"id": "group-fixture", "object": "chat.completion.chunk",
                                           "model": request["model"], "choices": [{"index": 0,
                                           "delta": {"content": text[start:start + 96]}, "finish_reason": None}]})
                        chunks.append({"id": "group-fixture", "object": "chat.completion.chunk",
                                       "model": request["model"], "choices": [{"index": 0,
                                       "delta": {}, "finish_reason": "stop"}]})
                        body = ("".join("data: " + json.dumps(chunk, ensure_ascii=False) + "\n\n"
                                        for chunk in chunks) + "data: [DONE]\n\n").encode("utf-8")
                        content_type = "text/event-stream"
                    else:
                        body = json.dumps({"id": "group-fixture", "object": "chat.completion",
                                           "model": request["model"], "choices": [{"index": 0,
                                           "message": {"role": "assistant", "content": text}, "finish_reason": "stop"}],
                                           "usage": {"prompt_tokens": 100, "completion_tokens": 100,
                                                     "total_tokens": 200}}, ensure_ascii=False).encode("utf-8")
                        content_type = "application/json"
                    self.send_response(200)
                    self.send_header("Content-Type", content_type)
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                except Exception as error:
                    fixture.failures.put(error)
                    self.send_error(500, "Group fixture assertion failed")

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def respond(self, request):
        """Plan A/B/A/B and require each subsequent native send to include the preceding real reply."""
        model = request["model"]
        if model == "fixture-planner":
            assert len(self.members) == 2
            return json.dumps({"rounds": [[{"id": member, "speak": True} for member in self.members]
                                         for _ in range(self.rounds)]})
        if model in ("fixture-role-a", "fixture-role-b"):
            index = len(self.replies)
            assert model == ("fixture-role-a" if index % 2 == 0 else "fixture-role-b"), (model, index)
            context = json.dumps(request["messages"], ensure_ascii=False)
            assert "CLI_GROUP_INPUT" in context
            if index:
                assert f"CLI_GROUP_REPLY_{index - 1}_END" in context, "Next role lost the prior reply"
            text = f"CLI_GROUP_REPLY_{index}_START\n" + ("群聊连续回复与流式持久化验证。" * 240)
            text += f"\nCLI_GROUP_REPLY_{index}_END"
            self.replies.append(text)
            return text
        assert model == "fixture-auxiliary", model
        return "CLI group regression"

    def close(self):
        """Stop the isolated protocol server after the CLI owner exits."""
        self.server.shutdown()
        self.server.server_close()


def main():
    """Run multiple submissions in one actual CLI process and preserve all verification artifacts."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[2] / "apps/cli/target/debug/operit2.exe")
    parser.add_argument("--submissions", type=int, default=3)
    parser.add_argument("--rounds", type=int, choices=range(1, 6), default=2)
    parser.add_argument("--diff-lines", type=int, default=25000)
    args = parser.parse_args()
    assert args.submissions > 0
    root = Path(tempfile.mkdtemp(prefix="operit-group-cli-"))
    print("Artifacts:", root, flush=True)
    fixture = GroupModelFixture(root, args.rounds)
    cli = None
    try:
        cli = CliSession(args.binary.resolve(), root, fixture.failures)
        # Exercise the actual crashing write_file -> generateUnifiedDiff path before group execution.
        old = "".join(f"line {index}\n" for index in range(args.diff_lines))
        for content in (old, old.replace(f"line {args.diff_lines // 2}\n", "CLI_DIFF_CHANGED\n")):
            written = cli.core("tool", "exec", "write_file", json.dumps({
                "path": "/operitapp/data/cli-group-diff.txt", "content": content}))
            assert written["success"], written
        assert "+CLI_DIFF_CHANGED" in json.dumps(written, ensure_ascii=False), written
        print(f"PASS real write_file exact diff for {args.diff_lines} lines (former quadratic allocation)", flush=True)
        endpoint = f"http://127.0.0.1:{fixture.server.server_port}/v1/chat/completions"
        provider = cli.core("model", "provider-create", "Group loopback fixture", "OPENAI_GENERIC", endpoint)["providerId"]
        cli.core("model", "provider-set-key", provider, "fixture-not-a-real-key")
        for model in ("fixture-planner", "fixture-role-a", "fixture-role-b", "fixture-auxiliary"):
            cli.core("model", "provider-model-create", provider, model)
        for binding in cli.core("model", "function-list")["bindings"]:
            model = "fixture-planner" if binding["functionType"] == "ROLE_RESPONSE_PLANNER" else "fixture-auxiliary"
            cli.core("model", "function-set", binding["functionType"], provider, model)
        for name, model in (("群聊角色甲", "fixture-role-a"), ("群聊角色乙", "fixture-role-b")):
            card = cli.core("character", "create", "--record", json.dumps({
                "name": name, "characterSetting": "你参与 CLI 群聊回归测试。",
                "chatModelBindingMode": "FIXED_MODEL", "chatProviderId": provider, "chatModelId": model,
                "toolAccessConfig": {"enabled": True, "allowedBuiltinTools": [], "allowedPackages": [],
                                     "allowedSkills": [], "allowedMcpServers": []}}, ensure_ascii=False))
            fixture.members.append(card["id"])
        group = cli.core("group", "create", "--record", json.dumps({"name": "CLI 群聊崩溃回归",
                         "members": [{"characterCardId": member, "orderIndex": index}
                                     for index, member in enumerate(fixture.members)]}, ensure_ascii=False))
        chat = cli.core("chat", "new", "--input", json.dumps({"com.operit.character_cards": {
            "version": 1, "selection": "group:" + group["id"]}}))["chatId"]
        assert cli.core("chat", "show", chat)["messages"] == []
        for submission in range(args.submissions):
            receipt = cli.core("chat", "send", "--chat", chat, f"CLI_GROUP_INPUT_{submission}：请两位角色依次讨论。")
            assert receipt["outcome"]["type"] == "consumed", receipt
            submission_id = receipt["outcome"]["metadata"]["submissionId"]
            identity = json.loads((root / "config/storage.json").read_text(encoding="utf-8"))["activeIdentityId"]
            log_path = root / "runtime/identities" / identity / "logs/operit.log"
            completed_log = f"[character_cards] 群聊结束（completed） chat={chat[:8]} task={submission_id}"
            expected = (submission + 1) * args.rounds * 2
            deadline = time.monotonic() + 180
            while True:
                history = cli.core("chat", "show", chat)
                assistants = [message for message in history["messages"] if message["sender"] == "ai"]
                log = log_path.read_text(encoding="utf-8")
                assert f"[character_cards] 群聊失败 chat={chat[:8]} task={submission_id}:" not in log, log_path
                if (assistants and all(message["completedAt"] > 0 for message in assistants)
                        and completed_log in log):
                    break
                assert time.monotonic() < deadline, f"Group did not finish; artifacts: {root}"
                time.sleep(0.1)
            verify_persisted_pages(cli, root, identity, chat, fixture.members, submission + 1, expected)
            print(f"PASS submission {submission + 1}: one user input, {expected} complete ordered role replies", flush=True)
        assert len(fixture.replies) == args.submissions * args.rounds * 2
        print("PASS real CLI group takeover, planner, per-role overrides, multi-round continuation and persistence; loopback AI", flush=True)
    finally:
        if cli is not None:
            cli.close()
        fixture.close()


def verify_persisted_pages(cli, root, identity, chat, members, expected_users, expected_replies):
    """Read CLI content in bounded pages and audit only persistence metadata through a read-only connection."""
    database = root / "runtime/identities" / identity / "data/database/operit2.sqlite"
    users = replies = start = 0
    with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True) as connection:
        connection.row_factory = sqlite3.Row
        while True:
            rows = connection.execute(
                "SELECT timestamp, sender, completedAt, pluginExtensions FROM messages "
                "WHERE chatId = ? ORDER BY timestamp LIMIT 20 OFFSET ?", (chat, start)).fetchall()
            if not rows:
                break
            page = cli.core("tool", "exec", "get_chat_messages_range", json.dumps({
                "chat_id": chat, "order": "asc", "start": start, "end": start + len(rows) - 1}))
            assert page["success"], page
            messages = page["result"]["messages"]
            visible_rows = [row for row in rows if row["sender"] != "summary"]
            assert [message["timestamp"] for message in messages] == [row["timestamp"] for row in visible_rows]
            for message, metadata in zip(messages, visible_rows):
                if message["sender"] == "user":
                    users += 1
                elif message["sender"] == "ai":
                    assert f"CLI_GROUP_REPLY_{replies}_END" in message["content"], message
                    assert message["modelName"] == ("fixture-role-a" if replies % 2 == 0 else "fixture-role-b")
                    assert metadata["completedAt"] > 0, dict(metadata)
                    author = json.loads(metadata["pluginExtensions"])["com.operit.character_cards"]["participantId"]
                    assert author == members[replies % 2], dict(metadata)
                    replies += 1
            (root / f"history-page-{start:06d}.json").write_text(json.dumps({
                "messages": messages, "metadata": [dict(row) for row in rows]}, ensure_ascii=False, indent=2), encoding="utf-8")
            start += len(rows)
    assert users == expected_users, "Group persisted its original input more than once"
    assert replies == expected_replies, (replies, expected_replies)


if __name__ == "__main__":
    main()
