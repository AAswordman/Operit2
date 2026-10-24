import { BinaryMessenger } from '@ohos/flutter_ohos/src/main/ets/plugin/common/BinaryMessenger';
import MethodChannel, { MethodCallHandler, MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import { InstanceManager } from './InstanceManager';
import { WebViewPlatformView } from './WebViewPlatformView';

/** Routes document-start registration to the native ArkWeb component. */
export class WebViewUserScripts implements MethodCallHandler {
  private channel: MethodChannel;
  private instanceManager: InstanceManager;

  /** Registers the per-engine script channel separately from resource interception. */
  constructor(messenger: BinaryMessenger, instanceManager: InstanceManager) {
    this.instanceManager = instanceManager;
    this.channel = new MethodChannel(messenger, 'operit/webview_user_scripts');
    this.channel.setMethodCallHandler(this);
  }

  /** Acknowledges requests only after the native script-list update is committed. */
  onMethodCall(call: ESObject, result: MethodResult): void {
    this.execute(call).then((identifier: string | null): void => {
      result.success(identifier);
    }).catch((error: Error): void => {
      result.error('document_start_script_failed', error.message, null);
    });
  }

  /** Resolves the native view and performs the exact requested registration operation. */
  private async execute(call: ESObject): Promise<string | null> {
    const viewIdentifier = call.argument('viewIdentifier') as number;
    const view = this.instanceManager.getInstance<WebViewPlatformView>(viewIdentifier);
    if (view === undefined || view === null) throw new Error('Unknown WebView');
    switch (call.method) {
      case 'add': {
        const source = call.argument('source') as string;
        if (typeof source !== 'string') throw new Error('Missing document-start source');
        return await view.addDocumentStartScript(source);
      }
      case 'remove': {
        const identifier = call.argument('identifier') as string;
        if (typeof identifier !== 'string') throw new Error('Missing script handle');
        await view.removeDocumentStartScript(identifier);
        return null;
      }
      case 'removeAll':
        await view.removeAllDocumentStartScripts();
        return null;
      default:
        throw new Error('Unknown document-start operation');
    }
  }

  /** Detaches the script channel when the Flutter engine is destroyed. */
  dispose(): void {
    this.channel.setMethodCallHandler(null);
  }
}
