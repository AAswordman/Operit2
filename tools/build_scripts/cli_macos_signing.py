"""Stable local CLI signing without exporting keys or weakening Keychain access controls."""
from __future__ import annotations

import json
import os
import platform
from pathlib import Path
import subprocess
import sys

CLI_SIGNING_IDENTIFIER = "com.operit.cli"
LOCAL_SIGNING_CONFIG = Path(__file__).with_name("cli_macos_signing.local")


def signing_identity(explicit: str | None = None) -> str | None:
    """Resolve an opt-in identity; developer-specific certificate references stay git-ignored."""
    identity = explicit if explicit is not None else os.environ.get("OPERIT_MACOS_SIGNING_IDENTITY")
    if identity is None and LOCAL_SIGNING_CONFIG.is_file():
        config = json.loads(LOCAL_SIGNING_CONFIG.read_text(encoding="utf-8"))
        if not isinstance(config, dict) or not isinstance(config.get("identity"), str):
            raise ValueError(f"Expected a signing identity in {LOCAL_SIGNING_CONFIG}")
        identity = config["identity"]
    if identity is None:
        return None
    if not identity.strip() or identity.strip() == "-":
        raise ValueError("Use a real macOS signing identity, not an empty or ad-hoc identity")
    return identity


def sign_cli_macos(binary: Path, identity: str | None = None) -> bool:
    """Sign and verify only the supplied CLI artifact; never edit Keychain ACLs or secret items."""
    if platform.system() != "Darwin":
        return False
    selected = signing_identity(identity)
    if selected is None:
        print("macOS CLI signing not configured; rebuilt ad-hoc binaries may request Keychain authorization. "
              "Set OPERIT_MACOS_SIGNING_IDENTITY to a real signing identity.", file=sys.stderr)
        return False
    binary = binary.resolve(strict=True)
    subprocess.run(["codesign", "--force", "--sign", selected, "--identifier", CLI_SIGNING_IDENTIFIER,
                    "--timestamp=none", str(binary)], check=True, stdout=sys.stderr)
    subprocess.run(["codesign", "--verify", "--strict", str(binary)], check=True, stdout=sys.stderr)
    print(f"Verified macOS CLI signature: {CLI_SIGNING_IDENTIFIER}", file=sys.stderr)
    return True
