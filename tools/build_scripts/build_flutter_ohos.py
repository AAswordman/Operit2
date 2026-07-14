#!/usr/bin/env python3
import argparse
import io
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import time
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    REPO_ROOT,
    copy_required_file,
    dart_pub_get,
    flutter_command,
    host_platform,
    node_package_command,
    prepare_web_access_embedded_assets,
    read_properties,
    run,
)


OHOS_PROJECT_DIR = FLUTTER_APP_DIR / "ohos"
OHOS_LOCAL_PROPERTIES = OHOS_PROJECT_DIR / "local.properties"
OHOS_BUILD_PROFILE = OHOS_PROJECT_DIR / "build-profile.json5"
OHOS_ROOT_PACKAGE = OHOS_PROJECT_DIR / "oh-package.json5"
OHOS_ENTRY_PACKAGE = OHOS_PROJECT_DIR / "entry" / "oh-package.json5"
OHOS_HAR_DIR = OHOS_PROJECT_DIR / "har"
OHOS_STAGED_PLUGINS_DIR = OHOS_PROJECT_DIR / ".flutter_ohos_plugins"
FLUTTER_PLUGINS_DEPENDENCIES = FLUTTER_APP_DIR / ".flutter-plugins-dependencies"
OHOS_HAP_PATH = OHOS_PROJECT_DIR / "entry" / "build" / "default" / "outputs" / "default" / "entry-default-signed.hap"
OHOS_UNSIGNED_HAP_PATH = OHOS_PROJECT_DIR / "entry" / "build" / "default" / "outputs" / "default" / "entry-default-unsigned.hap"
OHOS_ENTRY_LIB_DIR = OHOS_PROJECT_DIR / "entry" / "libs" / "arm64-v8a"
OHOS_BRIDGE_CRATE_DIR = FLUTTER_APP_DIR.parent / "native" / "operit-flutter-bridge"
OHOS_OHPM_ADAPTER = REPO_ROOT / "tools" / "build_scripts" / "ohpm_hvigor_adapter.cmd"
OHOS_SIGNING_DIR = OHOS_PROJECT_DIR / "signing" / "openharmony"
OHOS_RELEASE_SIGNING_PROPERTIES = REPO_ROOT / "tools" / "release" / "secrets" / "ohos-signing" / "ohos-signing.properties"
OHOS_RUST_TARGET = "aarch64-unknown-linux-ohos"
OHOS_CLANG_TARGET = "aarch64-linux-ohos"
OHOS_BRIDGE_LIBRARY_NAME = "liboperit_flutter_bridge.so"
LIBCLANG_PACKAGE_VERSION = "21.1.8"
OHOS_REQUIRED_SDK_COMPONENTS = ("ets", "js", "native", "previewer", "toolchains")
OHOS_SIGNING_SECRET_OPTIONS = {"-keyPwd", "-keystorePwd"}


@dataclass(frozen=True)
class OhosSigningConfig:
    app_store_file: Path
    app_store_password: str
    app_key_alias: str
    app_key_password: str
    app_cert_file: Path
    profile_store_file: Path
    profile_store_password: str
    profile_key_alias: str
    profile_key_password: str
    profile_cert_file: Path


class ValueEnum(str, Enum):
    # Returns the raw enum value for command-line interpolation.
    def __str__(self):
        return self.value


class OhosTargetPlatform(ValueEnum):
    ARM64 = "ohos-arm64"


# Parses command-line options for the OpenHarmony Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 OpenHarmony Flutter app.")
    parser.add_argument(
        "--output",
        type=Path,
        default=DIST_DIR / "operit2-app-ohos-arm64.hap",
    )
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the signed OpenHarmony HAP through the FVM-selected Flutter SDK.
def main() -> int:
    args = parse_args()
    prepare_web_access_embedded_assets()
    if not OHOS_PROJECT_DIR.is_dir():
        raise RuntimeError(
            "OpenHarmony Flutter project not found at "
            f"{OHOS_PROJECT_DIR}. Generate it with the OpenHarmony Flutter SDK."
        )
    verify_ohos_sdk_preflight()
    build_ohos_rust_bridge()
    flutter = flutter_command()
    dart_pub_get(enforce_lockfile=args.enforce_lockfile)
    prepare_ohos_package_dependencies()
    env = ohos_hvigor_env()
    build_unsigned_ohos_hap(flutter, args, env)
    sign_ohos_hap()
    copy_required_file(OHOS_HAP_PATH, args.output)
    print(f"OpenHarmony HAP: {args.output}", flush=True)
    return 0


