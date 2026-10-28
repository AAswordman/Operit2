#!/usr/bin/env python3
"""Verify manufacturer dependency boundaries without building a full Core."""
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[2]
manifest = root / "edge/sdk/Cargo.toml"
for feature in ["", "space", "http-agent", "scene"]:
    command = ["cargo", "tree", "--locked", "--manifest-path", str(manifest),
               "-p", "operit-edge-sdk", "-e", "normal,build", "--prefix", "none"]
    if feature:
        command.extend(["--features", feature])
    lines = subprocess.check_output(command, text=True, cwd=root).splitlines()
    packages = {line.split(" (", 1)[0] for line in lines}
    forbidden = {"operit-runtime", "operit-core-application", "operit-tools",
                 "operit-local-models", "v8", "rusqlite", "sqlparser", "reqwest"}
    found = sorted(package for package in packages if package.split()[0] in forbidden)
    if found:
        raise SystemExit(f"{feature or 'default'} pulls forbidden packages: {found}")
    if not feature and len(packages) != 1:
        raise SystemExit("Default SDK must have zero external dependencies")
    print(f"{feature or 'default'}: {len(packages) - 1} dependency packages; boundary OK")
