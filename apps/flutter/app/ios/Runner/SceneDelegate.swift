import Flutter
import UIKit

class SceneDelegate: FlutterSceneDelegate {
  override func scene(
    _ scene: UIScene,
    willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    AppleRuntimeChannel.dataRescueRequested = connectionOptions.shortcutItem?.type == "app.operit.data-rescue"
    super.scene(scene, willConnectTo: session, options: connectionOptions)
    if let controller = window?.rootViewController as? FlutterViewController {
      AppleSnapshotImportInputChannel.register(
        binaryMessenger: controller.binaryMessenger,
        presenter: controller
      )
    }
  }

  /// Makes a warm shortcut explicit instead of silently routing the running business UI.
  override func windowScene(
    _ windowScene: UIWindowScene,
    performActionFor shortcutItem: UIApplicationShortcutItem,
    completionHandler: @escaping (Bool) -> Void
  ) {
    guard shortcutItem.type == "app.operit.data-rescue" else {
      completionHandler(false)
      return
    }
    let alert = UIAlertController(
      title: "数据救援 / Data Rescue",
      message: "请先完全关闭 Operit，再从主屏幕快捷入口重新打开数据救援。",
      preferredStyle: .alert
    )
    alert.addAction(UIAlertAction(title: "知道了", style: .default))
    window?.rootViewController?.present(alert, animated: true)
    completionHandler(true)
  }
}