# Builds the unsigned OpenHarmony HAP with Flutter and Hvigor.
def build_unsigned_ohos_hap(flutter: Path | str, args: argparse.Namespace, env: dict[str, str]) -> None:
    command = (
        [
            flutter,
            "build",
            "hap",
            "--release",
            "--no-pub",
            "--no-codesign",
            "--target-platform",
            OhosTargetPlatform.ARM64.value,
        ]
        + (["--build-name", args.build_name] if args.build_name else [])
        + (["--build-number", args.build_number] if args.build_number else [])
    )
    print("+ " + " ".join(str(part) for part in command), flush=True)
    merged_env = os.environ.copy()
    merged_env.update(env)
    completed = subprocess.run([str(part) for part in command], cwd=FLUTTER_APP_DIR, env=merged_env)
    if completed.returncode != 0:
        raise subprocess.CalledProcessError(completed.returncode, [str(part) for part in command])
    if not OHOS_UNSIGNED_HAP_PATH.is_file():
        raise RuntimeError(f"OpenHarmony unsigned HAP was not produced: {OHOS_UNSIGNED_HAP_PATH}")


# Signs the unsigned OpenHarmony HAP with project release signing material.
def sign_ohos_hap() -> None:
    signing = ohos_release_signing_config()
    sdk_toolchains_lib = ohos_sdk_toolchains_lib_dir()
    sign_tool = sdk_toolchains_lib / "hap-sign-tool.jar"
    profile_template = sdk_toolchains_lib / "UnsgnedReleasedProfileTemplate.json"
    unsigned_profile_file = OHOS_SIGNING_DIR / "operit_release_profile.json"
    signed_profile_file = OHOS_SIGNING_DIR / "operit_release_profile.p7b"
    OHOS_SIGNING_DIR.mkdir(parents=True, exist_ok=True)
    write_openharmony_unsigned_profile(profile_template, signing.app_cert_file, unsigned_profile_file)
    run_ohos_signing_command(
        [
            "java",
            "-jar",
            sign_tool,
            "sign-profile",
            "-mode",
            "localSign",
            "-keyAlias",
            signing.profile_key_alias,
            "-keyPwd",
            signing.profile_key_password,
            "-profileCertFile",
            signing.profile_cert_file,
            "-inFile",
            unsigned_profile_file,
            "-signAlg",
            "SHA256withECDSA",
            "-keystoreFile",
            signing.profile_store_file,
            "-keystorePwd",
            signing.profile_store_password,
            "-outFile",
            signed_profile_file,
        ]
    )
    if OHOS_HAP_PATH.exists():
        OHOS_HAP_PATH.unlink()
    run_ohos_signing_command(
        [
            "java",
            "-jar",
            sign_tool,
            "sign-app",
            "-mode",
            "localSign",
            "-keyAlias",
            signing.app_key_alias,
            "-keyPwd",
            signing.app_key_password,
            "-appCertFile",
            signing.app_cert_file,
            "-profileFile",
            signed_profile_file,
            "-inFile",
            OHOS_UNSIGNED_HAP_PATH,
            "-signAlg",
            "SHA256withECDSA",
            "-keystoreFile",
            signing.app_store_file,
            "-keystorePwd",
            signing.app_store_password,
            "-outFile",
            OHOS_HAP_PATH,
            "-compatibleVersion",
            ohos_project_api_version(),
            "-signCode",
            "1",
        ]
    )


# Runs one OpenHarmony signing command while masking secret argument values in logs.
def run_ohos_signing_command(command: list[str | Path]) -> None:
    print("+ " + " ".join(mask_ohos_signing_command(command)), flush=True)
    subprocess.run([str(part) for part in command], cwd=REPO_ROOT, check=True)


# Returns a command string list with signing passwords hidden.
def mask_ohos_signing_command(command: list[str | Path]) -> list[str]:
    masked: list[str] = []
    hide_next = False
    for part in command:
        value = str(part)
        if hide_next:
            masked.append("<redacted>")
            hide_next = False
        else:
            masked.append(value)
            hide_next = value in OHOS_SIGNING_SECRET_OPTIONS
    return masked


# Reads the project OpenHarmony release signing configuration.
def ohos_release_signing_config() -> OhosSigningConfig:
    if not OHOS_RELEASE_SIGNING_PROPERTIES.is_file():
        raise RuntimeError(f"OpenHarmony release signing properties not found: {OHOS_RELEASE_SIGNING_PROPERTIES}")
    signing = read_properties(OHOS_RELEASE_SIGNING_PROPERTIES)
    return OhosSigningConfig(
        app_store_file=ohos_required_signing_path(signing, "OHOS_APP_STORE_FILE"),
        app_store_password=ohos_required_signing_value(signing, "OHOS_APP_STORE_PASSWORD"),
        app_key_alias=ohos_required_signing_value(signing, "OHOS_APP_KEY_ALIAS"),
        app_key_password=ohos_required_signing_value(signing, "OHOS_APP_KEY_PASSWORD"),
        app_cert_file=ohos_required_signing_path(signing, "OHOS_APP_CERT_FILE"),
        profile_store_file=ohos_required_signing_path(signing, "OHOS_PROFILE_STORE_FILE"),
        profile_store_password=ohos_required_signing_value(signing, "OHOS_PROFILE_STORE_PASSWORD"),
        profile_key_alias=ohos_required_signing_value(signing, "OHOS_PROFILE_KEY_ALIAS"),
        profile_key_password=ohos_required_signing_value(signing, "OHOS_PROFILE_KEY_PASSWORD"),
        profile_cert_file=ohos_required_signing_path(signing, "OHOS_PROFILE_CERT_FILE"),
    )


