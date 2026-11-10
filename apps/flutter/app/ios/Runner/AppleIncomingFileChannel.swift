import Flutter
import Foundation
import CoreFoundation

/// Delivers App Group inbox files through the same bounded-input contract as Android.
final class AppleIncomingFileChannel {
  private static var shared: AppleIncomingFileChannel?
  private static var startupURLs: [[(URL, Bool)]] = []
  private let channel: FlutterMethodChannel
  private let worker = DispatchQueue(label: "operit.incoming-files", qos: .userInitiated)
  private var inputs: [String: URL] = [:]
  private var handles: [String: FileHandle] = [:]
  private var ready = false
  private var pendingErrors: [String] = []

  /// Attaches the main engine's input receiver before Flutter starts its handshake.
  static func register(binaryMessenger: FlutterBinaryMessenger) {
    shared = AppleIncomingFileChannel(binaryMessenger: binaryMessenger)
    for urls in startupURLs { shared?.importURLs(urls) }
    startupURLs.removeAll()
  }

  /// Retains document scopes even when scene connection precedes engine registration.
  static func receive(_ urls: [URL]) {
    guard !urls.isEmpty else { return }
    // Acquire access before leaving the scene callback; release after the copy completes.
    let scoped = urls.map { ($0, $0.startAccessingSecurityScopedResource()) }
    if let shared { shared.importURLs(scoped) }
    else { startupURLs.append(scoped) }
  }

  /// Copies opened documents on the input worker while retaining provider authorization.
  private func importURLs(_ scoped: [(URL, Bool)]) {
    worker.async {
      defer { for (url, acquired) in scoped where acquired { url.stopAccessingSecurityScopedResource() } }
      do {
        let batch = try IncomingFileStore.beginBatch()
        do {
          for (url, _) in scoped { try IncomingFileStore.append(url, name: url.lastPathComponent, to: batch) }
          try IncomingFileStore.commit(batch)
        } catch {
          try? FileManager.default.removeItem(at: batch)
          throw error
        }
        self.dispatchPending()
      } catch { self.emitError(error) }
    }
  }

  /// Collects newly committed share-extension files whenever the app becomes active.
  static func refresh() {
    guard let receiver = shared else { return }
    receiver.worker.async { receiver.dispatchPending() }
  }

  /// Creates a channel whose file handles are exclusively owned by the serial worker.
  private init(binaryMessenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(name: "operit/incoming_files", binaryMessenger: binaryMessenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else { return }
      self.worker.async { self.handle(call, result: result) }
    }
    CFNotificationCenterAddObserver(CFNotificationCenterGetDarwinNotifyCenter(),
      Unmanaged.passUnretained(self).toOpaque(), { _, observer, _, _, _ in
        guard let observer else { return }
        let receiver = Unmanaged<AppleIncomingFileChannel>.fromOpaque(observer).takeUnretainedValue()
        receiver.worker.async { receiver.dispatchPending() }
      }, IncomingFileStore.notificationName, nil, .deliverImmediately)
  }

  /// Drains startup files atomically and handles bounded reads and temporary-file cleanup.
  private func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    do {
      switch call.method {
      case "ready":
        ready = true
        let batches = try collect()
        let errors = pendingErrors
        pendingErrors.removeAll()
        DispatchQueue.main.async {
          result(batches)
          for message in errors { self.channel.invokeMethod("error", arguments: message) }
        }
      case "readChunk":
        guard let args = call.arguments as? [String: Any], let token = args["token"] as? String,
              let maxBytes = args["maxBytes"] as? Int, maxBytes > 0, let url = inputs[token] else {
          throw IncomingFileStore.InboxError.invalidFile
        }
        let handle: FileHandle
        if let existing = handles[token] { handle = existing }
        else { handle = try FileHandle(forReadingFrom: url); handles[token] = handle }
        let bytes = try handle.read(upToCount: min(maxBytes, 1024 * 1024)) ?? Data()
        DispatchQueue.main.async { result(FlutterStandardTypedData(bytes: bytes)) }
      case "close":
        guard let args = call.arguments as? [String: Any], let token = args["token"] as? String else {
          throw IncomingFileStore.InboxError.invalidFile
        }
        try handles.removeValue(forKey: token)?.close()
        if let url = inputs.removeValue(forKey: token) { try IncomingFileStore.remove(url) }
        DispatchQueue.main.async { result(nil) }
      default:
        DispatchQueue.main.async { result(FlutterMethodNotImplemented) }
      }
    } catch {
      DispatchQueue.main.async { result(FlutterError(code: "INCOMING_FILE_FAILED", message: error.localizedDescription, details: nil)) }
    }
  }

  /// Marks each input as delivered once per main-engine lifetime.
  private func collect() throws -> [[[String: Any]]] {
    try IncomingFileStore.batches().compactMap { batch in
      let files = batch.compactMap { entry, url -> [String: Any]? in
        guard inputs[entry.token] == nil else { return nil }
        inputs[entry.token] = url
        return ["token": entry.token, "name": entry.name, "byteLength": entry.byteLength]
      }
      return files.isEmpty ? nil : files
    }
  }

  /// Emits committed files after Dart's receiver has completed its startup handshake.
  private func dispatchPending() {
    guard ready else { return }
    do {
      for batch in try collect() {
        DispatchQueue.main.async { self.channel.invokeMethod("files", arguments: batch) }
      }
    } catch { emitError(error) }
  }

  /// Reports actual provider, copy, or App Group failures through the installed receiver.
  private func emitError(_ error: Error) {
    guard ready else { pendingErrors.append(error.localizedDescription); return }
    DispatchQueue.main.async { self.channel.invokeMethod("error", arguments: error.localizedDescription) }
  }

  /// Releases handles when the Flutter engine's channel is replaced.
  deinit {
    CFNotificationCenterRemoveEveryObserver(CFNotificationCenterGetDarwinNotifyCenter(),
      Unmanaged.passUnretained(self).toOpaque())
    for handle in handles.values { try? handle.close() }
  }
}
