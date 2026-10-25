#!/usr/bin/env python3
"""Build and cache a matching V8 library, snapshot and Rust bindings for Android.

rusty_v8 publishes no prebuilt static libraries for Android targets, so the
Android build cannot use the default crate download: it 404s on GitHub with
"no downloader (deno, python, curl) could fetch the URL". Instead this helper
builds the V8 version pinned in the bridge's Cargo.lock from source and
supplies the resulting static library and generated Rust bindings together.
The embedded startup snapshot comes from that same build.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tomllib
import urllib.request
import zipfile

try:
    import fcntl
except ImportError:  # Windows: Gradle serializes task execution instead.
    fcntl = None

REPO_ROOT = Path(__file__).resolve().parents[2]
BRIDGE_LOCK = REPO_ROOT / "apps/flutter/native/operit-flutter-bridge/Cargo.lock"
CACHE_ROOT = REPO_ROOT / "tools/release/work/android-v8"
IS_WINDOWS = sys.platform.startswith("win")
HOST_TAG = {"win32": "windows-x86_64", "darwin": "darwin-x86_64"}.get(sys.platform, "linux-x86_64")
# rusty_v8's build script maps only the x86_64 and aarch64 android targets to
# GN cpus; armv7 would end up as "unknown".
TARGETS = {
    "aarch64-linux-android": "arm64",
    "x86_64-linux-android": "x64",
}
# Android permits JIT (iOS does not), so the optimizing tiers and WebAssembly
# stay enabled. The sandbox, pointer compression and caged heap stay disabled
# to keep both mobile builds on the same memory layout, avoiding their large
# aligned virtual-address reservations. Temporal is disabled because the
# published source crate omits Temporal's Rust vendor dependencies; current
# runtime plugins do not use Temporal. Intl stays on. The vendored Chromium
# build config asserts that Android builds pick a static analysis mode
# explicitly, and source builds need none of it.
GN_OPTIONS = {
    "v8_jitless": False,
    "v8_enable_webassembly": True,
    "v8_enable_sandbox": False,
    "v8_enable_pointer_compression": False,
    "cppgc_enable_caged_heap": False,
    "v8_enable_temporal_support": False,
    "use_custom_libcxx": True,
    "android_static_analysis": "off",
}
# Must stay >= Chromium's min_supported_sdk_version (23, 27 with ASAN) and
# match the app's minSdkVersion so the library loads on every supported device.
DEFAULT_API_LEVEL = 24
# The NDK version rusty_v8's build script would download itself, used only as
# a last resort when no local NDK matches.
NDK_FALLBACK_URL = "https://dl.google.com/android/repository/android-ndk-r26c-linux.zip"
# rusty_v8's build script hardcodes this API level in its NDK guard path.
NDK_GUARD_API_LEVEL = 24
# Pinned gn/ninja are resolved through these CIPD endpoints; the crate's own
# downloader opens raw connections that bypass proxy environment settings.
CIPD_RESOLVE_URL = ("https://chrome-infra-packages.appspot.com/_ah/api/repo/v1/"
                    "instance/resolve?package_name={}&version={}")
CIPD_INSTANCE_URL = ("https://chrome-infra-packages.appspot.com/_ah/api/repo/v1/"
                     "instance?package_name={}&instance_id={}")
ARTIFACTS = ("librusty_v8.a", "src_binding.rs", "args.gn")


def run(command, **kwargs):
    print("+ " + shlex.join(map(str, command)), file=sys.stderr, flush=True)
    kwargs.setdefault("check", True)
    return subprocess.run(list(map(str, command)), **kwargs)


def output(command):
    return run(command, capture_output=True, text=True).stdout.strip()


def api_level():
    value = int(os.environ.get("OPERIT_ANDROID_V8_API_LEVEL", DEFAULT_API_LEVEL))
    if value < 23:
        raise RuntimeError(f"OPERIT_ANDROID_V8_API_LEVEL must be >= 23, got {value}")
    return value


def version_from_lock(lock=BRIDGE_LOCK):
    packages = tomllib.loads(lock.read_text())["package"]
    versions = {p["version"] for p in packages if p["name"] == "v8"}
    if len(versions) != 1:
        raise RuntimeError(f"Expected exactly one V8 version in {lock}: {versions}")
    return versions.pop()


def configuration(version, target, ndk=None):
    cpu = TARGETS[target]
    # target_os/target_cpu/v8_target_cpu/use_sysroot/treat_warnings_as_errors
    # are set by rusty_v8's build script for Android cross builds; they are
    # recorded here so a build-script change shows up as a cache mismatch.
    # The vendored Chromium config references android_ndk_version without
    # declaring it, and it must match the NDK feeding the build.
    gn = {"target_os": "android", "target_cpu": cpu, "v8_target_cpu": cpu,
          "use_sysroot": True, "treat_warnings_as_errors": False,
          "android_ndk_api_level": api_level(),
          "android_ndk_version": ndk_version(ndk), **GN_OPTIONS}
    return {"recipe": 1, "version": version, "target": target, "gn": gn}


def ndk_version(ndk):
    """Derive the android_ndk_version GN value from the NDK directory name."""
    if ndk is None:
        return "unknown"
    if re.fullmatch(r"\d+\.\d+\.\d+", ndk.name):
        return ndk.name
    match = re.search(r"r(\d+)([a-z])", ndk.name, re.IGNORECASE)
    if match:
        return f"r{match.group(1)}{match.group(2).lower()}"
    return ndk.name


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


def remove_path(path):
    """Delete a file, symlink, directory junction or directory tree."""
    if path.is_symlink() or (path.exists() and not path.is_dir()):
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)


def link_directory(link, target):
    """Create a symlink, or a directory junction on Windows (no privileges needed)."""
    link.parent.mkdir(parents=True, exist_ok=True)
    if IS_WINDOWS:
        run(["cmd", "/c", "mklink", "/J", link, target])
    else:
        link.symlink_to(target, target_is_directory=True)


def ndk_prebuilt(ndk):
    return ndk / "toolchains/llvm/prebuilt" / HOST_TAG


def ndk_toolchain_clang(ndk, api_level_value=NDK_GUARD_API_LEVEL):
    return ndk_prebuilt(ndk) / "bin" / f"aarch64-linux-android{api_level_value}-clang++"


def is_usable_ndk(ndk):
    return ndk_toolchain_clang(ndk).is_file() and ndk_toolchain_clang(ndk, api_level()).is_file()


def candidate_ndks():
    for name in ("OPERIT_ANDROID_V8_NDK", "ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK"):
        if os.environ.get(name):
            yield Path(os.environ[name])
    sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    homes = [Path(sdk) / "ndk"] if sdk else []
    homes.append(Path.home() / "Android" / "ndk")
    for home in homes:
        if home.is_dir():
            # Prefer the newest installed NDK.
            for entry in sorted(home.iterdir(), reverse=True):
                yield entry


def download_fallback_ndk(cache):
    if HOST_TAG != "linux-x86_64":
        raise RuntimeError(
            "No usable Android NDK found; set OPERIT_ANDROID_V8_NDK to an NDK r23+ "
            f"installation with toolchains/llvm/prebuilt/{HOST_TAG}")
    directory = cache / "android-ndk-r26c"
    if not is_usable_ndk(directory):
        archive = cache / "android-ndk-r26c-linux.zip"
        cache.mkdir(parents=True, exist_ok=True)
        print(f"Downloading fallback NDK: {NDK_FALLBACK_URL}", file=sys.stderr, flush=True)
        with urllib.request.urlopen(NDK_FALLBACK_URL, timeout=1800) as response, \
                archive.open("wb") as file:
            shutil.copyfileobj(response, file)
        remove_path(directory)
        run(["unzip", "-d", cache, "-o", "-q", archive])
        archive.unlink()
    return directory


def find_ndk(cache):
    for candidate in candidate_ndks():
        if is_usable_ndk(candidate):
            return candidate.resolve()
    return download_fallback_ndk(cache)


def place_ndk(source, ndk):
    """Expose the NDK inside the V8 crate source where it is looked up.

    rusty_v8's build script probes third_party/android_ndk (hardcoded
    linux-x86_64 guard path) before downloading a Linux-only NDK zip, and the
    vendored Chromium config defaults android_ndk_root to
    third_party/android_toolchain/ndk. Both get junctions for the host prebuilt
    and for the hardcoded guard path so one NDK serves every consumer.
    """
    for destination in (source / "third_party/android_ndk",
                        source / "third_party/android_toolchain/ndk"):
        if ndk_toolchain_clang(destination).is_file() \
                and ndk_toolchain_clang(destination, api_level()).is_file():
            continue
        remove_path(destination)
        prebuilt = destination / "toolchains/llvm/prebuilt"
        for name in sorted({HOST_TAG, "linux-x86_64"}):
            remove_path(prebuilt / name)
            link_directory(prebuilt / name, ndk_prebuilt(ndk))


def patch_android_config(source):
    """Declare android_ndk_version in the vendored Chromium build config.

    Chromium's build/config/android/BUILD.gn reads android_ndk_version, but the
    copy vendored into the published crate lacks the upstream declare_args
    block, so GN fails with "Undefined identifier" no matter what args.gn sets.
    """
    config = source / "build/config/android/config.gni"
    if "android_ndk_version" in config.read_text():
        return
    with config.open("a") as file:
        file.write(
            "\n# Added by Operit2's prepare_android_v8.py: the vendored config\n"
            "# references android_ndk_version without declaring it upstream.\n"
            "declare_args() {\n"
            "  android_ndk_version = \"unknown\"\n"
            "}\n")


def stub_missing_pydeps(source):
    """Create empty placeholders for python dependency manifests the crate omits.

    The vendored build/android/BUILD.gn evaluates python_library targets whose
    pydeps_file manifests are not shipped in the published crate. They only
    carry data dependencies of test-only targets that a rusty_v8 build never
    compiles, so empty manifests let GN evaluate the build files.
    """
    build_android = source / "build/android"
    for match in re.finditer(r'pydeps_file = "([^"]+)"', (build_android / "BUILD.gn").read_text()):
        manifest = build_android / match.group(1)
        if manifest.is_file():
            continue
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text("# stub: omitted from the published rusty_v8 crate\n")


def install_linux_sysroot(source):
    """Install the host debian sysroot that GN's linux toolchains assert on.

    rusty_v8's build script only installs it for android x86_64 targets, but
    the aarch64 cross build's host snapshot toolchain (clang_x64_v8_arm64)
    needs the same amd64 sysroot.
    """
    if IS_WINDOWS or sys.platform == "darwin":
        return
    linux = source / "build/linux"
    if linux.is_dir() and any(linux.glob("debian_bullseye_amd64-sysroot")):
        return
    run([sys.executable, "build/linux/sysroot_scripts/install-sysroot.py",
         "--arch=amd64"], cwd=source)


def prepare_icu(source):
    deps = (source / "v8/DEPS").read_text()
    revision = re.search(r"/chromium/deps/icu\.git'\s*\+\s*'@'\s*\+\s*'([a-f0-9]{40})'", deps)
    if not revision:
        raise RuntimeError("Cannot find V8's pinned ICU revision")
    for variant in ("common", "android"):
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


def brew_prefix(formula):
    brew = shutil.which("brew") or next(
        (str(p) for p in (Path("/opt/homebrew/bin/brew"), Path("/usr/local/bin/brew")) if p.is_file()), None)
    if not brew:
        return None
    result = subprocess.run([brew, "--prefix", formula], capture_output=True, text=True)
    return Path(result.stdout.strip()) if result.returncode == 0 else None


def find_host_clang():
    """Locate a host clang for GN's host toolchain (None downloads Chromium's).

    System or distro clang installs often use the pre-16 compiler-runtime
    layout and miss libclang_rt.builtins.a at the path Chromium's toolchains
    compute, so the hermetic download is the reliable Linux default.
    OPERIT_ANDROID_V8_LLVM overrides it; macOS prefers Homebrew LLVM like the
    iOS helper.
    """
    if "OPERIT_ANDROID_V8_LLVM" in os.environ:
        llvm = Path(os.environ["OPERIT_ANDROID_V8_LLVM"])
        if not (llvm / "bin/clang").is_file():
            raise RuntimeError(f"OPERIT_ANDROID_V8_LLVM has no bin/clang: {llvm}")
        return llvm
    if sys.platform == "darwin":
        llvm = brew_prefix("llvm@23") or brew_prefix("llvm")
        if not llvm or not (llvm / "bin/clang").is_file():
            raise RuntimeError("Install V8 build tools: brew install llvm@23")
        return llvm
    return None


def find_libclang():
    """Locate libclang 21.1+ for the source bindings generation."""
    if IS_WINDOWS:
        directory = (Path(os.environ["OPERIT_LIBCLANG_DIR"])
                     if "OPERIT_LIBCLANG_DIR" in os.environ else None)
        if not directory or not (directory / "libclang.dll").is_file():
            raise RuntimeError("Set OPERIT_LIBCLANG_DIR to the directory containing libclang.dll (21.1+)")
        return directory
    pattern = "libclang.so*"
    if sys.platform == "darwin":
        llvm = find_host_clang()
        candidates = [llvm / "lib"]
    else:
        candidates = [Path("/usr/lib/llvm21/lib")]
        candidates += sorted(Path("/usr/lib").glob("llvm*/lib"), reverse=True)
        candidates += [Path("/usr/lib/x86_64-linux-gnu"), Path("/usr/lib64"), Path("/usr/lib")]
    libraries = []
    for directory in candidates:
        libraries += sorted(directory.glob(pattern))
    for library in libraries:
        match = re.search(r"libclang\.so\.(\d+)", library.name)
        if match and int(match.group(1)) >= 21:
            return library.parent
    if sys.platform == "darwin" and libraries:
        return libraries[0].parent
    raise RuntimeError(
        "V8 source bindings require LLVM/libclang 21.1 or newer "
        "(apt install llvm-21-dev / brew install llvm@23)")


def find_lld(clang_base):
    """Locate the host ld.lld used by Chromium's toolchain (not needed on Windows)."""
    if IS_WINDOWS:
        return None
    candidates = []
    if clang_base:
        candidates.append(clang_base / "bin/ld.lld")
    if sys.platform == "darwin":
        for formula in ("lld@23", "lld"):
            prefix = brew_prefix(formula)
            if prefix:
                candidates.append(prefix / "bin/ld.lld")
    candidates += sorted(Path("/usr/lib").glob("llvm*/bin/ld.lld"), reverse=True)
    candidates.append(Path("/usr/bin/ld.lld"))
    for candidate in candidates:
        if candidate.is_file():
            return candidate.parent
    raise RuntimeError("Install the LLVM linker: apt install lld / brew install lld@23")


