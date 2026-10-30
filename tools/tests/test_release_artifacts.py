"""Exercise versioned packaging paths without compiling platform applications."""

import importlib.util
from contextlib import ExitStack
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile
import fnmatch
import re


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/build_scripts"))
import build_apple_release
import build_flutter_ios
import build_flutter_ohos
import build_local
import cli_common
import common
import release_artifacts

SPEC = importlib.util.spec_from_file_location(
    "release_build_under_test", REPO_ROOT / "tools/release/build_release.py"
)
release_build = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = release_build
SPEC.loader.exec_module(release_build)
sys.path.insert(0, str(REPO_ROOT / "tools/release"))
import download_action_artifacts
import publish_dist

TARGETS = (
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


class ReleaseArtifactTests(unittest.TestCase):
    """Verify real filename consumers, version validation, and ZIP creation."""

    # Creates isolated manifests shared by every packaging entry point.
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        patcher = patch.object(release_artifacts, "REPO_ROOT", self.root)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.write_versions("2.0.0-preview.16", "2.0.0-preview.16")

    # Writes explicit App and CLI version sources for each scenario.
    def write_versions(self, app_version, cli_version):
        for relative, version in zip(
            release_artifacts.RELEASE_MANIFESTS, (app_version, cli_version)
        ):
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f'[package]\nversion = "{version}"\n', encoding="utf-8")

    # Preserves the full stable, preview, RC, and development release versions.
    def test_names_every_packaged_target_with_full_version(self):
        for version in ("2.0.0", "2.0.0-preview.16", "2.0.0-rc.1", "2.0.0-dev.20261020"):
            self.write_versions(version, version)
            for product, platform, arch, extension in TARGETS:
                with self.subTest(version=version, target=(product, platform, arch)):
                    self.assertEqual(
                        release_artifacts.release_asset_name(product, platform, arch, extension),
                        f"operit2-{product}-{platform}-{arch}"
                        f"{'-sandbox' if (product, platform) == ('app', 'macos') else ''}-{version}.{extension}",
                    )

    # Rejects missing, mismatched, malformed, and metadata-only release versions.
    def test_rejects_invalid_version_sources(self):
        self.write_versions("2.0.0-preview.16", "2.0.0")
        with self.assertRaisesRegex(ValueError, "versions differ"):
            release_artifacts.release_version()
        for version in ("2.0", "v2.0.0", "2.0.0-preview.01", "2.0.0+20", "../2.0.0"):
            self.write_versions(version, version)
            with self.subTest(version=version), self.assertRaises(ValueError):
                release_artifacts.release_version()
        (self.root / release_artifacts.RELEASE_MANIFESTS[0]).unlink()
        with self.assertRaises(FileNotFoundError):
            release_artifacts.release_version()

    # Rejects path components rather than writing outside the release directory.
    def test_rejects_invalid_filename_components(self):
        for target in (("app", "../linux", "x86_64", "zip"), ("cli", "linux", "x86/64", "tar.gz")):
            with self.subTest(target=target), self.assertRaises(ValueError):
                release_artifacts.release_asset_name(*target)

    # Ensures ZIP writers retain dots in the complete release version.
    def test_zip_writers_preserve_versioned_filenames_and_contents(self):
        source = self.root / "bundle"
        source.mkdir()
        (source / "payload.txt").write_text("release payload", encoding="utf-8")
        for index, writer in enumerate((common.compress_zip, release_build.compress_zip)):
            for version in ("2.0.0", "2.0.0-preview.16"):
                destination = self.root / f"operit2-app-windows-x86_64-{version}-{index}.zip"
                with self.subTest(writer=writer.__module__, version=version):
                    writer(source, destination)
                    self.assertTrue(destination.is_file())
                    with zipfile.ZipFile(destination) as archive:
                        self.assertEqual(archive.read("payload.txt"), b"release payload")
                    self.assertFalse(destination.with_suffix(".zip.sha256").exists())

    # Keeps current-host and release CLI entry points on the same versioned names.
    def test_cli_entry_points_share_names_for_every_architecture(self):
        for platform in ("windows", "linux", "macos"):
            for arch in ("x86_64", "aarch64"):
                with self.subTest(platform=platform, arch=arch):
                    target = cli_common.cli_target(platform, arch)
                    name = release_artifacts.release_asset_name(
                        "cli", platform, arch, cli_common.cli_archive_extension(platform)
                    )
                    self.assertEqual(cli_common.cli_package_path(target).name, name)
                    release_target = release_build.CliBuildTarget(
                        release_build.HostPlatform(platform), arch, target.rust_target
                    )
                    self.assertEqual(release_build.cli_package_path(release_target).name, name)

    # Keeps local App paths and mobile argument defaults on the shared scheme.
    def test_app_entry_points_share_versioned_default_names(self):
        for platform, arch, extension in (
            ("windows", "x86_64", "zip"), ("linux", "aarch64", "tar.gz"),
            ("macos", "universal", "zip"),
        ):
            self.assertEqual(
                build_local.local_app_archive_path(platform, arch).name,
                release_artifacts.release_asset_name("app", platform, arch, extension),
            )
        with patch.object(sys, "argv", ["build_flutter_ios.py"]):
            self.assertEqual(
                build_flutter_ios.parse_args().archive_path.name,
                release_artifacts.release_asset_name("app", "ios", "arm64", "zip"),
            )
        # OHOS resolves the target-dependent output in main, after parsing.
        with ExitStack() as stack:
            stack.enter_context(patch.object(sys, "argv", ["build_flutter_ohos.py"]))
            stack.enter_context(patch.object(build_flutter_ohos, "OHOS_PROJECT_DIR", self.root))
            for operation in (
                "verify_ohos_sdk_preflight", "prepare_plugin_assets", "build_ohos_rust_bridge",
                "ohos_hvigor_env", "flutter_pub_get", "prepare_ohos_package_dependencies",
                "build_unsigned_ohos_hap", "verify_ohos_hap_native_libraries",
                "package_ohos_terminal_hnp", "sign_ohos_hap",
            ):
                stack.enter_context(patch.object(build_flutter_ohos, operation))
            copy = stack.enter_context(patch.object(build_flutter_ohos, "copy_required_file"))
            self.assertEqual(build_flutter_ohos.main(), 0)
            self.assertEqual(
                copy.call_args.args[1].name,
                release_artifacts.release_asset_name("app", "ohos", "arm64", "hap"),
            )

    # Copies only the requested current-version Apple CLI packages.
    def test_apple_copy_excludes_stale_versions_and_other_architectures(self):
        source = self.root / "source"
        output = self.root / "output"
        source.mkdir()
        expected = release_artifacts.release_asset_name("cli", "macos", "aarch64", "tar.gz")
        (source / expected).write_bytes(b"current package")
        (source / "operit2-cli-macos-aarch64-1.0.0.tar.gz").write_bytes(b"old package")
        (source / release_artifacts.release_asset_name("cli", "macos", "x86_64", "tar.gz")).write_bytes(b"other arch")
        with patch.object(build_apple_release, "DIST_DIR", source), patch.object(build_apple_release, "run"):
            build_apple_release.build_apple_cli("aarch64", output)
        self.assertEqual([path.name for path in output.iterdir()], [expected])

    # Ensures the WSL command writes the same versioned Linux asset as native builds.
    def test_wsl_app_archive_uses_shared_release_name(self):
        with (
            patch.object(release_build, "wsl_check_command", return_value=True),
            patch.object(release_build, "windows_path_to_wsl", side_effect=lambda path: str(path)),
            patch.object(release_build, "wsl_run") as run,
        ):
            release_build.build_wsl_linux_app("test", "2.0.0", "20")
        command = run.call_args.args[1]
        expected = release_artifacts.release_asset_name("app", "linux", "x86_64", "tar.gz")
        self.assertIn(expected, command)
        self.assertNotIn("/operit2-app-linux-x86_64.tar.gz", command)


    # Collects all current-version artifacts while excluding stale packages and sidecars.
    def test_actions_collection_accepts_every_versioned_target(self):
        download = self.root / "download"
        download.mkdir()
        expected = []
        for product, platform, arch, extension in TARGETS:
            name = release_artifacts.release_asset_name(product, platform, arch, extension)
            path = download / name
            path.write_bytes(b"artifact")
            expected.append(path)
        for name in (
            "operit2-app-android-arm64-v8a.apk",
            "operit2-app-macos-universal-1.0.0.zip",
            "operit2-app-ios-arm64-2.0.0-preview.16.zip.sha256",
        ):
            (download / name).write_bytes(b"excluded")
        self.assertEqual(download_action_artifacts.release_assets(download), sorted(expected))

    # Rejects outdated package names and checksum sidecars before any release upload.
    def test_publish_rejects_files_outside_the_current_naming_rule(self):
        dist = self.root / "dist"
        dist.mkdir()
        name = release_artifacts.release_asset_name("app", "android", "arm64-v8a", "apk")
        apk = dist / name
        apk.write_bytes(b"apk")
        with patch.object(publish_dist, "DIST_DIR", dist):
            self.assertEqual(publish_dist.release_assets(), [apk])
            for rejected in ("operit2-app-android-arm64-v8a.apk", name + ".sha256"):
                invalid = dist / rejected
                invalid.write_bytes(b"invalid asset")
                with self.subTest(name=rejected), self.assertRaisesRegex(RuntimeError, "naming rule"):
                    publish_dist.release_assets()
                invalid.unlink()
        invalid = dist / "operit2-cli-linux-x86_64-1.0.0.tar.gz"
        invalid.write_bytes(b"old version")
        with patch.object(release_build, "DIST_DIR", dist), patch.object(release_build, "github_request") as request:
            with self.assertRaisesRegex(RuntimeError, "naming rule"):
                release_build.publish_release("v2.0.0-preview.16", "owner/repo", False, True, None)
            request.assert_not_called()

    # Keeps workflow upload patterns able to find every platform package they build.
    def test_workflow_paths_find_versioned_packages(self):
        cases = (
            ("android-flutter-build.yml", "app", "android", "arm64-v8a", "apk"),
            ("windows-release-build.yml", "app", "windows", "x86_64", "zip"),
            ("windows-release-build.yml", "cli", "windows", "aarch64", "zip"),
            ("linux-release-build.yml", "app", "linux", "x86_64", "tar.gz"),
            ("linux-release-build.yml", "cli", "linux", "aarch64", "tar.gz"),
            ("macos-flutter-build.yml", "app", "macos", "universal", "zip"),
            ("macos-flutter-build.yml", "cli", "macos", "aarch64", "tar.gz"),
            ("ios-flutter-build.yml", "app", "ios", "arm64", "zip"),
            ("ios-app-package-build.yml", "app", "ios", "arm64", "zip"),
        )
        for workflow, product, platform, arch, extension in cases:
            with self.subTest(workflow=workflow, product=product):
                text = (REPO_ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8")
                name = release_artifacts.release_asset_name(product, platform, arch, extension)
                patterns = re.findall(r"path: tools/release/dist/(\S+)", text)
                self.assertTrue(any(fnmatch.fnmatchcase(name, pattern) for pattern in patterns))


if __name__ == "__main__":
    unittest.main()
