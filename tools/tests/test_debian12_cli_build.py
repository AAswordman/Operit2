"""Exercise the Debian 12 CLI build driver's pure helpers without containers."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "build_scripts"))

import build_cli_linux_debian12 as driver


class HighestGlibcVersionTests(unittest.TestCase):
    def test_returns_the_highest_referenced_symbol_version(self):
        output = "\n".join(
            (
                "0000000000000000 g DF .init 0000000000000000 GLIBC_2.34 __libc_start_main",
                "0000000000000000 g DF .text 0000000000000000 GLIBC_2.2.5 malloc",
                "0000000000000000 w DF .text 0000000000000000 GLIBC_2.35 pthread_create",
            )
        )
        self.assertEqual(driver.highest_glibc_version(output), (2, 35))

    def test_compares_multi_part_versions_numerically(self):
        self.assertEqual(driver.highest_glibc_version("R GLIBC_2.10\nR GLIBC_2.9\n"), (2, 10))

    def test_returns_none_without_glibc_symbols(self):
        self.assertIsNone(driver.highest_glibc_version("no symbols here"))


class SelectedArchesTests(unittest.TestCase):
    def test_host_resolves_to_the_detected_host_architecture(self):
        self.assertIn(driver.selected_arches("host"), (["x86_64"], ["aarch64"]))

    def test_all_lists_both_targets_in_release_order(self):
        self.assertEqual(driver.selected_arches("all"), ["x86_64", "aarch64"])

    def test_explicit_arch_passes_through(self):
        self.assertEqual(driver.selected_arches("aarch64"), ["aarch64"])


if __name__ == "__main__":
    unittest.main()
