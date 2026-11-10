import Foundation
import CoreFoundation

/// Stores share-extension and document-open files in an atomic App Group inbox.
enum IncomingFileStore {
  static let notificationName = "app.operit.incoming-files" as CFString
  struct Entry: Codable {
    let token: String
    let name: String
    let byteLength: Int64
    let ordinal: Int
  }

  /// Locates the explicitly configured container shared by the app and extension.
  static func root() throws -> URL {
    guard let group = Bundle.main.object(forInfoDictionaryKey: "OperitIncomingAppGroup") as? String,
          let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group) else {
      throw InboxError.unavailable
    }
    let root = container.appendingPathComponent("IncomingAttachments", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    return root
  }

  /// Creates a private batch that becomes visible only after all providers finish.
  static func beginBatch() throws -> URL {
    let batch = try root().appendingPathComponent("Staging", isDirectory: true)
      .appendingPathComponent(String(format: "%020.0f-", Date().timeIntervalSince1970 * 1000) + UUID().uuidString, isDirectory: true)
    try FileManager.default.createDirectory(at: batch, withIntermediateDirectories: true)
    return batch
  }

  /// Copies one provider-owned file before its temporary URL or security scope expires.
  static func append(_ source: URL, name: String, to batch: URL) throws {
    guard source.isFileURL, !name.isEmpty else { throw InboxError.invalidFile }
    let scoped = source.startAccessingSecurityScopedResource()
    defer { if scoped { source.stopAccessingSecurityScopedResource() } }
    let token = UUID().uuidString
    let ordinal = try FileManager.default.contentsOfDirectory(atPath: batch.path).count
    let directory = batch.appendingPathComponent(token, isDirectory: true)
    try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
    do {
      let target = directory.appendingPathComponent("content")
      var coordinationError: NSError?
      var copyError: Error?
      NSFileCoordinator().coordinate(readingItemAt: source, options: .withoutChanges, error: &coordinationError) { url in
        do { try FileManager.default.copyItem(at: url, to: target) }
        catch { copyError = error }
      }
      if let error = coordinationError { throw error }
      if let error = copyError { throw error }
      let values = try target.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
      guard values.isRegularFile == true, let size = values.fileSize else { throw InboxError.invalidFile }
      let entry = Entry(token: token, name: name, byteLength: Int64(size), ordinal: ordinal)
      try JSONEncoder().encode(entry).write(to: directory.appendingPathComponent("metadata.json"), options: .atomic)
    } catch {
      try? FileManager.default.removeItem(at: directory)
      throw error
    }
  }

  /// Atomically publishes a fully copied batch for the main application's composer.
  static func commit(_ batch: URL) throws {
    let inbox = try root().appendingPathComponent("Pending", isDirectory: true)
    try FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true)
    try FileManager.default.moveItem(at: batch, to: inbox.appendingPathComponent(batch.lastPathComponent))
    CFNotificationCenterPostNotification(CFNotificationCenterGetDarwinNotifyCenter(),
      CFNotificationName(rawValue: notificationName), nil, nil, true)
  }

  /// Enumerates committed batches, preserving files across app and extension restarts.
  static func batches() throws -> [[(Entry, URL)]] {
    let inbox = try root().appendingPathComponent("Pending", isDirectory: true)
    try FileManager.default.createDirectory(at: inbox, withIntermediateDirectories: true)
    let batches = try FileManager.default.contentsOfDirectory(at: inbox, includingPropertiesForKeys: nil).sorted { $0.lastPathComponent < $1.lastPathComponent }
    return try batches.map { batch in
      let files: [(Entry, URL)] = try FileManager.default.contentsOfDirectory(at: batch, includingPropertiesForKeys: nil)
        .map { directory in
          let metadata = try Data(contentsOf: directory.appendingPathComponent("metadata.json"))
          let entry = try JSONDecoder().decode(Entry.self, from: metadata)
          guard entry.token == directory.lastPathComponent else { throw InboxError.invalidFile }
          return (entry, directory.appendingPathComponent("content"))
        }
      return files.sorted { $0.0.ordinal < $1.0.ordinal }
    }
  }

  /// Removes one consumed input and its now-empty batch directory.
  static func remove(_ file: URL) throws {
    let directory = file.deletingLastPathComponent()
    let batch = directory.deletingLastPathComponent()
    try FileManager.default.removeItem(at: directory)
    if try FileManager.default.contentsOfDirectory(atPath: batch.path).isEmpty {
      try FileManager.default.removeItem(at: batch)
    }
  }

  enum InboxError: LocalizedError {
    case unavailable
    case invalidFile

    /// Explains configuration and provider failures to the application or share sheet.
    var errorDescription: String? {
      switch self {
      case .unavailable: return "The Operit2 incoming-file App Group is unavailable."
      case .invalidFile: return "The shared item is not a readable file."
      }
    }
  }
}
