import Cocoa
import FlutterMacOS

/// Retains the main engine behind a menu-bar item and protects active Core execution from App Nap.
public final class OperitKeepAlivePlugin: NSObject, FlutterPlugin, NSWindowDelegate, NSMenuDelegate {
    private weak var view: NSView?
    private weak var ownerWindow: NSWindow?
    private weak var previousDelegate: NSWindowDelegate?
    private let channel: FlutterMethodChannel
    private var statusItem: NSStatusItem?
    private var activity: NSObjectProtocol?
    private var leases = Set<String>()
    private var workRevision: Int64 = -1
    private var initialized = false
    private var title = ""
    private var quitMessage = ""

    /// Retains only this engine's channel and view until the main entrypoint claims residency.
    private init(view: NSView?, channel: FlutterMethodChannel) {
        self.view = view
        self.channel = channel
        super.init()
    }

    /// Registers a per-engine transport without creating trays for detached windows.
    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "operit/keep_alive", binaryMessenger: registrar.messenger)
        let plugin = OperitKeepAlivePlugin(view: registrar.view, channel: channel)
        registrar.addMethodCallDelegate(plugin, channel: channel)
    }

    /// Restores delegate ownership and releases native activity resources with the engine.
    deinit {
        if let activity = activity { ProcessInfo.processInfo.endActivity(activity) }
        if let item = statusItem { NSStatusBar.system.removeStatusItem(item) }
        if let window = ownerWindow, window.delegate === self { window.delegate = previousDelegate }
    }

    /// Validates exact caller-owned presentation fields.
    private func string(_ arguments: [String: Any], _ key: String) throws -> String {
        guard let value = arguments[key] as? String, !value.isEmpty else {
            throw NSError(domain: "operit_keep_alive", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Missing keep-alive argument: \(key)"])
        }
        return value
    }

    /// Reports persistent window residency without promising survival of user or system termination.
    private func report() -> [String: Any] {
        return ["protection": "continuous", "closeToTray": initialized,
                "activeLeases": leases.count,
                "detail": "The main engine remains behind its menu-bar item; active leases disable App Nap."]
    }

    /// Reuses native residency after a Dart hot restart without replacing the window delegate.
    private func initialize(_ arguments: [String: Any]) throws {
        if initialized {
            // The new Dart controller starts its work revisions over; native leases remain valid.
            workRevision = -1
            return
        }
        guard let window = view?.window else {
            throw NSError(domain: "operit_keep_alive", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "Main keep-alive window is unavailable"])
        }
        title = try string(arguments, "applicationName")
        quitMessage = try string(arguments, "quitMessage")
        let menu = NSMenu()
        let open = NSMenuItem(title: try string(arguments, "openLabel"), action: #selector(openWindow), keyEquivalent: "")
        open.target = self
        menu.addItem(open)
        menu.addItem(.separator())
        let exit = NSMenuItem(title: try string(arguments, "exitLabel"), action: #selector(exitApplication), keyEquivalent: "")
        exit.target = self
        menu.addItem(exit)
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        guard let button = item.button, let source = NSApp.applicationIconImage,
              let icon = source.copy() as? NSImage else {
            NSStatusBar.system.removeStatusItem(item)
            throw NSError(domain: "operit_keep_alive", code: 3,
                          userInfo: [NSLocalizedDescriptionKey: "Unable to create the menu-bar return path"])
        }
        icon.size = NSSize(width: 18, height: 18)
        button.image = icon
        button.toolTip = title
        item.menu = menu
        statusItem = item
        previousDelegate = window.delegate
        ownerWindow = window
        window.isReleasedWhenClosed = false
        window.delegate = self
        initialized = true
    }

    /// Keeps the main view, engine, and Core alive when its normal close button is clicked.
    public func windowShouldClose(_ sender: NSWindow) -> Bool {
        sender.orderOut(nil)
        return false
    }

    /// Preserves optional window callbacks implemented by Flutter's original delegate.
    public override func responds(to selector: Selector!) -> Bool {
        return super.responds(to: selector) || previousDelegate?.responds(to: selector) == true
    }

    /// Forwards unimplemented delegate messages to the original Flutter window delegate.
    public override func forwardingTarget(for selector: Selector!) -> Any? {
        if previousDelegate?.responds(to: selector) == true { return previousDelegate }
        return super.forwardingTarget(for: selector)
    }

    /// Restores the existing main window instead of creating another runtime owner.
    @objc private func openWindow() {
        NSApp.activate(ignoringOtherApps: true)
        ownerWindow?.makeKeyAndOrderFront(nil)
    }

    /// Requires explicit confirmation before quitting a process with active work leases.
    @objc private func exitApplication() {
        if !leases.isEmpty {
            let alert = NSAlert()
            alert.messageText = title
            alert.informativeText = quitMessage
            alert.alertStyle = .warning
            alert.addButton(withTitle: "Quit")
            alert.addButton(withTitle: "Cancel")
            if alert.runModal() != .alertFirstButtonReturn { return }
        }
        NSApp.terminate(nil)
    }

    /// Owns App Nap protection for the entire period containing at least one execution lease.
    private func acquire(_ id: String, reason: String) throws {
        guard !leases.contains(id) else {
            throw NSError(domain: "operit_keep_alive", code: 4,
                          userInfo: [NSLocalizedDescriptionKey: "Duplicate keep-alive lease"])
        }
        if leases.isEmpty {
            activity = ProcessInfo.processInfo.beginActivity(
                options: [.userInitiatedAllowingIdleSystemSleep, .automaticTerminationDisabled, .suddenTerminationDisabled],
                reason: reason)
        }
        leases.insert(id)
    }

    /// Releases OS execution protection only after the final acknowledged lease finishes.
    private func release(_ id: String) throws {
        guard leases.remove(id) != nil else {
            throw NSError(domain: "operit_keep_alive", code: 5,
                          userInfo: [NSLocalizedDescriptionKey: "Unknown keep-alive lease"])
        }
        if leases.isEmpty, let current = activity {
            ProcessInfo.processInfo.endActivity(current)
            activity = nil
        }
    }

    /// Replaces work menus using stable represented objects for every action.
    private func publishWork(_ arguments: [String: Any]) throws {
        guard let revision = arguments["revision"] as? Int64,
              let items = arguments["items"] as? [[String: Any]],
              let menu = statusItem?.menu else {
            throw NSError(domain: "operit_keep_alive", code: 20,
                          userInfo: [NSLocalizedDescriptionKey: "Invalid work presentation or missing menu"])
        }
        if revision <= workRevision { return }
        let summary = try string(arguments, "summary")
        let section = NSMenuItem(title: summary, action: nil, keyEquivalent: "")
        section.tag = 2401
        let submenu = NSMenu()
        for work in items {
            let item = NSMenuItem(title: try string(work, "title"), action: nil, keyEquivalent: "")
            let details = NSMenu()
            let status = NSMenuItem(title: try string(work, "subtitle"), action: nil, keyEquivalent: "")
            status.tag = 2402
            status.representedObject = work
            details.addItem(status)
            details.delegate = self
            guard let activation = work["activation"] as? [String: String] else { throw NSError(domain: "operit_keep_alive", code: 22) }
            let identity: [String: Any] = ["workId": try string(work, "workId"), "activation": activation]
            let open = NSMenuItem(title: try string(work, "openLabel"), action: #selector(workAction(_:)), keyEquivalent: "")
            open.target = self
            open.representedObject = identity.merging(["action": "open"]) { _, next in next }
            details.addItem(open)
            guard let canStop = work["canStop"] as? Bool else { throw NSError(domain: "operit_keep_alive", code: 21) }
            if canStop {
                let stop = NSMenuItem(title: try string(work, "stopLabel"), action: #selector(workAction(_:)), keyEquivalent: "")
                stop.target = self
                stop.representedObject = identity.merging(["action": "stop"]) { _, next in next }
                details.addItem(stop)
            }
            item.submenu = details
            submenu.addItem(item)
        }
        section.submenu = submenu
        for existing in menu.items where existing.tag == 2401 { menu.removeItem(existing) }
        menu.insertItem(section, at: 0)
        statusItem?.button?.toolTip = title + " · " + summary
        workRevision = revision
    }

    /// Updates elapsed time when a work submenu opens without polling the Core.
    public func menuNeedsUpdate(_ menu: NSMenu) {
        for item in menu.items where item.tag == 2402 {
            guard let work = item.representedObject as? [String: Any],
                  let started = work["startedAtMillis"] as? Int64,
                  let subtitle = work["subtitle"] as? String else { preconditionFailure("Invalid work menu clock") }
            let end: Int64
            if work["endedAtMillis"] is NSNull { end = Int64(Date().timeIntervalSince1970 * 1000) }
            else { end = work["endedAtMillis"] as! Int64 }
            let seconds = (end - started) / 1000
            item.title = subtitle + " · " + String(format: "%02lld:%02lld", seconds / 60, seconds % 60)
        }
    }

    /// Opens the retained window or requests cancellation of the exact represented execution.
    @objc private func workAction(_ sender: NSMenuItem) {
        guard let action = sender.representedObject as? [String: Any] else { preconditionFailure("Work menu has no action identity") }
        if action["action"] as? String == "open" { openWindow() }
        channel.invokeMethod("workAction", arguments: action)
    }

    /// Applies the same lease protocol used by every supported Flutter host.
    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        do {
            guard let arguments = call.arguments as? [String: Any] else {
                throw NSError(domain: "operit_keep_alive", code: 6,
                              userInfo: [NSLocalizedDescriptionKey: "Keep-alive requires an argument map"])
            }
            switch call.method {
            case "workCapabilities":
                result(["updatesOwner": "client", "surface": "tray", "liveUpdates": false]); return
            case "workActionsReady": result(nil); return
            case "publishWork": try publishWork(arguments); result(nil); return
            case "initialize": try initialize(arguments)
            case "acquire":
                guard initialized else { throw NSError(domain: "operit_keep_alive", code: 7) }
                try acquire(string(arguments, "id"), reason: string(arguments, "reason"))
            case "release": try release(string(arguments, "id"))
            default: result(FlutterMethodNotImplemented); return
            }
            result(report())
        } catch {
            result(FlutterError(code: "KEEP_ALIVE_ERROR", message: error.localizedDescription, details: nil))
        }
    }
}