# Reads one required OpenHarmony signing property.
def ohos_required_signing_value(signing: dict[str, str], key: str) -> str:
    value = signing.get(key)
    if not value:
        raise RuntimeError(f"OpenHarmony release signing property missing from {OHOS_RELEASE_SIGNING_PROPERTIES}: {key}")
    return value


# Reads one required OpenHarmony signing path and verifies that it exists.
def ohos_required_signing_path(signing: dict[str, str], key: str) -> Path:
    path = Path(ohos_required_signing_value(signing, key))
    if not path.is_absolute():
        path = OHOS_RELEASE_SIGNING_PROPERTIES.parent / path
    if not path.is_file():
        raise RuntimeError(f"OpenHarmony release signing file missing for {key}: {path}")
    return path


# Writes a bundle-specific unsigned OpenHarmony provision profile.
def write_openharmony_unsigned_profile(template_file: Path, app_cert_file: Path, out_file: Path) -> None:
    profile = json.loads(template_file.read_text(encoding="utf-8"))
    now = int(time.time())
    profile["validity"]["not-before"] = now - 86400
    profile["validity"]["not-after"] = now + (86400 * 3650)
    profile["bundle-info"]["bundle-name"] = ohos_bundle_name()
    if profile["type"] == "release":
        profile["bundle-info"]["distribution-certificate"] = app_cert_file.read_text(encoding="utf-8")
    else:
        profile["bundle-info"]["development-certificate"] = app_cert_file.read_text(encoding="utf-8")
    out_file.write_text(json.dumps(profile, indent=2), encoding="utf-8")


# Returns the SDK toolchains lib directory containing OpenHarmony signing material.
def ohos_sdk_toolchains_lib_dir() -> Path:
    sdk_version = select_ohos_sdk_version(ohos_sdk_dir())
    lib_dir = ohos_sdk_dir() / sdk_version / "toolchains" / "lib"
    if not lib_dir.is_dir():
        raise RuntimeError(f"OpenHarmony signing tools directory is missing: {lib_dir}")
    return lib_dir


# Reads the app bundle name from the OpenHarmony AppScope configuration.
def ohos_bundle_name() -> str:
    app_scope = OHOS_PROJECT_DIR / "AppScope" / "app.json5"
    config = read_json_package(app_scope)
    bundle_name = config.get("app", {}).get("bundleName")
    if not bundle_name:
        raise RuntimeError(f"OpenHarmony bundleName is missing in {app_scope}")
    return bundle_name


# Prepares Flutter HAR files and OHOS package links consumed by Hvigor.
def prepare_ohos_package_dependencies() -> None:
    copy_ohos_flutter_hars()
    clear_ohos_flutter_embedding_package_cache()
    plugin_names = stage_ohos_flutter_plugins()
    write_ohos_package_dependencies(plugin_names)


# Copies the Flutter engine HAR files required by a release arm64 HAP.
def copy_ohos_flutter_hars() -> None:
    engine_dir = (
        FLUTTER_APP_DIR
        / ".fvm"
        / "flutter_sdk"
        / "bin"
        / "cache"
        / "artifacts"
        / "engine"
        / "ohos-arm64-release"
    )
    required_hars = ("flutter_embedding_release.har", "arm64_v8a_release.har")
    OHOS_HAR_DIR.mkdir(parents=True, exist_ok=True)
    for filename in required_hars:
        copy_required_file(engine_dir / filename, OHOS_HAR_DIR / filename)
    patch_ohos_flutter_embedding_har(OHOS_HAR_DIR / "flutter_embedding_release.har")


# Clears the unpacked Flutter embedding package so ohpm expands the patched HAR.
def clear_ohos_flutter_embedding_package_cache() -> None:
    package_cache = OHOS_PROJECT_DIR / "oh_modules" / ".ohpm" / "@ohos+flutter_ohos@file+har+flutter_embedding_release.har"
    package_link = OHOS_PROJECT_DIR / "oh_modules" / "@ohos" / "flutter_ohos"
    if package_link.exists() or package_link.is_symlink():
        attributes = package_link.lstat().st_file_attributes
        if attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT:
            package_link.rmdir()
        elif package_link.is_dir():
            shutil.rmtree(package_link)
        else:
            package_link.unlink()
    if package_cache.exists():
        shutil.rmtree(package_cache)


