import Foundation

func expectEqual<T: Equatable>(_ actual: T, _ expected: T) {
  precondition(actual == expected, "Expected \(expected), received \(actual)")
}

func expectNil(_ value: Any?) { precondition(value == nil) }
func expectTrue(_ value: Bool) { precondition(value) }
func fail(_ message: String) { preconditionFailure(message) }

func expectThrows(_ action: @autoclosure () throws -> URL) {
  do { _ = try action() } catch { return }
  preconditionFailure("Expected selection to fail")
}

final class IOSFolderAccessStoreTests {
  private var defaults: UserDefaults!
  private var suite: String!
  private var directory: URL!

  func setUpWithError() throws {
    suite = "operit.file-selector-tests.\(UUID().uuidString)"
    defaults = UserDefaults(suiteName: suite)!
    directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
  }

  func tearDownWithError() throws {
    defaults.removePersistentDomain(forName: suite)
    try FileManager.default.removeItem(at: directory)
  }

  func testReadableFilesNeedNoNewScopeOrBookmark() throws {
    let store = IOSFolderAccessStore(
      defaults: defaults, startAccessing: { _ in false },
      makeBookmark: { _ in fail("Files must not require persistent bookmarks"); return Data() }
    )
    for name in ["script.js", "package.toolpkg", "document.md", "empty.txt"] {
      let url = directory.appendingPathComponent(name)
      let contents = name == "empty.txt" ? Data() : Data([0, 255, 1])
      try contents.write(to: url)
      expectEqual(try store.rememberSelection(url, persist: false), url)
      expectEqual(try Data(contentsOf: url), contents)
    }
    expectNil(defaults.dictionary(forKey: "operit.selectedResourceBookmarks"))
  }

  func testUnreadableFileStillFails() throws {
    let store = IOSFolderAccessStore(defaults: defaults, startAccessing: { _ in false })
    expectThrows(try store.rememberSelection(
      directory.appendingPathComponent("missing.js"), persist: false
    ))
  }

  func testOriginalURLIsUsedAndScopeIsBalanced() throws {
    // The picker URL must reach both scope operations unchanged, including path spelling.
    let url = URL(string: directory.absoluteString + "child/../script.js")!
    var opened: [String] = []
    var closed: [String] = []
    var store: IOSFolderAccessStore? = IOSFolderAccessStore(
      defaults: defaults,
      startAccessing: { opened.append($0.absoluteString); return true },
      stopAccessing: { closed.append($0.absoluteString) },
      makeBookmark: { _ in fail("Files must not create bookmarks"); return Data() }
    )
    expectEqual(try store!.rememberSelection(url, persist: false).absoluteString, url.absoluteString)
    // An equivalent path reuses the retained grant instead of leaking another scope.
    _ = try store!.rememberSelection(url.standardizedFileURL, persist: false)
    expectEqual(opened, [url.absoluteString])
    expectTrue(closed.isEmpty)
    store = nil
    expectEqual(closed, [url.absoluteString])
  }

  func testDirectoryStillRequiresAnAuthorizedGrant() throws {
    let store = IOSFolderAccessStore(defaults: defaults, startAccessing: { _ in false })
    expectThrows(try store.rememberSelection(directory))
    expectNil(defaults.dictionary(forKey: "operit.selectedResourceBookmarks"))
  }

  func testDirectoryPersistsBookmark() throws {
    let bookmark = Data([1, 2, 3])
    let store = IOSFolderAccessStore(
      defaults: defaults, startAccessing: { _ in true }, stopAccessing: { _ in },
      makeBookmark: { url in expectEqual(url, self.directory); return bookmark }
    )
    _ = try store.rememberSelection(directory)
    let saved = defaults.dictionary(forKey: "operit.selectedResourceBookmarks") as? [String: Data]
    expectEqual(saved?[directory.standardizedFileURL.path], bookmark)
  }

  func testFailedBookmarkReleasesScopeAndCanRetry() throws {
    var starts = 0
    var stops = 0
    let store = IOSFolderAccessStore(
      defaults: defaults,
      startAccessing: { _ in starts += 1; return true },
      stopAccessing: { _ in stops += 1 },
      makeBookmark: { _ in throw NSError(domain: "test", code: 1) }
    )
    expectThrows(try store.rememberSelection(directory))
    expectThrows(try store.rememberSelection(directory))
    expectEqual(starts, 2)
    expectEqual(stops, 2)
    expectNil(defaults.dictionary(forKey: "operit.selectedResourceBookmarks"))
  }
}

let tests: [(String, (IOSFolderAccessStoreTests) -> () throws -> Void)] = [
  ("readable files without a new scope", IOSFolderAccessStoreTests.testReadableFilesNeedNoNewScopeOrBookmark),
  ("unreadable files fail", IOSFolderAccessStoreTests.testUnreadableFileStillFails),
  ("original URL and balanced scopes", IOSFolderAccessStoreTests.testOriginalURLIsUsedAndScopeIsBalanced),
  ("directory grants remain required", IOSFolderAccessStoreTests.testDirectoryStillRequiresAnAuthorizedGrant),
  ("directory bookmarks persist", IOSFolderAccessStoreTests.testDirectoryPersistsBookmark),
  ("failed bookmarks release scopes", IOSFolderAccessStoreTests.testFailedBookmarkReleasesScopeAndCanRetry),
]
for (name, run) in tests {
  let fixture = IOSFolderAccessStoreTests()
  try fixture.setUpWithError()
  do {
    try run(fixture)()
    print("PASS: \(name)")
  } catch {
    try fixture.tearDownWithError()
    throw error
  }
  try fixture.tearDownWithError()
}
print("Executed \(tests.count) tests, with 0 failures")
