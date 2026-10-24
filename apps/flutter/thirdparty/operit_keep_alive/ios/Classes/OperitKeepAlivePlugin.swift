import Flutter
import UIKit
import UserNotifications

/// Exposes iOS's finite execution budget and surfaces its expiration instead of claiming residency.
public final class OperitKeepAlivePlugin: NSObject, FlutterPlugin {
    private let channel: FlutterMethodChannel
    private var leases = Set<String>()
    private var task = UIBackgroundTaskIdentifier.invalid
    private var expired = false
    private var initialized = false
    private var workRevision: Int64 = -1
    private var publishingWork = false
    private var workQueue = [WorkPublication]()

    /// Holds one acknowledged native publication while authorization APIs complete asynchronously.
    private struct WorkPublication {
        let arguments: [String: Any]
        let result: FlutterResult
    }
    private var actionsReady = false
    private static weak var workOwner: OperitKeepAlivePlugin?
    private static var pendingWorkActions = [[String: Any]]()

    /// Associates budget state with this engine's method transport.
    private init(channel: FlutterMethodChannel) {
        self.channel = channel
        super.init()
        NotificationCenter.default.addObserver(self, selector: #selector(foreground),
            name: UIApplication.didBecomeActiveNotification, object: nil)
        NotificationCenter.default.addObserver(self, selector: #selector(foreground),
            name: UIScene.didActivateNotification, object: nil)
    }

