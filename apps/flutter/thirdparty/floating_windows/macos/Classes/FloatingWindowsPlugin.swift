import Cocoa
import FlutterMacOS

import desktop_multi_window

public final class FloatingWindowsPlugin: NSObject, FlutterPlugin {
    private weak var view: NSView?
    private var configured = false
    private var focusable = false
    private var topmost = true
    private var draggable = false
    private var fill = false
    private var screenBounds: NSRect?
    private var pointerDown: NSPoint?
    private var windowDown: NSPoint?
    private var dragging = false
    private var monitor: Any?
    private var channel: FlutterMethodChannel?
    deinit {
        if let monitor = monitor { NSEvent.removeMonitor(monitor) }
        NotificationCenter.default.removeObserver(self)
    }
    private func trackPointer(_ event: NSEvent) {
        guard configured, draggable, let window = view?.window, event.window === window else { return }
        switch event.type {
        case .leftMouseDown:
            pointerDown = window.convertPoint(toScreen: event.locationInWindow)
            windowDown = window.frame.origin
            dragging = false
        case .leftMouseDragged, .leftMouseUp:
            guard let start = pointerDown, let origin = windowDown else { return }
            let current = NSEvent.mouseLocation
            let dx = current.x - start.x, dy = current.y - start.y
            if !dragging && hypot(dx, dy) >= 3 {
                dragging = true
                channel?.invokeMethod("dragStarted", arguments: nil)
            }
            if dragging { window.setFrameOrigin(NSPoint(x: origin.x + dx, y: origin.y + dy)) }
            if event.type == .leftMouseUp { pointerDown = nil; windowDown = nil; dragging = false }
        default: break
        }
    }
    private init(view: NSView?) { self.view = view; super.init() }
    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "floating_windows/window", binaryMessenger: registrar.messenger)
        let plugin = FloatingWindowsPlugin(view: registrar.view)
        plugin.channel = channel
        plugin.monitor = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDown, .leftMouseDragged, .leftMouseUp]) { [weak plugin] event in
            plugin?.trackPointer(event)
            return event
        }
        NotificationCenter.default.addObserver(plugin, selector: #selector(displayDidChange),
            name: NSApplication.didChangeScreenParametersNotification, object: nil)
        registrar.addMethodCallDelegate(plugin, channel: channel)
    }

    /// Keeps host-sized surfaces aligned with the primary display after display changes.
    @objc private func displayDidChange(_ notification: Notification) {
        guard configured, let window = view?.window,
              let display = NSScreen.screens.first else { return }
        if fill { window.setFrame(display.frame, display: true) }
        else if let bounds = screenBounds {
            window.setFrame(NSRect(x: bounds.minX, y: display.frame.maxY - bounds.maxY,
                width: bounds.width, height: bounds.height), display: true)
        }
    }

    /// Applies input and appearance flags and resolves the chosen geometry.
    private func apply(_ values: [String: Any], to window: NSWindow) throws {
        if let value = values["fill"] {
            guard let value = value as? Bool else { throw failure() }
            fill = value
            screenBounds = nil
        }
        var size = window.contentLayoutRect.size
        for key in ["width", "height", "x", "y"] {
            if let number = values[key] as? NSNumber {
                guard number.doubleValue.isFinite,
                      !["width", "height"].contains(key) || (16...4096).contains(number.doubleValue) else {
                    throw NSError(domain: "floating_windows", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid geometry"])
                }
            }
        }
        if let width = values["width"] as? NSNumber { size.width = CGFloat(width.doubleValue) }
        if let height = values["height"] as? NSNumber { size.height = CGFloat(height.doubleValue) }
        // AppKit y points upward; the public API uses the primary screen's top-left.
        let desktopTop = NSScreen.screens.first?.frame.maxY ?? 0
        if let bounds = values["screenBounds"] as? [String: Any] {
            let keys = ["left", "top", "right", "bottom"]
            let edges = keys.compactMap { (bounds[$0] as? NSNumber)?.doubleValue }
            guard !fill, edges.count == 4, edges.allSatisfy({ $0.isFinite && abs($0) <= 1000000 }),
                  edges[2] > edges[0], edges[3] > edges[1] else { throw failure() }
            screenBounds = NSRect(x: CGFloat(edges[0]), y: CGFloat(edges[1]),
                width: CGFloat(edges[2] - edges[0]), height: CGFloat(edges[3] - edges[1]))
        }
        var origin = window.frame.origin
        if let x = values["x"] as? NSNumber { origin.x = CGFloat(x.doubleValue) }
        let top = values["y"] as? NSNumber
        origin.y = desktopTop - CGFloat(top?.doubleValue ?? Double(desktopTop - window.frame.maxY)) - size.height
        if let value = values["alwaysOnTop"] as? Bool { topmost = value }
        if let value = values["focusable"] as? Bool { focusable = value }
        if let value = values["draggable"] as? Bool {
            draggable = value
            if !value { pointerDown = nil; windowDown = nil; dragging = false }
        }
        if let value = values["clickThrough"] as? Bool { window.ignoresMouseEvents = value }
        window.level = topmost
            ? (fill ? NSWindow.Level(rawValue: NSWindow.Level.statusBar.rawValue + 1) : .floating)
            : .normal
        if let floating = window as? CustomWindow {
            floating.floatingWindowFocusable = focusable
            floating.floatingWindowFillsDisplay = fill || screenBounds != nil
        }
        if fill {
            guard let display = NSScreen.screens.first else {
                throw NSError(domain: "floating_windows", code: 4, userInfo: [NSLocalizedDescriptionKey: "No primary display"])
            }
            window.setFrame(display.frame, display: true)
        } else if let bounds = screenBounds {
            window.setFrame(NSRect(x: bounds.minX, y: desktopTop - bounds.maxY,
                width: bounds.width, height: bounds.height), display: true)
        } else {
            window.setFrame(NSRect(origin: origin, size: size), display: true)
        }
    }
    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        if call.method == "capabilities" {
            result(["supported": true, "permissionGranted": true, "positioning": true,
                    "alwaysOnTop": true, "clickThrough": true, "focusable": true]); return
        }
        if call.method == "requestPermission" { result(true); return }
        guard let window = view?.window else {
            result(FlutterError(code: "WINDOW_CLOSED", message: "The engine has no window", details: nil)); return
        }
        do {
            switch call.method {
            case "configure":
                guard let args = call.arguments as? [String: Any], args["role"] as? String == "floating_windows.window" else {
                    throw NSError(domain: "floating_windows", code: 2, userInfo: [NSLocalizedDescriptionKey: "Explicit floating window role required"])
                }
                // Preserve the multi-window manager's original NSWindow identity:
                // its close observer owns engine removal. The multi-window
                // subclass exposes an optional keyboard-focus policy.
                window.styleMask = [.borderless]
                window.isOpaque = false; window.backgroundColor = .clear; window.hasShadow = false
                window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
                view?.wantsLayer = true; view?.layer?.backgroundColor = NSColor.clear.cgColor
                fill = args["fill"] as? Bool == true
                try apply(args, to: window)
                configured = true; result(nil)
            case "update":
                guard configured, let args = call.arguments as? [String: Any] else { throw failure() }
                try apply(args, to: window); result(nil)
            case "getBounds":
                guard configured else { throw failure() }
                result(["x": window.frame.minX,
                        "y": (NSScreen.screens.first?.frame.maxY ?? 0) - window.frame.maxY,
                        "width": window.contentLayoutRect.width, "height": window.contentLayoutRect.height])
            case "show":
                guard configured else { throw failure() }
                if focusable { window.makeKeyAndOrderFront(nil) } else { window.orderFrontRegardless() }
                result(nil)
            case "hide":
                guard configured else { throw failure() }
                window.orderOut(nil); result(nil)
            case "close":
                guard configured else { throw failure() }
                result(nil); DispatchQueue.main.async { window.close() }
            default: result(FlutterMethodNotImplemented)
            }
        } catch { result(FlutterError(code: "FLOATING_WINDOW_ERROR", message: error.localizedDescription, details: nil)) }
    }
    private func failure() -> NSError { NSError(domain: "floating_windows", code: 3,
        userInfo: [NSLocalizedDescriptionKey: "Configure the floating window first"]) }
}