# Patches the Flutter OHOS embedding HAR to compile against the local OpenHarmony SDK API surface.
def patch_ohos_flutter_embedding_har(har_path: Path) -> None:
    replacements = ohos_flutter_embedding_replacements()
    patched_members: set[str] = set()
    source_members: list[tuple[tarfile.TarInfo, bytes]] = []
    with tarfile.open(har_path, "r:gz") as source_har:
        for member in source_har.getmembers():
            file_object = source_har.extractfile(member) if member.isfile() else None
            data = file_object.read() if file_object is not None else b""
            if member.name in replacements:
                text = data.decode("utf-8")
                for old, new in replacements[member.name]:
                    text = replace_required(text, old, new, member.name)
                data = text.encode("utf-8")
                member.size = len(data)
                patched_members.add(member.name)
            source_members.append((member, data))
    missing_members = set(replacements) - patched_members
    if missing_members:
        raise RuntimeError(f"Flutter OHOS embedding HAR layout changed, missing members: {sorted(missing_members)}")
    rebuilt_har = har_path.with_suffix(har_path.suffix + ".tmp")
    with tarfile.open(rebuilt_har, "w:gz") as target_har:
        for member, data in source_members:
            if member.isfile():
                target_har.addfile(member, io.BytesIO(data))
            else:
                target_har.addfile(member)
    rebuilt_har.replace(har_path)


# Returns source patches for Flutter OHOS embedding files that reference newer SDK APIs.
def ohos_flutter_embedding_replacements() -> dict[str, list[tuple[str, str]]]:
    return {
        "package/src/main/ets/plugin/editing/OhosAutoFillHelper.ets": [
            ("  autoFillType: autoFillManager.AutoFillType;", "  autoFillType: number;"),
            (
                "  private static toSdkAutoFillType(value: number): autoFillManager.AutoFillType {\n"
                "    return value as autoFillManager.AutoFillType;\n"
                "  }",
                "  private static toSdkAutoFillType(value: number): number {\n"
                "    return value;\n"
                "  }",
            ),
            (
                "  private static toManagerViewData(viewData: OhosViewData): autoFillManager.ViewData {",
                "  private static toManagerViewData(viewData: OhosViewData): OhosViewData {",
            ),
            (
                "    return managerViewData as autoFillManager.ViewData;",
                "    return managerViewData as OhosViewData;",
            ),
            (
                "  static parseFillResult(viewData: autoFillManager.ViewData): Map<number, string> {",
                "  static parseFillResult(viewData: OhosViewData): Map<number, string> {",
            ),
            (
                "      triggerType: AUTOFILL_TRIGGER_AUTO_REQUEST as autoFillManager.AutoFillTriggerType,\n"
                "    } as autoFillManager.FillRequest;\n"
                "    const callback: autoFillManager.AutoFillCallback = {\n"
                "      onSuccess: (filledViewData: autoFillManager.ViewData): void => {",
                "      triggerType: AUTOFILL_TRIGGER_AUTO_REQUEST,\n"
                "    };\n"
                "    const callback = {\n"
                "      onSuccess: (filledViewData: OhosViewData): void => {",
            ),
            (
                "      onFailure: (result: autoFillManager.FillFailureResult): void => {",
                "      onFailure: (result: FillFailureResult): void => {",
            ),
            (
                "      autoFillManager.requestAutoFill(uiContext, fillRequest, callback);",
                "      (autoFillManager as ESObject).requestAutoFill(uiContext, fillRequest, callback);",
            ),
            (
                "      const businessError = error as BusinessError;",
                "      const businessError = error as ErrorPayload;",
            ),
            (
                "    const saveRequest: autoFillManager.SaveRequest = {\n"
                "      viewData: OhosAutoFillHelper.toManagerViewData(viewData),\n"
                "    };",
                "    const saveRequest = {\n"
                "      viewData: OhosAutoFillHelper.toManagerViewData(viewData),\n"
                "    };",
            ),
            (
                "      autoFillManager.requestAutoSave(uiContext, saveRequest, napiCallback);",
                "      (autoFillManager as ESObject).requestAutoSave(uiContext, saveRequest, napiCallback);",
            ),
            (
                "interface AutoFillCustomData {\n"
                "  data: Record<string, string>;\n"
                "}",
                "interface AutoFillCustomData {\n"
                "  data: Record<string, string>;\n"
                "}\n"
                "\n"
                "interface FillFailureResult {\n"
                "  errCode: number;\n"
                "}\n"
                "\n"
                "interface ErrorPayload {\n"
                "  code?: number;\n"
                "  message?: string;\n"
                "}",
            ),
        ],
        "package/src/main/ets/embedding/ohos/KeyEventHandler.ets": [
            (
                "      // Use event.isCapsLockOn to check CapsLock state\n"
                "      const isCapsLockOn = event.isCapsLockOn !== undefined ? event.isCapsLockOn : false;",
                "      // Uses the ArkUI modifier state API for CapsLock.\n"
                "      const isCapsLockOn = this.getModifierKeyStateSafe(event, ['CapsLock']);",
            ),
            (
                "          // Use event.isNumLockOn to check NumLock state\n"
                "          const isNumLockOn = event.isNumLockOn !== undefined ? event.isNumLockOn : true; // Default to true if not available",
                "          // Uses the ArkUI modifier state API for NumLock.\n"
                "          const isNumLockOn = this.getModifierKeyStateSafe(event, ['NumLock']);",
            ),
        ],
        "package/src/main/ets/embedding/ohos/EmbeddingNodeController.ets": [
            (
                "      (this.builderNode as ESObject)?.postInputEventWithStrategy(event, CompetitionStrategy.DEFAULT);",
                "      (this.builderNode as ESObject)?.postInputEventWithStrategy(event, 0);",
            ),
            (
                "      return (this.builderNode as ESObject)?.postInputEventWithStrategy(event, CompetitionStrategy.DEFAULT) as boolean;",
                "      return (this.builderNode as ESObject)?.postInputEventWithStrategy(event, 0) as boolean;\n"
            ),
        ],
        "package/src/main/ets/embedding/ohos/PiPVisibilityBridge.ets": [
            (
                "      const mode = await window.getGlobalWindowMode();\n"
                "      return (mode & window.GlobalWindowMode.PIP) !== 0;",
                "      const windowModule = window as ESObject;\n"
                "      const mode = await windowModule.getGlobalWindowMode();\n"
                "      const globalWindowMode = windowModule.GlobalWindowMode as ESObject;\n"
                "      return (mode & (globalWindowMode.PIP as number)) !== 0;",
            ),
        ],
    }


