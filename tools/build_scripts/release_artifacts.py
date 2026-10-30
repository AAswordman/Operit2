"""Build release filenames from the shared App and CLI release version."""

from pathlib import Path
import re
import tomllib


REPO_ROOT = Path(__file__).resolve().parents[2]
RELEASE_MANIFESTS = (
    Path("core/crates/runtime/application/Cargo.toml"),
    Path("apps/cli/Cargo.toml"),
)
RELEASE_TARGETS = (
    ("app", "windows", "x86_64", "zip"),
    ("app", "windows", "aarch64", "zip"),
    ("app", "linux", "x86_64", "tar.gz"),
    ("app", "linux", "aarch64", "tar.gz"),
    ("app", "macos", "universal", "zip"),
    ("app", "ios", "arm64", "zip"),
    ("app", "android", "arm64-v8a", "apk"),
    ("app", "android", "armeabi-v7a", "apk"),
    ("app", "android", "x86_64", "apk"),
    ("app", "ohos", "arm64", "hap"),
    ("cli", "windows", "x86_64", "zip"),
    ("cli", "windows", "aarch64", "zip"),
    ("cli", "linux", "x86_64", "tar.gz"),
    ("cli", "linux", "aarch64", "tar.gz"),
    ("cli", "macos", "x86_64", "tar.gz"),
    ("cli", "macos", "aarch64", "tar.gz"),
    ("app", "macos", "universal", "pkg"),
    ("firmware", "esp32", "2432s028", "zip"),
)
RELEASE_VERSION_PATTERN = re.compile(
    r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)


# Reads and validates the single release version shared by App and CLI.
def release_version() -> str:
    versions = []
    for relative_path in RELEASE_MANIFESTS:
        with (REPO_ROOT / relative_path).open("rb") as handle:
            version = tomllib.load(handle)["package"]["version"]
        if not isinstance(version, str) or RELEASE_VERSION_PATTERN.fullmatch(version) is None:
            raise ValueError(f"Invalid release version in {relative_path}: {version!r}")
        _, separator, prerelease = version.partition("-")
        if separator:
            for identifier in prerelease.split("."):
                if identifier.isdecimal() and len(identifier) > 1 and identifier.startswith("0"):
                    raise ValueError(f"Invalid numeric prerelease identifier: {version}")
        versions.append(version)
    if versions[0] != versions[1]:
        raise ValueError(f"App and CLI release versions differ: {versions}")
    return versions[0]


# Builds a versioned release filename without changing compound extensions.
def release_asset_name(product: str, platform: str, arch: str, extension: str, build_type: str | None = None) -> str:
    for value in (product, platform, arch):
        if re.fullmatch(r"[a-z0-9]+(?:[-_][a-z0-9]+)*", value) is None:
            raise ValueError(f"Invalid release target component: {value!r}")
    if extension not in ("apk", "hap", "zip", "tar.gz", "pkg", "ipa"):
        raise ValueError(f"Unsupported release extension: {extension}")
    if product == "app" and platform == "macos":
        build_type = build_type or "sandbox"
        if build_type not in ("sandbox", "unsandbox"):
            raise ValueError(f"Unsupported macOS build type: {build_type}")
    elif build_type is not None:
        raise ValueError("Build types apply only to macOS App packages")
    suffix = f"-{build_type}" if build_type else ""
    return f"operit2-{product}-{platform}-{arch}{suffix}-{release_version()}.{extension}"


# Returns the exact filenames accepted for the current release across all targets.
def release_asset_names() -> frozenset[str]:
    return frozenset(
        release_asset_name(product, platform, arch, extension, build_type)
        for product, platform, arch, extension in RELEASE_TARGETS
        for build_type in (("sandbox", "unsandbox") if (product, platform) == ("app", "macos") else (None,))
    )


# Rejects outdated filenames and unrelated files before publishing release assets.
def validate_release_assets(assets: list[Path]) -> None:
    expected = release_asset_names()
    invalid = [path.name for path in assets if path.name not in expected]
    if invalid:
        raise RuntimeError(f"Release assets do not match the current versioned naming rule: {invalid}")
