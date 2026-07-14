
import { FlutterAbility, FlutterEngine } from '@ohos/flutter_ohos';
import window from '@ohos.window';
import { GeneratedPluginRegistrant } from '../plugins/GeneratedPluginRegistrant';
import MethodChannel from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import { OperitRuntimeChannel } from './OperitRuntimeChannel';

export default class EntryAbility extends FlutterAbility {
  private operitRuntimeChannel: OperitRuntimeChannel | null = null;
  private currentWindowStage: window.WindowStage | null = null;

  /// Configures Flutter plugins and the Operit runtime channel.
  configureFlutterEngine(flutterEngine: FlutterEngine) {
    super.configureFlutterEngine(flutterEngine)
    GeneratedPluginRegistrant.registerWith(flutterEngine)
    const channel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/runtime')
    this.operitRuntimeChannel = OperitRuntimeChannel.register(channel, this.context, this.currentWindowStage)
  }

  /// Stores the active WindowStage for host screenshot capture.
  onWindowStageCreate(windowStage: window.WindowStage): void {
    super.onWindowStageCreate(windowStage)
    this.currentWindowStage = windowStage
    this.operitRuntimeChannel?.setWindowStage(windowStage)
  }

  /// Clears the WindowStage reference after the ability window is destroyed.
  onWindowStageDestroy(): void {
    this.currentWindowStage = null
    this.operitRuntimeChannel?.setWindowStage(null)
    super.onWindowStageDestroy()
  }

  /// Releases the native Operit runtime bridge.
  onDestroy(): void {
    this.operitRuntimeChannel?.destroy()
    this.operitRuntimeChannel = null
    super.onDestroy()
  }
}
