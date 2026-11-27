"""Keep Harbor orchestration and use native Podman for exec and file uploads."""

import asyncio
import shlex
from pathlib import Path

from harbor.environments.podman import PodmanEnvironment


class LocalPodmanEnvironment(PodmanEnvironment):
    """Preserve task isolation and grading while allowing compose to start."""

    # Resolve each service once while its container belongs to this trial.
    async def _container(self, service):
        if not hasattr(self, "_local_containers"):
            self._local_containers = {}
        if service not in self._local_containers:
            self._local_containers[service] = await self._platform._resolve_service_container(service)
        return self._local_containers[service]

    # Use Harbor's output collector and timeout handling for the direct CLI.
    async def _native(self, arguments, timeout_sec=None):
        process = await asyncio.create_subprocess_exec(
            "podman", *arguments, stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        return await self._collect_buffered_output(process, timeout_sec=timeout_sec)

    # Execute without a terminal or an additional compose/provider process.
    async def _compose_exec(self, command, *, service, cwd, env, timeout_sec, user):
        arguments = ["exec"]
        if cwd is not None:
            arguments.extend(["--workdir", cwd])
        if env is not None:
            for key, value in env.items():
                arguments.extend(["--env", f"{key}={value}"])
        if user is not None:
            arguments.extend(["--user", str(user)])
        arguments.extend([await self._container(service), "bash", "-c", command])
        return await self._native(arguments, timeout_sec)

    # Copy files directly instead of attempting the unsupported compose cp command.
    async def upload_file(self, source_path, target_path):
        result = await self._native(["cp", str(Path(source_path).resolve(strict=True)),
                                     f"{await self._container('main')}:{target_path}"])
        if result.return_code:
            raise RuntimeError(f"Podman upload failed: {result.stderr}")

    # Preserve directory contents and Harbor's original target layout.
    async def upload_dir(self, source_dir, target_dir):
        result = await self.exec("mkdir -p " + shlex.quote(target_dir))
        if result.return_code:
            raise RuntimeError(result.stderr)
        result = await self._native(["cp", str(Path(source_dir).resolve(strict=True)) + "/.",
                                     f"{await self._container('main')}:{target_dir}"])
        if result.return_code:
            raise RuntimeError(f"Podman directory upload failed: {result.stderr}")

    # Apply the readiness deadline to one probe rather than five-second retries.
    async def _wait_until_compose_exec_ready(self, timeout_sec=90):
        result = await self.exec("true", timeout_sec=timeout_sec)
        if result.return_code != 0:
            raise RuntimeError(f"Podman readiness failed: {result.stdout} {result.stderr}")
