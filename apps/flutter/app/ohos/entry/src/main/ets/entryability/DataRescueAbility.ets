import { FlutterAbility, FlutterEngine } from '@ohos/flutter_ohos';
import MethodChannel from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import runtimeNative from 'liboperit_runtime_ohos.so';
import FileSelectorOhosPlugin from 'file_selector_ohos';

interface RescueLaunchMode {
  rescue: boolean;
}

interface RescueLaunchResponse {
  ok: boolean;
  value: RescueLaunchMode;
}

interface OfflineNativeModule {
  /** Executes blocking rescue work on the platform worker pool. */
  dataRescue(defaultRuntimeRoot: string, request: string): Promise<string>;
}

/** Hosts only the maintenance channel without constructing the normal owner-system channel. */
export default class DataRescueAbility extends FlutterAbility {
  private rescueChannel: MethodChannel | null = null;

  /** Installs the independent offline ABI before Dart selects its startup route. */
  configureFlutterEngine(flutterEngine: FlutterEngine): void {
    flutterEngine.getPlugins()?.add(new FileSelectorOhosPlugin());
    const nativeBridge = runtimeNative as OfflineNativeModule;
    this.rescueChannel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/runtime');
    this.rescueChannel.setMethodCallHandler({
      onMethodCall: (call, result): void => {
        if (call.method !== 'dataRescue') { result.notImplemented(); return; }
        try {
          const encoded = call.args as string;
          const command = JSON.parse(encoded) as Record<string, string>;
          if (command['operation'] === 'launchMode') {
            const launch: RescueLaunchResponse = { ok: true, value: { rescue: true } };
            result.success(JSON.stringify(launch));
            return;
          }
          nativeBridge.dataRescue(`${this.context.filesDir}/runtime`, encoded)
            .then((response: string): void => { result.success(response); })
            .catch((error: Object): void => { result.error('DATA_RESCUE_ERROR', `${error}`, null); });
        } catch (error) { result.error('DATA_RESCUE_ERROR', `${error}`, null); }
      }
    });
  }

  /** Detaches the maintenance channel without invoking normal runtime teardown. */
  onDestroy(): void {
    this.rescueChannel?.setMethodCallHandler(null);
    this.rescueChannel = null;
    super.onDestroy();
  }
}
