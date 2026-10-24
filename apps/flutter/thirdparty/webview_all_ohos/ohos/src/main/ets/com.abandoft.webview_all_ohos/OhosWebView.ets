import { FrameCallback } from '@kit.ArkUI';
import web_webview from '@ohos.web.webview';
import PlatformView from '@ohos/flutter_ohos/src/main/ets/plugin/platform/PlatformView';
import { WebViewPlatformView } from './WebViewPlatformView';

@Observed
export declare class Params {
  direction: Direction;
  platformView: PlatformView;
}

/** Completes a native script-list update only after the ArkUI frame is committed. */
class DocumentStartScriptCommit extends FrameCallback {
  private settled: boolean = false;
  private complete: () => void;
  private fail: (error: Error) => void;

  /** Retains the platform-channel acknowledgement callbacks for one render frame. */
  constructor(complete: () => void, fail: (error: Error) => void) {
    super();
    this.complete = complete;
    this.fail = fail;
  }

  /** Acknowledges registration after native Web attributes have been applied. */
  onIdle(_timeLeftInNano: number): void {
    if (this.settled) return;
    this.settled = true;
    this.complete();
  }

  /** Rejects an unfinished registration when the native component is detached. */
  cancel(): void {
    if (this.settled) return;
    this.settled = true;
    this.fail(new Error('WebView detached before document-start scripts were committed'));
  }
}

@Component
export struct OhosWebView {
  @StorageLink('operitWebViewDark') preferredDark: boolean = false;
  @ObjectLink params: Params;

  webView: WebViewPlatformView = this.params.platformView as WebViewPlatformView;
  controller: web_webview.WebviewController = this.webView.getController();

  @State textZoomRatio: number = this.webView.getWebSettings().getTextZoom();
  @State disposed: boolean = false;
  @State documentStartScripts: Array<ScriptItem> = [];
  private scriptCommits: Set<DocumentStartScriptCommit> = new Set();

  /** Binds deterministic script registration to this native Web component. */
  aboutToAppear(): void {
    this.documentStartScripts = this.webView.getDocumentStartScripts();
    this.webView.bindDocumentStartScriptHost((scripts: Array<ScriptItem>): Promise<void> => this.commitDocumentStartScripts(scripts));
    this.webView.getWebSettings().onTextZoomChanged((ratio: number) => {
      this.textZoomRatio = ratio;
    });
    this.webView.setOnDisposeCallback(() => {
      this.disposed = true;
      this.cancelScriptCommits();
    });
  }

  /** Commits a new native script list before the channel permits navigation. */
  private commitDocumentStartScripts(scripts: Array<ScriptItem>): Promise<void> {
    if (this.disposed) return Promise.reject(new Error('WebView is disposed'));
    this.documentStartScripts = scripts;
    return new Promise((resolve, reject) => {
      const commit = new DocumentStartScriptCommit((): void => {
        this.scriptCommits.delete(commit);
        resolve();
      }, (error: Error): void => {
        this.scriptCommits.delete(commit);
        reject(error);
      });
      this.scriptCommits.add(commit);
      this.getUIContext().postFrameCallback(commit);
    });
  }

  /** Rejects pending frame acknowledgements when the component disappears. */
  aboutToDisappear(): void {
    this.webView.unbindDocumentStartScriptHost();
    this.cancelScriptCommits();
  }

  /** Cancels all in-flight registrations without reporting false success. */
  private cancelScriptCommits(): void {
    this.scriptCommits.forEach((commit: DocumentStartScriptCommit): void => commit.cancel());
    this.scriptCommits.clear();
  }

  /** Applies the registered scripts through ArkWeb's document-start attribute. */
  build() {
    Column() {
      if (!this.disposed) {
        Web({ src: this.webView.getUrl(), controller: this.controller })
          .darkMode(this.preferredDark ? WebDarkMode.On : WebDarkMode.Off)
          .forceDarkAccess(false)
          .keyboardAvoidMode(WebKeyboardAvoidMode.RESIZE_VISUAL)
          .backgroundColor(this.webView.getWebSettings().getBackgroundColor())
          .domStorageAccess(this.webView.getWebSettings().getDomStorageEnabled())
          .allowWindowOpenMethod(this.webView.getWebSettings().getJavaScriptCanOpenWindowsAutomatically())
          .multiWindowAccess(false)
          .javaScriptAccess(this.webView.getWebSettings().getJavaScriptEnabled())
          .runJavaScriptOnDocumentStart(this.documentStartScripts)
          .mediaPlayGestureAccess(this.webView.getWebSettings().getMediaPlaybackRequiresUserGesture())
          .zoomAccess(this.webView.getWebSettings().getSupportZoom())
          .overviewModeAccess(this.webView.getWebSettings().getLoadWithOverviewMode())
          .wideViewModeAccess(this.webView.getWebSettings().getUseWideViewPort())
          .metaViewport(this.webView.getWebSettings().getUseWideViewPort())
          .fileAccess(this.webView.getWebSettings().getAllowFileAccess())
          .textZoomRatio(this.textZoomRatio)
          .mixedMode(MixedMode.All)
          .onControllerAttached(this.webView.onControllerAttached)
          .onDownloadStart(this.webView.onDownloadStart)
          .onPageBegin(this.webView.onPageBegin)
          .onPageEnd(this.webView.onPageEnd)
          .onErrorReceive(this.webView.onErrorReceive)
          .onHttpErrorReceive(this.webView.onHttpErrorReceive)
          .onInterceptRequest(this.webView.onInterceptRequest)
          .onRefreshAccessedHistory(this.webView.onRefreshAccessedHistory)
          .onProgressChange(this.webView.onProgressChange)
          .onGeolocationShow(this.webView.onGeolocationShow)
          .onGeolocationHide(this.webView.onGeolocationHide)
          .onShowFileSelector(this.webView.onShowFileSelector)
          .onPermissionRequest(this.webView.onPermissionRequest)
          .onConsole(this.webView.onConsoleMessage)
          .onWindowExit(this.webView.onWindowExit)
          .onWindowNew(this.webView.onWindowNew)
          .onSslErrorEventReceive(this.webView.onSslErrorEventReceive)
          .onHttpAuthRequest(this.webView.onHttpAuthRequest)
          .onAlert(this.webView.onJsAlert)
          .onConfirm(this.webView.onJsConfirm)
          .onPrompt(this.webView.onJsPrompt)
          .onOverrideUrlLoading(this.webView.onOverrideUrlLoading)
          .onFullScreenEnter(this.webView.onFullScreenEnter)
          .onFullScreenExit(this.webView.onFullScreenExit)
          .onScroll(this.webView.onScroll)
      }
    }
  }
}

@Builder
export function WebBuilder(params: Params) {
  OhosWebView({
    params: params,
    webView: params.platformView as WebViewPlatformView,
    controller: (params.platformView as WebViewPlatformView).getController()
  }).backgroundColor(Color.Transparent)
}

@CustomDialog
struct PopupWebView {
  @StorageLink('operitWebViewDark') preferredDark: boolean = false;
  controller?: CustomDialogController;
  webViewController: web_webview.WebviewController = new web_webview.WebviewController();

  build() {
    Web({ src: '', controller: this.webViewController })
      .darkMode(this.preferredDark ? WebDarkMode.On : WebDarkMode.Off)
      .forceDarkAccess(false)
      .javaScriptAccess(true)
      .multiWindowAccess(false)
      .onWindowExit(() => {
        if (this.controller) {
          this.controller.close();
        }
      })
  }
}
