// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import Flutter
import ObjectiveC
import UIKit
import UniformTypeIdentifiers

/// Bridge between a UIDocumentPickerViewController and its Pigeon callback.
class PickerCompletionBridge: NSObject, UIDocumentPickerDelegate {
  let completion: (Result<[String], Error>) -> Void
  /// The plugin instance that owns this object, to ensure that it lives as long as the picker it
  /// serves as a delegate for. Instances are responsible for removing themselves from their owner
  /// on completion.
  let owner: FileSelectorPlugin
  let scoped: Bool

  /// Retains one explicit selection mode until the system picker completes.
  init(completion: @escaping (Result<[String], Error>) -> Void, owner: FileSelectorPlugin, scoped: Bool = false) {
    self.completion = completion
    self.owner = owner
    self.scoped = scoped
  }

  /// Keeps scoped source access alive before returning its file reference.
  func documentPicker(
    _ controller: UIDocumentPickerViewController,
    didPickDocumentsAt urls: [URL]
  ) {
    if scoped {
      do {
        sendResult(try urls.map { try IOSFolderAccessStore.shared.rememberSelection($0).path })
      } catch {
        completion(.failure(error))
        owner.pendingCompletions.remove(self)
      }
    } else {
      sendResult(urls.map({ $0.path }))
    }
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    sendResult([])
  }

  private func sendResult(_ result: [String]) {
    completion(.success(result))
    owner.pendingCompletions.remove(self)
  }
}

/// Bridge for the repository-owned directory picker method channel.
final class DirectoryPickerCompletionBridge: NSObject, UIDocumentPickerDelegate {
  let completion: FlutterResult
  let owner: FileSelectorPlugin
  let multiple: Bool

  init(completion: @escaping FlutterResult, owner: FileSelectorPlugin, multiple: Bool) {
    self.completion = completion
    self.owner = owner
    self.multiple = multiple
  }

  func documentPicker(
    _ controller: UIDocumentPickerViewController,
    didPickDocumentsAt urls: [URL]
  ) {
    do {
      let selected = try urls.map { try IOSFolderAccessStore.shared.rememberSelection($0).path }
      completion(multiple ? selected : Array(selected.prefix(1)))
    } catch {
      completion(FlutterError(
        code: "FOLDER_ACCESS_ERROR",
        message: error.localizedDescription,
        details: nil
      ))
    }
    owner.pendingDirectoryCompletions.remove(self)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    completion([String]())
    owner.pendingDirectoryCompletions.remove(self)
  }
}

/// Bridge for exporting one byte payload through the native iOS document picker.
final class SavePickerCompletionBridge: NSObject, UIDocumentPickerDelegate {
  let completion: FlutterResult
  let owner: FileSelectorPlugin
  let temporaryURL: URL

  init(
    completion: @escaping FlutterResult,
    owner: FileSelectorPlugin,
    temporaryURL: URL
  ) {
    self.completion = completion
    self.owner = owner
    self.temporaryURL = temporaryURL
  }

  func documentPicker(
    _ controller: UIDocumentPickerViewController,
    didPickDocumentsAt urls: [URL]
  ) {
    finish(urls.first?.path)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    finish(nil)
  }

  private func finish(_ path: String?) {
    try? FileManager.default.removeItem(at: temporaryURL)
    completion(path)
    owner.pendingSaveCompletions.remove(self)
  }
}

public class FileSelectorPlugin: NSObject, FlutterPlugin, FileSelectorApi {
  /// Owning references to pending completion callbacks.
  ///
  /// This is necessary since the objects need to live until a UIDocumentPickerDelegate method is
  /// called on the delegate, but the delegate is weak. Objects in this set are responsible for
  /// removing themselves from it.
  var pendingCompletions: Set<PickerCompletionBridge> = []
  var pendingDirectoryCompletions: Set<DirectoryPickerCompletionBridge> = []
  var pendingSaveCompletions: Set<SavePickerCompletionBridge> = []
  /// Overridden document picker, for testing.
  var documentPickerViewControllerOverride: UIDocumentPickerViewController?
  /// The view controller provider, for showing the document picker.
  let viewPresenterProvider: ViewPresenterProvider
  private var directoryChannel: FlutterMethodChannel?
  private var saveChannel: FlutterMethodChannel?
  private var preparingFileSave = false

  public static func register(with registrar: FlutterPluginRegistrar) {
    let instance = FileSelectorPlugin(
      viewPresenterProvider: DefaultViewPresenterProvider(registrar: registrar))
    FileSelectorApiSetup.setUp(binaryMessenger: registrar.messenger(), api: instance)
    instance.installDirectoryChannel(binaryMessenger: registrar.messenger())
    instance.installSaveChannel(binaryMessenger: registrar.messenger())
  }

