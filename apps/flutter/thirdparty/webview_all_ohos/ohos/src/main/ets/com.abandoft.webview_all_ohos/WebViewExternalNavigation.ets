import common from '@ohos.app.ability.common';
import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MethodChannel, { MethodCallHandler, MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';

/** Reads the protocol without interpreting an application URL as a webpage. */
function urlScheme(url: string): string {
  const match = /^([A-Za-z][A-Za-z0-9+.-]*):/.exec(url);
  return match === null ? '' : match[1].toLowerCase();
}

/** Keeps normal documents and private resources inside ArkWeb. */
function isPrivateResourceScheme(scheme: string): boolean {
  switch (scheme) {
    case 'file': case 'content': case 'about': case 'data': case 'blob': case 'javascript':
      return true;
    default:
      return false;
  }
}

/** Captures application protocols in both main-frame and subframe requests. */
export function isExternalApplicationUrl(url: string): boolean {
  const scheme = urlScheme(url);
  return scheme !== '' && scheme !== 'http' && scheme !== 'https' && !isPrivateResourceScheme(scheme);
}

/** Returns the actual system handoff result instead of reporting a pending launch as successful. */
export class WebViewExternalNavigation implements MethodCallHandler {
  private readonly channel: MethodChannel;
  private readonly getContext: () => common.UIAbilityContext | null;

  /** Registers the engine channel and resolves the currently attached Ability for each launch. */
  constructor(messenger: BinaryMessenger, getContext: () => common.UIAbilityContext | null) {
    this.getContext = getContext;
    this.channel = new MethodChannel(messenger, 'operit/webview_external_navigation');
    this.channel.setMethodCallHandler(this);
  }

  /** Validates browser targets and propagates asynchronous system refusal to Flutter. */
  onMethodCall(call: ESObject, result: MethodResult): void {
    if (call.method !== 'open') {
      result.notImplemented();
      return;
    }
    const url = call.argument('url') as string;
    if (typeof url !== 'string') {
      result.error('invalid_external_url', 'Expected an external URL', null);
      return;
    }
    const scheme = urlScheme(url);
    if (scheme === '' || scheme === 'intent' || isPrivateResourceScheme(scheme)) {
      result.error('invalid_external_url', 'The link is not supported by this browser owner', null);
      return;
    }
    const context = this.getContext();
    if (context === null) {
      result.error('external_navigation_unavailable', 'No Ability is attached to this browser owner', null);
      return;
    }
    try {
      context.startAbility({
        action: 'ohos.want.action.viewData',
        entities: ['entity.system.browsable'],
        uri: url
      }).then((): void => result.success(true))
        .catch((error: Error): void => result.error('external_navigation_failed', error.message, null));
    } catch (error) {
      result.error('external_navigation_failed', (error as Error).message, null);
    }
  }

  /** Releases this engine's channel handler during plugin teardown. */
  dispose(): void {
    this.channel.setMethodCallHandler(null);
  }
}
