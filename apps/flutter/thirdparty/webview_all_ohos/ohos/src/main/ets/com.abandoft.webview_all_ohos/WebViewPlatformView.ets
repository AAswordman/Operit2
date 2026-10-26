import common from '@ohos.app.ability.common';
import window from '@ohos.window';
import web_webview from '@ohos.web.webview';
import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import PlatformView, { Params } from '@ohos/flutter_ohos/src/main/ets/plugin/platform/PlatformView';
import Log from '@ohos/flutter_ohos/src/main/ets/util/Log';
import { deviceInfo } from '@kit.BasicServicesKit';
import { DownloadListener } from './DownloadListenerFlutterApiImpl';
import { InstanceManager } from './InstanceManager';
import { JavaScriptChannel } from './JavaScriptChannel';
import { HttpAuthHandler, SslAuthHandler } from './OhosWebViewCallbackApiChannel';
import { WebBuilder } from './OhosWebView';
import { WebChromeClient } from './WebChromeClientHostApiImpl';
import { WebSettings } from './WebSettingsHostApiImpl';
import { WebViewClient } from './WebViewClientHostApiImpl';
import { WebViewFlutterApiImpl } from './WebViewFlutterApiImpl';
import { WebViewSettings } from './WebViewSettings';
import { buildPageZoomScript } from './PageZoomScript';

const TAG: string = 'WebViewPlatformView';

export declare class PlatformViewVisibleAreaEventOptions {
  enable: boolean;
  ratios: Array<number>;
  expectedUpdateInterval: number;
  onInactiveThreshold: number;
  onActiveThreshold: number;
}

class OhosHttpAuthHandler implements HttpAuthHandler {
  private readonly nativeHandler: ESObject;

  constructor(handler: ESObject) {
    this.nativeHandler = handler;
  }

  cancel(): void {
    this.nativeHandler.cancel();
  }

  proceed(username: string, password: string): void {
    this.nativeHandler.confirm(username, password);
  }

  useHttpAuthUsernamePassword(): boolean {
    return this.nativeHandler.isHttpAuthInfoSaved?.() ?? false;
  }
}

class OhosSslAuthHandler implements SslAuthHandler {
  private readonly nativeHandler: ESObject;
  private completed: boolean = false;

  constructor(handler: ESObject) {
    this.nativeHandler = handler;
  }

  cancel(): void {
    if (this.completed) {
      return;
    }
    this.completed = true;
    if (deviceInfo.sdkApiVersion >= 20) {
      this.nativeHandler.handleCancel(true);
    } else {
      this.nativeHandler.handleCancel();
    }
  }

  proceed(): void {
    if (this.completed) {
      return;
    }
    this.completed = true;
    this.nativeHandler.handleConfirm();
  }
}

/** Completes registration when the ArkUI Web component binds its script host. */
class DocumentStartScriptHostWaiter {
  readonly resolve: (host: (scripts: Array<ScriptItem>) => Promise<void>) => void;
  readonly reject: (error: Error) => void;

  /** Retains promise callbacks until the platform component attaches. */
  constructor(
    resolve: (host: (scripts: Array<ScriptItem>) => Promise<void>) => void,
    reject: (error: Error) => void,
  ) {
    this.resolve = resolve;
    this.reject = reject;
  }
}

export class WebViewPlatformView extends PlatformView {
  private readonly cookieManager: web_webview.WebCookieManager = new web_webview.WebCookieManager();
  private readonly controller: web_webview.WebviewController = new web_webview.WebviewController();
  private readonly webViewFlutterApi: WebViewFlutterApiImpl;
  private readonly javaScriptChannels: Map<string, JavaScriptChannel> = new Map();

  private documentStartScripts: Map<string, ScriptItem> = new Map();
  private nextDocumentStartScriptIdentifier: number = 0;
  private pageZoomFactor: number = 1;
  private documentStartScriptHost?: (scripts: Array<ScriptItem>) => Promise<void>;
  private documentStartScriptHostWaiters: Array<DocumentStartScriptHostWaiter> = [];
  private documentStartScriptHostDisposed: boolean = false;

