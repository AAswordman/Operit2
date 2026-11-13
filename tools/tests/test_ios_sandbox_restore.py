"""Execute the iOS storage restore implementation with Foundation and fake FFI."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "apps/flutter/app/ios/Runner/AppleRuntimeChannel.swift"


@unittest.skipUnless(shutil.which("swiftc"), "Swift compiler is required")
class IosSandboxRestoreTests(unittest.TestCase):
    def test_restores_container_paths_without_changing_storage_ownership(self):
        source = SOURCE.read_text()
        start = source.index("  private func restoreSandboxPaths(")
        end = source.index("\n  /// Writes the client bootstrap", start)
        method = source[start:end].replace(
            "private func restoreSandboxPaths", "func restoreSandboxPaths", 1
        ).replace("NSHomeDirectory()", "testHome")
        harness = r'''
import Foundation
var testHome = ""
var writes = [String]()
enum RuntimeChannelError: Error { case createFailed(String) }
func operit_flutter_bridge_runtime_bootstrap_write(_ root: String, _ content: String) -> String {
  writes.append(content)
  return "{\"ok\":true}"
}
final class Harness {
  func takeString(_ value: String) -> String { value }
__METHOD__
}
struct Failure: Error { let message: String }
func expect(_ condition: @autoclosure () -> Bool, _ message: String) throws {
  if !condition() { throw Failure(message: message) }
}
let oldID = "AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA"
let newID = "BBBBBBBB-BBBB-BBBB-BBBB-BBBBBBBBBBBB"
let suffix = "/Library/Application Support/Operit2/runtime data"
let harness = Harness()
var checked = 0
func check(_ label: String, _ oldRoot: String, _ home: String, _ expected: String) throws {
  testHome = home
  writes.removeAll()
  let config: [String: Any] = [
    "confirmed": true, "runtimeRoot": oldRoot, "workspaceRoot": oldRoot + "-workspaces",
    "activeIdentityId": "existing-user", "identities": [["id": "existing-user"]],
    "startupThemeMode": "dark"
  ]
  let encoded = String(decoding: try JSONSerialization.data(withJSONObject: config), as: UTF8.self)
  let response = String(decoding: try JSONSerialization.data(withJSONObject: ["ok": true, "value": encoded]), as: UTF8.self)
  let output = try harness.restoreSandboxPaths(response, defaultRuntimeRoot: home + suffix)
  let envelope = try JSONSerialization.jsonObject(with: Data(output.utf8)) as! [String: Any]
  let restored = try JSONSerialization.jsonObject(with: Data((envelope["value"] as! String).utf8)) as! [String: Any]
  try expect(restored["runtimeRoot"] as? String == expected, "\(label): wrong runtime root: \(restored["runtimeRoot"]!)")
  try expect(restored["workspaceRoot"] as? String == expected + "-workspaces", "\(label): wrong workspace root")
  try expect(restored["activeIdentityId"] as? String == "existing-user", "\(label): identity changed")
  try expect(restored["startupThemeMode"] as? String == "dark", "\(label): theme changed")
  try expect(writes.count == (expected == oldRoot ? 0 : 1), "\(label): unexpected persistence")
  if let saved = writes.first {
    let persisted = try JSONSerialization.jsonObject(with: Data(saved.utf8)) as! NSDictionary
    try expect(persisted.isEqual(to: restored), "\(label): restored configuration was not persisted")
  }
  checked += 1
}
let mobile = "/var/mobile/Containers/Data/Application/"
for (oldPrefix, currentPrefix) in [
  (mobile, mobile), ("/private" + mobile, mobile),
  (mobile, "/private" + mobile), ("/private" + mobile, "/private" + mobile)
] {
  try check("relocated \(oldPrefix) to \(currentPrefix)", oldPrefix + oldID + suffix,
            currentPrefix + newID, currentPrefix + newID + suffix)
}
try check("unchanged container", mobile + newID + suffix, mobile + newID, mobile + newID + suffix)
try check("external folder", "/external/runtime", mobile + newID, "/external/runtime")
try check("App Group folder", "/var/mobile/Containers/Shared/AppGroup/" + oldID + suffix,
          mobile + newID, "/var/mobile/Containers/Shared/AppGroup/" + oldID + suffix)
try check("parent traversal", mobile + oldID + "/../other/runtime", mobile + newID,
          mobile + oldID + "/../other/runtime")
let simA = "/simulators/device-a/data/Containers/Data/Application/"
let simB = "/simulators/device-b/data/Containers/Data/Application/"
try check("different simulator", simA + oldID + suffix, simB + newID, simA + oldID + suffix)
for response in ["{\"ok\":true,\"value\":null}", "{\"ok\":false,\"error\":\"unavailable\"}"] {
  let output = try harness.restoreSandboxPaths(response, defaultRuntimeRoot: testHome + suffix)
  try expect(output == response, "empty or failed bootstrap response was modified")
  checked += 1
}
let manager = FileManager.default
let temp = manager.temporaryDirectory.appendingPathComponent(UUID().uuidString)
defer { try? manager.removeItem(at: temp) }
let base = temp.path + "/Containers/Data/Application/"
let oldRoot = base + oldID + suffix
let newRoot = base + newID + suffix
try manager.createDirectory(atPath: oldRoot, withIntermediateDirectories: true)
try Data("existing user data".utf8).write(to: URL(fileURLWithPath: oldRoot + "/marker"))
try check("preserve retained container data", oldRoot, base + newID, newRoot)
let copied = try String(contentsOfFile: newRoot + "/marker", encoding: .utf8)
try expect(copied == "existing user data", "retained container data was not copied")
try expect(manager.fileExists(atPath: oldRoot + "/marker"), "source container data was removed")
try Data("current user data".utf8).write(to: URL(fileURLWithPath: newRoot + "/marker"))
try check("preserve current container data", oldRoot, base + newID, newRoot)
let retained = try String(contentsOfFile: newRoot + "/marker", encoding: .utf8)
try expect(retained == "current user data", "current container data was overwritten")
print("Passed \(checked) iOS sandbox restore scenarios")
'''.replace("__METHOD__", method)
        with tempfile.TemporaryDirectory(prefix="operit-ios-storage-test-") as directory:
            swift = Path(directory) / "main.swift"
            binary = Path(directory) / "restore-test"
            swift.write_text(harness)
            compiled = subprocess.run(
                ["swiftc", str(swift), "-o", str(binary)],
                capture_output=True, text=True, timeout=60,
            )
            self.assertEqual(compiled.returncode, 0, compiled.stderr)
            result = subprocess.run([str(binary)], capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("Passed 13 iOS sandbox restore scenarios", result.stdout)


if __name__ == "__main__":
    unittest.main()