# Replaces an expected source fragment and reports the member when the fragment is absent.
def replace_required(text: str, old: str, new: str, member_name: str) -> str:
    if old not in text:
        raise RuntimeError(f"Flutter OHOS embedding source changed, patch fragment missing in {member_name}")
    return text.replace(old, new)


# Copies native OHOS Flutter plugin modules into the app project.
def stage_ohos_flutter_plugins() -> list[str]:
    if not FLUTTER_PLUGINS_DEPENDENCIES.is_file():
        raise RuntimeError(f"Flutter plugin dependency graph is missing: {FLUTTER_PLUGINS_DEPENDENCIES}")
    graph = json.loads(FLUTTER_PLUGINS_DEPENDENCIES.read_text(encoding="utf-8"))
    ohos_plugins = graph.get("plugins", {}).get("ohos", [])
    plugin_names: list[str] = []
    for plugin in ohos_plugins:
        if plugin.get("native_build") is False:
            continue
        name = plugin["name"]
        source = Path(plugin["path"]) / "ohos"
        destination = OHOS_STAGED_PLUGINS_DIR / name
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(source, destination)
        patch_staged_ohos_plugin_hvigorfile(destination)
        patch_staged_ohos_plugin_package(destination)
        plugin_names.append(name)
    return plugin_names


# Rewrites one staged plugin Hvigor file to load tasks from the active Hvigor installation.
def patch_staged_ohos_plugin_hvigorfile(plugin_dir: Path) -> None:
    hvigorfile = plugin_dir / "hvigorfile.ts"
    content = hvigorfile.read_text(encoding="utf-8")
    import_template = "import { harTasks } from '@ohos/hvigor-ohos-plugin';"
    export_template = "export { harTasks } from '@ohos/hvigor-ohos-plugin';"
    new = """import path from 'path'

declare const require: {
    (id: string): unknown
    main: { filename: string }
    resolve(id: string, options: { paths: string[] }): string
}

interface OhosHvigorPluginModule {
    harTasks: unknown
}

/** Loads a package from the current hvigorw installation. */
function requireActiveHvigorPackage<T>(packageName: string): T {
    const modulePath = require.resolve(packageName, {
        paths: [path.dirname(require.main.filename)]
    })
    return require(modulePath) as T
}

const { harTasks } = requireActiveHvigorPackage<OhosHvigorPluginModule>('@ohos/hvigor-ohos-plugin')"""
    if import_template in content:
        hvigorfile.write_text(content.replace(import_template, new), encoding="utf-8")
        return
    if export_template in content:
        hvigorfile.write_text(
            content.replace(export_template, f"{new}\n\nexport {{ harTasks }};"),
            encoding="utf-8",
        )
        return
    else:
        raise RuntimeError(f"Unsupported OHOS plugin Hvigor file: {hvigorfile}")


# Rewrites one staged plugin package to consume the app-level Flutter HAR override.
def patch_staged_ohos_plugin_package(plugin_dir: Path) -> None:
    package_file = plugin_dir / "oh-package.json5"
    content = package_file.read_text(encoding="utf-8")
    package_file.write_text(
        content.replace('"@ohos/flutter_ohos": "file:libs/flutter.har"', '"@ohos/flutter_ohos": ""'),
        encoding="utf-8",
    )


