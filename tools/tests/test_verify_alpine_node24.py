import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "runtime/verify_alpine_node24.py"
SPEC = importlib.util.spec_from_file_location("verify_alpine_node24", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class AlpineNode24Tests(unittest.TestCase):
    def test_reads_nodejs_package_version(self):
        database = "P:busybox\nV:1.37.0-r1\n\nP:nodejs\nV:24.18.1-r0\n"
        self.assertEqual(MODULE.installed_node_version(database), "24.18.1-r0")

    def test_ignores_similarly_named_packages(self):
        database = "P:nodejs-current\nV:24.18.1-r0\n\nP:npm\nV:11.0.0-r0\n"
        self.assertIsNone(MODULE.installed_node_version(database))

    def test_rejects_missing_version(self):
        self.assertIsNone(MODULE.installed_node_version("P:nodejs\n"))


if __name__ == "__main__":
    unittest.main()
