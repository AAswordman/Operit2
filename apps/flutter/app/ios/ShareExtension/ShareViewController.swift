import UIKit
import UniformTypeIdentifiers

/// Copies shared files into the app inbox without loading them into extension memory.
final class ShareViewController: UIViewController {
  private let worker = DispatchQueue(label: "operit.share-import", qos: .userInitiated)
  private var started = false
  private let status = UILabel()

  /// Shows progress while file providers supply their temporary representations.
  override func viewDidLoad() {
    super.viewDidLoad()
    view.backgroundColor = .systemBackground
    status.text = "正在添加附件…"
    status.textAlignment = .center
    status.numberOfLines = 0
    status.translatesAutoresizingMaskIntoConstraints = false
    view.addSubview(status)
    NSLayoutConstraint.activate([
      status.centerXAnchor.constraint(equalTo: view.centerXAnchor),
      status.centerYAnchor.constraint(equalTo: view.centerYAnchor),
      status.leadingAnchor.constraint(greaterThanOrEqualTo: view.leadingAnchor, constant: 24),
      status.trailingAnchor.constraint(lessThanOrEqualTo: view.trailingAnchor, constant: -24),
    ])
  }

  /// Starts exactly one import when the share sheet is presented.
  override func viewDidAppear(_ animated: Bool) {
    super.viewDidAppear(animated)
    guard !started else { return }
    started = true
    let items = extensionContext?.inputItems as? [NSExtensionItem] ?? []
    let providers = items.flatMap { $0.attachments ?? [] }
    worker.async {
      do {
        guard !providers.isEmpty else { throw IncomingFileStore.InboxError.invalidFile }
        let batch = try IncomingFileStore.beginBatch()
        self.importFile(providers, index: 0, batch: batch)
      } catch { self.fail(error) }
    }
  }

  /// Loads each file serially and copies it before the provider callback returns.
  private func importFile(_ providers: [NSItemProvider], index: Int, batch: URL) {
    guard index < providers.count else {
      do {
        try IncomingFileStore.commit(batch)
        DispatchQueue.main.async {
          self.status.text = "附件已保存。打开 Operit2 后将添加到当前对话。"
          let done = UIButton(type: .system)
          done.setTitle("完成", for: .normal)
          done.addTarget(self, action: #selector(self.finish), for: .touchUpInside)
          done.translatesAutoresizingMaskIntoConstraints = false
          self.view.addSubview(done)
          NSLayoutConstraint.activate([
            done.centerXAnchor.constraint(equalTo: self.view.centerXAnchor),
            done.topAnchor.constraint(equalTo: self.status.bottomAnchor, constant: 24),
          ])
        }
      } catch { fail(error, batch: batch) }
      return
    }
    let provider = providers[index]
    guard let type = provider.registeredTypeIdentifiers.first(where: {
      guard let type = UTType($0) else { return false }
      return type.conforms(to: .data) || type.conforms(to: .content)
    }) else {
      fail(IncomingFileStore.InboxError.invalidFile, batch: batch)
      return
    }
    let copyFile: (URL?, Error?) -> Void = { url, error in
      do {
        if let error { throw error }
        guard let url else { throw IncomingFileStore.InboxError.invalidFile }
        // The file exists only for this callback. Finish copying before returning.
        let suggested = provider.suggestedName
        var name = url.lastPathComponent
        if let suggested, !suggested.isEmpty {
          name = suggested
          if (name as NSString).pathExtension.isEmpty && !url.pathExtension.isEmpty {
            name += "." + url.pathExtension
          }
        }
        try IncomingFileStore.append(url, name: name, to: batch)
        self.worker.async { self.importFile(providers, index: index + 1, batch: batch) }
      } catch { self.fail(error, batch: batch) }
    }
    if provider.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
      // File URL providers grant access to the original document, not URL-text bytes.
      provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, error in
        copyFile(item as? URL, error)
      }
    } else {
      provider.loadFileRepresentation(forTypeIdentifier: type, completionHandler: copyFile)
    }
  }

  /// Closes a successfully committed share without sending a chat message.
  @objc private func finish() {
    extensionContext?.completeRequest(returningItems: nil)
  }

  /// Removes incomplete copies and exposes the actual import error in the share sheet.
  private func fail(_ error: Error, batch: URL? = nil) {
    if let batch { try? FileManager.default.removeItem(at: batch) }
    DispatchQueue.main.async {
      let alert = UIAlertController(title: "无法添加附件", message: error.localizedDescription, preferredStyle: .alert)
      alert.addAction(UIAlertAction(title: "关闭", style: .default) { _ in
        self.extensionContext?.cancelRequest(withError: error)
      })
      self.present(alert, animated: true)
    }
  }
}