# Writes OHOS package dependencies and overrides for Flutter and staged plugins.
def write_ohos_package_dependencies(plugin_names: list[str]) -> None:
    root_config = read_json_package(OHOS_ROOT_PACKAGE)
    root_dependencies = root_config.get("dependencies", {})
    root_dependencies.update(
        {
            "@ohos/flutter_ohos": "file:./har/flutter_embedding_release.har",
            "flutter_native_arm64_v8a": "file:./har/arm64_v8a_release.har",
        }
    )
    for name in plugin_names:
        root_dependencies[name] = f"file:./.flutter_ohos_plugins/{name}"
    root_config["dependencies"] = root_dependencies

    root_overrides = root_config.get("overrides", {})
    root_overrides.update(
        {
            "@ohos/flutter_ohos": "file:./har/flutter_embedding_release.har",
            "flutter_native_arm64_v8a": "file:./har/arm64_v8a_release.har",
        }
    )
    for name in plugin_names:
        root_overrides[name] = f"file:./.flutter_ohos_plugins/{name}"
    root_config["overrides"] = root_overrides
    write_json_package(OHOS_ROOT_PACKAGE, root_config)

    entry_config = read_json_package(OHOS_ENTRY_PACKAGE)
    entry_dependencies = entry_config.get("dependencies", {})
    entry_dependencies.update(
        {
            "@ohos/flutter_ohos": "file:../har/flutter_embedding_release.har",
            "flutter_native_arm64_v8a": "file:../har/arm64_v8a_release.har",
        }
    )
    for name in plugin_names:
        entry_dependencies[name] = f"file:../.flutter_ohos_plugins/{name}"
    entry_config["dependencies"] = entry_dependencies
    write_json_package(OHOS_ENTRY_PACKAGE, entry_config)


