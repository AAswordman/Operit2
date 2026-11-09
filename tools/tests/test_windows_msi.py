"""Check MSI upgrade ordering and complete, stable payload harvesting."""

from pathlib import Path
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import build_windows_msi as installer

NS = {"w": installer.WIX_NAMESPACE}


class WindowsMsiTests(unittest.TestCase):
    # Keep Windows upgrade ordering consistent with supported release channels.
    def test_versions_preserve_release_order(self):
        versions = (
            "2.0.0-dev.1", "2.0.0-dev.199", "2.0.0-preview.1", "2.0.0-preview.30",
            "2.0.0-preview.31", "2.0.0-preview.199", "2.0.0-rc.1", "2.0.0-rc.199",
            "2.0.0", "2.0.1-dev.1", "2.0.1", "2.1.0", "3.0.0",
        )
        mapped = [tuple(map(int, installer.msi_version(version).split("."))) for version in versions]
        self.assertEqual(mapped, sorted(set(mapped)))

    # Fail explicitly for versions that Windows Installer cannot represent safely.
    def test_versions_reject_overflow_and_unsupported_channels(self):
        for version in ("256.0.0", "2.256.0", "2.0.66", "2.0.0-preview.200",
                        "2.0.0-preview.01", "2.0.0-beta.1", "2.0.0+3"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                installer.msi_version(version)

    # Include Unicode assets and preserve component IDs when unrelated files change.
    def test_payload_includes_every_file_with_stable_component_identity(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            release = root / "release"
            for name in (*installer.REQUIRED_FILES, "data/flutter_assets/中文/hello.txt"):
                path = release / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"payload")
            payload = root / "payload.wxs"
            installer.write_payload(release, payload)
            before = ET.parse(payload)
            files = before.findall(".//w:File", NS)
            self.assertEqual(len(files), len(installer.REQUIRED_FILES) + 1)
            self.assertEqual(sum(file.get("Id") == "AppExecutable" for file in files), 1)
            components = {node.get("Id"): node.get("Guid") for node in before.findall(".//w:Component", NS)}
            for node in before.findall(".//w:Component", NS):
                registry = node.find("w:RegistryValue", NS)
                self.assertEqual(registry.get("Root"), "HKCU")
                self.assertEqual(registry.get("KeyPath"), "yes")
            directory_ids = {"INSTALLFOLDER"} | {node.get("Id") for node in before.findall(".//w:Directory", NS)}
            self.assertEqual(directory_ids, {node.get("Directory") for node in before.findall(".//w:RemoveFolder", NS)})
            (release / "added.txt").write_text("new file")
            installer.write_payload(release, payload)
            after = ET.parse(payload)
            after_components = {node.get("Id"): node.get("Guid") for node in after.findall(".//w:Component", NS)}
            self.assertTrue(components.items() <= after_components.items())

    # A missing runtime or asset must prevent generation of a broken installer.
    def test_payload_rejects_missing_runtime(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(RuntimeError, "Required Windows release file"):
                installer.write_payload(root, root / "payload.wxs")


if __name__ == "__main__":
    unittest.main()
