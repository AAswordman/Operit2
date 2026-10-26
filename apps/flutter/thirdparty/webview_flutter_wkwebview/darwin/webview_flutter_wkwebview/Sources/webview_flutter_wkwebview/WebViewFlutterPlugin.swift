// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#if os(iOS)
  import Flutter
#elseif os(macOS)
  import FlutterMacOS
  import AppKit
#else
  #error("Unsupported platform.")
#endif

import WebKit

public class WebViewFlutterPlugin: NSObject, FlutterPlugin {
  var proxyApiRegistrar: ProxyAPIRegistrar?
  private var themeChannel: FlutterMethodChannel?
  private var zoomChannel: FlutterMethodChannel?
  private var backgroundColorChannel: FlutterMethodChannel?

  init(binaryMessenger: FlutterBinaryMessenger) {
    proxyApiRegistrar = ProxyAPIRegistrar(
      binaryMessenger: binaryMessenger)
    super.init()
    proxyApiRegistrar?.setUp()
    themeChannel = FlutterMethodChannel(name: "operit/webview_theme", binaryMessenger: binaryMessenger)
    // Updates only browser views so the Flutter system-theme source remains unchanged.
    themeChannel?.setMethodCallHandler { call, result in
      guard call.method == "setPreferredColorScheme" else {
        result(FlutterMethodNotImplemented)
        return
      }
      guard let scheme = call.arguments as? String, scheme == "dark" || scheme == "light" else {
        result(FlutterError(code: "invalid_color_scheme", message: "Expected dark or light", details: nil))
        return
      }
      WebViewTheme.setDark(scheme == "dark")
      result(nil)
    }

    zoomChannel = FlutterMethodChannel(name: "operit/webview_zoom", binaryMessenger: binaryMessenger)
    zoomChannel?.setMethodCallHandler { [weak self] call, result in
      guard call.method == "setPageZoom" else {
        result(FlutterMethodNotImplemented)
        return
      }

      guard let arguments = call.arguments as? [String: Any],
        let identifierNumber = arguments["identifier"] as? NSNumber,
        let zoomNumber = arguments["zoomFactor"] as? NSNumber
      else {
        result(
          FlutterError(
            code: "invalid_arguments",
            message: "setPageZoom expects an identifier and a zoomFactor.",
            details: nil))
        return
      }

      let zoomFactor = zoomNumber.doubleValue
      guard zoomFactor.isFinite, zoomFactor > 0 else {
        result(
          FlutterError(
            code: "invalid_zoom_factor",
            message: "zoomFactor must be finite and greater than zero.",
            details: zoomFactor))
        return
      }

      let identifier = identifierNumber.int64Value
      DispatchQueue.main.async { [weak self] in
        guard let self else {
          result(
            FlutterError(
              code: "plugin_detached",
              message: "The WebView plugin was detached before page zoom could be applied.",
              details: nil))
          return
        }
        self.setPageZoom(
          identifier: identifier,
          zoomFactor: zoomFactor,
          attempt: 0,
          result: result)
      }
    }

    backgroundColorChannel = FlutterMethodChannel(
      name: "operit/webview_background_color", binaryMessenger: binaryMessenger)
    backgroundColorChannel?.setMethodCallHandler { [weak self] call, result in
      guard call.method == "setBackgroundColor" else {
        result(FlutterMethodNotImplemented)
        return
      }

      guard let arguments = call.arguments as? [String: Any],
        let identifierNumber = arguments["identifier"] as? NSNumber,
        let redNumber = arguments["red"] as? NSNumber,
        let greenNumber = arguments["green"] as? NSNumber,
        let blueNumber = arguments["blue"] as? NSNumber,
        let alphaNumber = arguments["alpha"] as? NSNumber
      else {
        result(
          FlutterError(
            code: "invalid_arguments",
            message: "setBackgroundColor expects an identifier and RGBA components.",
            details: nil))
        return
      }

      let components = [
        redNumber.doubleValue, greenNumber.doubleValue, blueNumber.doubleValue,
        alphaNumber.doubleValue,
      ]
      guard components.allSatisfy({ $0.isFinite && $0 >= 0 && $0 <= 1 }) else {
        result(
          FlutterError(
            code: "invalid_color", message: "RGBA components must be between 0 and 1.",
            details: components))
        return
      }

      DispatchQueue.main.async { [weak self] in
        guard let self else {
          result(
            FlutterError(
              code: "plugin_detached",
              message: "The WebView plugin was detached before its background color could be applied.",
              details: nil))
          return
        }
        self.setBackgroundColor(
          identifier: identifierNumber.int64Value,
          red: components[0],
          green: components[1],
          blue: components[2],
          alpha: components[3],
          attempt: 0,
          result: result)
      }
    }
  }

