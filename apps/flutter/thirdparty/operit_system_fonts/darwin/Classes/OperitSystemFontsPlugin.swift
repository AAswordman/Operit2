import CoreText
import Foundation
#if os(iOS)
import Flutter
#else
import FlutterMacOS
#endif

public class OperitSystemFontsPlugin: NSObject, FlutterPlugin {
  private var families = Set<String>()

  /// Registers the same CoreText adapter on iOS and macOS engines.
  public static func register(with registrar: FlutterPluginRegistrar) {
    #if os(iOS)
    let messenger = registrar.messenger()
    #else
    let messenger = registrar.messenger
    #endif
    let channel = FlutterMethodChannel(name: "operit/system_fonts", binaryMessenger: messenger)
    registrar.addMethodCallDelegate(OperitSystemFontsPlugin(), channel: channel)
  }

  /// Reads the exact CoreText face and identifies its index by PostScript name.
  public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    if call.method == "listFamilies" {
      families = Set(CTFontManagerCopyAvailableFontFamilyNames() as! [String])
      result(families.sorted())
      return
    }
    guard call.method == "readFamily" else { result(FlutterMethodNotImplemented); return }
    guard let family = call.arguments as? String, families.contains(family) else {
      result(FlutterError(code: "device_font_error", message: "Unknown installed font", details: nil))
      return
    }
    let descriptor = CTFontDescriptorCreateWithAttributes([kCTFontFamilyNameAttribute: family] as CFDictionary)
    let font = CTFontCreateWithFontDescriptor(descriptor, 14, nil)
    guard let url = CTFontCopyAttribute(font, kCTFontURLAttribute) as? URL else {
      result(FlutterError(code: "device_font_error", message: "Installed font has no readable resource", details: nil))
      return
    }
    do {
      let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize
      guard let size = size, size > 0, size <= 256 * 1024 * 1024 else {
        throw NSError(domain: "operit.system_fonts", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid installed font size"])
      }
      let data = try Data(contentsOf: url)
      result(["bytes": FlutterStandardTypedData(bytes: data), "postscriptName": CTFontCopyPostScriptName(font) as String])
    } catch {
      result(FlutterError(code: "device_font_error", message: error.localizedDescription, details: nil))
    }
  }
}
