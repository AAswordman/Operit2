#!/usr/bin/env python3
"""Exercise external subagents and Goal Mode through a real, isolated CLI and loopback model."""
import argparse
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import tempfile
import threading
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from xml.sax.saxutils import escape
import xml.etree.ElementTree as ET


class CliSession:
    """Keep one real Core alive so child observations survive individual commands."""

    def __init__(self, binary, root, failures):
        """Start an isolated CLI owner and retain every command and response for diagnosis."""
        self.root = root
        self.failures = failures
        self.failed = False
        config = root / "config"
        config.mkdir()
        identity = str(uuid.uuid4())
        (config / "storage.json").write_text(json.dumps({
            "runtimeRoot": str(root / "runtime"), "workspaceRoot": str(root / "workspace"),
            "activeIdentityId": identity,
            "identities": [{"id": identity, "name": "Subagents Goal CLI test", "createdAt": 1}],
        }), encoding="utf-8")
        self.transcript = (root / "cli.jsonl").open("w", encoding="utf-8")
        self.stderr = (root / "cli.stderr").open("w", encoding="utf-8")
        self.lines = queue.Queue()
        self.process = subprocess.Popen([
            str(binary), "cli", "--json", "link", "session", "tcp", "--bind", "127.0.0.1:0",
        ], env=dict(os.environ, OPERIT_CLI_CONFIG_DIR=str(config), OPERIT_CLI_SECRET_STORE="file"),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
            text=True, encoding="utf-8")
        threading.Thread(target=self.read_stdout, daemon=True).start()
        assert self.receive()["listening"] is True

    def read_stdout(self):
        """Drain stdout without platform-specific pipe polling or shell quoting."""
        for line in self.process.stdout:
            self.lines.put(line)
        self.lines.put(None)

    def receive(self, timeout=100):
        """Require one JSON result and preserve unexpected output as a real failure."""
        deadline = time.monotonic() + timeout
        while True:
            if not self.failures.empty():
                self.failed = True
                raise AssertionError("Model fixture assertion failed") from self.failures.get_nowait()
            try:
                line = self.lines.get(timeout=0.1)
                break
            except queue.Empty:
                if time.monotonic() >= deadline:
                    self.failed = True
                    raise TimeoutError("CLI command timed out; see " + str(self.root))
        assert line is not None, f"CLI exited: {self.process.poll()}; see {self.root}"
        self.transcript.write(line)
        self.transcript.flush()
        value = json.loads(line)
        assert not (isinstance(value, dict) and "error" in value), value
        return value

    def command(self, *args):
        """Execute a JSON argument array through the project's public CLI session."""
        self.transcript.write(json.dumps({"command": args}, ensure_ascii=False) + "\n")
        self.process.stdin.write(json.dumps(args, ensure_ascii=False) + "\n")
        self.process.stdin.flush()
        return self.receive()

    def core(self, *args):
        """Run an ordinary CLI core command in the same owning instance."""
        return self.command("core", *args)

    def close(self):
        """Stop the isolated owner and release the diagnostic files."""
        try:
            if self.process.poll() is None and not self.failed:
                self.command("quit")
                self.process.wait(timeout=15)
        finally:
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=10)
            self.stderr.close()
            self.transcript.close()


