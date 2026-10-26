import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MethodChannel, { MethodCallHandler, MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import { InstanceManager } from './InstanceManager';
import { WebViewPlatformView } from './WebViewPlatformView';

/** Per-engine bridge for page zoom, separate from Flutter application paint zoom. */
export class WebViewZoom implements MethodCallHandler {
  private channel: MethodChannel;
  private instanceManager: InstanceManager;

  /** Registers this engine's method channel and native instance owner. */
  constructor(messenger: BinaryMessenger, instanceManager: InstanceManager) {
    this.instanceManager = instanceManager;
    this.channel = new MethodChannel(messenger, 'operit/webview_zoom');
    this.channel.setMethodCallHandler(this);
  }

  /** Dispatches the supported native WebView operation. */
  onMethodCall(call: ESObject, result: MethodResult): void {
    if (call.method !== 'setPageZoom') {
      result.notImplemented();
      return;
    }
    const identifier = call.argument('viewIdentifier') as number;
    const factor = call.argument('zoomFactor') as number;
    const view = this.instanceManager.getInstance<WebViewPlatformView>(identifier);
    if (view === undefined || view === null || !Number.isFinite(factor) || factor <= 0) {
      result.error('invalid_page_zoom', 'Unknown WebView or invalid zoom factor', null);
      return;
    }
    view.setPageZoomFactor(factor).then((): void => result.success(null))
      .catch((error: Error): void => result.error('page_zoom_failed', error.message, null));
  }

  dispose(): void {
    this.channel.setMethodCallHandler(null);
  }
}
