import Cocoa
import FlutterMacOS

import desktop_multi_window

public final class FloatingWindowsPlugin: NSObject, FlutterPlugin {
    private weak var view: NSView?
    private var configured = false
    private var focusable = false
    private var topmost = true
    private init(view: NSView?) { self.view = view; super.init() }
    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "floating_windows/window", binaryMessenger: registrar.messenger)
        registrar.addMethodCallDelegate(FloatingWindowsPlugin(view: registrar.view), channel: channel)
    }
    private func apply(_ values: [String: Any], to window: NSWindow) throws {
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
        var origin = window.frame.origin
        if let x = values["x"] as? NSNumber { origin.x = CGFloat(x.doubleValue) }
        let top = values["y"] as? NSNumber
        origin.y = desktopTop - CGFloat(top?.doubleValue ?? Double(desktopTop - window.frame.maxY)) - size.height
        if let value = values["alwaysOnTop"] as? Bool { topmost = value }
        if let value = values["focusable"] as? Bool { focusable = value }
        if let value = values["clickThrough"] as? Bool { window.ignoresMouseEvents = value }
        window.level = topmost ? .floating : .normal
        if let floating = window as? CustomWindow { floating.floatingWindowFocusable = focusable }
        window.setFrame(NSRect(origin: origin, size: size), display: true)
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
                try apply(args, to: window)
                configured = true; result(nil)
            case "update":
                guard configured, let args = call.arguments as? [String: Any] else { throw failure() }
                try apply(args, to: window); result(nil)
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
