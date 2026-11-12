"""Check the module application's folder artifact and installation lifecycle."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

APP_ROOT = Path(__file__).resolve().parents[2] / "apps/android-module"
spec = importlib.util.spec_from_file_location("android_module_app", APP_ROOT / "build.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
import build_android_module_release as module_release
import release_artifacts
MODULE_SHELL = os.environ.get("OPERIT_MODULE_TEST_SHELL", shutil.which("sh"))


class ModuleFixture:
    """Supply bounded Android artifacts shared by package and installation checks."""

    def setUp(self):
        """Create small APK fixtures without compiling the shared application."""
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.apk = self.root / "app.apk"
        self.output = self.root / "module"
        self.elf = b"\x7fELF\x02\x01" + bytes(12) + (183).to_bytes(2, "little")

    def apk_fixture(self, *, compatible=True, valid_checksum=True, missing_library=None, invalid_library=None,
                    with_main=True, with_jni=True, application_id="app.operit.module"):
        """Write a fake application with exact module role, entry-point and library markers."""
        rootfs = b"test rootfs payload"
        checksum = hashlib.sha256(rootfs).hexdigest() if valid_checksum else "0" * 64
        with zipfile.ZipFile(self.apk, "w") as archive:
            archive.writestr("classes.dex", b"Lapp/operit/ModuleMain;" if with_main else b"Lapp/operit/MainActivity;")
            if compatible:
                marker = {**builder.APP_MARKER, "applicationId": application_id}
                archive.writestr("assets/operit-android-module.json", json.dumps(marker))
            for name in ["libbash.so", "liboperit_busybox.so", "liboperit_proot.so", "liboperit_loader.so",
                         "liboperit_local_llm.so", "liboperit_flutter_bridge.so", "libapp.so", "libflutter.so"]:
                if name == missing_library:
                    continue
                data = b"invalid ELF" if name == invalid_library else self.elf
                if name == "liboperit_flutter_bridge.so" and with_jni:
                    data += builder.APP_JNI
                archive.writestr("lib/arm64-v8a/" + name, data)
            archive.writestr(builder.ROOTFS_ENTRY, rootfs)
            archive.writestr(builder.ROOTFS_ENTRY + ".sha256", checksum + "\n")


class ModuleApplicationTests(ModuleFixture, unittest.TestCase):
    """Verify one shared application artifact rather than a duplicated Core distribution."""

    def test_builds_directory_with_one_apk_and_no_second_core_or_environment(self):
        """Verify canonical directory output, source equality and single-copy distribution payloads."""
        self.apk_fixture()
        report = builder.assemble(self.apk, self.output)
        self.assertTrue(self.output.is_dir())
        self.assertFalse(report["separateCoreExecutable"])
        self.assertEqual((self.output / builder.APK_ENTRY).read_bytes(), self.apk.read_bytes())
        names = {path.relative_to(self.output).as_posix() for path in self.output.rglob("*") if path.is_file()}
        self.assertFalse(any(name.endswith(".so") or "rootfs" in name or "module-core" in name for name in names))
        manifest = json.loads((self.output / "deployment-manifest.json").read_text())
        self.assertEqual(manifest["app"], "android-module")
        self.assertEqual(manifest["bootstrap"], "app_process")
        self.assertEqual(manifest["format"], 3)
        for name, checksum in manifest["archiveFiles"].items():
            self.assertEqual(hashlib.sha256((self.output / name).read_bytes()).hexdigest(), checksum)
        script = (self.output / "bin/operit-host-ctl").read_bytes()
        self.assertNotIn(b"\r", script)
        self.assertNotIn(b"pm install", script)
        self.assertIn(b"/system/bin/app_process", script)
        self.assertIn(b"app.operit.ModuleMain", script)
        self.assertTrue((self.output.parent / "module.verification.json").is_file())

    def test_incompatible_app_and_stale_entry_points_are_rejected(self):
        """Reject old trial APKs even when their UI could still start as a normal application."""
        for options, message in [({"compatible": False}, "module build"), ({"with_main": False}, "app_process"),
                                 ({"with_jni": False}, "JNI library")]:
            with self.subTest(options=options):
                self.apk_fixture(**options)
                with self.assertRaisesRegex(ValueError, message):
                    builder.assemble(self.apk, self.output)
                self.assertFalse(self.output.exists())

    def test_bad_environment_and_native_libraries_are_rejected(self):
        """Reject missing engine libraries and damaged payloads before publishing an output folder."""
        for options, message in [({"valid_checksum": False}, "checksum"), ({"missing_library": "libflutter.so"}, "libflutter.so"),
                                 ({"invalid_library": "libapp.so"}, "ARM64")]:
            with self.subTest(options=options):
                self.apk_fixture(**options)
                with self.assertRaisesRegex(ValueError, message):
                    builder.assemble(self.apk, self.output)
                self.assertFalse(self.output.exists())

    def test_old_shared_package_module_is_rejected(self):
        """Reject an old module APK that would replace the ordinary App instead of coexisting."""
        self.apk_fixture(application_id="app.operit")
        with self.assertRaisesRegex(ValueError, "app.operit.module"):
            builder.assemble(self.apk, self.output)
        self.assertFalse(self.output.exists())

    def test_zip_destination_and_existing_output_are_rejected(self):
        """Keep the module product a directory and preserve already published output files."""
        self.apk_fixture()
        with self.assertRaisesRegex(ValueError, "directory"):
            builder.assemble(self.apk, self.root / "module.zip")
        self.output.mkdir()
        marker = self.output / "existing-data"
        marker.write_bytes(b"preserved")
        with self.assertRaises(FileExistsError):
            builder.assemble(self.apk, self.output)
        self.assertEqual(marker.read_bytes(), b"preserved")


class ModuleReleaseTests(ModuleFixture, unittest.TestCase):
    """Exercise the release entry point, installer ZIP and checksum boundary without native builds."""

    def test_release_build_stamps_versions_and_archives_folder_contents(self):
        """Package the real directory builder output with current release metadata and executable modes."""
        self.apk_fixture()
        version = "2.0.0-preview.42"
        for relative in release_artifacts.RELEASE_MANIFESTS:
            path = self.root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(f'[package]\nversion = "{version}"\n', encoding="utf-8")
        flutter = self.root / "flutter"
        flutter.mkdir()
        (flutter / "pubspec.yaml").write_text("version: 2.0.0+2042\n", encoding="utf-8")
        release = self.root / "release"
        work = release / "work/android-module"
        dist = release / "dist"

        def build(command):
            """Replace only the expensive native compilation while using the real module assembler."""
            self.assertIn("--enforce-lockfile", command)
            self.assertEqual(command[command.index("--build-name") + 1], "2.0.0")
            self.assertEqual(command[command.index("--build-number") + 1], "2042")
            builder.assemble(self.apk, command[command.index("--output") + 1],
                             release_version=command[command.index("--release-version") + 1],
                             build_number=command[command.index("--build-number") + 1])

        with (patch.object(release_artifacts, "REPO_ROOT", self.root),
              patch.object(module_release, "FLUTTER_APP_DIR", flutter),
              patch.object(module_release, "RELEASE_DIR", release),
              patch.object(module_release, "MODULE_WORK_DIR", work),
              patch.object(module_release, "run", side_effect=build),
              patch.object(module_release.sys, "argv", ["module-release", "--enforce-lockfile", "--dist-dir", str(dist)])):
            self.assertEqual(module_release.main(), 0)
            target = dist / release_artifacts.release_asset_name("module", "android", "arm64-v8a", "zip")
            release_artifacts.validate_release_assets([target])
        module = work / "module-arm64-v8a"
        with zipfile.ZipFile(target) as archive:
            self.assertIsNone(archive.testzip())
            self.assertEqual(archive.read(builder.APK_ENTRY), self.apk.read_bytes())
            properties = dict(line.split("=", 1) for line in archive.read("module.prop").decode().splitlines())
            self.assertEqual(properties["version"], version)
            self.assertEqual(properties["versionCode"], "2042")
            for entry in archive.infolist():
                self.assertEqual(archive.read(entry), (module / entry.filename).read_bytes())
                self.assertFalse(entry.filename.startswith("module-arm64-v8a/"))
            self.assertEqual((archive.getinfo("service.sh").external_attr >> 16) & 0o777, 0o755)
            self.assertEqual((archive.getinfo("bin/operit-host-ctl").external_attr >> 16) & 0o777, 0o755)
            self.assertEqual((archive.getinfo(builder.APK_ENTRY).external_attr >> 16) & 0o777, 0o644)
        self.assertFalse(list(dist.glob("*.sha256")))

    def test_corrupt_folder_does_not_replace_staged_release(self):
        """Reject modified payload bytes before overwriting an already staged release ZIP."""
        self.apk_fixture()
        builder.assemble(self.apk, self.output)
        target = self.root / "release.zip"
        target.write_bytes(b"existing release")
        (self.output / builder.APK_ENTRY).write_bytes(b"damaged")
        with self.assertRaisesRegex(ValueError, "checksum mismatch"):
            module_release.package_module(self.output, target)
        self.assertEqual(target.read_bytes(), b"existing release")
        self.assertFalse(target.with_name(target.name + ".tmp").exists())

    def test_module_build_uses_separate_role_and_platform_version(self):
        """Keep module compilation on the module feature and the exact App platform metadata."""
        app = self.root / "flutter"
        apk = app / "build/app/outputs/flutter-apk/app-arm64-v8a-release.apk"
        apk.parent.mkdir(parents=True)
        apk.write_bytes(b"built apk")
        with (patch.object(builder, "FLUTTER_APP_DIR", app),
              patch.object(builder, "ensure_android_signing"),
              patch.object(builder, "flutter_command", return_value="flutter-test"),
              patch.object(builder, "configure_android_flutter_sdk"),
              patch.object(builder, "require_command"),
              patch.object(builder, "generate_dart_proxy_artifacts"),
              patch.object(builder, "flutter_pub_get") as dependencies,
              patch.object(builder, "run") as run):
            self.assertEqual(builder.build_apk("2.0.0", "2042", True), apk)
        dependencies.assert_called_once_with(enforce_lockfile=True)
        command = run.call_args.args[0]
        self.assertIn("--android-project-arg=operitRuntime=module", command)
        self.assertEqual(command[command.index("--build-name") + 1], "2.0.0")
        self.assertEqual(command[command.index("--build-number") + 1], "2042")


@unittest.skipUnless(MODULE_SHELL, "Set OPERIT_MODULE_TEST_SHELL to execute module installation checks")
class ModuleLifecycleTests(ModuleFixture, unittest.TestCase):
    """Exercise the real scripts against a synthetic module folder and bounded Android doubles."""

    def setUp(self):
        """Assemble the application fixture and isolate all Android filesystem paths."""
        super().setUp()
        self.apk_fixture()
        builder.assemble(self.apk, self.output)
        self.data = self.root / "data"
        (self.data / "run").mkdir(parents=True)
        self.system_apk = self.root / "system/app/Operit/Operit.apk"
        self.system_apk.parent.mkdir(parents=True)
        self.system_apk.write_bytes(b"mounted client")
        self.args_file = self.root / "runtime.args"
        self.environment = {**os.environ, "RUNTIME_ARGS": self.args_file.as_posix(),
                            "PATH": str(Path(MODULE_SHELL).parent) + os.pathsep + os.environ["PATH"],
                            "MOCK_PACKAGE_PATH": "package:" + self.system_apk.as_posix(),
                            "MOCK_PACKAGE_INFO": "package:app.operit.module uid:10234"}

    def control(self, command):
        """Run the real controller with strict PackageManager and app_process command doubles."""
        script = (self.output / "bin/operit-host-ctl").read_text(encoding="utf-8")
        script = script.replace('MODDIR=$(CDPATH= cd -- "${0%/*}/.." && pwd) || exit 1',
                                "MODDIR=" + shlex.quote(self.output.as_posix()))
        script = script.replace("DATADIR=/data/adb/operit2", "DATADIR=" + shlex.quote(self.data.as_posix()))
        script = script.replace("SYSTEM_APK=/system/app/Operit/Operit.apk", "SYSTEM_APK=" + shlex.quote(self.system_apk.as_posix()))
        script = script.replace('/system/bin/app_process "-Djava.library.path=', 'mock_app_process "-Djava.library.path=')
        commands = '''
pm() {
    [ "$*" = 'path --user 0 app.operit.module' ] || return 90
    printf '%s\\n' "$MOCK_PACKAGE_PATH"
}
cmd() {
    [ "$*" = 'package list packages --user 0 -s -U app.operit.module' ] || return 91
    printf '%s\\n' "$MOCK_PACKAGE_INFO"
}
am() {
    [ "$*" = 'start --user 0 -n app.operit.module/app.operit.MainActivity' ] || return 92
    echo 'Activity launched'
}
mock_app_process() {
    printf '%s\\n' "$@" > "$RUNTIME_ARGS"
    printf 'CLASSPATH=%s\\nLD_LIBRARY_PATH=%s\\n' "$CLASSPATH" "$LD_LIBRARY_PATH" >> "$RUNTIME_ARGS"
}
getprop() {
    case "$1" in
        sys.boot_completed) echo 1 ;;
        sys.user.0.ce_available) echo true ;;
        ro.product.model) echo TestDevice ;;
        ro.product.locale) echo zh-CN ;;
        *) return 93 ;;
    esac
}
'''
        return subprocess.run([MODULE_SHELL, "-s", "--", command], input=commands + script,
                              text=True, capture_output=True, env=self.environment, timeout=15)

    def install(self, *, corrupt_extraction=False):
        """Expand the actual fixture APK and verify checksums through the real installer commands."""
        modules = self.root / "modules"
        modules.mkdir(exist_ok=True)
        script = (self.output / "customize.sh").read_text(encoding="utf-8")
        script = script.replace("/data/adb/modules/*", shlex.quote(modules.as_posix()) + "/*")
        commands = '''
abort() { echo "$*" >&2; exit 1; }
ui_print() { echo "$*"; }
set_perm() { printf '%s\\n' "$*" >> "$PERMISSION_LOG"; }
set_perm_recursive() { printf '%s\\n' "$*" >> "$PERMISSION_LOG"; }
'''
        if corrupt_extraction:
            commands += '''
unzip() {
    if [ "$3" = 'lib/arm64-v8a/libapp.so' ]; then printf 'bad library'; else command unzip "$@"; fi
}
'''
        environment = {**self.environment, "ARCH": "arm64", "MODPATH": self.output.as_posix(),
                       "PERMISSION_LOG": (self.root / "permissions.log").as_posix()}
        return subprocess.run([MODULE_SHELL, "-s"], input=commands + script, text=True,
                              capture_output=True, env=environment, timeout=15)

    def enable_mount_provider(self):
        """Register an active mount provider without installing or modifying any real module."""
        provider = self.root / "modules/meta-test"
        provider.mkdir(parents=True, exist_ok=True)
        (provider / "module.prop").write_text("id=meta-test\nmetamodule=1\n")
        (provider / "metamount.sh").write_text("#!/bin/sh\n")
        return provider

    def test_system_client_opens_and_shared_runtime_uses_exact_module_inputs(self):
        """Verify the root JVM uses the shared APK/library and the resolved application's UID."""
        opened = self.control("open")
        self.assertEqual(opened.returncode, 0, opened.stderr)
        self.assertIn("Activity launched", opened.stdout)
        started = self.control("supervise")
        self.assertEqual(started.returncode, 0, started.stderr)
        arguments = self.args_file.read_text().splitlines()
        self.assertIn("app.operit.ModuleMain", arguments)
        self.assertEqual(arguments[arguments.index("app.operit.ModuleMain") + 1], self.output.as_posix())
        self.assertIn("10234", arguments)
        self.assertIn("CLASSPATH=" + self.output.as_posix() + "/system/app/Operit/Operit.apk", arguments)
        self.assertIn("LD_LIBRARY_PATH=" + self.output.as_posix() + "/system/app/Operit/lib/arm64", arguments)
        self.assertFalse((self.data / "run/supervisor.pid").exists())

    def test_data_partition_override_is_rejected_without_starting_runtime(self):
        """Reject an overriding APK without uninstalling, migrating or running another application."""
        self.environment["MOCK_PACKAGE_PATH"] = "package:/data/app/app.operit.module/base.apk"
        result = self.control("supervise")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Expected mounted system client", result.stderr)
        self.assertEqual(json.loads((self.data / "run/status.json").read_text())["state"], "failed")
        self.assertFalse(self.args_file.exists())
        self.assertFalse((self.data / "run/supervisor.pid").exists())

    def test_coinstalled_main_app_uid_is_not_used_by_module(self):
        """Resolve only the module UID when PackageManager also lists the ordinary App."""
        self.environment["MOCK_PACKAGE_INFO"] = "package:app.operit uid:10567\npackage:app.operit.module uid:10234"
        started = self.control("supervise")
        self.assertEqual(started.returncode, 0, started.stderr)
        arguments = self.args_file.read_text().splitlines()
        self.assertEqual(arguments[-3], "10234")
        self.assertNotIn("10567", arguments)

    def test_main_app_cannot_supply_module_client_uid(self):
        """Refuse to publish module credentials to the ordinary App when the module is absent."""
        self.environment["MOCK_PACKAGE_INFO"] = "package:app.operit uid:10567"
        started = self.control("supervise")
        self.assertNotEqual(started.returncode, 0)
        self.assertIn("Cannot resolve system client UID", started.stderr)
        self.assertFalse(self.args_file.exists())

    def test_missing_mount_or_system_uid_is_rejected(self):
        """Reject incomplete system application registration before launching the shared runtime."""
        self.environment["MOCK_PACKAGE_INFO"] = ""
        result = self.control("open")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cannot resolve system client UID", result.stderr)
        self.system_apk.unlink()
        result = self.control("open")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not mounted", result.stderr)

    def test_installer_checks_provider_then_expands_one_shared_artifact(self):
        """Verify install-time expansion, every installed checksum and the expected system labels."""
        missing = self.install()
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("mounting metamodule", missing.stderr)
        provider = self.enable_mount_provider()
        (provider / "disable").touch()
        self.assertNotEqual(self.install().returncode, 0)
        (provider / "disable").unlink()
        installed = self.install()
        self.assertEqual(installed.returncode, 0, installed.stdout + installed.stderr)
        manifest = json.loads((self.output / "deployment-manifest.json").read_text())
        for name, checksum in manifest["installedFiles"].items():
            self.assertEqual(hashlib.sha256((self.output / name).read_bytes()).hexdigest(), checksum)
        self.assertFalse((self.output / builder.APK_ENTRY).exists())
        permissions = (self.root / "permissions.log").read_text()
        self.assertIn("/system 0 0 0755 0644 u:object_r:system_file:s0", permissions)

    def test_installer_rejects_corrupted_input_and_expansion(self):
        """Check both verification boundaries before the manager can activate the module."""
        self.enable_mount_provider()
        damaged = self.install(corrupt_extraction=True)
        self.assertNotEqual(damaged.returncode, 0)
        self.assertIn("Expanded Host checksum mismatch", damaged.stderr)

    def test_installer_rejects_corrupted_archive(self):
        """Reject changed application bytes before any system application file is published."""
        self.enable_mount_provider()
        with (self.output / builder.APK_ENTRY).open("ab") as stream:
            stream.write(b"corruption")
        damaged = self.install()
        self.assertNotEqual(damaged.returncode, 0)
        self.assertIn("payload checksum mismatch", damaged.stderr)
        self.assertFalse((self.output / "system/app/Operit/Operit.apk").exists())


if __name__ == "__main__":
    unittest.main()
