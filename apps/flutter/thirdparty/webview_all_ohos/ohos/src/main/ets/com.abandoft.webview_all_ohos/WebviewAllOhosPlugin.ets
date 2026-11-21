import { WebViewUserScripts } from './WebViewUserScripts';
import { WebViewDownloads } from './WebViewDownloads';
import { WebViewExternalNavigation } from './WebViewExternalNavigation';
import { WebViewZoom } from './WebViewZoom';
import { WebViewLocalResources } from './WebViewLocalResources';
import common from '@ohos.app.ability.common';
import AbilityAware from '@ohos/flutter_ohos/src/main/ets/embedding/engine/plugins/ability/AbilityAware';
import { AbilityPluginBinding } from '@ohos/flutter_ohos/src/main/ets/embedding/engine/plugins/ability/AbilityPluginBinding';
import { FlutterPlugin, FlutterPluginBinding } from '@ohos/flutter_ohos/src/main/ets/embedding/engine/plugins/FlutterPlugin';
import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import PlatformViewRegistry from '@ohos/flutter_ohos/src/main/ets/plugin/platform/PlatformViewRegistry';
import { CookieManagerHostApiImpl } from './CookieManagerHostApiImpl';
import { CustomViewCallbackHostApiImpl } from './CustomViewCallbackHostApiImpl';
import { DownloadListenerFlutterApiImpl } from './DownloadListenerFlutterApiImpl';
import { DownloadListenerCreator, DownloadListenerHostApiImpl } from './DownloadListenerHostApiImpl';
import { FlutterAssetManager, PluginBindingFlutterAssetManager } from './FlutterAssetManager';
import { FlutterAssetManagerHostApiImpl } from './FlutterAssetManagerHostApiImpl';
import { FlutterWebViewFactory } from './FlutterWebViewFactory';
import { GeolocationPermissionsCallbackHostApiImpl } from './GeolocationPermissionsCallbackHostApiImpl';
import { HttpAuthHandlerHostApiImpl } from './HttpAuthHandlerHostApiImpl';
import { InstanceManager } from './InstanceManager';
import { JavaScriptChannelCreator, JavaScriptChannelHostApiImpl } from './JavaScriptChannelHostApiImpl';
import { JavaScriptChannelFlutterApiImpl } from './JavaScriptChannelFlutterApiImpl';
import { OhosObjectHostApiImpl } from './OhosObjectHostApiImpl';
import { OhosWebViewCallbackApiChannel } from './OhosWebViewCallbackApiChannel';
import { OhosWebViewHostApiChannel } from './OhosWebViewHostApiChannel';
import { PermissionRequestHostApiImpl } from './PermissionRequestHostApiImpl';
import { SslAuthHandlerHostApiImpl } from './SslAuthHandlerHostApiImpl';
import { WEBVIEW_ALL_OHOS_VIEW_TYPE } from './WebViewAllConstants';
import { WebChromeClientCreator, WebChromeClientHostApiImpl } from './WebChromeClientHostApiImpl';
import { WebChromeClientFlutterApiImpl } from './WebChromeClientFlutterApiImpl';
import { WebSettingsCreator, WebSettingsHostApiImpl } from './WebSettingsHostApiImpl';
import { WebStorageHostApiImpl } from './WebStorageHostApiImpl';
import { WebViewClientCreator, WebViewClientHostApiImpl } from './WebViewClientHostApiImpl';
import { WebViewClientFlutterApiImpl } from './WebViewClientFlutterApiImpl';
import { WebViewHostApiImpl, WebViewProxy } from './WebViewHostApiImpl';

const PLUGIN_NAME = 'WebviewAllOhosPlugin';

export class WebviewAllOhosPlugin implements FlutterPlugin, AbilityAware {
  private localResources: WebViewLocalResources | null = null;
  private userScripts: WebViewUserScripts | null = null;
  private downloads: WebViewDownloads | null = null;
  private externalNavigation: WebViewExternalNavigation | null = null;
  private abilityContext: common.UIAbilityContext | null = null;
  private zoom: WebViewZoom | null = null;
  private instanceManager: InstanceManager | null = null;
  private hostApiChannel: OhosWebViewHostApiChannel | null = null;

  getUniqueClassName(): string {
    return PLUGIN_NAME;
  }

