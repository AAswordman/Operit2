#!/usr/bin/env python3
"""Build and cache a matching jitless V8 library, snapshot and Rust bindings for iOS."""
from __future__ import annotations

import argparse
import base64
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tomllib
import urllib.request

REPO_ROOT = Path(__file__).resolve().parents[2]
BRIDGE_LOCK = REPO_ROOT / "apps/flutter/native/operit-flutter-bridge/Cargo.lock"
CACHE_ROOT = REPO_ROOT / "tools/release/work/ios-v8"
TARGETS = {
    "aarch64-apple-ios": ("device", "arm64"),
    "aarch64-apple-ios-sim": ("simulator", "arm64"),
    "x86_64-apple-ios": ("simulator", "x64"),
}
# Keep platform policy in the Apple build layer. No runtime OS checks are needed.
GN_OPTIONS = {
    "target_os": "ios",
    "ios_deployment_target": "14.0",
    "ios_enable_code_signing": False,
    "treat_warnings_as_errors": False,
    "v8_jitless": True,
    "v8_enable_webassembly": False,
    "v8_enable_sparkplug": False,
    "v8_enable_maglev": False,
    "v8_enable_turbofan": False,
    "cppgc_enable_caged_heap": False,
    # The published crate omits Temporal's Rust vendor sources. Intl stays on.
    "v8_enable_temporal_support": False,
}
ARTIFACTS = ("librusty_v8.a", "src_binding.rs", "args.gn")


def run(command, **kwargs):
    print("+ " + shlex.join(map(str, command)), file=sys.stderr, flush=True)
    return subprocess.run(list(map(str, command)), check=True, **kwargs)


def output(command):
    return run(command, capture_output=True, text=True).stdout.strip()


def version_from_lock(lock=BRIDGE_LOCK):
    packages = tomllib.loads(lock.read_text())["package"]
    versions = {p["version"] for p in packages if p["name"] == "v8"}
    if len(versions) != 1:
        raise RuntimeError(f"Expected exactly one V8 version in {lock}: {versions}")
    return versions.pop()


def configuration(version, target):
    environment, cpu = TARGETS[target]
    return {"recipe": 1, "version": version, "target": target,
            "gn": {**GN_OPTIONS, "target_environment": environment, "target_cpu": cpu,
                   "use_custom_libcxx": True, "v8_enable_sandbox": False,
                   "v8_enable_pointer_compression": False, "v8_enable_v8_checks": False}}


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def check_gn(path, config):
    args = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r"\s*(\w+)\s*=\s*(.*?)\s*", line)
        if match:
            args[match[1]] = match[2]
    for name, value in config["gn"].items():
        if args.get(name) != json.dumps(value):
            raise RuntimeError(f"V8 configuration mismatch: {name}={args.get(name)}, expected {value!r}")


def cache_valid(directory, config):
    try:
        manifest = json.loads((directory / "manifest.json").read_text())
        if manifest["configuration"] != config:
            return False
        if any(manifest["sha256"][name] != digest(directory / name) for name in ARTIFACTS):
            return False
        check_gn(directory / "args.gn", config)
        return True
    except (OSError, ValueError, KeyError, RuntimeError):
        return False


def store_artifacts(gn_out, directory, config):
    """Publish only a complete source build; never mix downloaded bindings and libraries."""
    check_gn(gn_out / "args.gn", config)
    directory.mkdir(parents=True, exist_ok=True)
    for name in ARTIFACTS:
        source = gn_out / ("obj/librusty_v8.a" if name == "librusty_v8.a" else name)
        temporary = directory / (name + ".tmp")
        shutil.copyfile(source, temporary)
        temporary.replace(directory / name)
    manifest = {"configuration": config,
                "sha256": {name: digest(directory / name) for name in ARTIFACTS}}
    temporary = directory / "manifest.json.tmp"
    temporary.write_text(json.dumps(manifest, indent=2) + "\n")
    temporary.replace(directory / "manifest.json")


def brew_prefix(formula):
    brew = shutil.which("brew") or next(
        (str(p) for p in (Path("/opt/homebrew/bin/brew"), Path("/usr/local/bin/brew")) if p.is_file()), None)
    if not brew:
        return None
    result = subprocess.run([brew, "--prefix", formula], capture_output=True, text=True)
    return Path(result.stdout.strip()) if result.returncode == 0 else None


def prepare_toolchain(cache):
    llvm = (Path(os.environ["OPERIT_IOS_V8_LLVM"]) if "OPERIT_IOS_V8_LLVM" in os.environ
            else brew_prefix("llvm@23") or brew_prefix("llvm"))
    lld = (Path(os.environ["OPERIT_IOS_V8_LLD"]) if "OPERIT_IOS_V8_LLD" in os.environ
           else brew_prefix("lld@23") or brew_prefix("lld"))
    if not llvm or not lld or not (lld / "bin/ld64.lld").is_file():
        raise RuntimeError("Install V8 build tools: brew install llvm@23 lld@23")
    resource = Path(output([llvm / "bin/clang", "-print-resource-dir"]))
    if int(resource.name.split(".")[0]) < 21:
        raise RuntimeError("V8 source bindings require LLVM/libclang 21.1 or newer")
    developer = Path(output(["xcode-select", "-p"]))
    apple_resources = developer / "Toolchains/XcodeDefault.xctoolchain/usr/lib/clang"
    apple_versions = sorted(apple_resources.iterdir(), key=lambda p: tuple(map(int, p.name.split("."))))
    apple_runtime = apple_versions[-1] / "lib/darwin"
    # Homebrew LLVM supplies libclang; Xcode supplies missing iOS compiler-rt files.
    # Overlay them without modifying either installed toolchain.
    overlay = cache / "llvm"
    overlay.mkdir(parents=True, exist_ok=True)
    for entry in llvm.iterdir():
        destination = overlay / entry.name
        if entry.name != "lib" and not destination.exists():
            destination.symlink_to(entry, target_is_directory=entry.is_dir())
    lib = overlay / "lib"
    lib.mkdir(exist_ok=True)
    for entry in (llvm / "lib").iterdir():
        if entry.name != "clang" and not (lib / entry.name).exists():
            (lib / entry.name).symlink_to(entry, target_is_directory=entry.is_dir())
    resources = lib / "clang" / resource.name
    resources.mkdir(parents=True, exist_ok=True)
    for entry in resource.iterdir():
        if entry.name != "lib" and not (resources / entry.name).exists():
            (resources / entry.name).symlink_to(entry, target_is_directory=entry.is_dir())
    runtime = resources / "lib/darwin"
    runtime.mkdir(parents=True, exist_ok=True)
    for parent in (resource / "lib/darwin", apple_runtime):
        for entry in parent.iterdir():
            destination = runtime / entry.name
            if not destination.exists():
                destination.symlink_to(entry)
    for name in ("libclang_rt.ios.a", "libclang_rt.iossim.a"):
        if not (runtime / name).is_file():
            raise RuntimeError(f"Missing Xcode iOS compiler runtime: {name}")
    return overlay, llvm / "lib", lld / "bin"