  init(viewPresenterProvider: ViewPresenterProvider) {
    self.viewPresenterProvider = viewPresenterProvider
  }

  func openFile(config: FileSelectorConfig, completion: @escaping (Result<[String], Error>) -> Void)
  {
    let completionBridge = PickerCompletionBridge(completion: completion, owner: self)
    let documentPicker =
      documentPickerViewControllerOverride
      ?? UIDocumentPickerViewController(
        documentTypes: config.utis,
        in: .import)
    documentPicker.allowsMultipleSelection = config.allowMultiSelection
    documentPicker.delegate = completionBridge

    if let presenter = viewPresenterProvider.viewPresenter {
      pendingCompletions.insert(completionBridge)
      presenter.present(documentPicker, animated: true, completion: nil)
    } else {
      completion(
        .failure(PigeonError(code: "error", message: "No view controller available.", details: nil))
      )
    }
  }

  private func installDirectoryChannel(binaryMessenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(
      name: "dev.flutter.packages.file_selector_ios/directory",
      binaryMessenger: binaryMessenger
    )
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handleDirectoryCall(call, result: result)
    }
    directoryChannel = channel
  }

  private func installSaveChannel(binaryMessenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(
      name: "dev.flutter.packages.file_selector_ios/save",
      binaryMessenger: binaryMessenger
    )
    channel.setMethodCallHandler { [weak self] call, result in
      self?.handleSaveCall(call, result: result)
    }
    saveChannel = channel
  }

  /// Routes a file-backed save separately from the existing byte-payload operation.
  private func handleSaveCall(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    if call.method == "openFileReference" {
      handleFileReferenceOpen(call, result: result)
      return
    }
    if call.method == "saveFileFromPath" {
      handleFileReferenceSave(call, result: result)
      return
    }
    guard call.method == "saveFile" else {
      result(FlutterMethodNotImplemented)
      return
    }
    guard pendingSaveCompletions.isEmpty && !preparingFileSave else {
      result(FlutterError(
        code: "SAVE_IN_PROGRESS",
        message: "A document save is already active.",
        details: nil
      ))
      return
    }
    guard let arguments = call.arguments as? [String: Any],
          let typedData = arguments["bytes"] as? FlutterStandardTypedData,
          let requestedName = arguments["name"] as? String,
          !requestedName.isEmpty else {
      result(FlutterError(
        code: "INVALID_SAVE_ARGS",
        message: "bytes and a non-empty name are required.",
        details: nil
      ))
      return
    }

    let safeName = URL(fileURLWithPath: requestedName).lastPathComponent
    let temporaryURL = FileManager.default.temporaryDirectory
      .appendingPathComponent(UUID().uuidString)
      .appendingPathComponent(safeName.isEmpty ? "untitled" : safeName)
    do {
      try FileManager.default.createDirectory(
        at: temporaryURL.deletingLastPathComponent(),
        withIntermediateDirectories: true
      )
      try typedData.data.write(to: temporaryURL, options: .atomic)
    } catch {
      result(FlutterError(
        code: "SAVE_TEMP_WRITE_FAILED",
        message: error.localizedDescription,
        details: nil
      ))
      return
    }

    presentSavePicker(temporaryURL: temporaryURL, arguments: arguments, result: result)
  }

  /// Opens an existing document directly instead of asking iOS to copy the entire archive.
  private func handleFileReferenceOpen(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let arguments = call.arguments as? [String: Any],
          let utis = arguments["utis"] as? [String], !utis.isEmpty else {
      result(FlutterError(code: "INVALID_OPEN_ARGS", message: "Document types are required.", details: nil))
      return
    }
    guard let presenter = viewPresenterProvider.viewPresenter else {
      result(FlutterError(code: "PICK_UNAVAILABLE", message: "No view controller available.", details: nil))
      return
    }
    let bridge = PickerCompletionBridge(completion: { selection in
      switch selection {
      case .success(let paths): result(paths.first)
      case .failure(let error): result(FlutterError(code: "SOURCE_OPEN_FAILED", message: error.localizedDescription, details: nil))
      }
    }, owner: self, scoped: true)
    let picker = UIDocumentPickerViewController(documentTypes: utis, in: .open)
    picker.allowsMultipleSelection = false
    picker.delegate = bridge
    pendingCompletions.insert(bridge)
    presenter.present(picker, animated: true, completion: nil)
  }

  /// Copies a host-owned file without passing its bytes through Dart or MethodChannel.
  private func handleFileReferenceSave(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard pendingSaveCompletions.isEmpty && !preparingFileSave else {
      result(FlutterError(code: "SAVE_IN_PROGRESS", message: "A document save is already active.", details: nil))
      return
    }
    guard let arguments = call.arguments as? [String: Any],
          let sourcePath = arguments["sourcePath"] as? String,
          !sourcePath.isEmpty,
          let requestedName = arguments["name"] as? String,
          !requestedName.isEmpty else {
      result(FlutterError(code: "INVALID_SAVE_ARGS", message: "sourcePath and name are required.", details: nil))
      return
    }
    let safeName = URL(fileURLWithPath: requestedName).lastPathComponent
    let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
    let temporaryURL = directory.appendingPathComponent(safeName)
    preparingFileSave = true
    DispatchQueue.global(qos: .userInitiated).async {
      do {
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try FileManager.default.copyItem(at: URL(fileURLWithPath: sourcePath), to: temporaryURL)
        DispatchQueue.main.async {
          self.preparingFileSave = false
          self.presentSavePicker(temporaryURL: temporaryURL, arguments: arguments, result: result)
        }
      } catch {
        var message = error.localizedDescription
        do {
          if FileManager.default.fileExists(atPath: directory.path) {
            try FileManager.default.removeItem(at: directory)
          }
        } catch {
          message += "; temporary export cleanup failed: \(error.localizedDescription)"
        }
        let failureMessage = message
        DispatchQueue.main.async {
          self.preparingFileSave = false
          result(FlutterError(code: "SAVE_TEMP_WRITE_FAILED", message: failureMessage, details: nil))
        }
      }
    }
  }

  /// Presents the export picker for a prepared file without materializing its contents.
  private func presentSavePicker(temporaryURL: URL, arguments: [String: Any], result: @escaping FlutterResult) {
    let bridge = SavePickerCompletionBridge(
      completion: result,
      owner: self,
      temporaryURL: temporaryURL
    )
    let picker: UIDocumentPickerViewController
    if #available(iOS 14.0, *) {
      picker = UIDocumentPickerViewController(forExporting: [temporaryURL], asCopy: true)
    } else {
      picker = UIDocumentPickerViewController(url: temporaryURL, in: .exportToService)
    }
    if let initialPath = arguments["initialDirectory"] as? String,
       (initialPath as NSString).isAbsolutePath {
      picker.directoryURL = URL(fileURLWithPath: initialPath)
    }
    picker.allowsMultipleSelection = false
    picker.delegate = bridge
    guard let presenter = viewPresenterProvider.viewPresenter else {
      try? FileManager.default.removeItem(at: temporaryURL)
      result(FlutterError(
        code: "SAVE_UNAVAILABLE",
        message: "No view controller available.",
        details: nil
      ))
      return
    }
    pendingSaveCompletions.insert(bridge)
    presenter.present(picker, animated: true, completion: nil)
  }

  private func handleDirectoryCall(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard call.method == "pickDirectory" else {
      result(FlutterMethodNotImplemented)
      return
    }
    guard pendingDirectoryCompletions.isEmpty else {
      result(FlutterError(
        code: "PICK_IN_PROGRESS",
        message: "A directory picker is already open.",
        details: nil
      ))
      return
    }
    let arguments = call.arguments as? [String: Any]
    let multiple = arguments?["multiple"] as? Bool ?? false
    let bridge = DirectoryPickerCompletionBridge(
      completion: result,
      owner: self,
      multiple: multiple
    )
    let picker = UIDocumentPickerViewController(
      forOpeningContentTypes: [UTType.folder],
      asCopy: false
    )
    if let initialPath = arguments?["initialDirectory"] as? String,
       (initialPath as NSString).isAbsolutePath {
      picker.directoryURL = URL(fileURLWithPath: initialPath)
    }
    picker.allowsMultipleSelection = multiple
    picker.delegate = bridge
    guard let presenter = viewPresenterProvider.viewPresenter else {
      result(FlutterError(
        code: "PICK_UNAVAILABLE",
        message: "No view controller available.",
        details: nil
      ))
      return
    }
    pendingDirectoryCompletions.insert(bridge)
    presenter.present(picker, animated: true, completion: nil)
  }

}

/// Keeps iOS security-scoped directory access active while the app process runs.
final class IOSFolderAccessStore {
  static let shared = IOSFolderAccessStore()
  private var activeURLs: [String: URL] = [:]

  func rememberSelection(_ inputURL: URL) throws -> URL {
    let url = inputURL.standardizedFileURL
    let path = url.path
    if activeURLs[path] != nil { return url }
    guard url.startAccessingSecurityScopedResource() else {
      throw NSError(
        domain: "file_selector_ios",
        code: 1,
        userInfo: [NSLocalizedDescriptionKey: "iOS did not grant access to the selected folder: \(path)"]
      )
    }
    do {
      activeURLs[path] = url
      return url
    } catch {
      url.stopAccessingSecurityScopedResource()
      throw error
    }
  }

}
