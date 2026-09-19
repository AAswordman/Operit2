import Flutter
import UIKit
import XCTest
import Darwin

class RunnerTests: XCTestCase {

  func testBundledTerminalRootFilesystem() throws {
    let url = try XCTUnwrap(Bundle.main.url(forResource: "ish-root", withExtension: "tar.gz"))
    let file = try FileHandle(forReadingFrom: url)
    defer { try? file.close() }
    XCTAssertEqual(try file.read(upToCount: 2), Data([0x1f, 0x8b]))
    XCTAssertGreaterThan(try file.seekToEnd(), 1_000_000)
  }

  func testNativeTerminalBridgeRejectsInvalidJSON() throws {
    let response = try terminalCall("terminalList", json: "not-json")
    XCTAssertEqual(response["error"] as? String, "iSH terminal request is not a JSON object")
  }

  func testNativeTerminalBridgeListsSessions() throws {
    let response = try terminalCall("terminalList", json: "{}")
    XCTAssertNil(response["error"])
    let result = try XCTUnwrap(response["result"] as? [String: Any])
    XCTAssertNotNil(result["sessions"] as? [[String: Any]])
  }

  private func terminalCall(_ command: String, json: String) throws -> [String: Any] {
    typealias Call = @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?
    typealias Release = @convention(c) (UnsafeMutablePointer<CChar>?) -> Void
    let process = try XCTUnwrap(dlopen(nil, RTLD_NOW))
    defer { dlclose(process) }
    let call = unsafeBitCast(try XCTUnwrap(dlsym(process, "operit_ios_ish_terminal_call")), to: Call.self)
    let release = unsafeBitCast(try XCTUnwrap(dlsym(process, "operit_ios_ish_terminal_free")), to: Release.self)
    let pointer = try XCTUnwrap(command.withCString { commandPointer in
      json.withCString { call(commandPointer, $0) }
    })
    defer { release(pointer) }
    let data = Data(String(cString: pointer).utf8)
    return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
  }