  onAttachedToEngine(binding: FlutterPluginBinding): void {
    const context = binding.getApplicationContext();
    const assets = new PluginBindingFlutterAssetManager(
      context.resourceManager,
      binding.getFlutterAssets()
    );
    this.attach(
      binding.getBinaryMessenger(),
      binding.getPlatformViewRegistry(),
      context,
      assets
    );
  }

  onDetachedFromEngine(_binding: FlutterPluginBinding): void {
    this.externalNavigation?.dispose();
    this.externalNavigation = null;
    this.abilityContext = null;
    this.downloads?.dispose();
    this.downloads = null;
    this.zoom?.dispose();
    this.zoom = null;
    this.userScripts?.dispose();
    this.userScripts = null;
    this.localResources?.dispose();
    this.localResources = null;
    if (this.hostApiChannel != null) {
      this.hostApiChannel.dispose();
      this.hostApiChannel = null;
    }
    if (this.instanceManager != null) {
      this.instanceManager.stopFinalizationListener();
      this.instanceManager = null;
    }
  }

  getInstanceManager(): InstanceManager | null {
    return this.instanceManager;
  }

  /** Tracks the active Ability used by approved external application launches. */
  onAttachedToAbility(binding: AbilityPluginBinding): void {
    this.abilityContext = binding.getAbility().context;
  }

  /** Prevents launches from using an Ability that has already detached. */
  onDetachedFromAbility(): void {
    this.abilityContext = null;
  }

  private attach(
    messenger: BinaryMessenger,
    viewRegistry: PlatformViewRegistry,
    context: common.Context,
    assetManager: FlutterAssetManager
  ): void {
    const callbackApi = new OhosWebViewCallbackApiChannel(messenger);
    const instanceManager = new InstanceManager({
      onFinalize: (identifier: number) => {
        callbackApi.invoke('OhosObject.dispose', new Map<string, ESObject>().set('identifier', identifier));
      }
    });

    viewRegistry.registerViewFactory(
      WEBVIEW_ALL_OHOS_VIEW_TYPE,
      new FlutterWebViewFactory(instanceManager)
    );

    const javaScriptFlutterApi = new JavaScriptChannelFlutterApiImpl(messenger, instanceManager);
    const webViewClientFlutterApi = new WebViewClientFlutterApiImpl(messenger, instanceManager);
    const webChromeClientFlutterApi = new WebChromeClientFlutterApiImpl(messenger, instanceManager);
    const downloadListenerFlutterApi = new DownloadListenerFlutterApiImpl(messenger, instanceManager);

    this.instanceManager = instanceManager;
    this.downloads = new WebViewDownloads(messenger, instanceManager, context);
    this.externalNavigation = new WebViewExternalNavigation(messenger, () => this.abilityContext);
    this.localResources = new WebViewLocalResources(messenger, instanceManager);
    this.userScripts = new WebViewUserScripts(messenger, instanceManager);
    this.zoom = new WebViewZoom(messenger, instanceManager);
    this.hostApiChannel = new OhosWebViewHostApiChannel(
      messenger,
      instanceManager,
      new OhosObjectHostApiImpl(instanceManager),
      new CookieManagerHostApiImpl(messenger, instanceManager),
      new WebViewHostApiImpl(instanceManager, new WebViewProxy(), messenger, context),
      new JavaScriptChannelHostApiImpl(
        instanceManager,
        new JavaScriptChannelCreator(),
        javaScriptFlutterApi
      ),
      new WebViewClientHostApiImpl(
        instanceManager,
        new WebViewClientCreator(),
        webViewClientFlutterApi
      ),
      new WebChromeClientHostApiImpl(
        instanceManager,
        new WebChromeClientCreator(),
        webChromeClientFlutterApi
      ),
      new DownloadListenerHostApiImpl(
        instanceManager,
        new DownloadListenerCreator(),
        downloadListenerFlutterApi
      ),
      new WebSettingsHostApiImpl(instanceManager, new WebSettingsCreator()),
      new FlutterAssetManagerHostApiImpl(assetManager),
      new WebStorageHostApiImpl(),
      new PermissionRequestHostApiImpl(messenger, instanceManager),
      new GeolocationPermissionsCallbackHostApiImpl(messenger, instanceManager),
      new CustomViewCallbackHostApiImpl(messenger, instanceManager),
      new HttpAuthHandlerHostApiImpl(messenger, instanceManager),
      new SslAuthHandlerHostApiImpl(messenger, instanceManager)
    );
  }
}
