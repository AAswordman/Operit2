import WebKit
#if os(iOS)
import Flutter
#else
import FlutterMacOS
#endif

/// Owns native WebKit transfers and their private staging files.
final class WebViewDownloads {
  private let channel: FlutterMethodChannel
  private unowned let registrar: ProxyAPIRegistrar
  private var records: [String: AnyObject] = [:]

  /// Registers transfer commands on the plugin's engine-scoped messenger.
  init(messenger: FlutterBinaryMessenger, registrar: ProxyAPIRegistrar) {
    self.registrar = registrar
    channel = FlutterMethodChannel(name: "operit/webview_downloads", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else { result(FlutterError(code: "detached", message: nil, details: nil)); return }
      guard #available(iOS 14.5, macOS 11.3, *) else {
        result(FlutterError(code: "unsupported_version", message: "Native downloads require iOS 14.5 or macOS 11.3", details: nil)); return
      }
      let args = call.arguments as? [String: Any] ?? [:]
      if call.method == "dispose", let identifier = args["identifier"] as? NSNumber {
        let ids = self.records.compactMap { key, value in
          (value as? Record)?.identifier == identifier.int64Value ? key : nil
        }
        for id in ids { self.release(id, cancel: true) }
        result(nil); return
      }
      guard let id = args["id"] as? String else {
        result(FlutterError(code: "invalid_download", message: "A transfer ID is required", details: nil)); return
      }
      guard let record = self.records[id] as? Record else {
        if call.method == "cancel" { result(nil); return }
        result(FlutterError(code: "download_not_found", message: "The engine transfer has ended", details: nil)); return
      }
      switch call.method {
      case "cancel":
        record.download.cancel { _ in }
        record.publish("cancelled")
        self.release(id, cancel: false)
        result(nil)
      case "read":
        guard record.completed, let file = record.file else {
          result(FlutterError(code: "download_incomplete", message: "The engine file is not complete", details: nil)); return
        }
        do {
          let bytes = try Data(contentsOf: file)
          self.release(id, cancel: false)
          result(FlutterStandardTypedData(bytes: bytes))
        } catch {
          self.release(id, cancel: false)
          result(FlutterError(code: "download_read_failed", message: error.localizedDescription, details: nil))
        }
      default: result(FlutterMethodNotImplemented)
      }
    }
  }

  /// Registers the delegate before WebKit chooses the download destination.
  @available(iOS 14.5, macOS 11.3, *)
  func start(_ download: WKDownload, webView: WKWebView) {
    guard let identifier = registrar.instanceManager.identifierWithStrongReference(forInstance: webView) else {
      download.cancel { _ in }; return
    }
    let record = Record(download: download, identifier: identifier, channel: channel)
    records[record.id] = record
    let id = record.id
    record.onFailure = { [weak self] in self?.release(id, cancel: false) }
    download.delegate = record
  }

  /// Disconnects delegates and removes only this record's staging directory.
  @available(iOS 14.5, macOS 11.3, *)
  private func release(_ id: String, cancel: Bool) {
    guard let record = records.removeValue(forKey: id) as? Record else { return }
    record.observation = nil
    record.download.delegate = nil
    if cancel { record.download.cancel { _ in } }
    if let directory = record.file?.deletingLastPathComponent() { try? FileManager.default.removeItem(at: directory) }
  }

  /// Cancels retained transfers when the Flutter engine is torn down.
  deinit {
    channel.setMethodCallHandler(nil)
    if #available(iOS 14.5, macOS 11.3, *) {
      for id in Array(records.keys) { release(id, cancel: true) }
    }
  }

  @available(iOS 14.5, macOS 11.3, *)
  private final class Record: NSObject, WKDownloadDelegate {
    let id = UUID().uuidString
    let download: WKDownload
    let identifier: Int64
    let channel: FlutterMethodChannel
    var file: URL?
    var name = "download"
    var completed = false
    var observation: NSKeyValueObservation?
    var onFailure: (() -> Void)?
    private var lastProgress = Date.distantPast

    /// Retains the engine operation, preserving its original request body and cookies.
    init(download: WKDownload, identifier: Int64, channel: FlutterMethodChannel) {
      self.download = download; self.identifier = identifier; self.channel = channel
    }

    /// Assigns an isolated file destination without exposing private paths to webpages.
    func download(_ download: WKDownload, decideDestinationUsing response: URLResponse,
                  suggestedFilename: String, completionHandler: @escaping (URL?) -> Void) {
      let directory = FileManager.default.temporaryDirectory.appendingPathComponent("operit-browser-\(id)", isDirectory: true)
      name = (suggestedFilename as NSString).lastPathComponent
      if name.isEmpty || name == "." || name == ".." { name = "download" }
      do {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: false)
        file = directory.appendingPathComponent(name)
        completionHandler(file)
        publish("running")
        observation = download.progress.observe(\.completedUnitCount, options: [.new]) { [weak self] _, _ in
          DispatchQueue.main.async {
            guard let self, !self.completed, Date().timeIntervalSince(self.lastProgress) >= 0.1 else { return }
            self.lastProgress = Date()
            self.publish("running")
          }
        }
      } catch {
        completionHandler(nil)
        publish("failed", error: error.localizedDescription)
        onFailure?()
      }
    }

    /// Publishes successful completion only after WebKit has finished writing the file.
    func downloadDidFinish(_ download: WKDownload) {
      completed = true
      observation = nil
      publish("completed")
    }

    /// Reports native interruptions as terminal failures with a concrete reason.
    func download(_ download: WKDownload, didFailWithError error: Error, resumeData: Data?) {
      observation = nil
      publish("failed", error: error.localizedDescription)
      onFailure?()
    }

    /// Sends engine byte counts to the owning Flutter controller.
    func publish(_ state: String, error: String? = nil) {
      channel.invokeMethod("download", arguments: [
        "identifier": identifier, "id": id,
        "url": download.originalRequest?.url?.absoluteString ?? "",
        "fileName": name, "state": state,
        "receivedBytes": download.progress.completedUnitCount,
        "totalBytes": download.progress.totalUnitCount,
        "error": error ?? ""
      ])
    }
  }
}
