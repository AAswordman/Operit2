import { OperitPdfPreview } from './OperitPdfPreview';
import { OperitAudioPlayers } from './OperitAudioPlayers';
import OperitKeepAlivePlugin from 'operit_keep_alive';

import { FlutterAbility, FlutterEngine } from '@ohos/flutter_ohos';
import promptAction from '@ohos.promptAction';
import window from '@ohos.window';
import bundleManager from '@ohos.bundle.bundleManager';
import { GeneratedPluginRegistrant } from '../plugins/GeneratedPluginRegistrant';
import MethodChannel from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import Want from '@ohos.app.ability.Want';
import AbilityConstant from '@ohos.app.ability.AbilityConstant';
import { OperitRuntimeChannel } from './OperitRuntimeChannel';

export default class EntryAbility extends FlutterAbility {
  private pdfPreview: OperitPdfPreview | null = null;
  private audioPlayers: OperitAudioPlayers | null = null;
  private operitRuntimeChannel: OperitRuntimeChannel | null = null;
  private crashChannel: MethodChannel | null = null;
  private themeChannel: MethodChannel | null = null;
  private packageInfoChannel: MethodChannel | null = null;
  private currentWindowStage: window.WindowStage | null = null;

  /// Captures the notification activation that launched this application process.
  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    AppStorage.setOrCreate<boolean>('operitDataRescueRequested', want.parameters?.['dataRescue'] === true || want.action === 'app.operit.DATA_RESCUE')
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
    this.registerPackageInfoChannel(flutterEngine)
    this.audioPlayers = new OperitAudioPlayers(flutterEngine, this.context)
    this.themeChannel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/webview_theme')
    // Updates ArkWeb preferences without overriding the application's system theme source.
    this.themeChannel.setMethodCallHandler({
      onMethodCall: (call, result): void => {
        if (call.method !== 'setPreferredColorScheme') {
          result.notImplemented()
          return
        }
        const scheme = call.args as string
        if (scheme !== 'dark' && scheme !== 'light') {
          result.error('invalid_color_scheme', 'Expected dark or light', null)
          return
        }
        AppStorage.setOrCreate<boolean>('operitWebViewDark', scheme === 'dark')
        result.success(null)
      }
    })
    const channel = new MethodChannel(flutterEngine.getDartExecutor(), 'operit/runtime')
    const runtime = OperitRuntimeChannel.register(channel, this.context, this.currentWindowStage)
    this.operitRuntimeChannel = runtime
    this.pdfPreview = new OperitPdfPreview(flutterEngine, this.context,
      (request: string): Promise<string> => runtime.renderPdf(request))
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

  /// Implements the shared package_info_plus protocol using installed HAP metadata.
  private registerPackageInfoChannel(flutterEngine: FlutterEngine): void {
    this.packageInfoChannel = new MethodChannel(
      flutterEngine.getDartExecutor(), 'dev.fluttercommunity.plus/package_info'
    )
    this.packageInfoChannel.setMethodCallHandler({
      onMethodCall: (call, result): void => {
        if (call.method !== 'getAll') {
          result.notImplemented()
          return
        }
        bundleManager.getBundleInfoForSelf(
          bundleManager.BundleFlag.GET_BUNDLE_INFO_WITH_APPLICATION
        ).then(async (info: bundleManager.BundleInfo): Promise<void> => {
          const appName = await this.context.resourceManager.getStringValue(info.appInfo.labelId)
          const metadata = new Map<string, Object>()
          metadata.set('appName', appName)
          metadata.set('packageName', info.name)
          metadata.set('version', info.versionName)
          metadata.set('buildNumber', info.versionCode.toString())
          result.success(metadata)
        }).catch((error: Object): void => {
          result.error('PACKAGE_INFO_ERROR', `${error}`, null)
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
    this.packageInfoChannel?.setMethodCallHandler(null)
    this.packageInfoChannel = null
    this.themeChannel?.setMethodCallHandler(null)
    this.themeChannel = null
    this.crashChannel?.setMethodCallHandler(null)
    this.crashChannel = null
    this.pdfPreview?.dispose()
    this.pdfPreview = null
    this.audioPlayers?.dispose().catch((error: Error): void => { console.error(error.message) })
    this.audioPlayers = null
    this.operitRuntimeChannel?.destroy()
    this.operitRuntimeChannel = null
    super.onDestroy()
  }

  /// Parses and forwards the Operit notification activation embedded in one Want.
  private recordNotificationActivation(want: Want): void {
    const parameters = want.parameters as Record<string, Object> | undefined
    const workActivation = parameters?.['operitKeepAliveWorkAction']
    if (typeof workActivation === 'string') {
      OperitKeepAlivePlugin.receiveWorkActivation(workActivation)
      return
    }
    const encodedActivation = parameters?.['operitNotificationActivation']
    if (typeof encodedActivation !== 'string') {
      return
    }
    const activation = JSON.parse(encodedActivation) as Record<string, Object>
    OperitRuntimeChannel.receiveNotificationActivation(activation)
  }
}
