"""Verify catalog packaging and publication without using the network."""
import gzip
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import models_dev_bundle as bundle


class ModelsDevBundleTests(unittest.TestCase):
    """Cover the generated asset's contents and build entry points."""

    def test_downloaded_pair_is_embedded_and_deterministic(self):
        """Pack the complete upstream pair into a reproducible gzip asset."""
        providers = {"openai": {"models": {"gpt-5": {"tool_call": True}}}}
        models = {"new-model": {"id": "new-model"}}
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "catalog.json.gz"
            with patch.object(bundle, "download_catalog", side_effect=[providers, models] * 2) as download:
                bundle.refresh_bundle(destination)
                first = destination.read_bytes()
                bundle.refresh_bundle(destination)
            self.assertEqual(first, destination.read_bytes())
            self.assertEqual(json.loads(gzip.decompress(first)),
                             {"providers": providers, "models": models})
            self.assertEqual([call.args[0] for call in download.call_args_list],
                             ["api", "models", "api", "models"])

    def test_failed_second_download_preserves_previous_snapshot(self):
        """Never publish half of a new catalog when the second request fails."""
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "catalog.json.gz"
            destination.write_bytes(b"previous snapshot")
            with patch.object(bundle, "download_catalog", side_effect=[{"provider": {}}, OSError("offline")]):
                with self.assertRaisesRegex(OSError, "offline"):
                    bundle.refresh_bundle(destination)
            self.assertEqual(destination.read_bytes(), b"previous snapshot")
            self.assertEqual(list(Path(temporary).iterdir()), [destination])

    def test_download_rejects_invalid_documents(self):
        """Reject an error page or empty catalog before it reaches the runtime."""
        import io
        for document in (b"{}", b"[]", b"not json"):
            with self.subTest(document=document):
                with patch.object(bundle.urllib.request, "urlopen", return_value=io.BytesIO(document)):
                    with self.assertRaises(ValueError):
                        bundle.download_catalog("api")

    def test_release_refreshes_existing_snapshot_once(self):
        """Ensure release builds refresh even when development already prepared an asset."""
        with patch.object(bundle, "_prepared", False), \
                patch.object(bundle.Path, "is_file", return_value=True), \
                patch.object(bundle, "refresh_bundle") as refresh:
            bundle.prepare_build_catalog(["cargo", "check"])
            refresh.assert_not_called()
            bundle.prepare_build_catalog(["cargo", "build", "--release"])
            bundle.prepare_build_catalog(["cargo", "rustc", "--release"])
            refresh.assert_called_once_with()

    def test_missing_development_snapshot_is_prepared(self):
        """Prepare first-use assets through the same wrapper used by desktop build hooks."""
        with patch.object(bundle, "_prepared", False), \
                patch.object(bundle.Path, "is_file", return_value=False), \
                patch.object(bundle, "refresh_bundle") as refresh:
            bundle.prepare_build_catalog(["cargo", "test"])
            refresh.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