def deps_version(source, name):
    """Read a pinned tool version from V8's DEPS (as the crate's v8_deps.py does)."""
    deps = (source / "v8/DEPS").read_text()
    match = re.search(rf"'{name}':\s*'([^']+)'", deps)
    if not match:
        raise RuntimeError(f"Cannot find V8's pinned {name}")
    return match.group(1)


def fetch_bytes(url, timeout):
    with urllib.request.urlopen(url, timeout=timeout) as response:
        return response.read()


def cipd_tool_tag():
    system = {"darwin": "mac", "win32": "windows"}.get(sys.platform, "linux")
    machine = "arm64" if platform.machine().lower() == "aarch64" else "amd64"
    return f"{system}-{machine}"


def extract_tool_archive(data, directory):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        archive.extractall(directory)
        if not IS_WINDOWS:
            for info in archive.infolist():
                if info.is_dir():
                    continue
                extracted = directory / info.filename
                mode = info.external_attr >> 16
                if mode:
                    os.chmod(extracted, os.stat(extracted).st_mode | mode)


def ensure_gn_ninja(cache, source):
    """Provision the pinned gn and ninja, returning their paths for GN/NINJA.

    rusty_v8's ninja_gn_binaries.py fetches the CIPD metadata over raw
    http.client connections that bypass proxy environment settings, so this
    resolves and downloads the same pinned packages with urllib instead.
    """
    tag = cipd_tool_tag()
    paths = {}
    for tool in ("gn", "ninja"):
        binary = f"{tool}.exe" if IS_WINDOWS else tool
        destination = cache / "tools" / tool / binary
        paths[tool] = destination
        if destination.is_file():
            continue
        package = f"gn/gn/{tag}" if tool == "gn" else f"infra/3pp/tools/ninja/{tag}"
        version = deps_version(source, f"{tool}_version")
        print(f"Fetching pinned {tool}: {version}", file=sys.stderr, flush=True)
        resolved = json.loads(fetch_bytes(CIPD_RESOLVE_URL.format(package, version), 300))
        instance = json.loads(fetch_bytes(CIPD_INSTANCE_URL.format(package, resolved["instance_id"]), 300))
        destination.parent.mkdir(parents=True, exist_ok=True)
        extract_tool_archive(fetch_bytes(instance["fetch_url"], 1800), destination.parent)
        if not destination.is_file():
            raise RuntimeError(f"CIPD archive did not contain {binary} ({package} {version})")
    return paths


