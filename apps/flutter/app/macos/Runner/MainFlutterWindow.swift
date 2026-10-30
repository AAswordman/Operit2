import Cocoa
import desktop_multi_window
import FlutterMacOS
import file_selector_macos

class MainFlutterWindow: NSWindow {
  /// Creates the Flutter engine while retaining the window after close.
  override func awakeFromNib() {
    isReleasedWhenClosed = false
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    registerPlugins(for: flutterViewController)
    AppleRuntimeChannel.register(binaryMessenger: flutterViewController.engine.binaryMessenger)
    if !ProcessInfo.processInfo.arguments.contains("--data-rescue") {
      FlutterMultiWindowPlugin.setOnWindowCreatedCallback { childFlutterViewController in
        self.registerPlugins(for: childFlutterViewController)
        AppleRuntimeChannel.register(binaryMessenger: childFlutterViewController.engine.binaryMessenger)
      }
    }

    super.awakeFromNib()
  }

  /// Registers only the file picker in the standalone rescue process.
  private func registerPlugins(for registry: FlutterPluginRegistry) {
    if ProcessInfo.processInfo.arguments.contains("--data-rescue") {
      FileSelectorPlugin.register(with: registry.registrar(forPlugin: "FileSelectorPlugin"))
    } else {
      RegisterGeneratedPlugins(registry: registry)
      OperitWindowChrome.register(with: registry.registrar(forPlugin: "OperitWindowChrome"))
    }
  }
}

/// Uses the native caption and controls over the Flutter navigation surface.
final class OperitWindowChrome: NSObject, FlutterPlugin {
  private weak var view: NSView?
  private let channel: FlutterMethodChannel
  private var observers: [NSObjectProtocol] = []
  private var lastTopInset: Double?

  private init(view: NSView?, channel: FlutterMethodChannel) {
    self.view = view
    self.channel = channel
    super.init()
  }

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "operit/window_chrome", binaryMessenger: registrar.messenger)
    let plugin = OperitWindowChrome(view: registrar.view, channel: channel)
    registrar.addMethodCallDelegate(plugin, channel: channel)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard call.method == "configure" else {
      result(FlutterMethodNotImplemented)
      return
    }
    guard let window = view?.window,
          let arguments = call.arguments as? [String: Any],
          let dark = arguments["dark"] as? Bool,
          let color = arguments["backgroundColor"] as? NSNumber else {
      result(FlutterError(code: "window_unavailable", message: "Window is unavailable", details: nil))
      return
    }
    window.titleVisibility = .hidden
    window.titlebarAppearsTransparent = true
    window.styleMask.insert(.fullSizeContentView)
    if #available(macOS 11.0, *) {
      window.titlebarSeparatorStyle = .none
    }
    window.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
    let argb = color.uint32Value
    window.backgroundColor = NSColor(
      srgbRed: CGFloat((argb >> 16) & 0xff) / 255,
      green: CGFloat((argb >> 8) & 0xff) / 255,
      blue: CGFloat(argb & 0xff) / 255, alpha: 1)
    if observers.isEmpty {
      for name in [NSWindow.didResizeNotification, NSWindow.didEnterFullScreenNotification,
                   NSWindow.didExitFullScreenNotification, NSWindow.didChangeScreenNotification] {
        observers.append(NotificationCenter.default.addObserver(
          forName: name, object: window, queue: .main
        ) { [weak self] _ in self?.publishTopInset() })
      }
    }
    let inset = topInset()
    lastTopInset = inset
    result(inset)
  }

  /// Full screen has no reserved caption; the system shows its controls on hover.
  private func topInset() -> Double {
    guard let window = view?.window, !window.styleMask.contains(.fullScreen) else { return 0 }
    return Double(max(0, (window.contentView?.bounds.height ?? 0) - window.contentLayoutRect.height))
  }

  private func publishTopInset() {
    let inset = topInset()
    if lastTopInset != inset {
      lastTopInset = inset
      channel.invokeMethod("topInsetChanged", arguments: inset)
    }
  }

  deinit {
    for observer in observers { NotificationCenter.default.removeObserver(observer) }
  }
}
