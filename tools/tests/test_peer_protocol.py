import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("peer_protocol_guard", Path(__file__).parents[1] / "check_peer_protocol.py")
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class PeerProtocolRevisionTests(unittest.TestCase):
    def setUp(self):
        self.old = {"sources": ["protocol.rs"], "revisions": [{"version": 1, "hash": "a" * 64}]}

    def test_software_only_release_keeps_protocol(self):
        guard.check_revisions(copy.deepcopy(self.old), self.old)

    def test_cannot_replace_published_hash_or_rewrite_history(self):
        for revisions in [[{"version": 1, "hash": "b" * 64}], [{"version": 2, "hash": "b" * 64}], []]:
            with self.subTest(revisions=revisions), self.assertRaises(ValueError):
                guard.check_revisions({"sources": self.old["sources"], "revisions": revisions}, self.old)

    def test_change_requires_appending_a_higher_version(self):
        current = copy.deepcopy(self.old)
        current["revisions"].append({"version": 1, "hash": "b" * 64})
        with self.assertRaises(ValueError):
            guard.check_revisions(current, self.old)
        current["revisions"][-1]["version"] = 2
        guard.check_revisions(current, self.old)

    def test_cannot_remove_coverage_without_bump(self):
        current = copy.deepcopy(self.old)
        current["sources"] = []
        with self.assertRaises(ValueError):
            guard.check_revisions(current, self.old)

    def test_cannot_change_schema_coverage_without_bump(self):
        current = copy.deepcopy(self.old)
        current["schemaRoots"] = ["dto"]
        with self.assertRaises(ValueError):
            guard.check_revisions(current, self.old)

    def test_versions_must_be_u32_and_hashes_unique(self):
        for version in [0, -1, True, 2**32]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                guard.check_revisions({"sources": [], "revisions": [{"version": version, "hash": "a" * 64}]})
        current = copy.deepcopy(self.old)
        current["revisions"].append({"version": 2, "hash": "a" * 64})
        with self.assertRaises(ValueError):
            guard.check_revisions(current)


if __name__ == "__main__":
    unittest.main()
