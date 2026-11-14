"""Build release filenames from the shared App and CLI release version."""

from pathlib import Path
import hashlib
import json
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
    ("app", "windows", "x86_64", "msi"),
    ("app", "linux", "x86_64", "tar.gz"),
    ("app", "linux", "aarch64", "tar.gz"),
    ("app", "macos", "universal", "zip"),
    ("app", "ios", "arm64", "zip"),
    ("app", "android", "arm64-v8a", "apk"),
    ("app", "android", "armeabi-v7a", "apk"),
    ("app", "android", "x86_64", "apk"),
    ("module", "android", "arm64-v8a", "zip"),
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
    if extension not in ("apk", "hap", "zip", "tar.gz", "pkg", "ipa", "msi"):
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
    available = {path.name: path for path in assets}
    allowed = set(expected)
    for manifest_path in assets:
        if not manifest_path.name.endswith(".update.json"):
            continue
        package_name = manifest_path.name.removesuffix(".update.json")
        if package_name not in expected:
            continue
        manifest_name = manifest_path.name
        package = available.get(package_name)
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if set(manifest) != {"schema", "version", "assetName", "sha256", "size", "patches"}:
            raise RuntimeError(f"Invalid update manifest fields: {manifest_name}")
        if (manifest["schema"] != 1 or manifest["version"] != release_version()
                or manifest["assetName"] != package_name
                or not isinstance(manifest["size"], int) or not 0 < manifest["size"] <= 16 * 1024 ** 3
                or not isinstance(manifest["sha256"], str)
                or re.fullmatch(r"[0-9a-f]{64}", manifest["sha256"]) is None
                or not isinstance(manifest["patches"], list) or len(manifest["patches"]) > 64):
            raise RuntimeError(f"Update manifest does not match package: {manifest_name}")
        if package is not None and (manifest["size"] != package.stat().st_size
                                    or manifest["sha256"] != file_sha256(package)):
            raise RuntimeError(f"Update manifest does not match package: {manifest_name}")
        if package is None and not manifest["patches"]:
            raise RuntimeError(f"Patch-only target has no patches: {manifest_name}")
        allowed.add(manifest_name)
        bases = set()
        for patch in manifest["patches"]:
            if set(patch) != {"baseVersion", "baseSha256", "baseSize", "assetName", "sha256", "size"}:
                raise RuntimeError(f"Invalid patch declaration: {manifest_name}")
            base_hash = patch["baseSha256"]
            if (re.fullmatch(r"[0-9a-f]{64}", base_hash) is None or base_hash in bases
                    or patch["baseSize"] <= 0 or not 0 < patch["size"] < manifest["size"]
                    or RELEASE_VERSION_PATTERN.fullmatch(patch["baseVersion"]) is None
                    or patch["assetName"] != f"{package_name}.from-{base_hash}.patch.gz"):
                raise RuntimeError(f"Invalid patch identity: {manifest_name}")
            bases.add(base_hash)
            delta = available.get(patch["assetName"])
            if delta is None or delta.stat().st_size != patch["size"] or file_sha256(delta) != patch["sha256"]:
                raise RuntimeError(f"Patch is missing or corrupt: {patch['assetName']}")
            allowed.add(delta.name)
    invalid = [path.name for path in assets if path.name not in allowed]
    if invalid:
        raise RuntimeError(f"Release assets do not match the current versioned naming rule: {invalid}")


# Hashes staged artifacts without loading large installers into memory.
def file_sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()
