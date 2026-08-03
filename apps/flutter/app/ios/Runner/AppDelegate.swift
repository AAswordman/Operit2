import Flutter
import UIKit
import UserNotifications

@main
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)
    if let controller = window?.rootViewController as? FlutterViewController {
      AppleRuntimeChannel.register(binaryMessenger: controller.binaryMessenger)
      AppleSnapshotImportInputChannel.register(
        binaryMessenger: controller.binaryMessenger,
        presenter: controller
      )
    }
    UNUserNotificationCenter.current().delegate = self
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  /// Forwards a local-notification click to the Flutter notification activation receiver.
  override func userNotificationCenter(
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
}
