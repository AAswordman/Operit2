// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import Foundation

/// Retains the original security-scoped URLs; only directory grants need bookmarks.
final class IOSFolderAccessStore {
  static let shared = IOSFolderAccessStore()
  private var activeURLs: [String: URL] = [:]
  private let bookmarkKey = "operit.selectedResourceBookmarks"
  private let defaults: UserDefaults
  private let startAccessing: (URL) -> Bool
  private let stopAccessing: (URL) -> Void
  private let makeBookmark: (URL) throws -> Data

  init(
    defaults: UserDefaults = .standard,
    startAccessing: @escaping (URL) -> Bool = { $0.startAccessingSecurityScopedResource() },
    stopAccessing: @escaping (URL) -> Void = { $0.stopAccessingSecurityScopedResource() },
    makeBookmark: @escaping (URL) throws -> Data = {
      try $0.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
    }
  ) {
    self.defaults = defaults
    self.startAccessing = startAccessing
    self.stopAccessing = stopAccessing
    self.makeBookmark = makeBookmark
    let saved = defaults.dictionary(forKey: bookmarkKey) as? [String: Data] ?? [:]
    for (_, data) in saved {
      var stale = false
      if let url = try? URL(
        resolvingBookmarkData: data, options: [.withoutUI], relativeTo: nil,
        bookmarkDataIsStale: &stale
      ), startAccessing(url) {
        let key = url.standardizedFileURL.path
        if activeURLs[key] != nil {
          stopAccessing(url)
        } else {
          activeURLs[key] = url
        }
      }
    }
  }

  deinit {
    for url in activeURLs.values { stopAccessing(url) }
  }

  func rememberSelection(_ url: URL, persist: Bool = true) throws -> URL {
    // Standardize only the lookup key. Rebuilding the selected URL can discard its grant.
    let path = url.standardizedFileURL.path
    if activeURLs[path] != nil { return url }
    let scoped = startAccessing(url)
    if !scoped {
      if persist {
        throw NSError(
          domain: "file_selector_ios", code: 1,
          userInfo: [NSLocalizedDescriptionKey: "iOS did not grant access to the selected folder: \(path)"]
        )
      }
      // Container files and files covered by an existing grant need no new scope.
      // Verify a real coordinated read can be opened before accepting that case.
      try Self.verifyReadableFile(url)
    }
    do {
      if persist {
        let data = try makeBookmark(url)
        var saved = defaults.dictionary(forKey: bookmarkKey) as? [String: Data] ?? [:]
        saved[path] = data
        defaults.set(saved, forKey: bookmarkKey)
      }
      if scoped { activeURLs[path] = url }
      return url
    } catch {
      if scoped { stopAccessing(url) }
      throw error
    }
  }

  private static func verifyReadableFile(_ url: URL) throws {
    var coordinationError: NSError?
    var readError: Error?
    NSFileCoordinator().coordinate(
      readingItemAt: url, options: .withoutChanges, error: &coordinationError
    ) { readableURL in
      do {
        let handle = try FileHandle(forReadingFrom: readableURL)
        // Opening the descriptor verifies access without loading document contents.
        handle.closeFile()
      } catch {
        readError = error
      }
    }
    if let error = coordinationError { throw error }
    if let error = readError { throw error }
  }
}
