"""Run the packaged Operit2 CLI inside a Harbor task sandbox."""

import json
import shlex
import tempfile
from pathlib import Path

from harbor.agents.base import BaseAgent
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext


class OperitAgent(BaseAgent):
    """Adapt the public CLI without replacing its prompts, tools, or agent loop."""

    # Resolve host artifacts before the sandbox starts.
    def __init__(self, logs_dir, model_name=None, *, bundle_path, providers_path,
                 provider_id="DEEPSEEK", thinking="high", **kwargs):
        kwargs.pop("skip_model_parsing", None)
        super().__init__(logs_dir, model_name, skip_model_parsing=True, **kwargs)
        self.bundle = Path(bundle_path).resolve(strict=True)
        self.providers = Path(providers_path).resolve(strict=True)
        self.provider_id = provider_id
        self.thinking = thinking
        self.chat_id = None
        self.cli_env = {
            "OPERIT_CLI_CONFIG_DIR": "/opt/operit-state/config",
            "OPERIT_CLI_SECRET_STORE": "file",
        }

    # Identify the adapter in Harbor results.
    @staticmethod
    def name():
        return "operit2"

    # Keep the source build identification in the packaged manifest.
    def version(self):
        return "2.0.0-preview.46"

    # Execute one CLI command and retain only its credential-free response.
    async def cli(self, environment, *args):
        result = await environment.exec(
            command=shlex.join(["/opt/operit/operit2", "cli", *args, "--json"]),
            env=self.cli_env,
            timeout_sec=180,
        )
        if result.return_code:
            raise RuntimeError(f"Operit2 {args[0]} {args[1:2]} failed: {result.stdout} {result.stderr}")
        return json.loads(result.stdout)

    # Install the local bundle and configure a fresh, isolated task identity.
    async def setup(self, environment: BaseEnvironment):
        result = await environment.exec(command="mkdir -p /opt/operit /opt/operit-state/config")
        if result.return_code:
            raise RuntimeError(result.stderr)
        await environment.upload_file(self.bundle, "/opt/operit-bundle.tar.gz")
        await environment.upload_file(self.providers, "/opt/operit-providers.json")
        result = await environment.exec(command="tar -xzf /opt/operit-bundle.tar.gz -C /opt/operit")
        if result.return_code:
            raise RuntimeError(result.stderr)
        storage = {
            "runtimeRoot": "/opt/operit-state/runtime",
            "workspaceRoot": "/opt/operit-state/workspaces",
            "activeIdentityId": "benchmark",
            "identities": [{"id": "benchmark", "name": "Harbor benchmark", "createdAt": 1}],
        }
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "storage.json"
            path.write_text(json.dumps(storage))
            await environment.upload_file(path, "/opt/operit-state/config/storage.json")
        await self.cli(environment, "model", "providers-import", "@/virtual/linux/opt/operit-providers.json")
        result = await environment.exec(command="rm -f /opt/operit-providers.json /opt/operit-bundle.tar.gz")
        if result.return_code:
            raise RuntimeError(result.stderr)
        await self.cli(environment, "approval", "full")
        bindings = await self.cli(environment, "model", "function-list")
        for binding in bindings["bindings"]:
            await self.cli(environment, "model", "function-set", binding["functionType"],
                           self.provider_id, self.model_name)
        await self.cli(environment, "model", "thinking", self.provider_id, self.model_name, self.thinking)
        workspace = await self.cli(environment, "workspace", "create", "Harbor task", "--native", "/app")
        chat = await self.cli(environment, "chat", "new", "--workspace", workspace["workspaceId"])
        self.chat_id = chat["chatId"]
        model = await self.cli(environment, "model", "show", self.provider_id, self.model_name)
        self.logs_dir.mkdir(parents=True, exist_ok=True)
        (self.logs_dir / "model.json").write_text(json.dumps(model, ensure_ascii=False, indent=2))

    # Submit exactly one task instruction to the real CLI and save its transcript.
    async def run(self, instruction: str, environment: BaseEnvironment, context: AgentContext):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "instruction.txt"
            path.write_text(instruction)
            await environment.upload_file(path, "/opt/operit-state/instruction.txt")
        command = shlex.join(["/opt/operit/operit2", "cli", "chat", "send", "--chat", self.chat_id])
        result = await environment.exec(
            command=command + ' "$(cat /opt/operit-state/instruction.txt)" --json',
            env=self.cli_env,
        )
        (self.logs_dir / "receipt.json").write_text(result.stdout or "")
        (self.logs_dir / "stderr.txt").write_text(result.stderr or "")
        history = await self.cli(environment, "chat", "show", self.chat_id)
        (self.logs_dir / "chat.json").write_text(json.dumps(history, ensure_ascii=False, indent=2))
        context.metadata = {"chat_id": self.chat_id, "return_code": result.return_code,
                            "thinking": self.thinking, "native_transcript": "chat.json"}
        if result.return_code:
            raise RuntimeError(f"Operit2 task exited with {result.return_code}: {result.stderr}")
        receipt = json.loads(result.stdout)
        context.metadata["outcome"] = receipt.get("outcome")