def prepare_icu(source):
    deps = (source / "v8/DEPS").read_text()
    revision = re.search(r"/chromium/deps/icu\.git'\s*\+\s*'@'\s*\+\s*'([a-f0-9]{40})'", deps)
    if not revision:
        raise RuntimeError("Cannot find V8's pinned ICU revision")
    for variant in ("ios", "common"):
        destination = source / "third_party/icu" / variant / "icudtl.dat"
        if destination.is_file() and destination.stat().st_size > 0:
            continue
        url = ("https://chromium.googlesource.com/chromium/deps/icu/+/"
               f"{revision[1]}/{variant}/icudtl.dat?format=TEXT")
        print(f"Fetching pinned ICU data: {variant} @ {revision[1]}", file=sys.stderr, flush=True)
        with urllib.request.urlopen(url, timeout=180) as response:
            data = base64.b64decode(response.read(), validate=True)
        if not data:
            raise RuntimeError("Downloaded empty ICU data")
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_suffix(".tmp")
        temporary.write_bytes(data)
        temporary.replace(destination)


def build(cache, config):
    probe = cache / "probe"
    (probe / "src").mkdir(parents=True, exist_ok=True)
    (probe / "Cargo.toml").write_text(
        '[package]\nname="operit-ios-v8-build"\nversion="0.0.0"\nedition="2021"\n'
        '[lib]\ncrate-type=["staticlib"]\n[dependencies]\n'
        f'v8="={config["version"]}"\n[workspace]\n')
    (probe / "src/lib.rs").write_text('pub fn version() -> &\'static str { v8::V8::get_version() }\n')
    metadata = json.loads(output(["cargo", "metadata", "--manifest-path", probe / "Cargo.toml",
                                  "--format-version", "1"]))
    source = Path(next(p["manifest_path"] for p in metadata["packages"] if p["name"] == "v8")).parent
    prepare_icu(source)
    clang, libclang, lld = prepare_toolchain(cache)
    env = os.environ.copy()
    for name in ("DOCS_RS", "RUSTY_V8_ARCHIVE", "RUSTY_V8_SRC_BINDING_PATH", "GN_ARGS",
                 "EXTRA_GN_ARGS", "CARGO_ENCODED_RUSTFLAGS", "V8_FORCE_DEBUG"):
        env.pop(name, None)
    args = {**config["gn"], "lld_path": str(lld)}
    env.update(V8_FROM_SOURCE="1", PYTHON=sys.executable, CLANG_BASE_PATH=str(clang),
               LIBCLANG_PATH=str(libclang), CARGO_TARGET_DIR=str(cache / "build"),
               GN_ARGS=" ".join(f"{name}={json.dumps(value)}" for name, value in args.items()))
    run(["cargo", "build", "--locked", "--manifest-path", probe / "Cargo.toml", "--release",
         "--target", config["target"], "--jobs", os.environ.get("OPERIT_IOS_V8_JOBS", "6")], env=env)
    return cache / "build" / config["target"] / "release/gn_out"


def prepare(target, env_file, cache_root=CACHE_ROOT):
    config = configuration(version_from_lock(), target)
    cache = cache_root / config["version"]
    cache.mkdir(parents=True, exist_ok=True)
    # Protect shared source downloads/toolchain overlays as well as each output pair.
    with (cache / "build.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        directory = cache / target
        if not cache_valid(directory, config):
            print(f"Building V8 {config['version']} jitless for {target} (first build takes time)",
                  file=sys.stderr, flush=True)
            store_artifacts(build(cache, config), directory, config)
        else:
            print(f"Using verified V8 jitless cache: {directory}", file=sys.stderr, flush=True)
        values = {"RUSTY_V8_ARCHIVE": str(directory / "librusty_v8.a"),
                  "RUSTY_V8_ARCHIVE_SHA256": digest(directory / "librusty_v8.a"),
                  "RUSTY_V8_SRC_BINDING_PATH": str(directory / "src_binding.rs")}
        env_file.parent.mkdir(parents=True, exist_ok=True)
        env_file.write_text("unset V8_FROM_SOURCE DOCS_RS V8_FORCE_DEBUG\n" + "".join(
            f"export {name}={shlex.quote(value)}\n" for name, value in values.items()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--env-file", type=Path, required=True)
    args = parser.parse_args()
    prepare(args.target, args.env_file)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"iOS V8 preparation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
