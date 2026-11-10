import font from '@ohos.font';
import fs from '@ohos.file.fs';
import { FlutterPlugin, FlutterPluginBinding } from '@ohos/flutter_ohos/src/main/ets/embedding/engine/plugins/FlutterPlugin';
import MethodChannel, { MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';

export default class OperitSystemFontsPlugin implements FlutterPlugin {
  private channel: MethodChannel | null = null;
  private families: Map<string, font.FontInfo> = new Map();

  /// Identifies this platform plugin to the Flutter engine.
  getUniqueClassName(): string { return 'OperitSystemFontsPlugin'; }

  /// Registers OpenHarmony's public font catalog on this engine.
  onAttachedToEngine(binding: FlutterPluginBinding): void {
    this.channel = new MethodChannel(binding.getBinaryMessenger(), 'operit/system_fonts');
    this.channel.setMethodCallHandler({ onMethodCall: (call: MethodCall, result: MethodResult): void => { this.handle(call, result); } });
  }

  /// Releases the channel and installed catalog with its engine.
  onDetachedFromEngine(binding: FlutterPluginBinding): void {
    this.channel?.setMethodCallHandler(null);
    this.channel = null;
    this.families.clear();
  }

  /// Enumerates native font metadata and reads only catalog-owned paths.
  private handle(call: MethodCall, result: MethodResult): void {
    try {
      if (call.method === 'listFamilies') {
        this.families.clear();
        for (const name of font.getSystemFontList()) {
          const info = font.getFontByName(name);
          if (info === null) continue;
          const previous = this.families.get(info.family);
          if (previous === undefined || Math.abs(info.weight - 400) < Math.abs(previous.weight - 400)) {
            this.families.set(info.family, info);
          }
        }
        result.success(Array.from(this.families.keys()).sort());
      } else if (call.method === 'readFamily') {
        const info = this.families.get(call.args as string);
        if (info === undefined) throw new Error('Unknown installed font');
        const file = fs.openSync(info.path, fs.OpenMode.READ_ONLY);
        try {
          const size = fs.statSync(file.fd).size;
          if (size <= 0 || size > 256 * 1024 * 1024) throw new Error('Invalid installed font size');
          const bytes = new ArrayBuffer(size);
          if (fs.readSync(file.fd, bytes) !== size) throw new Error('Incomplete installed font read');
          const response = new Map<string, Object>();
          response.set('bytes', new Uint8Array(bytes));
          response.set('postscriptName', info.postScriptName);
          result.success(response);
        } finally { fs.closeSync(file); }
      } else { result.notImplemented(); }
    } catch (error) { result.error('device_font_error', String(error), null); }
  }
}
