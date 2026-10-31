"""Check Android Cargo tasks receive the SDK's native build tools."""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]


class AndroidNativeToolchainTests(unittest.TestCase):
    """Keep native tool configuration at the Android Gradle boundary."""

    def test_cargo_uses_sdk_cmake_and_ninja(self):
        """Require CMake and Ninja from one explicit SDK package in Cargo's environment."""
        gradle = (REPO_ROOT / "apps/flutter/app/android/app/build.gradle.kts").read_text()
        self.assertIn('val operitCmakeBin = android.sdkDirectory.resolve("cmake/3.22.1/bin")', gradle)
        cargo = gradle.split('tasks.register<Exec>("cargoBuildOperitFlutterBridge', 1)[1]
        self.assertIn(
            'environment("CMAKE", operitCmakeBin.resolve("cmake${if (operitIsWindows) ".exe" else ""}").absolutePath)',
            cargo,
        )
        self.assertIn(
            'environment("PATH", "${operitCmakeBin.absolutePath}${File.pathSeparator}${System.getenv("PATH")}")',
            cargo,
        )

    def test_native_cmake_work_directory_is_short_and_cargo_isolated(self):
        """Shorten native builds without relocating Cargo's packaged shared libraries."""
        build = (REPO_ROOT / "core/crates/provider/local-model/build.rs").read_text()
        self.assertIn('std::hash::Hash::hash(&out_dir, &mut build_id)', build)
        self.assertIn('config.out_dir(', build)
        self.assertIn('.join("target/local-llm")', build)
        self.assertIn('std::hash::Hasher::finish(&build_id)', build)
        self.assertIn('config.define("OPERIT_LOCAL_LLM_RUNTIME_DIR", profile_dir)', build)
        self.assertIn('let path = profile_dir.join(&file)', build)

    @unittest.skipUnless(shutil.which("rustc"), "Rust is required for the build-path probe")
    def test_rust_build_path_is_stable_and_isolates_cargo_configurations(self):
        """Execute the production path-selection code without building inference engines."""
        build = (REPO_ROOT / "core/crates/provider/local-model/build.rs").read_text()
        selection = build.split('let mut build_id =', 1)[1].split('let profile_dir =', 1)[0]
        probe = '''
use std::path::{Path, PathBuf};
struct Config { output: PathBuf }
impl Config {
    /// Records the exact directory passed to the CMake builder.
    fn out_dir(&mut self, path: impl AsRef<Path>) { self.output = path.as_ref().to_path_buf(); }
}
/// Executes the production native work-directory selection.
fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut config = Config { output: PathBuf::new() };
    let mut build_id = ''' + selection + '''
    println!("{}", config.output.display());
}
'''
        with tempfile.TemporaryDirectory(prefix="operit native path ") as temporary:
            source = Path(temporary) / "probe.rs"
            binary = Path(temporary) / ("probe.exe" if os.name == "nt" else "probe")
            source.write_text(probe)
            subprocess.run(["rustc", "--edition=2021", str(source), "-o", str(binary)],
                           check=True, capture_output=True, text=True)
            env = dict(os.environ, CARGO_MANIFEST_DIR=str(
                REPO_ROOT / "core/crates/provider/local-model"))
            base = REPO_ROOT / "apps/flutter/native/operit-flutter-bridge/target"
            configurations = [
                base / "aarch64-linux-android/debug/build/operit-local-models-1111/out",
                base / "aarch64-linux-android/release/build/operit-local-models-1111/out",
                base / "x86_64-linux-android/debug/build/operit-local-models-1111/out",
                base / "aarch64-linux-android/debug/build/operit-local-models-2222/out",
                REPO_ROOT / "other-worktree/target/debug/build/operit-local-models-1111/out",
            ]
            results = []
            for out_dir in configurations + configurations[:1]:
                env["OUT_DIR"] = str(out_dir)
                result = subprocess.run([str(binary)], env=env, check=True,
                                        capture_output=True, text=True)
                path = Path(result.stdout.strip())
                self.assertEqual(path.parent, REPO_ROOT / "target/local-llm")
                self.assertRegex(path.name, r"^[0-9a-f]{16}$")
                results.append(path)
            self.assertEqual(len(set(results[:-1])), len(configurations))
            self.assertEqual(results[0], results[-1])


if __name__ == "__main__":
    unittest.main()