    /// Registers the same public method contract as the persistent desktop and Android hosts.
    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "operit/keep_alive", binaryMessenger: registrar.messenger())
        registrar.addMethodCallDelegate(OperitKeepAlivePlugin(channel: channel), channel: channel)
    }

    /// Ends only the application's own budget assertion when its engine is released.
    deinit {
        NotificationCenter.default.removeObserver(self)
        if task != .invalid { UIApplication.shared.endBackgroundTask(task) }
    }

    /// Describes budget exhaustion as unavailable protection while preserving the actual lease ledger.
    private func report() -> [String: Any] {
        return ["protection": expired ? "unavailable" : "timeLimited", "closeToTray": false,
                "activeLeases": leases.count,
                "detail": expired
                    ? "iOS ended the background execution budget; continuing in background is no longer protected."
                    : "iOS provides finite background time and may suspend the process when that budget expires."]
    }

    /// Ends the single native budget assertion without changing the work that owns it.
    private func endBudget() {
        if task != .invalid {
            UIApplication.shared.endBackgroundTask(task)
            task = .invalid
        }
    }

    /// Requests one real iOS background assertion and reports rejection explicitly.
    private func beginBudget(_ reason: String) throws {
        task = UIApplication.shared.beginBackgroundTask(withName: reason) { [weak self] in
            guard let owner = self else { return }
            owner.expired = true
            owner.endBudget()
            owner.channel.invokeMethod("protectionChanged", arguments: owner.report())
        }
        if task == .invalid {
            expired = true
            throw NSError(domain: "operit_keep_alive", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "iOS refused the background execution assertion"])
        }
    }

    /// Starts a fresh budget only after the user actually brings the application to the foreground.
    @objc private func foreground() {
        guard initialized, UIApplication.shared.applicationState == .active else { return }
        endBudget()
        expired = false
        if !leases.isEmpty {
            do { try beginBudget("Core runtime") }
            catch {
                channel.invokeMethod("protectionChanged", arguments: report())
                return
            }
        }
        channel.invokeMethod("protectionChanged", arguments: report())
    }

    /// Requires exact nonempty fields for each acknowledged lease operation.
    private func string(_ arguments: [String: Any], _ key: String) throws -> String {
        guard let value = arguments[key] as? String, !value.isEmpty else {
            throw NSError(domain: "operit_keep_alive", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "Missing keep-alive argument: \(key)"])
        }
        return value
    }

    /// Routes notification responses without replacing the embedding application's notification delegate.
    public static func receiveWorkNotification(_ userInfo: [AnyHashable: Any], actionIdentifier: String) -> Bool {
        guard var action = userInfo["operitKeepAliveWorkAction"] as? [String: Any] else { return false }
        action["action"] = actionIdentifier == "operit-work-stop" ? "stop" : "open"
        pendingWorkActions.append(action)
        workOwner?.flushWorkActions()
        return true
    }

    /// Delivers startup actions after Dart has installed its application-level receiver.
    private func flushWorkActions() {
        guard actionsReady else { return }
        let actions = Self.pendingWorkActions
        Self.pendingWorkActions.removeAll()
        for action in actions { channel.invokeMethod("workAction", arguments: action) }
    }

    /// Serializes asynchronous notification writes so older completions cannot overwrite newer work.
    private func drainWorkPublications() {
        guard !publishingWork, !workQueue.isEmpty else { return }
        let publication = workQueue.removeFirst()
        publishingWork = true
        do {
            try publishWork(publication.arguments) { response in
                self.publishingWork = false
                publication.result(response)
                self.drainWorkPublications()
            }
        } catch {
            publishingWork = false
            publication.result(FlutterError(code: "WORK_PRESENTATION_ERROR", message: error.localizedDescription, details: nil))
            drainWorkPublications()
        }
    }

    /// Publishes a replacement local work notification using the application's existing authorization.
    private func publishWork(_ arguments: [String: Any], result: @escaping FlutterResult) throws {
        guard let revision = arguments["revision"] as? Int64,
              let items = arguments["items"] as? [[String: Any]] else { throw NSError(domain: "operit_keep_alive", code: 20) }
        if revision <= workRevision { result(nil); return }
        guard let work = items.first else {
            UNUserNotificationCenter.current().removeDeliveredNotifications(withIdentifiers: ["operit-background-work"])
            workRevision = revision
            result(nil)
            return
        }
        let title = try string(work, "title")
        let subtitle = try string(work, "subtitle")
        let workId = try string(work, "workId")
        guard let activation = work["activation"] as? [String: String] else { throw NSError(domain: "operit_keep_alive", code: 22) }
        guard let canStop = work["canStop"] as? Bool else { throw NSError(domain: "operit_keep_alive", code: 21) }
        let center = UNUserNotificationCenter.current()
        let content = UNMutableNotificationContent()
        content.title = title
        content.body = subtitle + " · " + (try string(work, "elapsedText"))
        content.threadIdentifier = "operit-background-work"
        content.userInfo = ["operitKeepAliveWorkAction": ["workId": workId, "activation": activation, "action": "open"]]
        content.categoryIdentifier = canStop ? "operit-work-active" : "operit-work-result"
        let open = UNNotificationAction(identifier: "operit-work-open", title: try string(work, "openLabel"), options: [.foreground])
        let stop = UNNotificationAction(identifier: "operit-work-stop", title: try string(work, "stopLabel"), options: [.destructive])
        let active = UNNotificationCategory(identifier: "operit-work-active", actions: [open, stop], intentIdentifiers: [], options: [])
        let ended = UNNotificationCategory(identifier: "operit-work-result", actions: [open], intentIdentifiers: [], options: [])
        center.getNotificationCategories { categories in
            let other = categories.filter { $0.identifier != active.identifier && $0.identifier != ended.identifier }
            center.setNotificationCategories(Set(other).union([active, ended]))
            center.add(UNNotificationRequest(identifier: "operit-background-work", content: content, trigger: nil)) { error in
                DispatchQueue.main.async {
                    if let error = error { result(FlutterError(code: "WORK_PRESENTATION_ERROR", message: error.localizedDescription, details: nil)); return }
                    self.workRevision = revision
                    result(nil)
                }
            }
        }
    }

    /// Tracks work while respecting the host's finite, nonrenewable background budget.
    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        do {
            guard let arguments = call.arguments as? [String: Any] else {
                throw NSError(domain: "operit_keep_alive", code: 3)
            }
            switch call.method {
            case "workCapabilities":
                UNUserNotificationCenter.current().getNotificationSettings { settings in
                    let authorized = settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional
                    DispatchQueue.main.async { result(["updatesOwner": "client", "surface": authorized ? "notification" : "none", "liveUpdates": false]) }
                }
                return
            case "workActionsReady": actionsReady = true; flushWorkActions(); result(nil); return
            case "publishWork":
                workQueue.append(WorkPublication(arguments: arguments, result: result))
                drainWorkPublications()
                return
            case "initialize":
                if let owner = Self.workOwner, owner !== self { throw NSError(domain: "operit_keep_alive", code: 23) }
                Self.workOwner = self
                initialized = true
            case "acquire":
                guard initialized else { throw NSError(domain: "operit_keep_alive", code: 4) }
                let id = try string(arguments, "id")
                let reason = try string(arguments, "reason")
                guard !leases.contains(id), !expired else {
                    throw NSError(domain: "operit_keep_alive", code: 5,
                                  userInfo: [NSLocalizedDescriptionKey: "Duplicate lease or expired iOS execution budget"])
                }
                if leases.isEmpty { try beginBudget(reason) }
                leases.insert(id)
            case "release":
                let id = try string(arguments, "id")
                guard leases.remove(id) != nil else { throw NSError(domain: "operit_keep_alive", code: 6) }
                if leases.isEmpty { endBudget() }
            default: result(FlutterMethodNotImplemented); return
            }
            result(report())
        } catch {
            result(FlutterError(code: "KEEP_ALIVE_ERROR", message: error.localizedDescription, details: nil))
        }
    }
}
