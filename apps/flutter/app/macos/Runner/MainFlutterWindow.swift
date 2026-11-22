import Cocoa
import desktop_multi_window
import FlutterMacOS
import file_selector_macos

class MainFlutterWindow: NSWindow {
  private var lifecycleObservers: [NSObjectProtocol] = []
  private var visibilityChannel: FlutterMethodChannel?
  private var lastVisibility: Bool?

  private var mainUiVisible: Bool { isVisible && !isMiniaturized && !NSApp.isHidden }

  /// Only the retained main window determines application foreground/background.
  private func publishVisibility() {
    guard let channel = visibilityChannel else { return }
    let visible = mainUiVisible
    guard lastVisibility != visible else { return }
    lastVisibility = visible
    channel.invokeMethod("visibilityChanged", arguments: visible)
  }

  override func order(_ place: NSWindow.OrderingMode, relativeTo otherWin: Int) {
    super.order(place, relativeTo: otherWin)
    publishVisibility()
  }

  /// Distinguishes focus, minimization, application hiding and compositor occlusion.
  private func logLifecycle(_ event: String) {
    NSLog("[MainWindowLifecycle] platform=macos event=%@ window=%ld visible=%@ miniaturized=%@ key=%@ appActive=%@ appHidden=%@ occluded=%@",
      event, windowNumber, String(isVisible), String(isMiniaturized), String(isKeyWindow),
      String(NSApp.isActive), String(NSApp.isHidden), String(!occlusionState.contains(.visible)))
  }

  private func observeLifecycle() {
    for name in [NSWindow.didBecomeKeyNotification, NSWindow.didResignKeyNotification,
                 NSWindow.didMiniaturizeNotification, NSWindow.didDeminiaturizeNotification,
                 NSWindow.didChangeOcclusionStateNotification, NSWindow.willCloseNotification] {
      lifecycleObservers.append(NotificationCenter.default.addObserver(
        forName: name, object: self, queue: .main
      ) { [weak self] notification in
        self?.logLifecycle(notification.name.rawValue)
        self?.publishVisibility()
        if notification.name == NSWindow.willCloseNotification {
          DispatchQueue.main.async { [weak self] in self?.publishVisibility() }
        }
      })
    }
    for name in [NSApplication.didBecomeActiveNotification, NSApplication.didResignActiveNotification,
                 NSApplication.didHideNotification, NSApplication.didUnhideNotification] {
      lifecycleObservers.append(NotificationCenter.default.addObserver(
        forName: name, object: nil, queue: .main
      ) { [weak self] notification in
        self?.logLifecycle(notification.name.rawValue)
        self?.publishVisibility()
      })
    }
    logLifecycle("observers-installed")
  }

  deinit {
    for observer in lifecycleObservers { NotificationCenter.default.removeObserver(observer) }
  }

  /// Creates the Flutter engine while retaining the window after close.
  override func awakeFromNib() {
    isReleasedWhenClosed = false
    observeLifecycle()
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    registerPlugins(for: flutterViewController)
    let channel = FlutterMethodChannel(
      name: "operit/app_visibility", binaryMessenger: flutterViewController.engine.binaryMessenger)
    visibilityChannel = channel
    channel.setMethodCallHandler { [weak self] call, result in
      guard call.method == "getVisibility" else {
        result(FlutterMethodNotImplemented)
        return
      }
      result(self?.mainUiVisible ?? false)
    }
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
