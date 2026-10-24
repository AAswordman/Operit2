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
    }
  }
}
