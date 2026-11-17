"""Verify release and preview publication, immutable assets and retained bases without network writes."""
import hashlib
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/release"))
sys.path.insert(0, str(ROOT / "tools/updates"))
import gh_publish
import incremental
import prepare
import release_artifacts
from release_artifacts import file_sha256, release_version


class IncrementalPublishTests(unittest.TestCase):
    # Stages a real package and generated manifest in a private directory.
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dist = Path(temporary.name)
        self.version = release_version()
        self.package = self.dist / f"operit2-cli-windows-x86_64-{self.version}.zip"
        self.package.write_bytes(b"PK\x03\x04" + b"signed-package" * 1000)
        prepare.prepare(self.dist, [])
        self.calls = []
        self.remote = {"assets": [{"name": self.package.name, "digest": "sha256:" + file_sha256(self.package)}]}

    # Emulates gh metadata while preserving the full command order for assertions.
    def gh(self, *args, **kwargs):
        self.calls.append(args)
        if args[:2] == ("release", "list"):
            return "[]"
        if args[:2] == ("release", "view"):
            return json.dumps(self.remote)
        if args[0] == "api":
            return "true"
        return ""

    # The public transition must happen only after every uploaded digest has been checked.
    def test_publishes_only_after_verified_draft_upload(self):
        with patch.object(gh_publish, "gh", side_effect=self.gh):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, False, source_revision="a" * 40)
        self.assertEqual(self.calls[-1], ("release", "edit", "v" + self.version, "--repo", gh_publish.SOURCE_REPO, "--draft=false"))
        create = next(call for call in self.calls if call[:2] == ("release", "create"))
        self.assertIn("--draft", create)
        uploads = [Path(call[3]).name for call in self.calls if call[:2] == ("release", "upload")]
        self.assertEqual(uploads, [self.package.name])

    # GitHub digest disagreement leaves a hidden draft and never exposes the bad release.
    def test_digest_failure_keeps_release_hidden(self):
        self.remote["assets"][0]["digest"] = "sha256:" + "0" * 64
        with patch.object(gh_publish, "gh", side_effect=self.gh), self.assertRaisesRegex(ValueError, "digest mismatch"):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, False, source_revision="a" * 40)
        self.assertFalse(any("--draft=false" in call for call in self.calls))

    # A public version must never be rewritten, even when a caller requests a draft.
    def test_published_versions_are_immutable(self):
        def remote(*args, **kwargs):
            if args[:2] == ("release", "list"):
                return json.dumps([{"tagName": "v" + self.version, "isDraft": False, "isPrerelease": True}])
            return self.gh(*args, **kwargs)
        with patch.object(gh_publish, "gh", side_effect=remote), self.assertRaisesRegex(ValueError, "immutable"):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, True, source_revision="a" * 40)
        self.assertFalse(any(call[:2] == ("release", "upload") for call in self.calls))

    # Read-only preflight performs no remote mutation.
    def test_check_only_does_not_create_or_upload(self):
        with patch.object(gh_publish, "gh", side_effect=self.gh):
            result = gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, False, source_revision="a" * 40, check_only=True)
        self.assertIn("Validated", result)
        self.assertFalse(any(call[:2] in (("release", "create"), ("release", "upload"), ("release", "edit")) for call in self.calls))

    # Discontinuing deltas must not prevent publishing ordinary full installers.
    def test_full_only_release_does_not_require_incremental_sidecars(self):
        for path in self.dist.glob("*.update.json"):
            path.unlink()
        self.remote = {"assets": [{"name": self.package.name, "digest": "sha256:" + file_sha256(self.package)}]}
        with patch.object(gh_publish, "gh", side_effect=self.gh):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, False, source_revision="a" * 40)
        uploads = [call for call in self.calls if call[:2] == ("release", "upload")]
        self.assertEqual(len(uploads), 1)
        self.assertEqual(self.calls[-1][-1], "--draft=false")

    # Patch-only publication retains the original locally and uploads authenticated deltas only.
    def test_patch_only_publication_excludes_full_packages_without_deleting_local_original(self):
        base_dir = self.dist / "old"
        base_dir.mkdir()
        old = base_dir / "operit2-cli-windows-x86_64-1.0.0.zip"
        old.write_bytes(self.package.read_bytes() + b"old content")
        prepare.prepare(self.dist, [base_dir])
        self.remote = {"assets": [{"name": path.name, "digest": "sha256:" + file_sha256(path)}
                                  for path in self.dist.iterdir() if path.is_file() and path != self.package]}
        with patch.object(gh_publish, "gh", side_effect=self.gh):
            gh_publish.publish(self.dist, gh_publish.PATCH_REPO, True, source_revision="a" * 40, patch_only=True)
        uploads = [Path(call[3]).name for call in self.calls if call[:2] == ("release", "upload")]
        self.assertNotIn(self.package.name, uploads)
        self.assertEqual(set(uploads), {asset["name"] for asset in self.remote["assets"]})
        self.assertTrue(any(name.endswith(".patch.gz") for name in uploads))
        self.assertTrue(self.package.exists())
        create = next(call for call in self.calls if call[:2] == ("release", "create"))
        self.assertEqual(create[create.index("--repo") + 1], gh_publish.PATCH_REPO)
        self.assertNotIn("--target", create)

        self.calls.clear()
        self.remote = {"assets": [{"name": self.package.name, "digest": "sha256:" + file_sha256(self.package)}]}
        with patch.object(gh_publish, "gh", side_effect=self.gh):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, True, source_revision="a" * 40)
        uploads = [Path(call[3]).name for call in self.calls if call[:2] == ("release", "upload")]
        self.assertEqual(uploads, [self.package.name])

    # An empty manifest cannot make a release discoverable without any transferable bytes.
    def test_patch_only_requires_an_actual_patch(self):
        with patch.object(gh_publish, "gh") as remote, self.assertRaisesRegex(ValueError, "No staged release assets"):
            gh_publish.publish(self.dist, gh_publish.PATCH_REPO, True, source_revision="a" * 40, patch_only=True)
        remote.assert_not_called()

    # Incremental metadata must never create a version release in the source repository.
    def test_source_repository_rejects_patch_publication_before_any_remote_call(self):
        with patch.object(gh_publish, "gh") as remote, self.assertRaisesRegex(ValueError, "patches to Operit2-nightly"):
            gh_publish.publish(self.dist, gh_publish.SOURCE_REPO, True, source_revision="a" * 40, patch_only=True)
        remote.assert_not_called()

    # Collected builds publish to separate repositories for combined, patch-only and full-only modes.
    def test_collected_builds_route_packages_and_patches_to_their_repositories(self):
        import archive
        import download_action_artifacts
        old = self.dist / "old"
        old.mkdir()
        (old / "operit2-cli-windows-x86_64-1.0.0.zip").write_bytes(self.package.read_bytes() + b"old")
        history = self.dist / "history"
        archive.retain(old, history, "1.0.0")
        for mode, expected in (({}, [(gh_publish.PATCH_REPO, True), (gh_publish.SOURCE_REPO, False)]),
                               ({"patch_only": True}, [(gh_publish.PATCH_REPO, True)]),
                               ({"full_only": True}, [(gh_publish.SOURCE_REPO, False)])):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                dist = Path(temporary)
                # Copies the verified platform artifact as the real collector would.
                def collect(run_id, work, destination):
                    shutil.copyfile(self.package, destination / self.package.name)
                run = json.dumps({"headSha": "a" * 40, "status": "completed", "conclusion": "success"})
                with patch.object(incremental.subprocess, "check_output", return_value="a" * 40), \
                        patch.object(incremental, "gh", return_value=run), \
                        patch.object(download_action_artifacts, "collect_run", side_effect=collect), \
                        patch.object(gh_publish, "publish", return_value="release") as publish:
                    incremental.publish_runs(["123"], dist, 3, True, local_only=True,
                                             history_root=history, **mode)
                self.assertEqual([(call.args[1], call.kwargs.get("patch_only", False))
                                  for call in publish.call_args_list], expected)
                self.assertTrue((history / self.version / self.package.name).is_file())

    # The separate patch repository must never receive complete installers.
    def test_patch_repository_rejects_full_installers_before_any_remote_call(self):
        with patch.object(gh_publish, "gh") as remote, self.assertRaisesRegex(ValueError, "only manifests and patches"):
            gh_publish.publish(self.dist, gh_publish.PATCH_REPO, False, source_revision="a" * 40)
        remote.assert_not_called()

    # The CLI defaults to patch-only publication in the independent repository.
    def test_cli_defaults_to_patch_repository_without_full_installer_publication(self):
        with patch.object(sys, "argv", ["gh_publish.py", "--dist", str(self.dist)]), patch.object(gh_publish, "publish") as publish:
            gh_publish.main()
        self.assertEqual(publish.call_args.args[1], gh_publish.PATCH_REPO)
        self.assertTrue(publish.call_args.args[-1])

    # A patch-only manifest still authenticates every uploaded patch despite an absent full package.
    def test_patch_only_validation_rejects_corrupt_patch_bytes(self):
        base_dir = self.dist / "old"
        base_dir.mkdir()
        (base_dir / "operit2-cli-windows-x86_64-1.0.0.zip").write_bytes(self.package.read_bytes() + b"old content")
        prepare.prepare(self.dist, [base_dir])
        delta = next(self.dist.glob("*.patch.gz"))
        delta.write_bytes(b"corrupt")
        self.package.unlink()
        with self.assertRaisesRegex(RuntimeError, "Patch is missing or corrupt"):
            release_artifacts.validate_release_assets([path for path in self.dist.iterdir() if path.is_file()])

    # The version controls the GitHub prerelease flag for both existing update channels.
    def test_release_and_preview_use_the_source_repository_and_exact_commit(self):
        for version, prerelease in (("2.0.0", "false"), ("2.0.1-preview.1", "true")):
            with self.subTest(version=version), patch.object(release_artifacts, "release_version", return_value=version), patch.object(prepare, "release_version", return_value=version), patch.object(gh_publish, "release_version", return_value=version):
                dist = self.dist / version
                dist.mkdir()
                (dist / f"operit2-cli-windows-x86_64-{version}.zip").write_bytes(b"PK\x03\x04" + b"signed" * 1000)
                prepare.prepare(dist, [])
                self.calls.clear()
                self.remote = {"assets": [{"name": path.name, "digest": "sha256:" + file_sha256(path)}
                                          for path in dist.iterdir() if path.suffix == ".zip"]}
                with patch.object(gh_publish, "gh", side_effect=self.gh):
                    gh_publish.publish(dist, gh_publish.SOURCE_REPO, False, source_revision="a" * 40)
                create = next(call for call in self.calls if call[:2] == ("release", "create"))
                self.assertIn(f"--prerelease={prerelease}", create)
                self.assertEqual(create[create.index("--repo") + 1], gh_publish.SOURCE_REPO)
                self.assertEqual(create[create.index("--target") + 1], "a" * 40)

    # Preview churn cannot evict stable bases, and newer or draft versions are never used.
    def test_retains_both_channels_in_semver_order(self):
        versions = ("2.0.0-preview.9", "2.0.0-preview.10", "1.9.0", "1.8.0", "2.0.0", "2.1.0-preview.1")
        releases = [{"tagName": "v" + version, "isDraft": False, "isPrerelease": "-" in version} for version in versions]
        releases.append({"tagName": "v2.0.0-preview.11", "isDraft": True, "isPrerelease": True})
        requested = []
        payload = b"PK\x03\x04signed base"
        digest = "sha256:" + hashlib.sha256(payload).hexdigest()

        # Supplies only selected full packages and verifies the repository of every gh request.
        def remote(*args, **kwargs):
            if args[:2] == ("release", "list"):
                self.assertEqual(args[args.index("--repo") + 1], gh_publish.SOURCE_REPO)
                return json.dumps(releases)
            if args[0] == "api":
                version = args[1].rsplit("/v", 1)[1]
                requested.append(version)
                return json.dumps({"assets": [{"name": f"operit2-cli-windows-x86_64-{version}.zip", "digest": digest}]})
            self.assertEqual(args[:2], ("release", "download"))
            self.assertEqual(args[args.index("--repo") + 1], gh_publish.SOURCE_REPO)
            (Path(args[args.index("--dir") + 1]) / args[args.index("--pattern") + 1]).write_bytes(payload)
            return ""

        for target in ("2.0.0-preview.11", "2.0.0"):
            requested.clear()
            with self.subTest(target=target), patch.object(incremental, "gh", side_effect=remote), patch.object(incremental, "release_version", return_value=target), patch.object(release_artifacts, "release_version", return_value=target):
                bases = incremental.download_bases(self.dist / "bases" / target, 1)
            self.assertEqual(requested, ["2.0.0-preview.10", "1.9.0"])
            self.assertEqual([path.name for path in bases], ["v2.0.0-preview.10", "v1.9.0"])

    # Empty history is a valid first release for either channel and requires no download.
    def test_first_release_and_preview_have_no_bases(self):
        for version in ("2.0.0", "2.0.0-preview.1"):
            with self.subTest(version=version), patch.object(incremental, "gh", return_value="[]") as mocked, patch.object(incremental, "release_version", return_value=version):
                self.assertEqual(incremental.download_bases(self.dist / "bases", 3), [])
                self.assertEqual(mocked.call_count, 1)


if __name__ == "__main__":
    unittest.main()
