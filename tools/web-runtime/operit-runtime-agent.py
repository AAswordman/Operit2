#!/usr/bin/env python3
"""Run one Node or Python service inside the isolated V86 guest."""

from __future__ import annotations

import base64
import binascii
import json
import os
import posixpath
import subprocess
import sys
from pathlib import PurePosixPath


WORKSPACE_ROOT = PurePosixPath("/workspace")
READY_MARKER = "OPERIT_RUNTIME_READY"
EXIT_MARKER_PREFIX = "OPERIT_RUNTIME_EXIT:"


def write_protocol_error(message: str) -> None:
    """Write one structured agent diagnostic to the serial error stream."""
    print(json.dumps({"operitRuntimeError": message}), file=sys.stderr, flush=True)


def workspace_path(value: str) -> str:
    """Resolve one guest-relative path below the isolated workspace root."""
    candidate = PurePosixPath(value)
    if candidate.is_absolute():
        raise ValueError("deployment path must be relative")
    normalized = PurePosixPath(posixpath.normpath(str(WORKSPACE_ROOT / candidate)))
    if not normalized.is_relative_to(WORKSPACE_ROOT):
        raise ValueError("deployment path escapes the workspace")
    return str(normalized)


def install_file(message: dict[str, object]) -> None:
    """Write one base64-encoded workspace file supplied by the host."""
    relative_path = message.get("path")
    encoded = message.get("base64")
    if not isinstance(relative_path, str) or not isinstance(encoded, str):
        raise ValueError("file message requires string path and base64")
    destination = workspace_path(relative_path)
    content = base64.b64decode(encoded, validate=True)
    os.makedirs(os.path.dirname(destination), exist_ok=True)
    with open(destination, "wb") as file:
        file.write(content)


def start_service(message: dict[str, object]) -> int:
    """Run one requested managed runtime process and publish its exit code."""
    program = message.get("program")
    arguments = message.get("arguments")
    environment = message.get("environment")
    working_directory = message.get("workingDirectory")
    executables = {
        "node": "/usr/local/bin/node",
        "python3": "/usr/local/bin/python3",
    }
    if program not in executables:
        raise ValueError("program must be node or python3")
    if not isinstance(arguments, list) or not all(isinstance(argument, str) for argument in arguments):
        raise ValueError("start message requires string arguments")
    if not isinstance(environment, dict) or not all(
        isinstance(key, str) and isinstance(value, str) for key, value in environment.items()
    ):
        raise ValueError("start message requires string environment values")
    if not isinstance(working_directory, str):
        raise ValueError("start message requires workingDirectory")
    cwd = workspace_path(working_directory)
    os.makedirs(cwd, exist_ok=True)
    command = executables[program]
    os.chdir(cwd)
    process_environment = {
        "HOME": "/workspace",
        "PATH": "/usr/local/bin:/usr/bin:/bin",
        "NODE_PATH": "/usr/local/lib/node_modules",
        "PYTHONHOME": "/usr/local/operit-python",
    }
    process_environment.update(environment)
    process = subprocess.Popen(
        [command, *arguments],
        cwd=cwd,
        env=process_environment,
        stdin=sys.stdin,
        stdout=sys.stdout,
        stderr=sys.stderr,
    )
    exit_code = process.wait()
    print(f"\n{EXIT_MARKER_PREFIX}{exit_code}", flush=True)
    return exit_code


def serve() -> int:
    """Receive deployment frames and then hand serial stdio to the interpreter."""
    print(f"\n{READY_MARKER}", flush=True)
    for raw_line in sys.stdin:
        try:
            message = json.loads(raw_line)
            if not isinstance(message, dict):
                raise ValueError("protocol message must be an object")
            kind = message.get("kind")
            if kind == "file":
                install_file(message)
                continue
            if kind == "start":
                return start_service(message)
            raise ValueError("protocol kind must be file or start")
        except (ValueError, binascii.Error, json.JSONDecodeError, UnicodeDecodeError) as error:
            write_protocol_error(str(error))
    return 1


def main() -> int:
    """Run the guest serial deployment protocol."""
    return serve()


if __name__ == "__main__":
    raise SystemExit(main())
