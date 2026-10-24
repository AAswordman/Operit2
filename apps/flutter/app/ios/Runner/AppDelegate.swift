import Flutter
import UIKit
import UserNotifications
import file_selector_ios
import operit_keep_alive

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    UNUserNotificationCenter.current().delegate = self
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    if AppleRuntimeChannel.dataRescueRequested || ProcessInfo.processInfo.arguments.contains("--data-rescue") {
      // The iOS plugin registry returns an optional registrar.
      if let registrar = engineBridge.pluginRegistry.registrar(forPlugin: "FileSelectorPlugin") {
        FileSelectorPlugin.register(with: registrar)
      }
    } else {
      GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    }
    AppleRuntimeChannel.register(binaryMessenger: engineBridge.applicationRegistrar.messenger())
  }

  /// Forwards a local-notification click to the Flutter notification activation receiver.
  override func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    let userInfo = response.notification.request.content.userInfo
    if OperitKeepAlivePlugin.receiveWorkNotification(userInfo, actionIdentifier: response.actionIdentifier) {
      completionHandler()
      return
    }
    guard let activation = userInfo["operitNotificationActivation"] as? [String: Any] else {
      completionHandler()
      return
    }
    AppleRuntimeChannel.receiveNotificationActivation(activation)
    completionHandler()
  }
}
