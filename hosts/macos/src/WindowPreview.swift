import AppKit
import ApplicationServices
import CoreImage
import CoreMedia
import CoreVideo
import ScreenCaptureKit

/// Supports local dragging and menus without forwarding mouse events to the source application.
final class PreviewImageView: NSImageView {
    /// Accepts local preview interactions while leaving the source application's focus unchanged.
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

    /// Moves only the small panel using AppKit's native drag tracking.
    override func mouseDown(with event: NSEvent) { window?.performDrag(with: event) }

    /// Opens the preview actions without activating the captured application.
    override func rightMouseDown(with event: NSEvent) {
        guard let menu = menu else { return }
        NSMenu.popUpContextMenu(menu, with: event, for: self)
    }
}

/// Renders capture frames on the stream queue and delivers images to the AppKit main actor.
@available(macOS 12.3, *)
private final class PreviewStreamOutput: NSObject, SCStreamOutput {
    private let context = CIContext()
    private let sourceHeight: CGFloat
    private let titlebarInset: CGFloat
    private let present: @MainActor @Sendable (CGImage) -> Void
    private let fail: @MainActor @Sendable (String) -> Void

    init(sourceHeight: CGFloat, titlebarInset: CGFloat,
         present: @escaping @MainActor @Sendable (CGImage) -> Void,
         fail: @escaping @MainActor @Sendable (String) -> Void) {
        self.sourceHeight = sourceHeight; self.titlebarInset = titlebarInset
        self.present = present; self.fail = fail
    }

    /// Crops single-window streams using their actual frame metadata rather than sourceRect.
    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .screen, CMSampleBufferIsValid(sampleBuffer),
              let attachments = CMSampleBufferGetSampleAttachmentsArray(sampleBuffer, createIfNecessary: false) as? [[SCStreamFrameInfo: Any]],
              let raw = attachments.first?[.status] as? Int, SCFrameStatus(rawValue: raw) == .complete else { return }
        guard let dictionary = attachments.first?[.contentRect] as? [String: Any],
              let content = CGRect(dictionaryRepresentation: dictionary as CFDictionary), content.width > 0, content.height > 0,
              let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else {
            DispatchQueue.main.async { [fail] in fail("ScreenCaptureKit returned a frame without valid content geometry") }
            return
        }
        let inset = titlebarInset * content.height / sourceHeight
        let crop = CGRect(x: content.minX, y: CGFloat(CVPixelBufferGetHeight(buffer)) - content.maxY,
            width: content.width, height: content.height - inset)
        guard let image = context.createCGImage(CIImage(cvPixelBuffer: buffer), from: crop) else {
            DispatchQueue.main.async { [fail] in fail("ScreenCaptureKit could not render the selected window content") }
            return
        }
        DispatchQueue.main.async { [present] in present(image) }
    }
}