def build(cache, config, ndk):
    probe = cache / "probe"
    (probe / "src").mkdir(parents=True, exist_ok=True)
    (probe / "Cargo.toml").write_text(
        '[package]\nname="operit-android-v8-build"\nversion="0.0.0"\nedition="2021"\n'
        '[lib]\ncrate-type=["staticlib"]\n[dependencies]\n'
        f'v8="={config["version"]}"\n[workspace]\n')
    (probe / "src/lib.rs").write_text('pub fn version() -> &\'static str { v8::V8::get_version() }\n')
    metadata = json.loads(output(["cargo", "metadata", "--manifest-path", probe / "Cargo.toml",
                                  "--format-version", "1"]))
    source = Path(next(p["manifest_path"] for p in metadata["packages"] if p["name"] == "v8")).parent
    patch_android_config(source)
    stub_missing_pydeps(source)
    install_linux_sysroot(source)
    place_ndk(source, ndk)
    prepare_icu(source)
    gn_ninja = ensure_gn_ninja(cache, source)
    clang = find_host_clang()
    libclang = find_libclang()
    lld = find_lld(clang)
    try:
        run(["rustup", "target", "add", config["target"]], check=False)
    except OSError:
        print("rustup not found; assuming the Rust target is already installed",
              file=sys.stderr, flush=True)
    env = os.environ.copy()
    for name in ("DOCS_RS", "RUSTY_V8_ARCHIVE", "RUSTY_V8_SRC_BINDING_PATH", "GN_ARGS",
                 "EXTRA_GN_ARGS", "CARGO_ENCODED_RUSTFLAGS", "V8_FORCE_DEBUG"):
        env.pop(name, None)
    # GN_ARGS carries only what rusty_v8's build script does not set itself.
    args = {**GN_OPTIONS, "android_ndk_api_level": api_level(),
            "android_ndk_version": ndk_version(ndk), "android_ndk_root": str(ndk)}
    if lld:
        args["lld_path"] = str(lld)
    env.update(V8_FROM_SOURCE="1", PYTHON=sys.executable,
               CARGO_TARGET_DIR=str((cache / "build").resolve()),
               GN=str(gn_ninja["gn"]), NINJA=str(gn_ninja["ninja"]),
               GN_ARGS=" ".join(f"{name}={json.dumps(value)}" for name, value in args.items()))
    # Bindgen derives the triple from Cargo's TARGET but gets no sysroot, so it
    # falls back to the host's glibc headers, whose default wordsize makes
    # uintptr_t 32-bit and breaks the V8 headers. Point it at the NDK sysroot
    # via the lowercase spelling bindgen 0.72 reads (uppercase is ignored).
    sysroot = ndk_prebuilt(ndk) / "sysroot"
    for android_target in TARGETS:
        env[f"BINDGEN_EXTRA_CLANG_ARGS_{android_target.replace('-', '_')}"] = (
            f"--target={android_target}{api_level()} --sysroot={sysroot}")
    if clang:
        env["CLANG_BASE_PATH"] = str(clang)
    env["LIBCLANG_PATH"] = str(libclang)
    run(["cargo", "build", "--locked", "--manifest-path", probe / "Cargo.toml", "--release",
         "--target", config["target"], "--jobs", os.environ.get("OPERIT_ANDROID_V8_JOBS", "6")], env=env)
    return (cache / "build").resolve() / config["target"] / "release/gn_out"


def prepare(target, env_file, cache_root=CACHE_ROOT):
    cache = cache_root / version_from_lock()
    cache.mkdir(parents=True, exist_ok=True)
    # Protect shared source downloads/toolchain placements as well as each output set.
    with (cache / "build.lock").open("a") as lock:
        if fcntl:
            fcntl.flock(lock, fcntl.LOCK_EX)
        ndk = find_ndk(cache)
        config = configuration(version_from_lock(), target, ndk)
        directory = cache / target
        if not cache_valid(directory, config):
            print(f"Building V8 {config['version']} for {target} (first build takes time)",
                  file=sys.stderr, flush=True)
            store_artifacts(build(cache, config, ndk), directory, config)
        else:
            print(f"Using verified V8 cache: {directory}", file=sys.stderr, flush=True)
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
        print(f"Android V8 preparation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
