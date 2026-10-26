// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#if os(macOS)
  import AppKit
  import WebKit
  import XCTest

  @testable import webview_flutter_wkwebview

  class WebViewBackgroundTests: XCTestCase {
    @MainActor func testTransparentBackgroundDisablesWhitePageCanvas() {
      let view = WKWebView()
      view.operitSetBackgroundColor(.clear)

      XCTAssertEqual(view.value(forKey: "drawsBackground") as? Bool, false)
      XCTAssertEqual(view.layer?.backgroundColor?.alpha, 0)
      XCTAssertEqual(view.underPageBackgroundColor.alphaComponent, 0)
    }

    @MainActor func testOpaqueBackgroundCanBeChangedBackToTransparent() {
      let view = WKWebView()
      view.operitSetBackgroundColor(.red)

      XCTAssertEqual(view.value(forKey: "drawsBackground") as? Bool, false)
      XCTAssertEqual(view.layer?.backgroundColor, NSColor.red.cgColor)
      XCTAssertEqual(view.underPageBackgroundColor, NSColor.red)

      view.operitSetBackgroundColor(.clear)
      XCTAssertEqual(view.layer?.backgroundColor?.alpha, 0)
      XCTAssertEqual(view.underPageBackgroundColor.alphaComponent, 0)
    }

    @MainActor func testLoadedTransparentPageDoesNotPaintWhite() throws {
      let color = try snapshotPixel(background: "transparent")
      XCTAssertEqual(color.alphaComponent, 0, accuracy: 0.01)
    }

    @MainActor func testExplicitWhitePageBackgroundIsPreserved() throws {
      let color = try snapshotPixel(background: "white")
      XCTAssertEqual(color.alphaComponent, 1, accuracy: 0.01)
      XCTAssertEqual(color.redComponent, 1, accuracy: 0.01)
      XCTAssertEqual(color.greenComponent, 1, accuracy: 0.01)
      XCTAssertEqual(color.blueComponent, 1, accuracy: 0.01)
    }

    /// Checks real WebKit rendering, not just the native background properties.
    @MainActor private func snapshotPixel(background: String) throws -> NSColor {
      _ = NSApplication.shared
      let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 80, height: 80))
      view.operitSetBackgroundColor(.clear)
      let window = NSWindow(
        contentRect: view.frame, styleMask: .borderless, backing: .buffered, defer: false)
      window.contentView = view
      window.orderBack(nil)
      defer { window.orderOut(nil) }
      let finished = expectation(description: "Page rendered")
      var pixel: NSColor?
      let navigation = BackgroundNavigationDelegate {
        view.takeSnapshot(with: nil) { image, error in
          XCTAssertNil(error)
          if let cgImage = image?.cgImage(forProposedRect: nil, context: nil, hints: nil) {
            let bitmap = NSBitmapImageRep(cgImage: cgImage)
            pixel = bitmap.colorAt(x: bitmap.pixelsWide / 2, y: bitmap.pixelsHigh / 2)?
              .usingColorSpace(.deviceRGB)
          }
          finished.fulfill()
        }
      }
      view.navigationDelegate = navigation
      view.loadHTMLString(
        "<style>html,body{margin:0;width:100%;height:100%;background:\(background)}</style>",
        baseURL: nil)
      waitForExpectations(timeout: 10)
      return try XCTUnwrap(pixel)
    }

    @MainActor func testNativeViewIsTransparentBeforeAnyDartBackgroundCall() throws {
      let registrar = TestProxyApiRegistrar()
      let api = registrar.apiDelegate.pigeonApiNSViewWKWebView(registrar)
      let view = try api.pigeonDelegate.pigeonDefaultConstructor(
        pigeonApi: api, initialConfiguration: WKWebViewConfiguration())

      XCTAssertEqual(view.value(forKey: "drawsBackground") as? Bool, false)
      XCTAssertEqual(view.layer?.backgroundColor?.alpha, 0)
      XCTAssertEqual(view.underPageBackgroundColor.alphaComponent, 0)
    }
  }

  private class BackgroundNavigationDelegate: NSObject, WKNavigationDelegate {
    let onFinished: () -> Void

    init(onFinished: @escaping () -> Void) {
      self.onFinished = onFinished
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
      onFinished()
    }
  }
#endif