# Reads one JSON-compatible OHOS package file.
def read_json_package(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


# Writes one JSON-compatible OHOS package file.
def write_json_package(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


# Returns the environment needed by Hvigor and included OHOS plugin modules.
def ohos_hvigor_env() -> dict[str, str]:
    env = os.environ.copy()
    npm = node_package_command("npm")
    completed = subprocess.run(
        [npm, "root", "-g"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    global_node_modules = completed.stdout.strip()
    if not global_node_modules:
        raise RuntimeError("npm root -g returned an empty path")
    existing_node_path = env.get("NODE_PATH")
    node_paths = [global_node_modules]
    if existing_node_path:
        node_paths.append(existing_node_path)
    env["NODE_PATH"] = os.pathsep.join(node_paths)
    sdk_dir = str(ohos_sdk_dir())
    env["DEVECO_SDK_HOME"] = sdk_dir
    env["OHOS_BASE_SDK_HOME"] = sdk_dir
    env["ohpmBin"] = str(OHOS_OHPM_ADAPTER)
    env["OHPM_REAL_BIN"] = str(ohpm_real_bin())
    return env


# Builds and stages the Rust OpenHarmony runtime bridge.
def build_ohos_rust_bridge() -> None:
    sdk_dir = ohos_sdk_dir()
    sdk_version = select_ohos_sdk_version(sdk_dir)
    native_dir = sdk_dir / sdk_version / "native"
    llvm_bin = native_dir / "llvm" / "bin"
    sysroot = native_dir / "sysroot"
    clang = executable_path(llvm_bin / "clang")
    llvm_ar = executable_path(llvm_bin / "llvm-ar")
    libclang_dir = ensure_bindgen_libclang()
    clang_resource_dir = ohos_clang_resource_dir(native_dir)
    required_paths = [native_dir, sysroot, clang, llvm_ar, libclang_dir, clang_resource_dir]
    missing = [path for path in required_paths if not path.exists()]
    if missing:
        raise RuntimeError("OpenHarmony native SDK is incomplete: " + ", ".join(str(path) for path in missing))
    verify_ohos_rust_link_libraries(sysroot)

    env = os.environ.copy()
    sysroot_arg = sysroot.as_posix()
    clang_resource_arg = clang_resource_dir.as_posix()
    bindgen_args = (
        f"--target={OHOS_CLANG_TARGET} "
        f"--sysroot={sysroot_arg} "
        f"-resource-dir={clang_resource_arg}"
    )
    cflags = f"--target={OHOS_CLANG_TARGET} --sysroot={sysroot_arg}"
    env.update(
        {
            "RUSTFLAGS": (
                "-Awarnings "
                f"-C link-arg=--target={OHOS_CLANG_TARGET} "
                f"-C link-arg=--sysroot={sysroot_arg}"
            ),
            "CC_aarch64_unknown_linux_ohos": str(clang),
            "CXX_aarch64_unknown_linux_ohos": str(executable_path(llvm_bin / "clang++")),
            "AR_aarch64_unknown_linux_ohos": str(llvm_ar),
            "CFLAGS_aarch64_unknown_linux_ohos": cflags,
            "CXXFLAGS_aarch64_unknown_linux_ohos": cflags,
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER": str(clang),
            "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_AR": str(llvm_ar),
            "BINDGEN_EXTRA_CLANG_ARGS": bindgen_args,
            "BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_ohos": bindgen_args,
            "LIBCLANG_PATH": str(libclang_dir),
        }
    )
    run(
        [
            "cargo",
            "build",
            "--release",
            "--manifest-path",
            OHOS_BRIDGE_CRATE_DIR / "Cargo.toml",
            "--target",
            OHOS_RUST_TARGET,
        ],
        cwd=OHOS_BRIDGE_CRATE_DIR,
        env=env,
    )
    bridge_library = (
        OHOS_BRIDGE_CRATE_DIR
        / "target"
        / OHOS_RUST_TARGET
        / "release"
        / OHOS_BRIDGE_LIBRARY_NAME
    )
    copy_required_file(bridge_library, OHOS_ENTRY_LIB_DIR / OHOS_BRIDGE_LIBRARY_NAME)


# Returns the Clang resource directory shipped by the configured native SDK.
def ohos_clang_resource_dir(native_dir: Path) -> Path:
    clang_root = native_dir / "llvm" / "lib" / "clang"
    if not clang_root.exists():
        raise RuntimeError(f"OpenHarmony native SDK is incomplete: {clang_root}")
    versions = sorted(path for path in clang_root.iterdir() if path.is_dir())
    if len(versions) != 1:
        listed = ", ".join(str(path) for path in versions) or "none"
        raise RuntimeError(f"OpenHarmony native SDK must expose exactly one Clang resource directory: {listed}")
    return versions[0]


# Returns the OpenHarmony SDK directory from local.properties.
def ohos_sdk_dir() -> Path:
    properties = read_properties(OHOS_LOCAL_PROPERTIES)
    raw_path = properties.get("hwsdk.dir")
    if not raw_path:
        raise RuntimeError(f"OpenHarmony SDK path is missing in {OHOS_LOCAL_PROPERTIES}")
    return Path(raw_path.replace("\\\\", "\\"))


# Selects the OpenHarmony SDK version that can link the Rust native bridge.
def select_ohos_sdk_version(sdk_dir: Path) -> str:
    installed_versions = sorted(
        path.name for path in sdk_dir.iterdir() if path.is_dir() and path.name.isdigit()
    )
    requested_versions = ohos_requested_sdk_versions()
    candidates = unique_values([*requested_versions, *installed_versions])
    checked: list[str] = []
    missing_link_libraries: list[str] = []
    for version in candidates:
        native_dir = sdk_dir / version / "native"
        if not native_dir.is_dir():
            checked.append(f"{version}: missing native SDK directory")
            continue
        sysroot = native_dir / "sysroot"
        missing = missing_ohos_rust_link_libraries(sysroot)
        if missing:
            missing_link_libraries.extend(str(path) for path in missing)
            checked.append(f"{version}: missing {', '.join(path.name for path in missing)}")
            continue
        return version
    details = "; ".join(checked) if checked else "no numeric SDK versions are installed"
    raise RuntimeError(
        ohos_sdk_link_error_message(sdk_dir, requested_versions, installed_versions, details)
    )


# Verifies that the configured OpenHarmony SDK matches the project API and native linker requirements.
def verify_ohos_sdk_preflight() -> None:
    sdk_dir = ohos_sdk_dir()
    verify_ohos_hvigor_sdk_components(sdk_dir)
    select_ohos_sdk_version(sdk_dir)


# Verifies SDK components required by the HarmonyOS Hvigor project configuration.
def verify_ohos_hvigor_sdk_components(sdk_dir: Path) -> None:
    requested_versions = ohos_requested_sdk_versions()
    missing: list[str] = []
    for version in requested_versions:
        version_dir = sdk_dir / version
        for component in OHOS_REQUIRED_SDK_COMPONENTS:
            component_dir = version_dir / component
            if not component_dir.is_dir():
                missing.append(f"{version}/{component}")
    if missing:
        installed_versions = sorted(
            path.name for path in sdk_dir.iterdir() if path.is_dir() and path.name.isdigit()
        )
        requested = ", ".join(requested_versions) if requested_versions else "none"
        installed = ", ".join(installed_versions) if installed_versions else "none"
        components = ", ".join(OHOS_REQUIRED_SDK_COMPONENTS)
        raise RuntimeError(
            f"HarmonyOS SDK components are incomplete for Hvigor: missing {', '.join(missing)}. "
            f"Project requested API versions: {requested}. Installed numeric SDK versions under "
            f"{sdk_dir}: {installed}. Install HarmonyOS SDK API {requested} with components: {components}."
        )


# Builds the OpenHarmony SDK native linker error with project and installed SDK context.
def ohos_sdk_link_error_message(
    sdk_dir: Path,
    requested_versions: list[str],
    installed_versions: list[str],
    details: str,
) -> str:
    requested = ", ".join(requested_versions) if requested_versions else "none"
    installed = ", ".join(installed_versions) if installed_versions else "none"
    required_libraries = ", ".join(OHOS_REQUIRED_LINK_LIBRARY_NAMES)
    return (
        f"OpenHarmony SDK cannot link Rust target {OHOS_RUST_TARGET}: {details}. "
        f"Project requested API versions: {requested}. Installed numeric SDK versions under "
        f"{sdk_dir}: {installed}. Install an OpenHarmony native SDK containing {required_libraries}."
    )


# Returns SDK versions named by environment and project build metadata.
def ohos_requested_sdk_versions() -> list[str]:
    versions: list[str] = []
    env_version = os.environ.get("OHOS_SDK_VERSION")
    if env_version:
        versions.append(env_version)
    api_version = ohos_project_api_version()
    if api_version:
        versions.append(api_version)
    return versions


# Reads the numeric OpenHarmony API version from build-profile.json5.
def ohos_project_api_version() -> str | None:
    if not OHOS_BUILD_PROFILE.is_file():
        return None
    content = OHOS_BUILD_PROFILE.read_text(encoding="utf-8")
    string_match = re.search(r'"compatibleSdkVersion"\s*:\s*"[^"]*\((\d+)\)"', content)
    if string_match is not None:
        return string_match.group(1)
    integer_match = re.search(r'"compatibleSdkVersion"\s*:\s*(\d+)', content)
    if integer_match is not None:
        return integer_match.group(1)
    return None


# Returns values without duplicates while preserving source order.
def unique_values(values: list[str]) -> list[str]:
    seen: set[str] = set()
    unique: list[str] = []
    for value in values:
        if value in seen:
            continue
        seen.add(value)
        unique.append(value)
    return unique


# Returns the platform executable path.
def executable_path(path: Path) -> Path:
    if os.name == "nt":
        return path.with_suffix(".exe")
    return path


# Returns the real ohpm executable used behind the Hvigor adapter.
def ohpm_real_bin() -> Path:
    ohpm = shutil.which("ohpm")
    if not ohpm:
        raise RuntimeError("ohpm executable is not available on PATH")
    return Path(ohpm)


# Verifies libraries required by Rust's OpenHarmony target are present in the SDK.
def verify_ohos_rust_link_libraries(sysroot: Path) -> None:
    missing = missing_ohos_rust_link_libraries(sysroot)
    if missing:
        raise RuntimeError(
            "OpenHarmony SDK cannot link Rust target "
            f"{OHOS_RUST_TARGET}; missing: {', '.join(str(path) for path in missing)}"
        )


OHOS_REQUIRED_LINK_LIBRARY_NAMES = ("libtime_service_ndk.so",)


# Returns missing Rust link libraries for one OpenHarmony sysroot.
def missing_ohos_rust_link_libraries(sysroot: Path) -> list[Path]:
    lib_dir = sysroot / "usr" / "lib" / OHOS_CLANG_TARGET
    required_libraries = [lib_dir / name for name in OHOS_REQUIRED_LINK_LIBRARY_NAMES]
    return [path for path in required_libraries if not path.exists()]


# Ensures bindgen can load libclang on the current build host.
def ensure_bindgen_libclang() -> Path:
    platform_name = host_platform()
    if platform_name != "windows":
        raise RuntimeError(f"OpenHarmony bindgen libclang is not configured for {platform_name}")
    package_name = f"libclang.runtime.win-x64.{LIBCLANG_PACKAGE_VERSION}"
    tools_dir = REPO_ROOT / "target" / "operit-build-tools"
    package_dir = tools_dir / package_name
    libclang_dir = package_dir / "runtimes" / "win-x64" / "native"
    libclang = libclang_dir / "libclang.dll"
    if not libclang.exists():
        archive = tools_dir / f"{package_name}.nupkg"
        tools_dir.mkdir(parents=True, exist_ok=True)
        run(
            [
                "curl",
                "--fail",
                "--location",
                "--retry",
                "5",
                "--retry-all-errors",
                f"https://www.nuget.org/api/v2/package/libclang.runtime.win-x64/{LIBCLANG_PACKAGE_VERSION}",
                "--output",
                str(archive),
            ]
        )
        if package_dir.exists():
            import shutil

            shutil.rmtree(package_dir)
        package_dir.mkdir(parents=True, exist_ok=True)
        run(["tar", "-xf", str(archive), "-C", str(package_dir)])
    if not libclang.exists():
        raise RuntimeError(f"libclang.dll was not prepared at {libclang}")
    return libclang_dir


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
