import Cocoa
import FlutterMacOS
import UserNotifications

@main
class AppDelegate: FlutterAppDelegate, UNUserNotificationCenterDelegate {
  /// Installs the macOS notification delegate before Flutter creates its first window.
  override func applicationDidFinishLaunching(_ notification: Notification) {
    super.applicationDidFinishLaunching(notification)
    UNUserNotificationCenter.current().delegate = self
    installDataRescueMenuItem()
  }

  /// Adds a native menu command that starts a separate maintenance process.
  private func installDataRescueMenuItem() {
    let item = NSMenuItem(
      title: "Operit2 Data Rescue",
      action: #selector(openDataRescue),
      keyEquivalent: ""
    )
    item.target = self
    NSApp.mainMenu?.item(at: 0)?.submenu?.addItem(item)
  }

  /// Launches this bundle with the standalone rescue argument and no normal route.
  @objc private func openDataRescue() {
    guard let executable = Bundle.main.executableURL else {
      NSApp.presentError(NSError(domain: "Operit2", code: 1, userInfo: [NSLocalizedDescriptionKey: "Operit executable is unavailable."]))
      return
    }
    let process = Process()
    process.executableURL = executable
    process.arguments = ["--data-rescue"]
    do {
      try process.run()
    } catch {
      NSApp.presentError(error)
    }
  }

  /// Forwards a local-notification click to the Flutter notification activation receiver.
  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    let userInfo = response.notification.request.content.userInfo
    guard let activation = userInfo["operitNotificationActivation"] as? [String: Any] else {
      completionHandler()
      return
    }
    AppleRuntimeChannel.receiveNotificationActivation(activation)
    completionHandler()
  }

  /// Keeps the process-level Core alive after the final window closes.
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return false
  }

  /// Restores hidden application windows when the Dock icon is activated.
  override func applicationShouldHandleReopen(
    _ sender: NSApplication,
    hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag {
      for window in sender.windows {
        window.makeKeyAndOrderFront(nil)
      }
    }
    return true
  }

  /// Enables secure restoration for persisted macOS window state.
  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }
}
