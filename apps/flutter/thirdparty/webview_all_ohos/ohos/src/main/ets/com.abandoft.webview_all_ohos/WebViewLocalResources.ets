import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MethodChannel, { MethodCallHandler, MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import { InstanceManager } from './InstanceManager';
import { WebViewPlatformView } from './WebViewPlatformView';

/** Per-engine bridge; ArkWeb receives a deferred response rather than blocking
 * the UI thread while Dart invokes the existing plugin/Core VFS reader. */
export class WebViewLocalResources implements MethodCallHandler {
  private channel: MethodChannel;
  private instanceManager: InstanceManager;
  private views: Map<number, WebViewPlatformView> = new Map();
  private pending: Map<number, () => void> = new Map();
  private nextRequest: number = 0;

  /** Registers this engine's method channel and native instance owner. */
  constructor(messenger: BinaryMessenger, instanceManager: InstanceManager) {
    this.instanceManager = instanceManager;
    this.channel = new MethodChannel(messenger, 'operit/webview_resources');
    this.channel.setMethodCallHandler(this);
  }

  /** Dispatches the supported native WebView operation. */
  onMethodCall(call: MethodCall, result: MethodResult): void {
    if (call.method !== 'configure') { result.notImplemented(); return; }
    try {
      const id = call.argument('identifier') as number;
      const enabled = call.argument('enabled') as boolean;
      const view = this.instanceManager.getInstance<WebViewPlatformView>(id);
      if (enabled) {
        this.views.set(id, view);
        view.localResourceHandler = (request: WebResourceRequest): WebResourceResponse => this.request(id, request);
        view.onLocalResourceDispose = (): void => { this.views.delete(id); };
      } else {
        view.localResourceHandler = undefined;
        view.onLocalResourceDispose = undefined;
        this.views.delete(id);
      }
      result.success(null);
    } catch (error) {
      result.error('resource_configuration_failed', String(error), null);
    }
  }

  /** Completes the deferred ArkWeb response when Dart returns resource bytes. */
  private request(id: number, request: WebResourceRequest): WebResourceResponse {
    const response = new WebResourceResponse();
    response.setResponseIsReady(false);
    const token = this.nextRequest++;
    let completed = false;
    let timer: number = -1;
    const finish = (value: Object | null): void => {
      if (completed) return;
      completed = true;
      clearTimeout(timer);
      this.pending.delete(token);
      try {
        if (!(value instanceof Map) || !this.views.has(id)) throw new Error('Resource unavailable');
        const data = value as Map<string, Object>;
        const body = data.get('body') as Uint8Array;
        // StandardMessageCodec's Uint8List is Uint8Array; do not decode binary
        // files as text or accidentally include the surrounding buffer bytes.
        response.setResponseData(body.buffer.slice(body.byteOffset, body.byteOffset + body.byteLength));
        response.setResponseCode(data.get('statusCode') as number);
        response.setReasonMessage(data.get('reasonPhrase') as string);
        response.setResponseMimeType(data.get('mimeType') as string);
        response.setResponseEncoding(data.get('encoding') as string);
        const headers: Array<Header> = [];
        const source = data.get('headers') as Map<string, string>;
        source.forEach((value: string, key: string): void => { headers.push({ headerKey: key, headerValue: value }); });
        response.setResponseHeader(headers);
      } catch (_) {
        response.setResponseCode(503);
        response.setReasonMessage('Resource unavailable');
        response.setResponseMimeType('text/plain');
        response.setResponseData('');
      }
      response.setResponseIsReady(true);
    };
    this.pending.set(token, (): void => finish(null));
    timer = setTimeout((): void => finish(null), 35000);
    const headers = new Map<string, string>();
    request.getRequestHeader().forEach((header: Header): void => { headers.set(header.headerKey, header.headerValue); });
    const args = new Map<string, Object>()
      .set('identifier', id).set('url', request.getRequestUrl())
      .set('method', request.getRequestMethod()).set('headers', headers)
      .set('isMainFrame', request.isMainFrame());
    this.channel.invokeMethod('request', args, {
      success: (value: Object | null): void => finish(value),
      error: (_code: string, _message: string, _details: Object): void => finish(null),
      notImplemented: (): void => finish(null)
    });
    return response;
  }

  /** Detaches the channel and completes pending resource requests. */
  dispose(): void {
    this.channel.setMethodCallHandler(null);
    this.views.forEach((view: WebViewPlatformView): void => {
      view.localResourceHandler = undefined;
      view.onLocalResourceDispose = undefined;
    });
    this.views.clear();
    this.pending.forEach((cancel: () => void): void => cancel());
    this.pending.clear();
  }
}