/// Owns one independent window stream while leaving its source visible and interactive.
@available(macOS 12.3, *)
@MainActor
final class WindowPreview: NSObject, SCStreamDelegate, NSWindowDelegate {
    private let imageView = PreviewImageView()
    private let panel = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 320, height: 240),
        styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    private let frames = DispatchQueue(label: "app.operit.window-preview.frames")
    private var stream: SCStream?
    private var output: PreviewStreamOutput?
    private var sourceApplication: NSRunningApplication?
    private var sourceWindow: AXUIElement?
    private var contentSize = CGSize.zero
    private var expandedFrame = CGRect.zero
    private var ready = false
    private var closing = false
    private var docked = false
    private var lifetime: Timer?
    private var minimizeItem: NSMenuItem?
    private var failed = false

    /// Reads a required AX attribute without substituting another window or process.
    private func attribute(_ element: AXUIElement, _ key: CFString) throws -> CFTypeRef {
        var result: CFTypeRef?
        let status = AXUIElementCopyAttributeValue(element, key, &result)
        guard status == .success, let value = result else {
            throw NSError(domain: "OperitWindowPreview", code: Int(status.rawValue),
                userInfo: [NSLocalizedDescriptionKey: "The selected window's accessibility attributes are unavailable"])
        }
        return value
    }

    /// Omits the source's native title controls from the viewing surface using their actual AX bounds.
    private func titleInset(_ window: AXUIElement, frame: CGRect) -> CGFloat {
        var bottom = frame.minY
        for key in [kAXCloseButtonAttribute, kAXMinimizeButtonAttribute, kAXZoomButtonAttribute] {
            var button: CFTypeRef?
            guard AXUIElementCopyAttributeValue(window, key as CFString, &button) == .success, let button = button else { continue }
            let element = button as! AXUIElement
            guard let position = try? attribute(element, kAXPositionAttribute as CFString),
                  let size = try? attribute(element, kAXSizeAttribute as CFString) else { continue }
            var origin = CGPoint.zero, dimensions = CGSize.zero
            if AXValueGetValue(position as! AXValue, .cgPoint, &origin), AXValueGetValue(size as! AXValue, .cgSize, &dimensions) {
                bottom = max(bottom, origin.y + dimensions.height + 7)
            }
        }
        return min(max(0, bottom - frame.minY), max(0, frame.height - 1))
    }

    /// Closes the preview when the host closes its pipe, including during capture startup.
    func watchOwner() {
        DispatchQueue.global(qos: .utility).async { [weak self] in
            _ = FileHandle.standardInput.readDataToEndOfFile()
            DispatchQueue.main.async { self?.shutdown() }
        }
    }

    /// Resolves the same opaque AX identity used by the host and starts its exact native window stream.
    func start(_ identity: String) async throws {
        guard CGPreflightScreenCaptureAccess(), AXIsProcessTrusted() else {
            throw NSError(domain: "OperitWindowPreview", code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Grant Accessibility and Screen Recording permission to Operit before opening a window preview"])
        }
        let parts = identity.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 2, let pid = Int32(parts[0]), let hash = UInt(parts[1]),
              let application = NSRunningApplication(processIdentifier: pid) else {
            throw NSError(domain: "OperitWindowPreview", code: 2, userInfo: [NSLocalizedDescriptionKey: "Invalid source window identity"])
        }
        let appElement = AXUIElementCreateApplication(pid)
        let windows = try attribute(appElement, kAXWindowsAttribute as CFString) as! [AXUIElement]
        let matches = windows.filter { CFHash($0) == hash }
        guard matches.count == 1 else {
            throw NSError(domain: "OperitWindowPreview", code: 3, userInfo: [NSLocalizedDescriptionKey: "The selected AX window no longer exists"])
        }
        let position = try attribute(matches[0], kAXPositionAttribute as CFString) as! AXValue
        let size = try attribute(matches[0], kAXSizeAttribute as CFString) as! AXValue
        var origin = CGPoint.zero, dimensions = CGSize.zero
        guard AXValueGetValue(position, .cgPoint, &origin), AXValueGetValue(size, .cgSize, &dimensions),
              dimensions.width > 0, dimensions.height > 0 else {
            throw NSError(domain: "OperitWindowPreview", code: 4, userInfo: [NSLocalizedDescriptionKey: "The selected AX window has no drawable frame"])
        }
        let frame = CGRect(origin: origin, size: dimensions)
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
        let native = content.windows.filter { $0.owningApplication?.processID == pid && $0.windowLayer == 0 &&
            abs($0.frame.minX - frame.minX) < 1 && abs($0.frame.minY - frame.minY) < 1 &&
            abs($0.frame.width - frame.width) < 1 && abs($0.frame.height - frame.height) < 1 }
        guard native.count == 1 else {
            throw NSError(domain: "OperitWindowPreview", code: 5, userInfo: [NSLocalizedDescriptionKey: "The exact selected capture window is unavailable"])
        }
        sourceApplication = application; sourceWindow = matches[0]
        let inset = titleInset(matches[0], frame: frame)
        contentSize = CGSize(width: frame.width, height: frame.height - inset)
        panel.title = "Operit Window Preview"
        panel.isFloatingPanel = true; panel.level = .floating
        panel.hidesOnDeactivate = false; panel.isReleasedWhenClosed = false
        panel.isOpaque = false; panel.backgroundColor = .clear; panel.hasShadow = true
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.delegate = self
        imageView.imageScaling = .scaleProportionallyUpOrDown
        imageView.autoresizingMask = [.width, .height]
        imageView.wantsLayer = true
        imageView.layer?.cornerRadius = 22
        imageView.layer?.masksToBounds = true
        imageView.layer?.backgroundColor = NSColor(calibratedRed: 0.145, green: 0.165, blue: 0.2, alpha: 1).cgColor
        panel.contentView = imageView
        installActions(); try setPreviewWidth(320)
        if let screen = NSScreen.main {
            panel.setFrameTopLeftPoint(NSPoint(x: screen.visibleFrame.maxX - panel.frame.width - 16, y: screen.visibleFrame.maxY - 16))
        }
        let configuration = SCStreamConfiguration()
        configuration.width = max(1, Int(frame.width.rounded()))
        configuration.height = max(1, Int(frame.height.rounded()))
        if #available(macOS 14.0, *) {
            configuration.ignoreShadowsSingleWindow = true
            configuration.ignoreGlobalClipSingleWindow = true
        }
        configuration.pixelFormat = kCVPixelFormatType_32BGRA
        configuration.minimumFrameInterval = CMTime(value: 1, timescale: 15)
        configuration.queueDepth = 3; configuration.showsCursor = false
        let capture = SCStream(filter: SCContentFilter(desktopIndependentWindow: native[0]), configuration: configuration, delegate: self)
        let output = PreviewStreamOutput(sourceHeight: frame.height, titlebarInset: inset,
            present: { [weak self] image in self?.present(image) },
            fail: { [weak self] message in self?.fail(message) })
        try capture.addStreamOutput(output, type: .screen, sampleHandlerQueue: frames)
        self.output = output
        stream = capture
        try await capture.startCapture()
        lifetime = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self = self, let window = self.sourceWindow else { return }
                var role: CFTypeRef?
                if self.sourceApplication?.isTerminated == true || AXUIElementCopyAttributeValue(window, kAXRoleAttribute as CFString, &role) != .success {
                    self.shutdown()
                }
            }
        }
    }

    /// Builds compact context actions without adding title bars, buttons or resize handles.
    private func installActions() {
        let menu = NSMenu()
        for (title, action) in [("Open original window", #selector(openOriginal)), ("Fullscreen", #selector(fullscreen)),
                                ("Minimize preview", #selector(toggleMinimize))] {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: ""); item.target = self; menu.addItem(item)
            if action == #selector(toggleMinimize) { minimizeItem = item }
        }
        let sizes = NSMenu(); let sizeItem = NSMenuItem(title: "Preview size", action: nil, keyEquivalent: "")
        for width in [240, 320, 480] {
            let item = NSMenuItem(title: "\(width) px", action: #selector(resizePreview(_:)), keyEquivalent: "")
            item.target = self; item.tag = width; sizes.addItem(item)
        }
        sizeItem.submenu = sizes; menu.addItem(sizeItem); menu.addItem(.separator())
        let close = NSMenuItem(title: "Close preview", action: #selector(closePreview), keyEquivalent: "")
        close.target = self; menu.addItem(close); imageView.menu = menu
    }

    /// Changes only the viewing size while preserving the source application's layout.
    private func setPreviewWidth(_ width: CGFloat) throws {
        guard let screen = NSScreen.main else {
            throw NSError(domain: "OperitWindowPreview", code: 8, userInfo: [NSLocalizedDescriptionKey: "macOS has no available preview display"])
        }
        let limit = screen.visibleFrame.height * 0.7
        let scale = min(width / contentSize.width, limit / contentSize.height)
        panel.setContentSize(NSSize(width: max(64, contentSize.width * scale), height: max(64, contentSize.height * scale)))
        panel.invalidateShadow()
    }

    /// Applies a size chosen from the context menu.
    @objc private func resizePreview(_ sender: NSMenuItem) {
        docked = false; minimizeItem?.title = "Minimize preview"
        do { try setPreviewWidth(CGFloat(sender.tag)) } catch { fail(error.localizedDescription) }
    }

    /// Docks into the top-right corner and restores from the same context menu.
    @objc private func toggleMinimize() {
        if docked { panel.setFrame(expandedFrame, display: true) }
        else {
            expandedFrame = panel.frame; panel.setContentSize(NSSize(width: 64, height: 64))
            if let screen = NSScreen.main {
                panel.setFrameTopLeftPoint(NSPoint(x: screen.visibleFrame.maxX - 80, y: screen.visibleFrame.maxY - 16))
            }
        }
        docked.toggle(); minimizeItem?.title = docked ? "Restore preview" : "Minimize preview"; panel.invalidateShadow()
    }

    /// Raises the original window only when the user explicitly requests it.
    @objc private func openOriginal() {
        if let window = sourceWindow { AXUIElementPerformAction(window, kAXRaiseAction as CFString) }
        sourceApplication?.activate(options: [.activateIgnoringOtherApps]); shutdown()
    }

    /// Hands the original native window back to the user before requesting native fullscreen.
    @objc private func fullscreen() {
        if let window = sourceWindow {
            let result = AXUIElementSetAttributeValue(window, "AXFullScreen" as CFString, kCFBooleanTrue)
            if result != .success { fail("The selected application does not support native fullscreen"); return }
        }
        sourceApplication?.activate(options: [.activateIgnoringOtherApps]); shutdown()
    }

    /// Closes only the preview and leaves the original application running.
    @objc private func closePreview() { shutdown() }

    /// Presents complete frames directly, without encoding a screenshot for every refresh.
    private func present(_ image: CGImage) {
        guard !closing else { return }
        imageView.image = NSImage(cgImage: image, size: NSSize(width: CGFloat(image.width), height: CGFloat(image.height)))
        if !ready {
            panel.orderFrontRegardless(); panel.invalidateShadow()
            ready = true
            print("{\"ready\":true}"); fflush(stdout)
        }
    }

    /// Reports the actual capture failure and closes the viewing panel.
    nonisolated func stream(_ stream: SCStream, didStopWithError error: Error) {
        DispatchQueue.main.async { self.fail(error.localizedDescription) }
    }

    /// Releases preview resources when AppKit closes the small viewing panel.
    func windowWillClose(_ notification: Notification) { shutdown() }

    /// Publishes startup failures as structured data and later failures on stderr.
    func fail(_ message: String) {
        failed = true
        if !ready {
            let data = try! JSONSerialization.data(withJSONObject: ["ready": false, "error": message])
            print(String(data: data, encoding: .utf8)!); fflush(stdout)
        } else { FileHandle.standardError.write(Data((message + "\n").utf8)) }
        shutdown()
    }

    /// Releases only the viewing panel and stream when its host or preview closes.
    func shutdown() {
        guard !closing else { return }
        closing = true; lifetime?.invalidate(); panel.orderOut(nil)
        if failed { exit(1) }
        NSApplication.shared.terminate(nil)
    }
}

/// Runs the native helper under the host-owned pipe rather than creating an independent user session.
@main @MainActor struct WindowPreviewMain {
    private static var controller: AnyObject?

    /// Starts only after validating the explicit source request and supported system capture API.
    static func main() {
        guard let line = readLine(), let data = line.data(using: .utf8),
              let request = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let parameters = request["parameters"] as? [String: Any], let identity = parameters["window_id"] as? String else {
            print("{\"ready\":false,\"error\":\"Invalid native preview request\"}"); return
        }
        guard #available(macOS 12.3, *) else {
            print("{\"ready\":false,\"error\":\"Window previews require macOS 12.3 or later\"}"); return
        }
        let application = NSApplication.shared
        application.setActivationPolicy(.accessory)
        let preview = WindowPreview(); controller = preview; preview.watchOwner()
        Task { @MainActor in
            do { try await preview.start(identity) }
            catch { preview.fail(error.localizedDescription) }
        }
        application.run()
    }
}