class ModelFixture:
    """Supply protocol data only; every plugin operation executes in the real runtime."""

    def __init__(self, root):
        """Record all model requests and explicitly control each parent tool response."""
        self.root = root
        self.requests = []
        self.failures = queue.Queue()
        self.lock = threading.Lock()
        self.steps = []
        self.parent_index = 0
        self.reopened_steps = []
        self.reopened_index = 0
        self.child_release = threading.Event()
        self.child_started = threading.Event()
        self.slow_release = threading.Event()
        self.slow_started = {marker: threading.Event() for marker in ("CLI_CHILD_SLOW", "CLI_CHILD_SLOW_CLOSE")}
        self.child_number = 0
        self.review_sent = threading.Event()
        self.child_inputs = []
        fixture = self

        class Handler(BaseHTTPRequestHandler):
            """Serve OpenAI HTTP and SSE frames over loopback."""

            def log_message(self, *_args):
                """Keep protocol logs in the explicit JSON artifact."""
                pass

            def do_POST(self):
                """Return the selected fixture text through the real provider transport."""
                request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                with fixture.lock:
                    fixture.requests.append(request)
                    (root / "model-requests.json").write_text(json.dumps(fixture.requests, ensure_ascii=False, indent=2), encoding="utf-8")
                try:
                    text, hold = fixture.respond(request)
                except Exception as error:
                    fixture.failures.put(error)
                    self.send_error(500, "Model fixture assertion failed")
                    return
                if request.get("stream"):
                    self.send_response(200)
                    self.send_header("Content-Type", "text/event-stream")
                    self.end_headers()
                    if hold:
                        fixture.child_started.set()
                        if hold == "CLI_CHILD_INITIAL":
                            fixture.child_release.wait(timeout=80)
                        else:
                            fixture.slow_started[hold].set()
                            fixture.slow_release.wait(timeout=80)
                    chunks = [
                        {"id": "cli-fixture", "object": "chat.completion.chunk", "model": request["model"], "choices": [
                            {"index": 0, "delta": {"role": "assistant", "content": text}, "finish_reason": None}]},
                        {"id": "cli-fixture", "object": "chat.completion.chunk", "model": request["model"], "choices": [
                            {"index": 0, "delta": {}, "finish_reason": "stop"}]},
                    ]
                    body = ("".join("data: " + json.dumps(chunk, ensure_ascii=False) + "\n\n" for chunk in chunks) + "data: [DONE]\n\n").encode("utf-8")
                else:
                    body = json.dumps({"id": "cli-fixture", "object": "chat.completion", "model": request["model"], "choices": [
                        {"index": 0, "message": {"role": "assistant", "content": text}, "finish_reason": "stop"}],
                        "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}}, ensure_ascii=False).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                try:
                    self.wfile.write(body)
                    self.wfile.flush()
                    if text.startswith('{"decision":'):
                        fixture.review_sent.set()
                except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
                    # Cancellation intentionally closes this fixture's outstanding stream.
                    pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def respond(self, request):
        """Separate exact fixture conversation markers and independent goal reviews."""
        messages = request["messages"]
        if any(message["role"] == "system" and message["content"] == "You are a strict task-completion reviewer. Do not call tools." for message in messages):
            return json.dumps({"decision": "completed", "reason": "Fixture work is complete.", "missing": ""}), False
        users = [message["content"] for message in messages if message["role"] == "user"]
        if "CLI_REOPEN_START" in users:
            assert self.reopened_index < len(self.reopened_steps), (self.reopened_index, messages)
            step = self.reopened_steps[self.reopened_index]
            self.reopened_index += 1
            return step(request) if callable(step) else step, False
        if any(isinstance(content, str) and content.startswith("CLI_CHILD_") for content in users):
            self.child_number += 1
            latest = users[-1]
            self.child_inputs.append(latest)
            return "CLI_CHILD_RESULT_" + str(self.child_number), latest if latest in ("CLI_CHILD_INITIAL", "CLI_CHILD_SLOW", "CLI_CHILD_SLOW_CLOSE") else None
        if any(isinstance(content, str) and content.startswith("CLI_PARENT_") for content in users):
            assert self.parent_index < len(self.steps), (self.parent_index, messages)
            step = self.steps[self.parent_index]
            self.parent_index += 1
            return step(request) if callable(step) else step, False
        return "CLI_AUXILIARY_REPLY", False

    def close(self):
        """Release blocked fixture streams and stop its loopback listener."""
        self.child_release.set()
        self.slow_release.set()
        self.server.shutdown()
        self.server.server_close()


# Encode authored tool calls using the runtime's XML protocol.
def tool(tool_name, **params):
    """Build an actual model tool request without substituting its execution result."""
    return '\n<tool name="' + tool_name + '">\n' + "\n".join(
        '<param name="' + key + '">' + escape(value if isinstance(value, str) else json.dumps(value)) + '</param>'
        for key, value in params.items()) + "\n</tool>"


# Read the native tool protocol returned to the model, including its concrete randomized tag.
def last_result(request):
    """Parse the last real tool result and reject execution errors before scheduling another step."""
    content = request["messages"][-1]["content"]
    element = ET.fromstring(content)
    assert re.fullmatch(r"tool_result_[A-Za-z0-9]+", element.tag), content
    assert element.attrib["status"] == "success", content
    return json.loads(element.find("content").text)


# Drive the real plugin, chat and model interfaces together without real provider credentials.
def main():
    """Verify external installation, real model delegation, Goal Mode and durable native history."""
    repo = Path(__file__).resolve().parents[2]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--scenario", choices=("wait", "lifecycle"), default="lifecycle", help="Select the explicit integration scenario; lifecycle includes follow-up, interruption and closure")
    args = parser.parse_args()
    root = Path(tempfile.mkdtemp(prefix="operit-subagents-goal-cli-"))
    print("Diagnostics:", root, flush=True)
    cli = None
    fixture = ModelFixture(root)
    try:
        cli = CliSession(args.binary.resolve(), root, fixture.failures)
        package = repo / "plugins/packages/external/subagents/dist/subagents.toolpkg"
        assert cli.core("plugin", "import", str(package))["result"]["packageName"] == "com.operit.subagents"
        cli.core("plugin", "enable", "com.operit.subagents")
        plugin = cli.core("plugin", "show", "com.operit.subagents")
        assert plugin["enabled"] and plugin["sourceType"] == "EXTERNAL", plugin
        assert plugin["subpackages"][0]["toolCount"] == 5
        cli.core("plugin", "enable", "com.operit.goal_mode_bundle")
        runtimes = cli.command("call", "application.packageManager", "getEnabledToolPkgContainerRuntimes", "{}")
        slots = {runtime["packageName"]: runtime["chatComposerSlots"] for runtime in runtimes
                 if runtime["packageName"] in ("com.operit.subagents", "com.operit.goal_mode_bundle")}
        assert len(slots) == 2 and all(len(value) == 1 and value[0]["slot"] == "above_input" for value in slots.values()), slots
        assert slots["com.operit.goal_mode_bundle"][0]["order"] < slots["com.operit.subagents"][0]["order"]
        (root / "composer-slots.json").write_text(json.dumps(slots, indent=2), encoding="utf-8")
        cli.core("package", "use", "subagents")
        cli.core("approval", "full")
        endpoint = f"http://127.0.0.1:{fixture.server.server_port}/v1/chat/completions"
        provider = cli.core("model", "provider-create", "Loopback CLI fixture", "OPENAI_GENERIC", endpoint)["providerId"]
        cli.core("model", "provider-set-key", provider, "fixture-not-a-real-key")
        cli.core("model", "provider-model-create", provider, "fixture-chat")
        for binding in cli.core("model", "function-list")["bindings"]:
            cli.core("model", "function-set", binding["functionType"], provider, "fixture-chat")
        parent = cli.core("chat", "new")["chatId"]
        objective = "CLI_GOAL_DELEGATE_AND_VERIFY"
        assert cli.core("goal", objective)["goal"]["status"] == "active"
        assert cli.core("goal", "pause")["goal"]["status"] == "paused"
        assert cli.core("goal", "resume")["goal"]["status"] == "active"
        observed = {}

        def first_wait(request):
            """Capture the host-created identity and start the default 30-second wait."""
            observed["spawn"] = last_result(request)
            print("PASS actual model spawn and authenticated parent identity", flush=True)
            observed["wait_started"] = time.monotonic()
            return tool("subagents:wait_agent", agent_ids=[observed["spawn"]["id"]])

        def second_wait(request):
            """Require an actual timeout, then release the same child and wait again."""
            result = last_result(request)
            observed["timeout"] = result
            assert result["timed_out"] is True, result
            assert 29 <= time.monotonic() - observed["wait_started"] < 40
            print("PASS real 30-second timeout; task remains active", flush=True)
            assert fixture.child_started.is_set()
            fixture.child_release.set()
            return tool("subagents:wait_agent", agent_ids=[observed["spawn"]["id"]])

        def finish(request):
            """Require completion of the original turn without an additional child request."""
            observed["completion"] = last_result(request)
            assert observed["completion"]["timed_out"] is False, observed
            assert observed["completion"]["agents"][0]["status"] == "completed", observed
            print("PASS re-wait completed the original child turn", flush=True)
            if args.scenario == "wait":
                return "CLI_PARENT_FINISHED"
            return tool("subagents:send_input", agent_id=observed["spawn"]["id"], message="CLI_CHILD_FOLLOWUP")

        def wait_followup(request):
            """Observe the same child chat's second originating turn."""
            result = last_result(request)
            assert result["runNumber"] == 2 and result["id"] == observed["spawn"]["id"], result
            return tool("subagents:wait_agent", agent_ids=[result["id"]])

        def start_slow(request):
            """Start a third turn that stays live until real cancellation."""
            result = last_result(request)
            assert not result["timed_out"] and result["agents"][0]["status"] == "completed", result
            return tool("subagents:send_input", agent_id=observed["spawn"]["id"], message="CLI_CHILD_SLOW")

        def observe_slow(request):
            """Give native streaming admission time before interrupting the running turn."""
            result = last_result(request)
            assert result["runNumber"] == 3, result
            return tool("subagents:wait_agent", agent_ids=[result["id"]], timeout_ms=1000)

        def interrupt(request):
            """Require a live submitted stream before explicit interruption, even before its first part."""
            result = last_result(request)
            assert result["timed_out"] and result["agents"][0]["status"] in ("starting", "running"), result
            assert fixture.slow_started["CLI_CHILD_SLOW"].wait(timeout=10), "The third native model request never started"
            return tool("subagents:send_input", agent_id=observed["spawn"]["id"], message="CLI_CHILD_REPLACEMENT", interrupt=True)

        def wait_replacement(request):
            """Verify a replacement starts as a new run in the original native chat."""
            result = last_result(request)
            assert result["runNumber"] == 4 and result["id"] == observed["spawn"]["id"], result
            return tool("subagents:wait_agent", agent_ids=[result["id"]])

        def list_completed(request):
            """Require native completion before inspecting the parent-owned agent list."""
            result = last_result(request)
            observed["replacement"] = result
            assert not result["timed_out"] and result["agents"][0]["status"] == "completed", result
            return tool("subagents:list_agents")

        def close_completed(request):
            """Close the completed first child through the actual model tool."""
            result = last_result(request)
            assert len(result["agents"]) == 1 and result["agents"][0]["runNumber"] == 4, result
            return tool("subagents:close_agent", agent_id=observed["spawn"]["id"])

        def spawn_closing(request):
            """Preserve closed history while starting another child for active closure."""
            assert last_result(request)["status"] == "closed"
            return tool("subagents:spawn_agent", name="CLI close active", task="CLI_CHILD_SLOW_CLOSE")

        def wait_closing(request):
            """Capture the second child identity and observe its active native stream."""
            observed["closing"] = last_result(request)
            return tool("subagents:wait_agent", agent_ids=[observed["closing"]["id"]], timeout_ms=1000)

        def close_running(request):
            """Require active state and request real cancellation followed by closure."""
            result = last_result(request)
            assert result["timed_out"] and result["agents"][0]["status"] in ("starting", "running"), result
            assert fixture.slow_started["CLI_CHILD_SLOW_CLOSE"].wait(timeout=10), "The second child native model request never started"
            return tool("subagents:close_agent", agent_id=observed["closing"]["id"])

        def final_list(request):
            """Require a native cancelled receipt when closing a running child."""
            result = last_result(request)
            assert result["status"] == "closed" and result["result"]["outcome"]["status"] == "cancelled", result
            return tool("subagents:list_agents")

        def parent_done(request):
            """Confirm both terminal records remain visible to their owning parent."""
            result = last_result(request)
            assert len(result["agents"]) == 2 and all(agent["status"] == "closed" for agent in result["agents"]), result
            observed["closed_list"] = result
            return "CLI_PARENT_FINISHED"

        fixture.steps = [tool("subagents:spawn_agent", name="CLI child", task="CLI_CHILD_INITIAL"), first_wait, second_wait, finish, wait_followup, start_slow, observe_slow, interrupt, wait_replacement, list_completed, close_completed, spawn_closing, wait_closing, close_running, final_list, parent_done]
        if args.scenario == "wait":
            fixture.steps = fixture.steps[:4]
        receipt = cli.core("chat", "send", "--chat", parent, "CLI_PARENT_START")
        assert receipt["outcome"]["type"] == "committed" and receipt["outcome"]["status"] == "completed", receipt
        assert receipt["aiResponse"].strip() == "CLI_PARENT_FINISHED", receipt
        print("PASS committed parent receipt after actual tool sequence", flush=True)
        assert fixture.review_sent.wait(timeout=10), "Goal completion reviewer did not execute"
        time.sleep(0.5)
        history = cli.core("chat", "show", parent)
        (root / "parent-history.json").write_text(json.dumps(history, ensure_ascii=False, indent=2), encoding="utf-8")
        goal = cli.core("goal")
        print("Goal:", goal, flush=True)
        assert goal["goal"]["status"] == "completed", goal
        print("Parent steps:", fixture.parent_index, "Child requests:", fixture.child_number, flush=True)
        assert fixture.parent_index == len(fixture.steps)
        expected_inputs = ["CLI_CHILD_INITIAL"] if args.scenario == "wait" else ["CLI_CHILD_INITIAL", "CLI_CHILD_FOLLOWUP", "CLI_CHILD_SLOW", "CLI_CHILD_REPLACEMENT", "CLI_CHILD_SLOW_CLOSE"]
        expected_provider_inputs = list(expected_inputs)
        if args.scenario == "lifecycle":
            # ConversationService joins adjacent user turns after the cancelled empty response.
            expected_provider_inputs[3] = "CLI_CHILD_SLOW\nCLI_CHILD_REPLACEMENT"
        assert fixture.child_inputs == expected_provider_inputs, fixture.child_inputs
        initial_requests = [request for request in fixture.requests if request.get("stream") and
                            any(message["role"] == "user" and message["content"] == "CLI_PARENT_START" for message in request["messages"])]
        assert initial_requests and all(objective in request["messages"][0]["content"] for request in initial_requests)
        child_requests = [request for request in fixture.requests if request.get("stream") and
                          any(message["role"] == "user" and message["content"] == "CLI_CHILD_INITIAL" for message in request["messages"])]
        assert child_requests and all(objective not in json.dumps(request) for request in child_requests)
        assert cli.core("goal", "clear")["action"] == "clear"
        assert cli.core("goal")["goal"] is None
        child_history = cli.core("chat", "show", observed["spawn"]["id"])
        (root / "child-history.json").write_text(json.dumps(child_history, ensure_ascii=False, indent=2), encoding="utf-8")
        native_inputs = [message["parts"][0]["content"] for message in child_history["messages"] if message["sender"] == "user"]
        assert native_inputs == expected_inputs[:len(native_inputs)], native_inputs
        assert len(native_inputs) == (1 if args.scenario == "wait" else 4)
        if args.scenario == "lifecycle":
            def wait_reopened_child(request):
                """Verify a retained closed conversation authenticates as the new child's actual parent."""
                child = last_result(request)
                assert child["parentChatId"] == observed["spawn"]["chatId"], child
                assert child["id"] != observed["spawn"]["id"] and child["runNumber"] == 1, child
                observed["reopened_child"] = child
                return tool("subagents:wait_agent", agent_ids=[child["id"]])

            def close_reopened_child(request):
                """Require a real native completion before closing the new task."""
                result = last_result(request)
                assert not result["timed_out"] and result["agents"][0]["status"] == "completed", result
                return tool("subagents:close_agent", agent_id=observed["reopened_child"]["id"])

            def reopened_done(request):
                """Complete the directly submitted user turn after its own subagent has closed."""
                result = last_result(request)
                assert result["id"] == observed["reopened_child"]["id"] and result["status"] == "closed", result
                observed["reopened_closed"] = result
                return "CLI_REOPEN_FINISHED"

            fixture.reopened_steps = [
                tool("subagents:spawn_agent", name="Retained chat child", task="CLI_CHILD_FROM_REOPENED"),
                wait_reopened_child, close_reopened_child, reopened_done,
            ]
            reopened_receipt = cli.core("chat", "send", "--chat", observed["spawn"]["chatId"], "CLI_REOPEN_START")
            assert reopened_receipt["outcome"]["status"] == "completed", reopened_receipt
            assert reopened_receipt["aiResponse"].strip() == "CLI_REOPEN_FINISHED", reopened_receipt
            assert fixture.reopened_index == len(fixture.reopened_steps)
            assert fixture.child_inputs == expected_provider_inputs + ["CLI_CHILD_FROM_REOPENED"], fixture.child_inputs
            reopened_history = cli.core("chat", "show", observed["spawn"]["chatId"])
            (root / "reopened-history.json").write_text(json.dumps(reopened_history, ensure_ascii=False, indent=2), encoding="utf-8")
            assert any(message["sender"] == "user" and message["parts"][0]["content"] == "CLI_REOPEN_START" for message in reopened_history["messages"])
            print("PASS user turn in retained closed chat creates, waits and closes a new subagent; reopened steps:", fixture.reopened_index, "total child requests:", fixture.child_number, flush=True)
        (root / "observed.json").write_text(json.dumps(observed, ensure_ascii=False, indent=2), encoding="utf-8")
        print("PASS", args.scenario, "external installation, composer coexistence, real model delegation, durable history and Goal completion review; loopback fixture, no live provider", flush=True)
    except Exception:
        if cli is not None:
            cli.failed = True
        print("FAIL; diagnostics retained:", root, flush=True)
        raise
    finally:
        fixture.close()
        if cli is not None:
            cli.close()


if __name__ == "__main__":
    main()
