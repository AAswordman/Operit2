import Flutter
import Foundation
import UIKit

/// Streams user-selected snapshot files to Flutter without materializing their full contents.
final class AppleSnapshotImportInputChannel: NSObject, UIDocumentPickerDelegate {
  private static let channelName = "operit/snapshot_import_input"
  private static var shared: AppleSnapshotImportInputChannel?

  private final class OpenSnapshotInput {
    let url: URL
    let handle: FileHandle
    let usesSecurityScope: Bool

    /// Opens a selected URL for bounded reads while retaining its security scope.
    init(url: URL) throws {
      self.url = url
      usesSecurityScope = url.startAccessingSecurityScopedResource()
      handle = try FileHandle(forReadingFrom: url)
    }

    /// Closes the file handle and releases its acquired security scope.
    func close() {
      try? handle.close()
      if usesSecurityScope {
        url.stopAccessingSecurityScopedResource()
      }
    }
  }

  private var channel: FlutterMethodChannel
  private weak var presenter: UIViewController?
  private var pickResult: FlutterResult?
  private var inputs: [String: OpenSnapshotInput] = [:]

  /// Registers the process-level input channel on the current Flutter binary messenger.
  static func register(binaryMessenger: FlutterBinaryMessenger, presenter: UIViewController) {
    if let shared {
      shared.attach(binaryMessenger: binaryMessenger, presenter: presenter)
      return
    }
    shared = AppleSnapshotImportInputChannel(
      binaryMessenger: binaryMessenger,
      presenter: presenter
    )
  }

  /// Creates a stream-owning input channel attached to the initial Flutter engine.
  private init(binaryMessenger: FlutterBinaryMessenger, presenter: UIViewController) {
    channel = FlutterMethodChannel(name: Self.channelName, binaryMessenger: binaryMessenger)
    self.presenter = presenter
    super.init()
    installMethodHandler()
  }

  /// Rebinds this input channel to a replacement Flutter engine.
  private func attach(binaryMessenger: FlutterBinaryMessenger, presenter: UIViewController) {
    channel.setMethodCallHandler(nil)
    channel = FlutterMethodChannel(name: Self.channelName, binaryMessenger: binaryMessenger)
    self.presenter = presenter
    installMethodHandler()
  }

  /// Installs MethodChannel dispatch for picker, bounded-read, and close operations.
  private func installMethodHandler() {
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handle(call: call, result: result)
    }
  }

  /// Dispatches one Flutter snapshot-input request to the matching native operation.
  private func handle(call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "pick":
      pickSnapshot(result: result)
    case "readChunk":
      readChunk(call: call, result: result)
    case "close":
      closeInput(call: call, result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  /// Presents the system document picker for a single ZIP-based snapshot file.
  private func pickSnapshot(result: @escaping FlutterResult) {
    guard pickResult == nil else {
      result(FlutterError(code: "PICK_IN_PROGRESS", message: "A snapshot picker is already open", details: nil))
      return
    }
    guard let presenter else {
      result(FlutterError(code: "PICK_UNAVAILABLE", message: "No view controller is available for snapshot selection", details: nil))
      return
    }
    pickResult = result
    let picker = UIDocumentPickerViewController(
      documentTypes: ["public.zip-archive", "public.data"],
      in: .open
    )
    picker.allowsMultipleSelection = false
    picker.delegate = self
    presenter.present(picker, animated: true)
  }

  /// Resolves a selected document into a token, display name, byte length, and open stream.
  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    let result = pickResult
    pickResult = nil
    guard let result else { return }
    guard let url = urls.first else {
      result(FlutterError(code: "MISSING_DOCUMENT", message: "Snapshot picker returned no document", details: nil))
      return
    }
    do {
      let input = try OpenSnapshotInput(url: url)
      let byteLength = try byteLength(for: url)
      let name = url.lastPathComponent
      guard !name.isEmpty && byteLength >= 0 else {
        input.close()
        throw SnapshotInputError.invalidMetadata
      }
      let token = UUID().uuidString
      inputs[token] = input
      result(["token": token, "name": name, "byteLength": byteLength])
    } catch {
      result(FlutterError(code: "OPEN_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Completes a cancelled picker request without opening an input stream.
  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    let result = pickResult
    pickResult = nil
    result?(nil)
  }

  /// Reads one bounded byte chunk from an open selected snapshot file.
  private func readChunk(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let arguments = call.arguments as? [String: Any],
          let token = arguments["token"] as? String,
          let maxBytes = arguments["maxBytes"] as? Int,
          maxBytes > 0 else {
      result(FlutterError(code: "INVALID_ARGS", message: "readChunk expects a token and positive maxBytes", details: nil))
      return
    }
    guard let input = inputs[token] else {
      result(FlutterError(code: "UNKNOWN_INPUT", message: "Snapshot input token is not open", details: nil))
      return
    }
    do {
      let data = try input.handle.read(upToCount: min(maxBytes, 64 * 1024)) ?? Data()
      result(FlutterStandardTypedData(bytes: data))
    } catch {
      result(FlutterError(code: "READ_FAILED", message: error.localizedDescription, details: nil))
    }
  }

  /// Closes one selected snapshot input and frees its native resources.
  private func closeInput(call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let arguments = call.arguments as? [String: Any],
          let token = arguments["token"] as? String else {
      result(FlutterError(code: "INVALID_ARGS", message: "close expects a token", details: nil))
      return
    }
    guard let input = inputs.removeValue(forKey: token) else {
      result(FlutterError(code: "UNKNOWN_INPUT", message: "Snapshot input token is not open", details: nil))
      return
    }
    input.close()
    result(nil)
  }

  /// Reads a selected document's authoritative byte length before upload begins.
  private func byteLength(for url: URL) throws -> Int64 {
    let values = try url.resourceValues(forKeys: [.fileSizeKey])
    guard let byteLength = values.fileSize else {
      throw SnapshotInputError.invalidMetadata
    }
    return Int64(byteLength)
  }

  /// Releases every open file handle if this channel is discarded with the Flutter engine.
  deinit {
    for input in inputs.values {
      input.close()
    }
  }

  /// Represents picker metadata that cannot form a valid upload session.
  private enum SnapshotInputError: LocalizedError {
    case invalidMetadata

    /// Provides the user-visible reason for rejecting a selected document.
    var errorDescription: String? {
      "Selected snapshot metadata is invalid"
    }
  }
}
