"""Verify shipped Rust diagnostics do not retain build machine paths."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/build_scripts"))
from cargo_build import cargo_build_environment


class CargoBuildTests(unittest.TestCase):
    def test_preserves_encoded_flags_with_spaces_and_takes_precedence(self):
        encoded = "-C\x1flink-arg=/a directory/library.a"
        env = cargo_build_environment(["cargo", "build"], {
            "CARGO_ENCODED_RUSTFLAGS": encoded,
            "RUSTFLAGS": "-C opt-level=1",
        })
        self.assertTrue(env["CARGO_ENCODED_RUSTFLAGS"].startswith(encoded + "\x1f"))
        self.assertNotIn("RUSTFLAGS", env)
        self.assertNotIn("opt-level=1", env["CARGO_ENCODED_RUSTFLAGS"])

    def test_keeps_cross_target_linker_flags(self):
        for target_args in (["--target", "aarch64-unknown-linux-gnu"],
                            ["--target=aarch64-unknown-linux-gnu"]):
            with self.subTest(arguments=target_args):
                env = cargo_build_environment(["cargo", "build", *target_args], {
                    "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "-Awarnings -C link-arg=--sysroot=/sdk",
                })
                flags = env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f")
                self.assertEqual(flags[:3], ["-Awarnings", "-C", "link-arg=--sysroot=/sdk"])

    @unittest.skipUnless(shutil.which("cargo"), "Cargo is required for source path verification")
    def test_compiled_generated_source_dependency_and_backtrace_hide_private_paths(self):
        with tempfile.TemporaryDirectory(prefix="operit privacy ") as temporary:
            private = Path(temporary) / "private-username"
            repo = private / "My Projects" / "Operit"
            crate = repo / "app"
            cache = private / "cargo cache"
            dependency = cache / "fixture-dependency"
            target = private / "build output"
            for directory in (crate / "src", dependency / "src"):
                directory.mkdir(parents=True)
            (dependency / "Cargo.toml").write_text(
                '[package]\nname = "fixture-dependency"\nversion = "0.1.0"\nedition = "2021"\n')
            (dependency / "src/lib.rs").write_text('pub fn source() -> &\'static str { file!() }\n')
            # A dependency and an OUT_DIR outside the checkout must also be remapped.
            relative_dependency = os.path.relpath(dependency, crate).replace(os.sep, "/")
            (crate / "Cargo.toml").write_text(
                '[package]\nname = "path-privacy-fixture"\nversion = "0.1.0"\nedition = "2021"\n'
                '[dependencies]\nfixture-dependency = { path = "' + relative_dependency + '" }\n'
                '[profile.dev]\ndebug = 2\n')
            (crate / "build.rs").write_text('''
fn main() {
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("generated_core_dispatch.rs"),
        r#"#[inline(never)] pub fn generated() { println!("{}", capture()); println!("{}", std::backtrace::Backtrace::force_capture()); }"#).unwrap();
}
''')
            (crate / "src/main.rs").write_text('''
include!(concat!(env!("OUT_DIR"), "/generated_core_dispatch.rs"));
#[track_caller]
fn capture() -> &'static std::panic::Location<'static> { std::panic::Location::caller() }
fn main() {
    println!("{}", file!());
    println!("{}", fixture_dependency::source());
    generated();
}
''')
            base_env = dict(os.environ)
            base_env.pop("CARGO_ENCODED_RUSTFLAGS", None)
            base_env.update({"CARGO_HOME": str(cache), "CARGO_TARGET_DIR": str(target), "RUSTFLAGS": "-C debuginfo=2"})
            command = ["cargo", "build", "--offline", "--manifest-path", str(crate / "Cargo.toml")]
            env = cargo_build_environment(command, base_env, repo_root=repo)
            build = subprocess.run(command, env=env, cwd=crate, capture_output=True, text=True, timeout=90)
            self.assertEqual(build.returncode, 0, build.stderr)
            executable = target / "debug" / ("path-privacy-fixture.exe" if os.name == "nt" else "path-privacy-fixture")
            result = subprocess.run([str(executable)], capture_output=True, text=True, timeout=15)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("/cargo/fixture-dependency/src/lib.rs", result.stdout.replace("\\", "/"))
            self.assertIn("/operit-target/debug/build/", result.stdout.replace("\\", "/"))
            self.assertIn("generated_core_dispatch.rs:", result.stdout)
            self.assertNotIn("private-username", result.stdout + result.stderr)
            self.assertNotIn(str(private), result.stdout + result.stderr)
            # Debug Mach-O files carry linker OSO references to physical object
            # files for dsymutil. Check the shipped release binary separately.
            release_command = [*command, "--release", "--target-dir", "../../../release output"]
            base_env["RUSTFLAGS"] = "-C opt-level=1"
            release = subprocess.run(release_command,
                                     env=cargo_build_environment(release_command, base_env, repo_root=repo, cwd=crate),
                                     cwd=crate, capture_output=True, text=True, timeout=90)
            self.assertEqual(release.returncode, 0, release.stderr)
            release_executable = private / "release output/release" / executable.name
            self.assertFalse(b"private-username" in release_executable.read_bytes(),
                             "private build path remains in the release executable")


if __name__ == "__main__":
    unittest.main()
