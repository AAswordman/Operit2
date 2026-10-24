#!/usr/bin/env python3
"""Stages the BIOS assets used by the production Web build for V86 smoke tests."""

import hashlib
import re
import subprocess
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
hook = (repo / "apps/flutter/app/hook/build.dart").read_text()
# Use the build hook's pins so CI boots the same BIOS as the shipped Web host.
base = re.search(r"const String _v86RuntimeAssetBaseUrl\s*=\s*'([^']+)'", hook).group(1)
assets = re.findall(
    r"relativePath: '(v86/[^']+)',\s*url: '\$\{_v86RuntimeAssetBaseUrl\}([^']+)',\s*sha256: '([0-9a-f]{64})'",
    hook,
)
if len(assets) != 2:
    raise RuntimeError("Expected exactly two pinned V86 BIOS assets in the Web build hook")
for relative, remote, expected in assets:
    data = subprocess.check_output([
        "curl", "--fail", "--location", "--retry", "3", "--max-time", "60",
        base + remote,
    ])
    if hashlib.sha256(data).hexdigest() != expected:
        raise RuntimeError(f"BIOS checksum mismatch: {relative}")
    target = repo / "apps/flutter/app/web/runtime/generated" / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    print(f"Verified {relative}")
