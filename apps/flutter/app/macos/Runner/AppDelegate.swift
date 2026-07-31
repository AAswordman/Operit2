import Cocoa
import FlutterMacOS
import UserNotifications

@main
class AppDelegate: FlutterAppDelegate, UNUserNotificationCenterDelegate {
  /// Installs the macOS notification delegate before Flutter creates its first window.
  override func applicationDidFinishLaunching(_ notification: Notification) {
    super.applicationDidFinishLaunching(notification)
    UNUserNotificationCenter.current().delegate = self
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