  /// Applies zoom after the Pigeon-created WebView has been registered.
  ///
  /// The Dart proxy constructor and this custom method channel use different
  /// channels, so their first messages can cross in flight during startup.
  private func setPageZoom(
    identifier: Int64,
    zoomFactor: Double,
    attempt: Int,
    result: @escaping FlutterResult
  ) {
    guard let webView: WKWebView = proxyApiRegistrar?.instanceManager.instance(
      forIdentifier: identifier)
    else {
      guard attempt < 100 else {
        result(
          FlutterError(
            code: "webview_not_found",
            message: "No WKWebView is registered for the supplied identifier.",
            details: identifier))
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.005) { [weak self] in
        guard let self else {
          result(
            FlutterError(
              code: "plugin_detached",
              message: "The WebView plugin was detached before page zoom could be applied.",
              details: nil))
          return
        }
        self.setPageZoom(
          identifier: identifier,
          zoomFactor: zoomFactor,
          attempt: attempt + 1,
          result: result)
      }
      return
    }

    #if os(iOS)
      if #available(iOS 14.0, *) {
        webView.pageZoom = zoomFactor
        result(nil)
      } else {
        result(
          FlutterError(
            code: "unsupported_os_version",
            message: "WKWebView.pageZoom requires iOS 14.0 or newer.",
            details: nil))
      }
    #elseif os(macOS)
      if #available(macOS 11.0, *) {
        webView.pageZoom = zoomFactor
      } else {
        // pageZoom was introduced in macOS 11. WKWebView's older
        // magnification property changes the same page content scale and
        // is available for the app's macOS 10.15 deployment target.
        webView.magnification = zoomFactor
      }
      result(nil)
    #else
      result(
        FlutterError(
          code: "unsupported_platform",
          message: "WKWebView page zoom is not supported on this platform.",
          details: nil))
    #endif
  }

  /// Applies a macOS WKWebView background color after the Pigeon-created view
  /// has been registered.
  private func setBackgroundColor(
    identifier: Int64,
    red: Double,
    green: Double,
    blue: Double,
    alpha: Double,
    attempt: Int,
    result: @escaping FlutterResult
  ) {
    guard let webView: WKWebView = proxyApiRegistrar?.instanceManager.instance(
      forIdentifier: identifier)
    else {
      guard attempt < 100 else {
        result(
          FlutterError(
            code: "webview_not_found",
            message: "No WKWebView is registered for the supplied identifier.",
            details: identifier))
        return
      }
      DispatchQueue.main.asyncAfter(deadline: .now() + 0.005) { [weak self] in
        guard let self else {
          result(
            FlutterError(
              code: "plugin_detached",
              message: "The WebView plugin was detached before its background color could be applied.",
              details: nil))
          return
        }
        self.setBackgroundColor(
          identifier: identifier,
          red: red,
          green: green,
          blue: blue,
          alpha: alpha,
          attempt: attempt + 1,
          result: result)
      }
      return
    }

    #if os(macOS)
      webView.operitSetBackgroundColor(
        NSColor(calibratedRed: red, green: green, blue: blue, alpha: alpha))
      result(nil)
    #else
      result(
        FlutterError(
          code: "unsupported_platform",
          message: "WKWebView background colors are only supported on macOS.",
          details: nil))
    #endif
  }

  public static func register(with registrar: FlutterPluginRegistrar) {
    #if os(iOS)
      let binaryMessenger = registrar.messenger()
    #else
      let binaryMessenger = registrar.messenger
    #endif
    let plugin = WebViewFlutterPlugin(binaryMessenger: binaryMessenger)

    let viewFactory = FlutterViewFactory(instanceManager: plugin.proxyApiRegistrar!.instanceManager)

    #if os(iOS)
      registrar.addApplicationDelegate(plugin)
      registrar.addSceneDelegate(plugin)
    #endif

    registrar.register(viewFactory, withId: "plugins.flutter.io/webview")
    registrar.publish(plugin)
  }

  public func detachFromEngine(for registrar: FlutterPluginRegistrar) {
    themeChannel?.setMethodCallHandler(nil)
    zoomChannel?.setMethodCallHandler(nil)
    backgroundColorChannel?.setMethodCallHandler(nil)
    tearDownProxyAPIRegistrar()
  }

  private func tearDownProxyAPIRegistrar() {
    proxyApiRegistrar?.ignoreCallsToDart = true
    proxyApiRegistrar?.tearDown()
    try? proxyApiRegistrar?.instanceManager.removeAllObjects()
    proxyApiRegistrar = nil
  }
}

/// Retains the application browser preference without retaining WebView instances.
enum WebViewTheme {
  private static var dark: Bool?
  private static let views = NSHashTable<WebViewImpl>.weakObjects()

  /// Applies the most recent preference to each newly constructed browser.
  static func register(_ view: WebViewImpl) {
    views.add(view)
    if let dark { apply(view, dark: dark) }
  }

  /// Propagates a preference change to all live browser views without navigation.
  static func setDark(_ value: Bool) {
    dark = value
    for view in views.allObjects { apply(view, dark: value) }
  }

  /// Changes the native appearance that WebKit exposes to page media queries.
  private static func apply(_ view: WebViewImpl, dark: Bool) {
    #if os(iOS)
      view.overrideUserInterfaceStyle = dark ? .dark : .light
    #elseif os(macOS)
      view.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
    #endif
  }
}

#if os(iOS)
  extension WebViewFlutterPlugin: FlutterApplicationLifeCycleDelegate, FlutterSceneLifeCycleDelegate
  {
    public func applicationWillTerminate(_ application: UIApplication) {
      tearDownProxyAPIRegistrar()
    }

    public func sceneDidDisconnect(_ scene: UIScene) {
      tearDownProxyAPIRegistrar()
    }
  }
#endif
