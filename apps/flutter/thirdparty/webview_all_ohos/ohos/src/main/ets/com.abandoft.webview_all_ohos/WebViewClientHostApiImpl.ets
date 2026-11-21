import { HttpAuthHandler, Reply, SslAuthHandler } from './OhosWebViewCallbackApiChannel';
import { InstanceManager } from './InstanceManager';
import { WebViewClientFlutterApiImpl } from './WebViewClientFlutterApiImpl';
import { WebViewPlatformView } from './WebViewPlatformView';
import { isExternalApplicationUrl } from './WebViewExternalNavigation';

export interface WebViewClient {
  onPageStarted(view: WebViewPlatformView, url: string): void;
  onPageFinished(view: WebViewPlatformView, url: string): void;
  onReceivedError(view: WebViewPlatformView, request: WebResourceRequest, error: WebResourceError): void;
  onReceivedHttpError(view: WebViewPlatformView, request: WebResourceRequest, response: WebResourceResponse): void;
  shouldOverrideUrlLoading(view: WebViewPlatformView, request: WebResourceRequest): boolean;
  doUpdateVisitedHistory(view: WebViewPlatformView, url: string, isReload: boolean): void;
  onReceivedHttpAuthRequest(
    view: WebViewPlatformView,
    handler: HttpAuthHandler,
    host: string,
    realm: string
  ): void;
  onReceivedSslAuthError(
    view: WebViewPlatformView,
    handler: SslAuthHandler,
    url: string,
    errorCode: number,
    description: string
  ): void;
}

class OhosWebViewClient implements WebViewClient {
  private readonly flutterApi: WebViewClientFlutterApiImpl;
  private shouldOverride: boolean = false;
  private readonly emptyReply: Reply<void> = { reply: () => {} };

  constructor(flutterApi: WebViewClientFlutterApiImpl) {
    this.flutterApi = flutterApi;
  }

  onPageStarted(view: WebViewPlatformView, url: string): void {
    this.flutterApi.onPageStartedImpl(this, view, url, this.emptyReply);
  }

  onPageFinished(view: WebViewPlatformView, url: string): void {
    this.flutterApi.onPageFinishedImpl(this, view, url, this.emptyReply);
  }

  onReceivedError(view: WebViewPlatformView, request: WebResourceRequest, error: WebResourceError): void {
    this.flutterApi.onReceivedRequestErrorImpl(this, view, request, error, this.emptyReply);
  }

  onReceivedHttpError(view: WebViewPlatformView, request: WebResourceRequest, response: WebResourceResponse): void {
    this.flutterApi.onReceivedHttpErrorImpl(this, view, request, response, this.emptyReply);
  }

  /** Leaves document subframes in ArkWeb while routing application links through the owner. */
  shouldOverrideUrlLoading(view: WebViewPlatformView, request: WebResourceRequest): boolean {
    this.flutterApi.requestLoadingImpl(this, view, request, this.emptyReply);
    return this.shouldOverride && (request.isMainFrame() || isExternalApplicationUrl(request.getRequestUrl()));
  }

  doUpdateVisitedHistory(view: WebViewPlatformView, url: string, isReload: boolean): void {
    this.flutterApi.doUpdateVisitedHistoryImpl(this, view, url, isReload, this.emptyReply);
  }

  onReceivedHttpAuthRequest(
    view: WebViewPlatformView,
    handler: HttpAuthHandler,
    host: string,
    realm: string
  ): void {
    this.flutterApi.onReceivedHttpAuthRequestImpl(this, view, handler, host, realm, this.emptyReply);
  }

  onReceivedSslAuthError(
    view: WebViewPlatformView,
    handler: SslAuthHandler,
    url: string,
    errorCode: number,
    description: string
  ): void {
    this.flutterApi.onReceivedSslAuthErrorImpl(this, view, handler, url, errorCode, description, this.emptyReply);
  }

  setReturnValueForShouldOverrideUrlLoading(value: boolean): void {
    this.shouldOverride = value;
  }
}

export class WebViewClientCreator {
  createWebViewClient(flutterApi: WebViewClientFlutterApiImpl): WebViewClient {
    return new OhosWebViewClient(flutterApi);
  }
}

export class WebViewClientHostApiImpl {
  private readonly instanceManager: InstanceManager;
  private readonly creator: WebViewClientCreator;
  private readonly flutterApi: WebViewClientFlutterApiImpl;

  constructor(
    instanceManager: InstanceManager,
    creator: WebViewClientCreator,
    flutterApi: WebViewClientFlutterApiImpl
  ) {
    this.instanceManager = instanceManager;
    this.creator = creator;
    this.flutterApi = flutterApi;
  }

  create(instanceId: number): void {
    this.instanceManager.addDartCreatedInstance(
      this.creator.createWebViewClient(this.flutterApi),
      instanceId
    );
  }

  setSynchronousReturnValueForShouldOverrideUrlLoading(instanceId: number, value: boolean): void {
    const client = this.instanceManager.getInstance<WebViewClient>(instanceId);
    if (client instanceof OhosWebViewClient) {
      client.setReturnValueForShouldOverrideUrlLoading(value);
      return;
    }
    throw new Error('The registered WebViewClient does not expose synchronous URL-loading control.');
  }
}
