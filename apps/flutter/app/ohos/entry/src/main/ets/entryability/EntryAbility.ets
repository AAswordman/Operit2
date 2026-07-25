
import { FlutterAbility, FlutterEngine } from '@ohos/flutter_ohos';
import promptAction from '@ohos.promptAction';
import window from '@ohos.window';
import { GeneratedPluginRegistrant } from '../plugins/GeneratedPluginRegistrant';
import MethodChannel from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import Want from '@ohos.app.ability.Want';
import AbilityConstant from '@ohos.app.ability.AbilityConstant';
import { OperitRuntimeChannel } from './OperitRuntimeChannel';

export default class EntryAbility extends FlutterAbility {
  private operitRuntimeChannel: OperitRuntimeChannel | null = null;
  private crashChannel: MethodChannel | null = null;
  private currentWindowStage: window.WindowStage | null = null;

  /// Captures the notification activation that launched this application process.
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    super.onCreate(want, launchParam)
    this.recordNotificationActivation(want)
  }

  /// Captures a notification activation delivered to an already-running ability.
  onNewWant(want: Want, launchParams: AbilityConstant.LaunchParam): void {
    super.onNewWant(want, launchParams)
    this.recordNotificationActivation(want)
  }

  /// Configures Flutter plugins and the Operit runtime channel.
  configureFlutterEngine(flutterEngine: FlutterEngine) {
    super.configureFlutterEngine(flutterEngine)
    GeneratedPluginRegistrant.registerWith(flutterEngine)
    const channel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/runtime')
    this.operitRuntimeChannel = OperitRuntimeChannel.register(channel, this.context, this.currentWindowStage)
    this.crashChannel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/crash')
    this.crashChannel.setMethodCallHandler({
      onMethodCall: (call, result): void => {
        if (call.method !== 'present') {
          result.notImplemented()
          return
        }
        const details = call.argument('details') as string | null
        if (details === null) {
          result.error('INVALID_ARGS', 'present requires crash details', null)
          return
        }
        promptAction.showDialog({
          title: 'Operit2 has stopped',
          message: details,
          buttons: [{ text: 'Close', color: '#007DFF' }],
        }).then((): void => {
          result.success(null)
        })
      }
    })
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
    this.crashChannel?.setMethodCallHandler(null)
    this.crashChannel = null
    this.operitRuntimeChannel?.destroy()
    this.operitRuntimeChannel = null
    super.onDestroy()
  }

  /// Parses and forwards the Operit notification activation embedded in one Want.
  private recordNotificationActivation(want: Want): void {
    const parameters = want.parameters as Record<string, Object> | undefined
    const encodedActivation = parameters?.['operitNotificationActivation']
    if (typeof encodedActivation !== 'string') {
      return
    }
    const activation = JSON.parse(encodedActivation) as Record<string, Object>
    OperitRuntimeChannel.receiveNotificationActivation(activation)
  }
}
