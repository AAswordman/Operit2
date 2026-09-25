import AppKit
import FlutterMacOS
import Security

/// Federated macOS implementation for the cross-platform folder access API.
public final class OperitFolderAccessPlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "operit/folder_access",
      binaryMessenger: registrar.messenger
    )
    registrar.addMethodCallDelegate(OperitFolderAccessPlugin(), channel: channel)
    FolderAccessStore.shared.restorePersistedAccess()
  }

  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard call.method == "pickDirectory" || call.method == "pickDirectories" else {
      result(FlutterMethodNotImplemented)
      return
    }
    let multiple = call.method == "pickDirectories"
    let args = call.arguments as? [String: Any] ?? [:]
    let initialPath = args["initialDirectory"] as? String
    DispatchQueue.main.async {
      let panel = NSOpenPanel()
      panel.canChooseFiles = false
      panel.canChooseDirectories = true
      panel.allowsMultipleSelection = multiple
      if let initialPath, (initialPath as NSString).isAbsolutePath {
        panel.directoryURL = URL(fileURLWithPath: initialPath)
      }
      let finish: (NSApplication.ModalResponse) -> Void = { response in
        guard response == .OK else {
          result(multiple ? [] : nil)
          return
        }
        do {
          let paths = try panel.urls.map {
            try FolderAccessStore.shared.rememberSelection($0).path
          }
          if multiple {
            result(paths)
          } else {
            result(paths.first)
          }
        } catch {
          result(FlutterError(
            code: "FOLDER_ACCESS_ERROR",
            message: error.localizedDescription,
            details: nil
          ))
        }
      }
      if let window = NSApp.keyWindow {
        panel.beginSheetModal(for: window, completionHandler: finish)
      } else {
        finish(panel.runModal())
      }
    }
  }
}

/// Owns macOS security-scoped bookmarks for both Flutter and the in-process Rust host.
/// Bookmark bytes stay native and survive restarts; the public API only exposes paths.
public final class FolderAccessStore {
  public static let shared = FolderAccessStore()
  private let bookmarkKey = "operit.securityScopedBookmarks"
  private var activeURLs: [String: URL] = [:]
  private init() {}

  deinit {
    activeURLs.values.forEach { $0.stopAccessingSecurityScopedResource() }
  }

  public func restorePersistedAccess() {
    dispatchPrecondition(condition: .onQueue(.main))
    guard isSandboxed else { return }
    for path in bookmarks().keys {
      restore(path: path)
    }
  }

  public func rememberSelection(_ inputURL: URL) throws -> URL {
    dispatchPrecondition(condition: .onQueue(.main))
    let url = inputURL.standardizedFileURL
    guard isSandboxed, !isInContainer(url) else { return url }
    let path = url.path
    if activeURLs[path] != nil { return url }
    guard url.startAccessingSecurityScopedResource() else {
      throw FolderAccessError.denied(path)
    }
    do {
      let data = try url.bookmarkData(
        options: [.withSecurityScope],
        includingResourceValuesForKeys: nil,
        relativeTo: nil
      )
      save(data, for: path)
      activeURLs[path] = url
      return url
    } catch {
      url.stopAccessingSecurityScopedResource()
      throw error
    }
  }

  public func ensureAccess(to inputURL: URL) throws {
    dispatchPrecondition(condition: .onQueue(.main))
    let url = inputURL.standardizedFileURL
    guard isSandboxed, !isInContainer(url) else { return }
    if activeURLs.keys.contains(where: { isWithin(url.path, root: $0) }) { return }
    for (path, _) in bookmarks().sorted(by: { $0.key.count > $1.key.count })
      where isWithin(url.path, root: path) {
      restore(path: path)
      if activeURLs.keys.contains(where: { isWithin(url.path, root: $0) }) { return }
    }
    throw FolderAccessError.notAuthorized(url.path)
  }

  private func restore(path: String) {
    guard activeURLs[path] == nil, let data = bookmarks()[path] else { return }
    do {
      var stale = false
      let url = try URL(
        resolvingBookmarkData: data,
        options: [.withSecurityScope, .withoutUI],
        relativeTo: nil,
        bookmarkDataIsStale: &stale
      ).standardizedFileURL
      // A moved bookmark must not authorize the old configured path. Ask the
      // user to select its new location rather than silently changing storage.
      guard url.path == path else { return }
      guard url.startAccessingSecurityScopedResource() else { return }
      activeURLs[path] = url
      if stale { save(url, for: path) }
    } catch {
      NSLog("Folder access bookmark resolve failed (%@): %@", path, error.localizedDescription)
    }
  }

  private var isSandboxed: Bool {
    var code: SecCode?
    var staticCode: SecStaticCode?
    var info: CFDictionary?
    guard SecCodeCopySelf(SecCSFlags(), &code) == errSecSuccess,
          let code,
          SecCodeCopyStaticCode(code, SecCSFlags(), &staticCode) == errSecSuccess,
          let staticCode,
          SecCodeCopySigningInformation(
            staticCode,
            SecCSFlags(rawValue: kSecCSSigningInformation),
            &info
          ) == errSecSuccess,
          let details = info as? [String: Any],
          let entitlements = details[kSecCodeInfoEntitlementsDict as String] as? [String: Any]
    else { return true }
    return entitlements["com.apple.security.app-sandbox"] as? Bool ?? false
  }

  private func isInContainer(_ url: URL) -> Bool {
    let container = FileManager.default.urls(
      for: .applicationSupportDirectory,
      in: .userDomainMask
    )[0].standardizedFileURL.path
    return isWithin(url.path, root: container)
  }

  private func isWithin(_ path: String, root: String) -> Bool {
    path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
  }

  private func bookmarks() -> [String: Data] {
    UserDefaults.standard.dictionary(forKey: bookmarkKey) as? [String: Data] ?? [:]
  }

  private func save(_ url: URL, for path: String) {
    do {
      let data = try url.bookmarkData(
        options: [.withSecurityScope],
        includingResourceValuesForKeys: nil,
        relativeTo: nil
      )
      save(data, for: path)
    } catch {
      NSLog("Folder access bookmark refresh failed (%@): %@", path, error.localizedDescription)
    }
  }

  private func save(_ data: Data, for path: String) {
    var values = bookmarks()
    values[path] = data
    UserDefaults.standard.set(values, forKey: bookmarkKey)
  }
}

private enum FolderAccessError: LocalizedError {
  case denied(String)
  case notAuthorized(String)

  var errorDescription: String? {
    switch self {
    case .denied(let path):
      return "macOS did not grant access to the selected folder: \(path). Please choose it again."
    case .notAuthorized(let path):
      return "Choose this folder with the macOS folder picker before using it: \(path)"
    }
  }
}