  private url: string = '';
  private downloadListener: DownloadListener | null = null;
  private webChromeClient: WebChromeClient | null = null;
  private webViewClient: WebViewClient | null = null;
  private webSettings: WebSettings | null = null;
  private dialogController: CustomDialogController | null = null;
  private scrollX: number = 0;
  private scrollY: number = 0;
  private controllerAttached: boolean = false;
  private onDisposeCallback?: () => void;
  private rotation: number = 0;
  private visibleAreaEventEnable: boolean = true;
  private visibleAreaEventRatios: Array<number> = [0, 0.1, 0.2, 0.5, 0.8, 1];
  private visibleAreaEventExpectedUpdateInterval: number = 1;
  private visibleAreaEventInactiveThreshold: number = 0;
  private visibleAreaEventActiveThreshold: number = 0.1;

  userAgentString: string = '';
  isWebviewBinding: boolean = false;

  constructor(binaryMessenger: BinaryMessenger, instanceManager: InstanceManager) {
    super();
    this.webViewFlutterApi = new WebViewFlutterApiImpl(binaryMessenger, instanceManager);
  }

  localResourceHandler?: (request: WebResourceRequest) => WebResourceResponse;
  onLocalResourceDispose?: () => void;

  onInterceptRequest = (event: ESObject): WebResourceResponse | null => {
    const request = event.request as WebResourceRequest;
    // Only virtual HTTPS hosts are routed to Dart. The loader validates the
    // exact plugin-owned host; unrelated network traffic stays native.
    const match = /^https:\/\/([^/?#:]+)(?::443)?(?:[/?#]|$)/i.exec(request.getRequestUrl());
    if (match == null || !match[1].toLowerCase().endsWith('.operit.invalid')) return null;
    if (this.localResourceHandler != null) return this.localResourceHandler(request);
    const response = new WebResourceResponse();
    response.setResponseCode(403);
    response.setReasonMessage('Forbidden');
    response.setResponseMimeType('text/plain');
    response.setResponseData('');
    return response;
  };

  onDownloadStart = (event: ESObject): void => {
    if (this.downloadListener) {
      this.downloadListener.onDownloadStart(
        event.url,
        event.userAgent,
        event.contentDisposition,
        event.mimetype,
        event.contentLength
      );
    }
  }

  onPageBegin = (event: ESObject): void => {
    if (this.webViewClient) {
      this.webViewClient.onPageStarted(this, event.url);
    }
  }

  onPageEnd = (event: ESObject): void => {
    if (this.webViewClient) {
      this.webViewClient.onPageFinished(this, event.url);
    }
  }

  onErrorReceive = (event: ESObject): void => {
    if (this.webViewClient) {
      this.webViewClient.onReceivedError(this, event.request, event.error);
    }
  }

  onHttpErrorReceive = (event: ESObject): void => {
    if (this.webViewClient) {
      this.webViewClient.onReceivedHttpError(this, event.request, event.response);
    }
  }

  onOverrideUrlLoading = (request: WebResourceRequest): boolean => {
    const requestUrl = request.getRequestUrl();
    if (requestUrl === 'loaded:///') {
      return true;
    }
    if (this.webViewClient) {
      return this.webViewClient.shouldOverrideUrlLoading(this, request);
    }
    return false;
  }

  onRefreshAccessedHistory = (event: ESObject): void => {
    if (this.webViewClient) {
      this.webViewClient.doUpdateVisitedHistory(this, event.url, event.isRefreshed);
    }
  }

  onHttpAuthRequest = (event: ESObject): boolean => {
    if (this.webViewClient) {
      this.webViewClient.onReceivedHttpAuthRequest(
        this,
        new OhosHttpAuthHandler(event.handler),
        event.host,
        event.realm
      );
      return true;
    }
    return false;
  }

  onSslErrorEventReceive = (event: ESObject): void => {
    const handler = new OhosSslAuthHandler(event.handler as ESObject);
    if (this.webViewClient) {
      const error: ESObject = event.error as ESObject;
      const url: string = (event.url as string) ?? this.url;
      const errorCode: number = (event.errorCode as number) ?? (error?.getErrorCode?.() as number) ?? 0;
      const description: string =
        (event.description as string) ??
        (error?.getErrorInfo?.() as string) ??
        'SSL certificate error';
      this.webViewClient.onReceivedSslAuthError(
        this,
        handler,
        url,
        errorCode,
        description
      );
      return;
    }
    handler.cancel();
  }

  onProgressChange = (event: ESObject): void => {
    if (this.webChromeClient) {
      this.webChromeClient.onProgressChanged(this, event.newProgress);
    }
  }

  onGeolocationShow = (event: ESObject): void => {
    if (this.webChromeClient) {
      this.webChromeClient.onGeolocationPermissionsShowPrompt(event.origin, event.geolocation);
    }
  }

  onGeolocationHide = (): void => {
    if (this.webChromeClient) {
      this.webChromeClient.onGeolocationPermissionsHidePrompt();
    }
  }

  onShowFileSelector = (event: ESObject): boolean => {
    if (this.webChromeClient) {
      return this.webChromeClient.onShowFileChooser(this, event.result, event.fileSelector);
    }
    return false;
  }

  onPermissionRequest = (event: ESObject): void => {
    if (this.webChromeClient) {
      this.webChromeClient.onPermissionRequest(event.request);
    }
  }

  onConsoleMessage = (event: ESObject): boolean => {
    if (this.webChromeClient) {
      this.webChromeClient.onConsoleMessage(event.message);
      return true;
    }
    const message = event.message as ConsoleMessage;
    Log.d(TAG, 'console[' + message.getMessageLevel() + '] ' + message.getMessage());
    return true;
  }

  onWindowNew = (event: ESObject): void => {
    if (this.webChromeClient) {
      this.webChromeClient.onCreateWindow(this, event.isAlert, event.isUserTrigger, event.handler);
    }
  }

  onWindowExit = (): void => {
    if (this.dialogController) {
      this.dialogController.close();
      this.dialogController = null;
    }
  }

  onScroll = (event: ESObject): void => {
    const oldX = this.scrollX;
    const oldY = this.scrollY;
    this.scrollX = event.xOffset;
    this.scrollY = event.yOffset;
    this.webViewFlutterApi.onScrollChanged(this, this.scrollX, this.scrollY, oldX, oldY, { reply: () => {} });
  }

  onControllerAttached = (): void => {
    this.controllerAttached = true;
    if (this.userAgentString.length > 0) {
      this.setCustomUserAgent(this.userAgentString);
      this.controller.refresh();
    }
    this.reRegisterJavaScriptChannels();
  }

  onJsAlert = (event: ESObject): boolean => {
    if (this.webChromeClient && event) {
      return this.webChromeClient.onJsAlert(this, event.url, event.message, event.result);
    }
    return true;
  }

  onJsConfirm = (event: ESObject): boolean => {
    if (this.webChromeClient && event) {
      return this.webChromeClient.onJsConfirm(this, event.url, event.message, event.result);
    }
    return true;
  }

  onJsPrompt = (event: ESObject): boolean => {
    if (this.webChromeClient && event) {
      return this.webChromeClient.onJsPrompt(this, event.url, event.message, event.value, event.result);
    }
    return true;
  }

  onFullScreenEnter = (_event: ESObject): void => {
    const context = getContext(this) as common.UIAbilityContext;
    const windowClass = context.windowStage.getMainWindowSync();
    windowClass.setWindowSystemBarEnable([]);
    this.rotation = windowClass.getPreferredOrientation();
    if (this.getWebSettings().getAllowFullScreenRotate()) {
      windowClass.setPreferredOrientation(window.Orientation.LANDSCAPE);
    }
  }

  onFullScreenExit = (): void => {
    const context = getContext(this) as common.UIAbilityContext;
    const windowClass = context.windowStage.getMainWindowSync();
    windowClass.setWindowSystemBarEnable(['status', 'navigation']);
    windowClass.setPreferredOrientation(this.rotation);
    this.reRegisterJavaScriptChannels();
  }

  aboutToAppear(): void {
    this.isWebviewBinding = true;
  }

  aboutToDisappear(): void {
    this.isWebviewBinding = false;
  }

  getType(): string {
    return 'web';
  }

  getView(): WrappedBuilder<[Params]> {
    return new WrappedBuilder(WebBuilder);
  }

  /** Binds the ArkUI component that commits native document-start script lists. */
  bindDocumentStartScriptHost(host: (scripts: Array<ScriptItem>) => Promise<void>): void {
    this.documentStartScriptHost = host;
    this.documentStartScriptHostWaiters.forEach((waiter: DocumentStartScriptHostWaiter): void => waiter.resolve(host));
    this.documentStartScriptHostWaiters = [];
  }

  /** Restores existing native registrations when an ArkUI Web component reattaches. */
  getDocumentStartScripts(): Array<ScriptItem> {
    const scripts = Array.from(this.documentStartScripts.values());
    if (this.pageZoomFactor !== 1) {
      scripts.unshift({ script: buildPageZoomScript(this.pageZoomFactor), scriptRules: ['*'] });
    }
    return scripts;
  }

  /** Retains page zoom across documents without changing Flutter paint scaling. */
  async setPageZoomFactor(factor: number): Promise<void> {
    if (!Number.isFinite(factor) || factor <= 0) throw new Error('Invalid page zoom factor');
    const host = await this.waitDocumentStartScriptHost();
    this.pageZoomFactor = factor;
    await host(this.getDocumentStartScripts());
    await this.waitControllerAttached();
    await this.controller.runJavaScript(buildPageZoomScript(factor));
  }

  /** Stops new registrations from targeting a component that has disappeared. */
  unbindDocumentStartScriptHost(): void {
    this.documentStartScriptHost = undefined;
  }

  /** Waits for the real Web component instead of racing page navigation. */
  private waitDocumentStartScriptHost(): Promise<(scripts: Array<ScriptItem>) => Promise<void>> {
    if (this.documentStartScriptHostDisposed) return Promise.reject(new Error('WebView script host is disposed'));
    const host = this.documentStartScriptHost;
    if (host !== undefined) return Promise.resolve(host);
    return new Promise((resolve, reject) => {
      this.documentStartScriptHostWaiters.push(new DocumentStartScriptHostWaiter(resolve, reject));
    });
  }

  /** Registers a native ArkWeb script and waits for the render commit before replying. */
  async addDocumentStartScript(source: string): Promise<string> {
    const host = await this.waitDocumentStartScriptHost();
    const identifier = 'operit-user-script-' + (++this.nextDocumentStartScriptIdentifier);
    this.documentStartScripts.set(identifier, { script: source, scriptRules: ['*'] });
    await host(this.getDocumentStartScripts());
    return identifier;
  }

  /** Removes one application registration from future ArkWeb documents. */
  async removeDocumentStartScript(identifier: string): Promise<void> {
    const host = await this.waitDocumentStartScriptHost();
    if (!this.documentStartScripts.delete(identifier)) throw new Error('Unknown script handle');
    await host(this.getDocumentStartScripts());
  }

  /** Clears application registrations without changing native JavaScript proxies. */
  async removeAllDocumentStartScripts(): Promise<void> {
    const host = await this.waitDocumentStartScriptHost();
    this.documentStartScripts.clear();
    await host(this.getDocumentStartScripts());
  }

  /** Rejects pending script-host registration when the native view is destroyed. */
  dispose(): void {
    this.documentStartScriptHostDisposed = true;
    this.documentStartScriptHost = undefined;
    this.documentStartScriptHostWaiters.forEach((waiter: DocumentStartScriptHostWaiter): void => waiter.reject(new Error('WebView script host is disposed')));
    this.documentStartScriptHostWaiters = [];
    this.documentStartScripts.clear();
    this.localResourceHandler = undefined;
    this.onLocalResourceDispose?.();
    this.onLocalResourceDispose = undefined;
    this.controller.stopAllMedia();
    if (this.onDisposeCallback) {
      this.onDisposeCallback();
    }
  }

  onInactive(): void {
    this.controller.onInactive();
  }

  onActive(): void {
    this.controller.onActive();
  }

  getPlatformViewVisibleAreaEventOptions(): PlatformViewVisibleAreaEventOptions {
    return {
      enable: this.visibleAreaEventEnable,
      ratios: this.visibleAreaEventRatios,
      expectedUpdateInterval: this.visibleAreaEventExpectedUpdateInterval,
      onInactiveThreshold: this.visibleAreaEventInactiveThreshold,
      onActiveThreshold: this.visibleAreaEventActiveThreshold
    } as PlatformViewVisibleAreaEventOptions;
  }

  getController(): web_webview.WebviewController {
    return this.controller;
  }

  getWebCookieManager(): web_webview.WebCookieManager {
    return this.cookieManager;
  }

  getUrl(): string {
    return this.url;
  }

  setUrl(url: string): void {
    this.url = url;
  }

  setCustomUserAgent(userAgentString: string): void {
    this.userAgentString = userAgentString;
    this.controller.setCustomUserAgent(userAgentString);
  }

  setOnDisposeCallback(callback: () => void): void {
    this.onDisposeCallback = callback;
  }

  setDownloadListener(listener: DownloadListener): void {
    this.downloadListener = listener;
  }

  setWebViewClient(webViewClient: WebViewClient): void {
    this.webViewClient = webViewClient;
  }

  setWebChromeClient(webChromeClient: WebChromeClient): void {
    this.webChromeClient = webChromeClient;
  }

  setDialogController(dialogController: CustomDialogController): void {
    this.dialogController = dialogController;
  }

  getDialogController(): CustomDialogController | null {
    return this.dialogController;
  }

  getWebSettings(): WebSettings {
    if (this.webSettings == null) {
      this.webSettings = new WebViewSettings(this);
    }
    return this.webSettings;
  }

  getScrollX(): number {
    return this.scrollX;
  }

  getScrollY(): number {
    return this.scrollY;
  }

  setScrollPosition(x: number, y: number): void {
    const oldX = this.scrollX;
    const oldY = this.scrollY;
    this.scrollX = x;
    this.scrollY = y;
    this.webViewFlutterApi.onScrollChanged(this, x, y, oldX, oldY, { reply: () => {} });
  }

  async waitControllerAttached(): Promise<void> {
    if (this.controllerAttached) {
      return;
    }
    await new Promise<void>((resolve, reject) => {
      try {
        const intervalId = setInterval(() => {
          if (this.controllerAttached) {
            clearInterval(intervalId);
            resolve();
          }
        }, 20);
      } catch (err) {
        reject(err);
      }
    });
  }

  addJavaScriptChannel(channel: JavaScriptChannel): void {
    this.javaScriptChannels.set(channel.getJavaScriptChannelName(), channel);
  }

  removeJavaScriptChannel(channelName: string): void {
    this.javaScriptChannels.delete(channelName);
  }

  reRegisterJavaScriptChannels(): void {
    if (this.javaScriptChannels.size === 0) {
      return;
    }
    try {
      this.javaScriptChannels.forEach((channel: JavaScriptChannel, channelName: string) => {
        this.controller.registerJavaScriptProxy(
          {
            postMessage: (message: string): void => {
              channel.postMessage(message);
            }
          },
          channelName,
          ['postMessage']
        );
      });
      this.controller.refresh();
    } catch (err) {
      Log.e(TAG, 'Failed to re-register JavaScript channels: ' + err);
    }
  }
}