  func testIshShellExecutesCommand() throws {
    let finished = expectation(description: "iSH shell executes a command")
    DispatchQueue.global(qos: .userInitiated).async {
      defer { finished.fulfill() }
      do {
        let created = try self.terminalCall("terminalCreateOrGet",
          json: "{\"sessionName\":\"ios-smoke-test\",\"terminalType\":\"shell\"}")
        XCTAssertNil(created["error"], "\(created)")
        let result = try XCTUnwrap(created["result"] as? [String: Any])
        let session = try XCTUnwrap(result["sessionId"] as? String)
        let request: [String: Any] = ["sessionId": session,
          "command": "printf '%s_%s\\n' OPERIT ISH_OK; uname -m", "timeoutMs": 15000]
        let json = String(decoding: try JSONSerialization.data(withJSONObject: request), as: UTF8.self)
        let executed = try self.terminalCall("terminalExecute", json: json)
        XCTAssertNil(executed["error"], "\(executed)")
        let output = try XCTUnwrap(executed["result"] as? [String: Any])
        XCTAssertEqual(output["exitCode"] as? Int, 0, "\(output)")
        XCTAssertEqual(output["timedOut"] as? Bool, false)
        XCTAssertTrue((output["output"] as? String ?? "").contains("OPERIT_ISH_OK"))
        XCTAssertTrue((output["output"] as? String ?? "").contains("i686"))
        for (command, exitCode, expected) in [
          ("sh -c 'exit 7'", 7, ""),
          ("seq 1 3000; printf '%s_%s\\n' LARGE OUTPUT_OK", 0, "LARGE_OUTPUT_OK"),
          ("python3 -c \"print('PYTHON' + '_OK')\"", 0, "PYTHON_OK"),
          ("node -e \"console.log('NODE' + '_OK')\"", 0, "NODE_OK"),
          ("printf '%s_%s\\n' REPEAT OK", 0, "REPEAT_OK")
        ] {
          let request: [String: Any] = ["sessionId": session, "command": command, "timeoutMs": 15000]
          let response = try self.terminalCall("terminalExecute",
            json: String(decoding: try JSONSerialization.data(withJSONObject: request), as: UTF8.self))
          XCTAssertNil(response["error"], "\(response)")
          let result = try XCTUnwrap(response["result"] as? [String: Any])
          XCTAssertEqual(result["exitCode"] as? Int, exitCode, "\(result)")
          XCTAssertEqual(result["timedOut"] as? Bool, false)
          if !expected.isEmpty {
            XCTAssertTrue((result["output"] as? String ?? "").contains(expected), "\(result)")
          }
          NSLog("iSH smoke command passed: %@; exit=%d", command, exitCode)
        }
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("ish-mount-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        try Data("MOUNT_READ_OK".utf8).write(to: directory.appendingPathComponent("probe.txt"))
        let mountPoint = directory.path
        let mountRequest = ["hostDirectory": directory.path, "mountPoint": mountPoint]
        let mounted = try self.terminalCall("managedRuntimeMount",
          json: String(decoding: try JSONSerialization.data(withJSONObject: mountRequest), as: UTF8.self))
        XCTAssertNil(mounted["error"], "\(mounted)")
        let readRequest: [String: Any] = ["sessionId": session,
          "command": "cat '\(mountPoint)/probe.txt'", "timeoutMs": 15000]
        let read = try self.terminalCall("terminalExecute",
          json: String(decoding: try JSONSerialization.data(withJSONObject: readRequest), as: UTF8.self))
        let readResult = try XCTUnwrap(read["result"] as? [String: Any])
        XCTAssertEqual(readResult["exitCode"] as? Int, 0, "\(read)")
        XCTAssertTrue((readResult["output"] as? String ?? "").contains("MOUNT_READ_OK"), "\(read)")
        let startRequest: [String: Any] = ["sessionName": "workspace-path-test",
          "terminalType": "shell", "workingDir": directory.path, "rows": 24, "cols": 42]
        let started = try self.terminalCall("terminalStart",
          json: String(decoding: try JSONSerialization.data(withJSONObject: startRequest), as: UTF8.self))
        let workspace = try XCTUnwrap((started["result"] as? [String: Any])?["sessionId"] as? String, "\(started)")
        let probe: [String: Any] = ["sessionId": workspace,
          "command": "pwd; cat probe.txt; stty size; printf '%s_%s' SHELL WRITE_OK > shell-write.txt", "timeoutMs": 15000]
        let probed = try self.terminalCall("terminalExecute",
          json: String(decoding: try JSONSerialization.data(withJSONObject: probe), as: UTF8.self))
        let probeResult = try XCTUnwrap(probed["result"] as? [String: Any])
        let probeOutput = probeResult["output"] as? String ?? ""
        XCTAssertEqual(probeResult["exitCode"] as? Int, 0, "\(probed)")
        XCTAssertTrue(probeOutput.contains("MOUNT_READ_OK"), probeOutput)
        XCTAssertTrue(probeOutput.contains("24 42"), probeOutput)
        XCTAssertFalse(probeOutput.contains("can't cd"), probeOutput)
        XCTAssertTrue(probeOutput.contains(directory.path), probeOutput)
        XCTAssertEqual(try String(contentsOf: directory.appendingPathComponent("shell-write.txt"), encoding: .utf8), "SHELL_WRITE_OK")
        NSLog("Workspace path and PTY size verified: %@", probeOutput)
      } catch {
        XCTFail("iSH smoke test failed: \(error)")
      }
    }
    wait(for: [finished], timeout: 120)
  }

  // Run twice in separate app processes: first seeds persistent files, second
  // verifies them, including an executable in /usr/local/bin, then cleans up.
  func testIshPersistenceAcrossLaunches() {
    let finished = expectation(description: "iSH persistent filesystem")
    DispatchQueue.global(qos: .userInitiated).async {
      defer { finished.fulfill() }
      do {
        let defaults = UserDefaults.standard
        let key = "operit.ish.persistence-test-token"
        let previous = defaults.string(forKey: key)
        let token = previous ?? UUID().uuidString
        let filename = "/root/.operit-persistence-\(token)"
        let executable = "/usr/local/bin/operit-persistence-\(token)"
        let created = try self.terminalCall("terminalCreateOrGet",
          json: "{\"sessionName\":\"persistence-test\",\"terminalType\":\"shell\"}")
        let session = try XCTUnwrap((created["result"] as? [String: Any])?["sessionId"] as? String)
        let command: String
        if previous == nil {
          command = "printf '%s' '\(token)' > '\(filename)'; printf '#!/bin/sh\\nprintf PERSISTENT_EXEC_OK' > '\(executable)'; chmod +x '\(executable)'; sync"
        } else {
          command = "cat '\(filename)'; '\(executable)'; rm '\(filename)' '\(executable)'"
        }
        let request: [String: Any] = ["sessionId": session, "command": command, "timeoutMs": 15000]
        let response = try self.terminalCall("terminalExecute",
          json: String(decoding: try JSONSerialization.data(withJSONObject: request), as: UTF8.self))
        let result = try XCTUnwrap(response["result"] as? [String: Any])
        XCTAssertEqual(result["exitCode"] as? Int, 0, "\(response)")
        if previous == nil {
          defaults.set(token, forKey: key)
          NSLog("iSH persistent files seeded for restart check")
        } else {
          let output = result["output"] as? String ?? ""
          // Avoid accepting the echoed command, which also contains the token.
          XCTAssertTrue(output.contains("\(token)PERSISTENT_EXEC_OK"), output)
          defaults.removeObject(forKey: key)
          NSLog("iSH home file and installed executable verified across process restart")
        }
      } catch {
        XCTFail("Persistence test failed: \(error)")
      }
    }
    wait(for: [finished], timeout: 60)
  }

}
