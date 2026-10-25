"""Execute the shipped importer and host root lifecycle against real SQLite/files.

Uses verified, locally cached upstream archives. No network, device or app build.
"""
import io
import os
import re
from pathlib import Path
import shutil
import signal
import sqlite3
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile

REPO = Path(__file__).resolve().parents[2]
TOOLING = REPO / "tools/ios-runtime/ish"


class IshRootLifecycleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if sys.platform != "darwin" or not shutil.which("clang"):
            raise unittest.SkipTest("macOS Foundation and clang are required")
        archives = {
            "ish": ("ish-7864dd601e615d0fc09b888a93d327f458b25a1d.zip",
                    "ish-7864dd601e615d0fc09b888a93d327f458b25a1d/"),
            "linux": ("linux-8ec9bf17f89c6dba818f3ed2427de4223e78644a.zip",
                      "linux-8ec9bf17f89c6dba818f3ed2427de4223e78644a/"),
        }
        if any(not (TOOLING / "downloads" / name).is_file() for name, _ in archives.values()):
            raise unittest.SkipTest("pinned upstream archives are not cached")
        temporary = tempfile.TemporaryDirectory(prefix="operit ish lifecycle ")
        cls.addClassCleanup(temporary.cleanup)
        cls.work = Path(temporary.name)
        cls.source = cls.work / "source"
        cls.source.mkdir()
        name, prefix = archives["ish"]
        with zipfile.ZipFile(TOOLING / "downloads" / name) as archive:
            for member in archive.infolist():
                if member.is_dir():
                    continue
                relative = member.filename[len(prefix):]
                if relative:
                    target = cls.source / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(archive.read(member))
        patches = sorted((TOOLING / "patches").glob("*.patch"))
        for patch in patches:
            for line in patch.read_text().splitlines():
                if line.startswith("--- a/deps/linux/"):
                    name = line[len("--- a/"):]
                    archive_name, prefix = archives["linux"]
                    target = cls.source / name
                    if not target.exists():
                        target.parent.mkdir(parents=True, exist_ok=True)
                        with zipfile.ZipFile(TOOLING / "downloads" / archive_name) as archive:
                            target.write_bytes(archive.read(prefix + name[len("deps/linux/"):]))
            subprocess.run(["patch", "--batch", "-p1", "-d", str(cls.source), "-i", str(patch)],
                           capture_output=True, text=True, check=True, timeout=15)
        libarchive = TOOLING / "downloads/libarchive-fc6563f5130d8a7ee1fc27c0e55baef35119f26c.zip"
        if not libarchive.is_file():
            raise unittest.SkipTest("pinned libarchive headers are not cached")
        with zipfile.ZipFile(libarchive) as archive:
            for header in ("archive.h", "archive_entry.h"):
                (cls.source / header).write_bytes(archive.read(
                    "libarchive-fc6563f5130d8a7ee1fc27c0e55baef35119f26c/libarchive/" + header))
        common = ["-g", "-O1", "-fsanitize=address,undefined", "-fno-omit-frame-pointer",
                  "-I", str(cls.source)]
        objects = []
        for name in ("tools/fakefs.c", "fs/fake-db.c", "fs/fake-migrate.c", "fs/fake-rebuild.c", "util/fchdir.c"):
            output = cls.work / (Path(name).stem + ".o")
            subprocess.run(["clang", *common, "-include", "stdio.h", "-include", "unistd.h", "-DFS_H",
                            "-c", str(cls.source / name), "-o", str(output)],
                           text=True, check=True, timeout=60)
            objects.append(str(output))
        cls.executable = cls.work / "lifecycle"
        subprocess.run(["clang++", *common, "-std=c++17", "-fobjc-arc",
                        "-I", str(REPO / "apps/flutter/app/ios/Runner"),
                        str(REPO / "tools/tests/fixtures/ish_root_lifecycle_harness.mm"),
                        str(REPO / "apps/flutter/app/ios/Runner/IshRootFilesystem.mm"),
                        *objects, "-framework", "Foundation", "-lsqlite3", "-larchive", "-o", str(cls.executable)],
                       text=True, check=True, timeout=60)

        # Execute the actual boot and startup-result functions with syscall faults.
        boot = (cls.source / "app/LinuxRoot.c").read_text()
        boot = "\n".join(line for line in boot.splitlines() if not line.startswith("#include"))
        interop = (cls.source / "app/LinuxInterop.c").read_text()
        actuate = re.search(r"int actuate_kernel\([^}]+}", interop).group(0)
        template = (REPO / "tools/tests/fixtures/ish_root_boot_harness.c").read_text()
        harness = cls.work / "boot.c"
        harness.write_text(template.replace("/* ROOT_BOOT_IMPLEMENTATION */", boot)
                          .replace("/* ACTUATE_KERNEL_IMPLEMENTATION */", actuate))
        cls.boot = cls.work / "boot"
        subprocess.run(["clang", *common, "-Wall", "-Wextra", "-Werror", str(harness), "-o", str(cls.boot)],
                       text=True, check=True, timeout=60)

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(dir=self.work, prefix="case ")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.runtime = self.directory / "runtime"
        self.root = self.runtime / "rootfs"
        self.archive = self.directory / "root.tar.gz"
        self.make_archive()

    def make_archive(self, large=False):
        with tarfile.open(self.archive, "w:gz") as archive:
            for name in (".", "bin", "home", "proc", "dev", "sbin"):
                entry = tarfile.TarInfo(name)
                entry.type = tarfile.DIRTYPE
                entry.mode = 0o755
                archive.addfile(entry)
            for name, contents in (("bin/sh", b"shell"), ("home/user.txt", b"original"),
                                   ("sbin/init", b"init")):
                entry = tarfile.TarInfo(name)
                entry.size = len(contents)
                entry.mode = 0o755
                archive.addfile(entry, io.BytesIO(contents))
            if large:
                contents = os.urandom(24 * 1024 * 1024)
                entry = tarfile.TarInfo("large")
                entry.size = len(contents)
                archive.addfile(entry, io.BytesIO(contents))

    def run_native(self, mode="prepare", root=None, expected=0, archive=None):
        result = subprocess.run([str(self.executable), mode, str(root or self.runtime), str(archive or self.archive)],
                                capture_output=True, text=True, timeout=30,
                                env={**os.environ, "ASAN_OPTIONS": "detect_leaks=0:halt_on_error=1",
                                     "UBSAN_OPTIONS": "halt_on_error=1"})
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        self.assertNotIn("AddressSanitizer", result.stderr)
        return result

    def connect(self):
        db = sqlite3.connect(self.root / "meta.db")
        self.addCleanup(db.close)
        return db

    def test_new_import_prepares_inodes_and_mounts_without_migration(self):
        self.run_native()
        self.assertFalse((self.runtime / "rootfs.importing").exists())
        db = self.connect()
        self.assertEqual(db.execute("pragma user_version").fetchone(), (3,))
        self.assertEqual(db.execute("select db_inode from meta").fetchone()[0], (self.root / "meta.db").stat().st_ino)
        self.assertEqual(db.execute("select inode from paths where path=?", (b"/bin/sh",)).fetchone()[0],
                         (self.root / "data/bin/sh").stat().st_ino)
        self.run_native("mount", self.root)

    def test_healthy_legacy_root_preserves_user_data_without_archive(self):
        self.runtime.mkdir()
        self.run_native("import", self.root)
        data = self.root / "data/home/user.txt"
        data.write_bytes(b"user's existing data")
        self.run_native(archive=self.directory / "missing")
        self.assertEqual(data.read_bytes(), b"user's existing data")
        self.assertEqual(list(self.runtime.glob("rootfs.incomplete-*")), [])
        self.run_native("mount", self.root)

    def test_old_uncommitted_import_is_retained_and_replaced(self):
        self.runtime.mkdir()
        self.run_native("cancel", self.root, expected=4)
        saved = self.root / "data/user-sentinel"
        saved.write_bytes(b"never delete me")
        self.run_native()
        retained = list(self.runtime.glob("rootfs.incomplete-*"))
        self.assertEqual(len(retained), 1)
        self.assertEqual((retained[0] / "data/user-sentinel").read_bytes(), b"never delete me")
        self.run_native("mount", self.root)

    def test_invalid_archive_returns_error_and_next_import_succeeds(self):
        invalid = self.directory / "invalid.tar.gz"
        invalid.write_bytes(b"not an archive")
        self.run_native(archive=invalid, expected=6)
        self.assertFalse(self.root.exists())
        self.assertFalse((self.runtime / "rootfs.importing").exists())
        self.run_native()

    def test_sigkill_during_import_never_publishes_and_relaunch_succeeds(self):
        self.make_archive(large=True)
        process = subprocess.Popen([str(self.executable), "prepare", str(self.runtime), str(self.archive)],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            deadline = time.monotonic() + 15
            candidate = self.runtime / "rootfs.importing/data/large"
            while not candidate.exists() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.002)
            self.assertTrue(candidate.exists(), "import never reached the large archive entry")
            process.send_signal(signal.SIGKILL)
            process.wait(timeout=5)
            self.assertEqual(process.returncode, -signal.SIGKILL)
            self.assertFalse(self.root.exists())
            self.run_native()
            self.run_native("mount", self.root)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)

    def test_unknown_future_schema_is_rejected_without_changing_user_files(self):
        self.run_native()
        db = self.connect()
        db.execute("pragma user_version=999")
        db.commit()
        self.run_native(expected=6)
        self.assertEqual(db.execute("pragma user_version").fetchone(), (999,))
        self.assertEqual((self.root / "data/home/user.txt").read_bytes(), b"original")
        self.assertEqual(list(self.runtime.glob("rootfs.incomplete-*")), [])

    def test_failed_migration_rolls_back_and_never_panics(self):
        self.run_native()
        db = self.connect()
        db.execute("pragma user_version=2") # Missing v2 trigger deliberately causes migration failure.
        db.commit()
        self.run_native(expected=6)
        self.assertEqual(db.execute("pragma user_version").fetchone(), (2,))
        self.assertEqual(db.execute("select count(*) from paths").fetchone()[0], 9)
        self.assertEqual(list(self.runtime.glob("rootfs.incomplete-*")), [])

    def test_version_two_migrates_once_and_mount_does_not_upgrade(self):
        self.run_native()
        db = self.connect()
        db.execute("create trigger delete_path after delete on paths begin select 1; end")
        db.execute("pragma user_version=2")
        db.commit()
        self.run_native()
        self.assertEqual(db.execute("pragma user_version").fetchone(), (3,))
        self.assertIsNone(db.execute("select name from sqlite_master where name='delete_path'").fetchone())
        self.run_native("mount", self.root)

    def test_boot_success_and_each_mount_failure_returns_actual_status(self):
        for fault in range(7):
            subprocess.run([str(self.boot), str(fault)], check=True, timeout=5)

    def test_mount_rejects_old_schema_without_migrating(self):
        self.run_native()
        db = self.connect()
        db.execute("create trigger delete_path after delete on paths begin select 1; end")
        db.execute("pragma user_version=2")
        db.commit()
        self.run_native("mount", self.root, expected=5)
        self.assertEqual(db.execute("pragma user_version").fetchone(), (2,))

    def test_all_supported_legacy_versions_upgrade_in_host(self):
        for version in (0, 1):
            with self.subTest(version=version):
                runtime = self.directory / f"legacy-{version}"
                root = runtime / "rootfs"
                self.run_native(root=runtime)
                with sqlite3.connect(root / "meta.db") as db:
                    if version == 0:
                        db.execute("drop index inode_to_path")
                    db.execute(f"pragma user_version={version}")
                self.run_native(root=runtime)
                with sqlite3.connect(root / "meta.db") as db:
                    self.assertEqual(db.execute("pragma user_version").fetchone(), (3,))
                self.run_native("mount", root)

    def test_inode_rebuild_preserves_hardlinks_after_filesystem_copy(self):
        self.run_native()
        data = self.root / "data"
        os.link(data / "home/user.txt", data / "home/alias.txt")
        with self.connect() as db:
            inode = (data / "home/user.txt").stat().st_ino
            db.execute("insert into paths values (?, ?)", (b"/home/alias.txt", inode))
        copied_runtime = self.directory / "copied"
        copied_root = copied_runtime / "rootfs"
        shutil.copytree(self.root, copied_root)
        copied_data = copied_root / "data"
        self.assertNotEqual((copied_data / "home/user.txt").stat().st_ino,
                            (copied_data / "home/alias.txt").stat().st_ino)
        self.run_native(root=copied_runtime)
        self.assertEqual((copied_data / "home/user.txt").read_bytes(), b"original")
        self.assertEqual((copied_data / "home/user.txt").stat().st_ino,
                         (copied_data / "home/alias.txt").stat().st_ino)
        self.run_native("mount", copied_root)

    def test_rebuild_sql_failure_rolls_back_inode_marker(self):
        self.run_native()
        db = self.connect()
        db.execute("update meta set db_inode=0")
        db.execute("create trigger reject_insert before insert on paths begin select raise(fail, 'test fault'); end")
        db.commit()
        self.run_native(expected=6)
        self.assertEqual(db.execute("select db_inode from meta").fetchone(), (0,))
        self.assertEqual(db.execute("select count(*) from paths").fetchone(), (9,))
        self.assertEqual((self.root / "data/home/user.txt").read_bytes(), b"original")
        db.execute("drop trigger reject_insert")
        db.commit()
        self.run_native()
        self.run_native("mount", self.root)

    def test_complete_shipped_alpine_archive_prepares_and_mounts(self):
        archive = REPO / "apps/flutter/app/ios/Runner/ish-root.tar.gz"
        if not archive.is_file():
            self.skipTest("shipped Alpine archive is not staged")
        self.run_native(archive=archive)
        self.run_native("mount", self.root)

    def test_malformed_existing_database_returns_error_without_reset(self):
        self.run_native()
        db = self.connect()
        db.execute("drop table paths")
        db.commit()
        self.run_native(expected=6)
        self.run_native("mount", self.root, expected=5)
        self.assertTrue(self.root.exists())
        self.assertEqual(list(self.runtime.glob("rootfs.incomplete-*")), [])


if __name__ == "__main__":
    unittest.main()
