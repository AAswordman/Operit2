import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  /// Creates the Flutter engine while retaining the window after close.
  override func awakeFromNib() {
    isReleasedWhenClosed = false
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    AppleRuntimeChannel.register(binaryMessenger: flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
  }
}
