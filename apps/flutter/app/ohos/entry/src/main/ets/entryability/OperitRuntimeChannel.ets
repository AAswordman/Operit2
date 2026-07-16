import common from '@ohos.app.ability.common';
import commonEventManager from '@ohos.commonEventManager';
import Want from '@ohos.app.ability.Want';
import abilityAccessCtrl, { Permissions } from '@ohos.abilityAccessCtrl';
import wantConstant from '@ohos.ability.wantConstant';
import bundle from '@ohos.bundle';
import bundleState from '@ohos.bundleState';
import settings from '@ohos.settings';
import bluetoothManager from '@ohos.bluetoothManager';
import batteryInfo from '@ohos.batteryInfo';
import deviceInfo from '@ohos.deviceInfo';
import display from '@ohos.display';
import fs from '@ohos.file.fs';
import fileUri from '@ohos.file.fileuri';
import geoLocationManager from '@ohos.geoLocationManager';
import i18n from '@ohos.i18n';
import image from '@ohos.multimedia.image';
import audio from '@ohos.multimedia.audio';
import media from '@ohos.multimedia.media';
import notificationManager from '@ohos.notificationManager';
import connection from '@ohos.net.connection';
import promptAction from '@ohos.promptAction';
import taskpool from '@ohos.taskpool';
import util from '@ohos.util';
import window from '@ohos.window';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import MethodChannel, {
  MethodResult
} from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import runtimeNative from 'liboperit_runtime_ohos.so';

const CHANNEL = 'operit/runtime';

interface FileOperationResponse {
  success: boolean;
  error: string | null;
}

interface AudioPlayResponse {
  path: string;
  started: boolean;
  details: string;
}

interface MusicPlaybackResponse {
  state: string;
  source: string | null;
  sourceType: string | null;
  title: string | null;
  artist: string | null;
  durationMs: number | null;
  positionMs: number;
  bufferedPositionMs: number;
  volume: number;
  loopPlayback: boolean;
  message: string;
}

interface TtsPlaybackResponse {
  path: string;
  active: boolean;
  paused: boolean;
  details: string;
}

interface SystemOperationResponse {
  resultJson: string;
}

interface SystemCaptureScreenshotResponse {
  path: string;
}

interface JsonPayload {
  message?: string;
  title?: string;
  namespace?: string;
  setting?: string;
  value?: string;
  path?: string;
  packageName?: string | null;
  includeSystemApps?: boolean;
  limit?: number;
  includeOngoing?: boolean;
  sinceHours?: number;
  timeout?: number;
  highAccuracy?: boolean;
  includeAddress?: boolean;
  durationMs?: number;
  includeBle?: boolean;
  address?: string | null;
  uuid?: string;
  name?: string | null;
  listenerSessionId?: string;
  sessionId?: string;
  maxBytes?: number;
  serviceUuid?: string;
  characteristicUuid?: string;
  enable?: boolean;
  text?: string | null;
  dataBase64?: string;
  writeServiceUuid?: string;
  writeCharacteristicUuid?: string;
  readServiceUuid?: string;
  readCharacteristicUuid?: string;
  success?: boolean;
  operationType?: string;
  details?: string;
  includesSystemApps?: boolean;
  packages?: Array<string>;
  notifications?: Array<JsonPayload | BleNotificationEntry>;
  timestamp?: number;
  startTime?: number;
  endTime?: number;
  requestedPackageName?: string | null;
  totalEntries?: number;
  entries?: Array<JsonPayload>;
  appName?: string;
  totalForegroundTimeMs?: number;
  lastTimeUsed?: number;
  isSystemApp?: boolean;
  latitude?: number;
  longitude?: number;
  accuracy?: number;
  provider?: string;
  rawData?: string;
  city?: string;
  province?: string;
  country?: string;
  deviceId?: string;
  model?: string;
  manufacturer?: string;
  androidVersion?: string;
  sdkVersion?: number;
  screenResolution?: string;
  screenDensity?: number;
  totalMemory?: string;
  availableMemory?: string;
  totalStorage?: string;
  availableStorage?: string;
  batteryLevel?: number;
  batteryCharging?: boolean;
  cpuInfo?: string;
  networkType?: string;
  additionalInfo?: DeviceAdditionalInfo;
  supported?: boolean;
  enabled?: boolean;
  state?: string;
  devices?: Array<JsonPayload>;
  type?: string;
  bondState?: string;
  source?: string | null;
  rssi?: number | null;
  includesBle?: boolean;
  mode?: string;
  bytesWritten?: number;
  bytesRead?: number;
  services?: Array<BleServicePayload>;
  characteristics?: Array<BleCharacteristicPayload>;
  properties?: Array<string>;
}

interface JsonStringPayload {
  runtimeRoot?: string;
  workspaceRoot?: string;
  languageCode?: string;
  resultJson?: string;
}

interface OnboardingSnapshot {
  'ohos.location': OnboardingRequirementStatus;
  'ohos.bluetooth': OnboardingRequirementStatus;
}

interface StoragePaths {
  runtimeRoot: string;
  workspaceRoot: string;
}

interface OnboardingRequirementStatus {
  id: string;
  status: string;
}

interface ClassicBluetoothSession {
  address: string;
  mode: string;
  clientSocket: number | null;
  serverSocket: number | null;
  readQueue: Array<ArrayBuffer>;
  readCallback: ((data: ArrayBuffer) => void) | null;
}

interface BleNotificationEntry {
  characteristicUuid: string;
  bytesRead: number;
  text: string | null;
  dataBase64: string;
  timestamp: number;
}

interface BleBluetoothSession {
  address: string;
  client: bluetoothManager.GattClientDevice;
  notifications: Array<BleNotificationEntry>;
  notifyCallback: ((characteristic: bluetoothManager.BLECharacteristic) => void) | null;
}

interface OhosApplicationInfo {
  name: string;
  systemApp: boolean;
}

interface DeviceAdditionalInfo {
  deviceType: string;
  brand: string;
  marketName: string;
  displayVersion: string;
  buildType: string;
  displayName: string;
}

interface BleServicePayload {
  uuid: string;
  characteristics: Array<BleCharacteristicPayload>;
}

interface BleCharacteristicPayload {
  uuid: string;
  properties: Array<string>;
}

interface RuntimeNativeModule {
  create(runtimeRoot: string, workspaceRoot: string, systemLanguageCode: string): bigint;
  destroy(handle: bigint): void;
  call(handle: bigint, request: ArrayBuffer): ArrayBuffer;
  pushOpen(handle: bigint, request: ArrayBuffer): ArrayBuffer;
  pushItem(handle: bigint, item: ArrayBuffer): ArrayBuffer;
  pushClose(handle: bigint, pushId: string): ArrayBuffer;
  watchSnapshot(handle: bigint, request: ArrayBuffer): ArrayBuffer;
  watchStream(handle: bigint, request: ArrayBuffer): ArrayBuffer;
  nextWatchChannelEvent(handle: bigint): ArrayBuffer;
  closeWatchStream(handle: bigint, subscriptionId: string): ArrayBuffer;
  startWebAccessServer(
    handle: bigint,
    bindHost: string,
    requestedPort: string,
    authToken: string,
    serverName: string,
    advertise: string,
    pairingCodePath: string
  ): string;
  discoverDevices(handle: bigint, timeoutMs: string): string;
  stopWebAccessServer(handle: bigint): string;
  remotePairStart(handle: bigint, baseUrl: string, tokenHash: string, clientDeviceInfo: string): string;
  remotePairFinish(handle: bigint, pairingId: string, pairingCode: string): string;
  emitRuntimeEvent(handle: bigint, eventJson: string): string;
}

interface RuntimeEventDeliveryResponse {
  ok: boolean;
  error?: string;
}

const nativeBridge = runtimeNative as RuntimeNativeModule;

/// Reads the next runtime watch event on an ArkTS taskpool worker.
@Concurrent
function readWatchChannelEvent(handle: bigint): ArrayBuffer {
  const bridge = runtimeNative as RuntimeNativeModule;
  return bridge.nextWatchChannelEvent(handle);
}

/// Dispatches one CoreProxy call on an ArkTS taskpool worker.
@Concurrent
function dispatchRuntimeCoreCall(handle: bigint, request: ArrayBuffer): ArrayBuffer {
  const bridge = runtimeNative as RuntimeNativeModule;
  return bridge.call(handle, request);
}

export class OperitRuntimeChannel {
  private channel: MethodChannel;
  private context: common.UIAbilityContext;
  private runtimeHandle: bigint = BigInt(0);
  private runtimeRoot: string = '';
  private workspaceRoot: string = '';
  private windowStage: window.WindowStage | null = null;
  private watchPumpRunning: boolean = false;
  private musicPlayer: media.AVPlayer | null = null;
  private musicState: string = 'stopped';
  private musicSource: string | null = null;
  private musicSourceType: string | null = null;
  private musicTitle: string | null = null;
  private musicArtist: string | null = null;
  private musicVolume: number = 1.0;
  private musicLoopPlayback: boolean = false;
  private ttsPlayer: media.AVPlayer | null = null;
  private ttsState: string = 'stopped';
  private ttsPath: string = '';
  private bluetoothSessionSeq: number = 1;
  private classicBluetoothSessions: Map<string, ClassicBluetoothSession> = new Map();
  private bleBluetoothSessions: Map<string, BleBluetoothSession> = new Map();
  private hostEventMonitoringInstalled: boolean = false;
  private commonEventSubscriber: commonEventManager.CommonEventSubscriber | null = null;
  private networkConnection: connection.NetConnection | null = null;
  private readonly audioRoutingManager: audio.AudioRoutingManager = audio.getAudioManager().getRoutingManager();
  private readonly bluetoothStateCallback = (state: bluetoothManager.BluetoothState): void => {
    this.emitHostEvent('bluetooth.adapter.powered_changed', {
      'powered': state === bluetoothManager.BluetoothState.STATE_ON ||
        state === bluetoothManager.BluetoothState.STATE_BLE_ON,
      'connected': null
    });
  };
  private readonly bluetoothDeviceFindCallback = (deviceIds: Array<string>): void => {
    deviceIds.forEach((deviceId: string): void => {
      this.emitHostEvent('bluetooth.device.found', {
        'deviceAddress': deviceId,
        'deviceName': bluetoothManager.getRemoteDeviceName(deviceId),
        'connected': null,
        'bonded': null,
        'rssi': null
      });
    });
  };
  private readonly bluetoothBondStateCallback = (state: bluetoothManager.BondStateParam): void => {
    this.emitHostEvent('bluetooth.device.bond_state_changed', {
      'deviceAddress': state.deviceId,
      'deviceName': bluetoothManager.getRemoteDeviceName(state.deviceId),
      'connected': null,
      'bonded': state.state === bluetoothManager.BondState.BOND_STATE_BONDED,
      'rssi': null
    });
  };
  private readonly audioDeviceChangeCallback = (action: audio.DeviceChangeAction): void => {
    action.deviceDescriptors.forEach((device: audio.AudioDeviceDescriptor): void => {
      if (!this.isOhosHeadsetDevice(device.deviceType)) {
        return;
      }
      this.emitHostEvent('system.headset.plug', {
        'connected': action.type === audio.DeviceChangeType.CONNECT,
        'deviceName': device.name,
        'hasMicrophone': this.ohosHeadsetHasMicrophone(device.deviceType)
      });
    });
  };

  /// Creates the OpenHarmony runtime channel.
  constructor(channel: MethodChannel, context: common.UIAbilityContext, windowStage: window.WindowStage | null) {
    this.channel = channel;
    this.context = context;
    this.windowStage = windowStage;
  }

  /// Registers the OpenHarmony runtime MethodChannel handler.
  static register(
    channel: MethodChannel,
    context: common.UIAbilityContext,
    windowStage: window.WindowStage | null
  ): OperitRuntimeChannel {
    const runtimeChannel = new OperitRuntimeChannel(channel, context, windowStage);
    channel.setMethodCallHandler({
      onMethodCall: (call: MethodCall, result: MethodResult): void => {
        runtimeChannel.onMethodCall(call, result);
      }
    });
    return runtimeChannel;
  }

  /// Updates the active WindowStage used by window-bound owner operations.
  setWindowStage(windowStage: window.WindowStage | null): void {
    this.windowStage = windowStage;
  }

  /// Handles one Dart runtime MethodChannel call.
  onMethodCall(call: MethodCall, result: MethodResult): void {
    try {
      switch (call.method) {
        case 'localRuntimeStorageDefaults':
          result.success(this.defaultStoragePaths());
          return;
        case 'localRuntimeStoragePaths':
          result.success(this.storagePaths(call));
          return;
        case 'setLocalRuntimeStorage':
          this.setStorageRoots(call);
          result.success(null);
          return;
        case 'startLocalCoreService':
          this.ensureRuntimeHandle();
          result.success(null);
          return;
        case 'call': {
          const handle = this.ensureRuntimeHandle();
          const request = this.requiredArrayBuffer(call, call.method);
          taskpool.execute(dispatchRuntimeCoreCall, handle, request)
            .then((response: Object): void => {
              result.success(response as ArrayBuffer);
            })
            .catch((error: Object): void => {
              result.error('RUNTIME_BRIDGE_ERROR', `${error}`, null);
            });
          return;
        }
        case 'pushOpen':
          result.success(nativeBridge.pushOpen(this.ensureRuntimeHandle(), this.requiredArrayBuffer(call, call.method)));
          return;
        case 'pushItem':
          result.success(nativeBridge.pushItem(this.ensureRuntimeHandle(), this.requiredArrayBuffer(call, call.method)));
          return;
        case 'pushClose':
          result.success(nativeBridge.pushClose(this.ensureRuntimeHandle(), this.requiredStringArg(call, 'pushClose')));
          return;
        case 'watchSnapshot':
          result.success(nativeBridge.watchSnapshot(this.ensureRuntimeHandle(), this.requiredArrayBuffer(call, call.method)));
          return;
        case 'watchStream':
          result.success(nativeBridge.watchStream(this.ensureRuntimeHandle(), this.requiredArrayBuffer(call, call.method)));
          this.ensureWatchPump();
          return;
        case 'closeWatchStream':
          result.success(nativeBridge.closeWatchStream(this.ensureRuntimeHandle(), this.requiredStringArg(call, 'closeWatchStream')));
          return;
        case 'startWebAccessServer':
          result.success(this.startWebAccessServer(call));
          return;
        case 'discoverDevices':
          result.success(nativeBridge.discoverDevices(this.ensureRuntimeHandle(), `${this.requiredNumberArg(call, 'discoverDevices')}`));
          return;
        case 'stopWebAccessServer':
          result.success(nativeBridge.stopWebAccessServer(this.ensureRuntimeHandle()));
          return;
        case 'remotePairStart':
          result.success(this.remotePairStart(call));
          return;
        case 'remotePairFinish':
          result.success(this.remotePairFinish(call));
          return;
        case 'ownerSystemLanguageCode':
          result.success(this.ownerSystemLanguageCode());
          return;
        case 'ownerSystemCaptureScreenshot':
          this.ownerSystemCaptureScreenshot()
            .then((response: SystemCaptureScreenshotResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_SYSTEM_CAPTURE_SCREENSHOT_ERROR', `${error}`, null);
            });
          return;
        case 'ownerSystemRecognizeText':
          result.error('OWNER_SYSTEM_RECOGNIZE_TEXT_ERROR', 'OpenHarmony OCR is unavailable in the configured SDK', null);
          return;
        case 'ownerSystemOperation':
          this.ownerSystemOperation(call)
            .then((response: SystemOperationResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_SYSTEM_OPERATION_ERROR', `${error}`, null);
            });
          return;
        case 'ownerAudioPlay':
          this.ownerAudioPlay(call)
            .then((response: AudioPlayResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_AUDIO_PLAY_ERROR', `${error}`, null);
            });
          return;
        case 'ownerMusicPlayback':
          this.ownerMusicPlayback(call)
            .then((response: MusicPlaybackResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_MUSIC_PLAYBACK_ERROR', `${error}`, null);
            });
          return;
        case 'ownerBluetooth':
          this.ownerBluetooth(call)
            .then((response: JsonStringPayload) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_BLUETOOTH_ERROR', `${error}`, null);
            });
          return;
        case 'ownerTtsPlayback':
          this.ownerTtsPlayback(call)
            .then((response: TtsPlaybackResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.error('OWNER_TTS_PLAYBACK_ERROR', `${error}`, null);
            });
          return;
        case 'ownerFileOpen':
          this.ownerFileOpen(call)
            .then((response: FileOperationResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.success(this.fileOperationError(error));
            });
          return;
        case 'ownerFileShare':
          this.ownerFileShare(call)
            .then((response: FileOperationResponse) => {
              result.success(response);
            })
            .catch((error: Object) => {
              result.success(this.fileOperationError(error));
            });
          return;
        case 'hostOnboardingPermissionSnapshot':
          result.success(this.hostOnboardingPermissionSnapshot(call));
          return;
        case 'hostOnboardingRequestPermission':
          this.hostOnboardingRequestPermission(call)
            .then(() => {
              result.success(null);
            })
            .catch((error: Object) => {
              result.error('OHOS_ONBOARDING_PERMISSION_ERROR', `${error}`, null);
            });
          return;
        default:
          result.notImplemented();
          return;
      }
    } catch (error) {
      result.error('RUNTIME_BRIDGE_ERROR', `${error}`, null);
    }
  }

  /// Releases the native runtime bridge handle.
  destroy(): void {
    this.watchPumpRunning = false;
    this.releaseHostEventMonitoring();
    if (this.runtimeHandle !== BigInt(0)) {
      nativeBridge.destroy(this.runtimeHandle);
      this.runtimeHandle = BigInt(0);
    }
  }

  /// Returns default OpenHarmony runtime storage paths.
  private defaultStoragePaths(): StoragePaths {
    return {
      runtimeRoot: `${this.context.filesDir}/runtime`,
      workspaceRoot: `${this.context.filesDir}/workspaces`
    };
  }

  /// Returns normalized requested OpenHarmony runtime storage paths.
  private storagePaths(call: MethodCall): StoragePaths {
    return {
      runtimeRoot: this.requiredRoot(call.argument('runtimeRoot') as string | null, 'runtimeRoot'),
      workspaceRoot: this.requiredRoot(call.argument('workspaceRoot') as string | null, 'workspaceRoot')
    };
  }

  /// Stores OpenHarmony runtime storage roots.
  private setStorageRoots(call: MethodCall): void {
    const roots = this.storagePaths(call);
    if (this.runtimeHandle !== BigInt(0) &&
      (this.runtimeRoot !== roots.runtimeRoot || this.workspaceRoot !== roots.workspaceRoot)) {
      throw new Error('Runtime and workspace roots cannot change after runtime creation');
    }
    this.runtimeRoot = roots.runtimeRoot;
    this.workspaceRoot = roots.workspaceRoot;
  }

  /// Returns the active native runtime handle.
  private ensureRuntimeHandle(): bigint {
    if (this.runtimeHandle !== BigInt(0)) {
      return this.runtimeHandle;
    }
    if (this.runtimeRoot.length === 0 || this.workspaceRoot.length === 0) {
      const roots = this.defaultStoragePaths();
      this.runtimeRoot = roots.runtimeRoot;
      this.workspaceRoot = roots.workspaceRoot;
    }
    this.runtimeHandle = nativeBridge.create(
      this.runtimeRoot,
      this.workspaceRoot,
      i18n.System.getSystemLocale()
    );
    this.installHostEventMonitoring();
    return this.runtimeHandle;
  }

  /// Installs OpenHarmony common-event, network, battery, and Bluetooth producers once.
  private installHostEventMonitoring(): void {
    if (this.hostEventMonitoringInstalled) {
      return;
    }
    this.hostEventMonitoringInstalled = true;
    this.installCommonEventSubscriber();
    this.installNetworkMonitoring();
    bluetoothManager.on('stateChange', this.bluetoothStateCallback);
    bluetoothManager.on('bluetoothDeviceFind', this.bluetoothDeviceFindCallback);
    bluetoothManager.on('bondStateChange', this.bluetoothBondStateCallback);
    this.audioRoutingManager.on(
      'deviceChange',
      audio.DeviceFlag.OUTPUT_DEVICES_FLAG,
      this.audioDeviceChangeCallback
    );
  }

  /// Releases every OpenHarmony host event subscription owned by this runtime.
  private releaseHostEventMonitoring(): void {
    if (!this.hostEventMonitoringInstalled) {
      return;
    }
    this.hostEventMonitoringInstalled = false;
    if (this.commonEventSubscriber !== null) {
      commonEventManager.unsubscribe(this.commonEventSubscriber);
      this.commonEventSubscriber = null;
    }
    if (this.networkConnection !== null) {
      this.networkConnection.unregister((error: Object): void => {
        if (error) {
          console.error(`OpenHarmony network monitor unregister failed: ${error}`);
        }
      });
      this.networkConnection = null;
    }
    bluetoothManager.off('stateChange', this.bluetoothStateCallback);
    bluetoothManager.off('bluetoothDeviceFind', this.bluetoothDeviceFindCallback);
    bluetoothManager.off('bondStateChange', this.bluetoothBondStateCallback);
    this.audioRoutingManager.off('deviceChange', this.audioDeviceChangeCallback);
  }

  /// Subscribes to documented OpenHarmony system common events.
  private installCommonEventSubscriber(): void {
    const events: Array<string> = [
      commonEventManager.Support.COMMON_EVENT_BOOT_COMPLETED,
      commonEventManager.Support.COMMON_EVENT_POWER_CONNECTED,
      commonEventManager.Support.COMMON_EVENT_POWER_DISCONNECTED,
      commonEventManager.Support.COMMON_EVENT_BATTERY_LOW,
      commonEventManager.Support.COMMON_EVENT_BATTERY_OKAY,
      commonEventManager.Support.COMMON_EVENT_SCREEN_ON,
      commonEventManager.Support.COMMON_EVENT_SCREEN_OFF,
      commonEventManager.Support.COMMON_EVENT_USER_PRESENT,
      commonEventManager.Support.COMMON_EVENT_TIME_TICK,
      commonEventManager.Support.COMMON_EVENT_TIME_CHANGED,
      commonEventManager.Support.COMMON_EVENT_DATE_CHANGED,
      commonEventManager.Support.COMMON_EVENT_TIMEZONE_CHANGED,
      commonEventManager.Support.COMMON_EVENT_ENTER_FORCE_SLEEP,
      commonEventManager.Support.COMMON_EVENT_EXIT_FORCE_SLEEP,
      commonEventManager.Support.COMMON_EVENT_ENTER_HIBERNATE,
      commonEventManager.Support.COMMON_EVENT_EXIT_HIBERNATE,
      commonEventManager.Support.COMMON_EVENT_SCREEN_LOCKED,
      commonEventManager.Support.COMMON_EVENT_SCREEN_UNLOCKED,
      commonEventManager.Support.COMMON_EVENT_AIRPLANE_MODE_CHANGED,
      commonEventManager.Support.COMMON_EVENT_BLUETOOTH_REMOTEDEVICE_ACL_CONNECTED,
      commonEventManager.Support.COMMON_EVENT_BLUETOOTH_REMOTEDEVICE_ACL_DISCONNECTED
    ];
    commonEventManager.createSubscriber({ 'events': events }).then((subscriber): void => {
      this.commonEventSubscriber = subscriber;
      commonEventManager.subscribe(subscriber, (error, data): void => {
        if (error) {
          console.error(`OpenHarmony common event subscription failed: ${error}`);
          return;
        }
        this.handleCommonEvent(data);
      });
    }).catch((error: Object): void => {
      console.error(`OpenHarmony common event subscriber creation failed: ${error}`);
    });
  }

  /// Maps one documented OpenHarmony common event to its canonical topic data.
  private handleCommonEvent(data: commonEventManager.CommonEventData): void {
    switch (data.event) {
      case commonEventManager.Support.COMMON_EVENT_BOOT_COMPLETED:
        this.emitHostEvent('system.boot.completed', { 'bootCompleted': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_POWER_CONNECTED:
        this.emitHostEvent('system.power.connected', this.ohosPowerData(true));
        return;
      case commonEventManager.Support.COMMON_EVENT_POWER_DISCONNECTED:
        this.emitHostEvent('system.power.disconnected', this.ohosPowerData(false));
        return;
      case commonEventManager.Support.COMMON_EVENT_BATTERY_LOW:
        this.emitHostEvent('system.battery.low', this.ohosBatteryData(true));
        return;
      case commonEventManager.Support.COMMON_EVENT_BATTERY_OKAY:
        this.emitHostEvent('system.battery.okay', this.ohosBatteryData(false));
        return;
      case commonEventManager.Support.COMMON_EVENT_SCREEN_ON:
        this.emitHostEvent('system.screen.on', { 'screenOn': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_SCREEN_OFF:
        this.emitHostEvent('system.screen.off', { 'screenOn': false });
        return;
      case commonEventManager.Support.COMMON_EVENT_USER_PRESENT:
        this.emitHostEvent('system.user.present', { 'present': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_TIME_TICK:
      case commonEventManager.Support.COMMON_EVENT_TIME_CHANGED:
        this.emitHostEvent('system.time.tick', this.ohosTimeData());
        return;
      case commonEventManager.Support.COMMON_EVENT_DATE_CHANGED:
        this.emitHostEvent('system.date.changed', this.ohosTimeData());
        return;
      case commonEventManager.Support.COMMON_EVENT_TIMEZONE_CHANGED:
        this.emitHostEvent('system.timezone.changed', this.ohosTimeData());
        return;
      case commonEventManager.Support.COMMON_EVENT_ENTER_FORCE_SLEEP:
      case commonEventManager.Support.COMMON_EVENT_ENTER_HIBERNATE:
        this.emitHostEvent('system.power.sleep', { 'sleeping': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_EXIT_FORCE_SLEEP:
      case commonEventManager.Support.COMMON_EVENT_EXIT_HIBERNATE:
        this.emitHostEvent('system.power.wake', { 'sleeping': false });
        return;
      case commonEventManager.Support.COMMON_EVENT_SCREEN_LOCKED:
        this.emitHostEvent('system.session.lock', { 'locked': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_SCREEN_UNLOCKED:
        this.emitHostEvent('system.session.unlock', { 'locked': false });
        this.emitHostEvent('system.user.present', { 'present': true });
        return;
      case commonEventManager.Support.COMMON_EVENT_AIRPLANE_MODE_CHANGED:
        this.emitOhosAirplaneMode();
        return;
      case commonEventManager.Support.COMMON_EVENT_BLUETOOTH_REMOTEDEVICE_ACL_CONNECTED:
        this.emitOhosBluetoothConnection(true);
        return;
      case commonEventManager.Support.COMMON_EVENT_BLUETOOTH_REMOTEDEVICE_ACL_DISCONNECTED:
        this.emitOhosBluetoothConnection(false);
        return;
    }
  }

  /// Reads and emits the canonical OpenHarmony airplane-mode state.
  private async emitOhosAirplaneMode(): Promise<void> {
    const value = await settings.getValue(this.context, settings.general.AIRPLANE_MODE_STATUS);
    if (value !== '0' && value !== '1') {
      throw new Error(`OpenHarmony returned an invalid airplane-mode value: ${value}`);
    }
    this.emitHostEvent('system.airplane_mode.changed', { 'enabled': value === '1' });
  }

  /// Emits one canonical OpenHarmony Bluetooth ACL connection transition.
  private emitOhosBluetoothConnection(connected: boolean): void {
    this.emitHostEvent(
      connected ? 'bluetooth.device.connected' : 'bluetooth.device.disconnected',
      {
        'deviceAddress': null,
        'deviceName': null,
        'connected': connected,
        'bonded': null,
        'rssi': null
      }
    );
  }

  /// Reports whether one OpenHarmony audio device is a headset route.
  private isOhosHeadsetDevice(deviceType: audio.DeviceType): boolean {
    switch (deviceType) {
      case audio.DeviceType.WIRED_HEADSET:
      case audio.DeviceType.WIRED_HEADPHONES:
      case audio.DeviceType.BLUETOOTH_SCO:
      case audio.DeviceType.BLUETOOTH_A2DP:
      case audio.DeviceType.USB_HEADSET:
        return true;
      default:
        return false;
    }
  }

  /// Reports microphone capability for one OpenHarmony headset route.
  private ohosHeadsetHasMicrophone(deviceType: audio.DeviceType): boolean {
    switch (deviceType) {
      case audio.DeviceType.WIRED_HEADSET:
      case audio.DeviceType.BLUETOOTH_SCO:
      case audio.DeviceType.USB_HEADSET:
        return true;
      case audio.DeviceType.WIRED_HEADPHONES:
      case audio.DeviceType.BLUETOOTH_A2DP:
        return false;
      default:
        throw new Error(`OpenHarmony audio device is not a headset route: ${deviceType}`);
    }
  }

  /// Builds the shared OpenHarmony external-power structure from batteryInfo.
  private ohosPowerData(connected: boolean): Record<string, Object | null> {
    const source = connected ? this.ohosPowerSource() : 'battery';
    return {
      'connected': connected,
      'source': source,
      'batteryLevel': batteryInfo.batterySOC
    };
  }

  /// Builds the shared OpenHarmony battery threshold structure from batteryInfo.
  private ohosBatteryData(low: boolean): Record<string, Object | null> {
    const charging = batteryInfo.chargingStatus === batteryInfo.BatteryChargeState.ENABLE ||
      batteryInfo.chargingStatus === batteryInfo.BatteryChargeState.FULL;
    return {
      'low': low,
      'level': batteryInfo.batterySOC,
      'charging': charging
    };
  }

  /// Maps OpenHarmony's charger enum to the shared power source vocabulary.
  private ohosPowerSource(): string {
    switch (batteryInfo.pluggedType) {
      case batteryInfo.BatteryPluggedType.AC:
        return 'ac';
      case batteryInfo.BatteryPluggedType.USB:
        return 'usb';
      case batteryInfo.BatteryPluggedType.WIRELESS:
        return 'wireless';
      case batteryInfo.BatteryPluggedType.NONE:
        return 'battery';
      default:
        throw new Error(`OpenHarmony battery plugged type is unsupported: ${batteryInfo.pluggedType}`);
    }
  }

  /// Builds the shared OpenHarmony clock and timezone structure.
  private ohosTimeData(): Record<string, Object | null> {
    return {
      'timestampMillis': Date.now(),
      'timezone': i18n.getTimeZone().getID()
    };
  }

  /// Installs OpenHarmony's default-network capability observer.
  private installNetworkMonitoring(): void {
    const monitor = connection.createNetConnection();
    monitor.on('netCapabilitiesChange', (event): void => {
      this.emitOhosNetwork(event.netHandle, event.netCap);
    });
    monitor.on('netLost', (): void => {
      this.emitHostEvent('system.network.changed', {
        'connected': false,
        'networkType': 'none',
        'metered': null,
        'interfaceName': null
      });
    });
    monitor.on('netUnavailable', (): void => {
      this.emitHostEvent('system.network.changed', {
        'connected': false,
        'networkType': 'none',
        'metered': null,
        'interfaceName': null
      });
    });
    monitor.register((error: Object): void => {
      if (error) {
        console.error(`OpenHarmony network monitor registration failed: ${error}`);
      }
    });
    this.networkConnection = monitor;
  }

  /// Resolves OpenHarmony network metadata and emits the canonical network structure.
  private emitOhosNetwork(
    netHandle: connection.NetHandle,
    capabilities: connection.NetCapabilities
  ): void {
    connection.getConnectionProperties(netHandle).then((properties: connection.ConnectionProperties): void => {
      const caps = capabilities.networkCap ?? [];
      const connected = caps.includes(connection.NetCap.NET_CAPABILITY_INTERNET);
      const metered = !caps.includes(connection.NetCap.NET_CAPABILITY_NOT_METERED);
      this.emitHostEvent('system.network.changed', {
        'connected': connected,
        'networkType': connected ? this.ohosNetworkType(capabilities.bearerTypes) : 'none',
        'metered': metered,
        'interfaceName': properties.interfaceName
      });
    }).catch((error: Object): void => {
      console.error(`OpenHarmony network metadata query failed: ${error}`);
    });
  }

  /// Maps OpenHarmony bearer types to the shared network vocabulary.
  private ohosNetworkType(bearerTypes: Array<connection.NetBearType>): string {
    if (bearerTypes.includes(connection.NetBearType.BEARER_WIFI)) {
      return 'wifi';
    }
    if (bearerTypes.includes(connection.NetBearType.BEARER_CELLULAR)) {
      return 'cellular';
    }
    if (bearerTypes.includes(connection.NetBearType.BEARER_ETHERNET)) {
      return 'ethernet';
    }
    return 'other';
  }

  /// Serializes and forwards one canonical OpenHarmony event into Core.
  private emitHostEvent(topic: string, data: Record<string, Object | null>): void {
    if (this.runtimeHandle === BigInt(0)) {
      return;
    }
    const response = JSON.parse(nativeBridge.emitRuntimeEvent(
      this.runtimeHandle,
      JSON.stringify({
        'domain': 'host',
        'source': 'ohos.system',
        'topic': topic,
        'platform': 'ohos',
        'payload': data,
        'occurredAtMillis': Date.now()
      })
    )) as RuntimeEventDeliveryResponse;
    if (!response.ok) {
      throw new Error(response.error ?? 'OpenHarmony host event delivery failed');
    }
  }

  /// Starts the watch-channel event pump.
  private ensureWatchPump(): void {
    if (this.watchPumpRunning) {
      return;
    }
    this.watchPumpRunning = true;
    this.pumpWatchChannel();
  }

  /// Pumps watch-channel events from Rust into Dart.
  private pumpWatchChannel(): void {
    taskpool.execute(readWatchChannelEvent, this.ensureRuntimeHandle())
      .then((frame: Object) => {
        if (!this.watchPumpRunning) {
          return;
        }
        this.channel.invokeMethod('watchChannelEvent', frame as ArrayBuffer);
        this.pumpWatchChannel();
      })
      .catch(() => {
        this.watchPumpRunning = false;
      });
  }

  /// Starts the Web Access server through the Rust bridge.
  private startWebAccessServer(call: MethodCall): string {
    const pairingCodePath = `${this.context.filesDir}/runtime/link/pairing_codes.json`;
    return nativeBridge.startWebAccessServer(
      this.ensureRuntimeHandle(),
      this.mapString(call, 'bindHost'),
      `${this.mapNumber(call, 'requestedPort')}`,
      this.mapString(call, 'authToken'),
      this.mapString(call, 'serverName'),
      `${this.mapBool(call, 'advertise')}`,
      pairingCodePath
    );
  }

  /// Starts one remote pairing request through the Rust bridge.
  private remotePairStart(call: MethodCall): string {
    return nativeBridge.remotePairStart(
      this.ensureRuntimeHandle(),
      this.mapString(call, 'baseUrl'),
      this.mapString(call, 'tokenHash'),
      this.mapString(call, 'clientDeviceInfo')
    );
  }

  /// Finishes one remote pairing request through the Rust bridge.
  private remotePairFinish(call: MethodCall): string {
    return nativeBridge.remotePairFinish(
      this.ensureRuntimeHandle(),
      this.mapString(call, 'pairingId'),
      this.mapString(call, 'pairingCode')
    );
  }

  /// Returns the current OpenHarmony system language code.
  private ownerSystemLanguageCode(): JsonStringPayload {
    return {
      languageCode: i18n.System.getSystemLocale()
    };
  }

  /// Captures the active OpenHarmony window into a PNG file.
  private async ownerSystemCaptureScreenshot(): Promise<SystemCaptureScreenshotResponse> {
    if (this.windowStage === null) {
      throw new Error('OpenHarmony WindowStage is not available for screenshot capture');
    }
    const mainWindow = await this.windowStage.getMainWindow();
    const pixelMap = await mainWindow.snapshot();
    const packer = image.createImagePacker();
    const png = await packer.packing(pixelMap, {
      format: 'image/png',
      quality: 100,
      bufferSize: 20 * 1024 * 1024
    });
    await packer.release();
    await pixelMap.release();
    const path = `${this.context.filesDir}/screenshot_${Date.now()}.png`;
    const file = fs.openSync(path, fs.OpenMode.CREATE | fs.OpenMode.TRUNC | fs.OpenMode.WRITE_ONLY);
    fs.writeSync(file.fd, png);
    fs.closeSync(file);
    return {
      path: path
    };
  }

  /// Handles one generic OpenHarmony system operation command.
  private async ownerSystemOperation(call: MethodCall): Promise<SystemOperationResponse> {
    const operation = this.mapString(call, 'operation');
    const paramsJson = this.mapString(call, 'paramsJson');
    const params = JSON.parse(paramsJson) as JsonPayload;
    let value: Object;
    switch (operation) {
      case 'toast':
        value = this.systemToast(params);
        break;
      case 'send_notification':
        value = await this.systemSendNotification(params);
        break;
      case 'modify_system_setting':
        value = this.systemModifySetting(params);
        break;
      case 'get_system_setting':
        value = this.systemGetSetting(params);
        break;
      case 'install_app':
        value = await this.systemInstallApp(params);
        break;
      case 'uninstall_app':
        value = await this.systemUninstallApp(params);
        break;
      case 'list_installed_apps':
        value = await this.systemListInstalledApps(params);
        break;
      case 'start_app':
        value = await this.systemStartApp(params);
        break;
      case 'stop_app':
        value = this.systemStopApp(params);
        break;
      case 'get_notifications':
        value = await this.systemGetNotifications(params);
        break;
      case 'get_app_usage_time':
        value = await this.systemGetAppUsageTime(params);
        break;
      case 'get_device_location':
        value = await this.systemGetDeviceLocation(params);
        break;
      case 'device_info':
        value = this.systemDeviceInfo();
        break;
      default:
        throw new Error(`unsupported OpenHarmony system operation: ${operation}`);
    }
    return {
      resultJson: JSON.stringify(value)
    };
  }

  /// Shows one OpenHarmony toast.
  private systemToast(params: JsonPayload): JsonPayload {
    const message = this.objectString(params, 'message');
    promptAction.showToast({ message: message });
    return {
      success: true
    };
  }

  /// Publishes one OpenHarmony notification.
  private async systemSendNotification(params: JsonPayload): Promise<JsonPayload> {
    const title = this.objectString(params, 'title');
    const message = this.objectString(params, 'message');
    await notificationManager.requestEnableNotification();
    await notificationManager.publish({
      id: Date.now() % 2147483647,
      slotType: notificationManager.SlotType.OTHER_TYPES,
      content: {
        contentType: notificationManager.ContentType.NOTIFICATION_CONTENT_BASIC_TEXT,
        normal: {
          title: title,
          text: message
        }
      }
    } as notificationManager.NotificationRequest);
    return {
      success: true
    };
  }

  /// Writes one OpenHarmony settings value.
  private systemModifySetting(params: JsonPayload): JsonPayload {
    const namespace = this.objectString(params, 'namespace');
    const setting = this.objectString(params, 'setting');
    const value = this.objectString(params, 'value');
    const success = settings.setValueSync(this.context, setting, value);
    if (!success) {
      throw new Error(`OpenHarmony setting write failed: ${setting}`);
    }
    return {
      namespace: namespace,
      setting: setting,
      value: value
    };
  }

  /// Reads one OpenHarmony settings value.
  private systemGetSetting(params: JsonPayload): JsonPayload {
    const namespace = this.objectString(params, 'namespace');
    const setting = this.objectString(params, 'setting');
    const value = settings.getValueSync(this.context, setting, '');
    return {
      namespace: namespace,
      setting: setting,
      value: value
    };
  }

  /// Requests package installation through the OpenHarmony ability resolver.
  private async systemInstallApp(params: JsonPayload): Promise<JsonPayload> {
    const path = this.objectString(params, 'path');
    this.requireAccessibleFile(path);
    await this.context.startAbility({
      action: wantConstant.Action.ACTION_VIEW_DATA,
      uri: fileUri.getUriFromPath(path),
      type: 'application/vnd.ohos.hap',
      flags: wantConstant.Flags.FLAG_AUTH_READ_URI_PERMISSION
    } as Want);
    return {
      operationType: 'install_app',
      packageName: path,
      success: true,
      details: 'ohos install ability requested'
    };
  }

  /// Opens the OpenHarmony application details screen for uninstall management.
  private async systemUninstallApp(params: JsonPayload): Promise<JsonPayload> {
    const packageName = this.objectString(params, 'packageName');
    await this.context.startAbility({
      action: wantConstant.Action.ACTION_APPLICATION_DETAILS_SETTINGS,
      parameters: {
        bundleName: packageName
      }
    } as Want);
    return {
      operationType: 'uninstall_app',
      packageName: packageName,
      success: true,
      details: 'ohos application details ability requested'
    };
  }

  /// Lists OpenHarmony installed applications.
  private async systemListInstalledApps(params: JsonPayload): Promise<JsonPayload> {
    const includeSystemApps = this.objectBool(params, 'includeSystemApps');
    const applications = await bundle.getAllApplicationInfo(0) as Array<OhosApplicationInfo>;
    const packages: Array<string> = [];
    applications.forEach((entry: OhosApplicationInfo) => {
      if (includeSystemApps || !entry.systemApp) {
        packages.push(entry.name);
      }
    });
    return {
      includesSystemApps: includeSystemApps,
      packages: packages
    };
  }

  /// Starts one OpenHarmony application by bundle name.
  private async systemStartApp(params: JsonPayload): Promise<JsonPayload> {
    const packageName = this.objectString(params, 'packageName');
    const want = await bundle.getLaunchWantForBundle(packageName);
    await this.context.startAbility(want);
    return {
      operationType: 'start_app',
      packageName: packageName,
      success: true,
      details: 'ohos launch want started'
    };
  }

  /// Reports unavailable arbitrary application stop control.
  private systemStopApp(params: JsonPayload): JsonPayload {
    const packageName = this.objectString(params, 'packageName');
    throw new Error(`OpenHarmony public appManager API does not expose stop for arbitrary bundle: ${packageName}`);
  }

  /// Reads active OpenHarmony notifications visible to this application.
  private async systemGetNotifications(params: JsonPayload): Promise<JsonPayload> {
    const limit = this.objectNumber(params, 'limit');
    const includeOngoing = this.objectBool(params, 'includeOngoing');
    const notifications = await notificationManager.getActiveNotifications();
    const entries: Array<JsonPayload> = [];
    notifications.forEach((entry: notificationManager.NotificationRequest) => {
      if (!includeOngoing && entry.isOngoing === true) {
        return;
      }
      if (entries.length >= limit) {
        return;
      }
      const content = entry.content.normal;
      entries.push({
        packageName: entry.creatorBundleName ?? this.context.abilityInfo.bundleName,
        text: content === undefined ? '' : `${content.title}: ${content.text}`,
        timestamp: entry.deliveryTime ?? Date.now()
      });
    });
    return {
      notifications: entries,
      timestamp: Date.now()
    };
  }

  /// Reads usage data exposed by the current OpenHarmony bundle-state API.
  private async systemGetAppUsageTime(params: JsonPayload): Promise<JsonPayload> {
    const packageName = this.objectString(params, 'packageName');
    const sinceHours = this.objectNumber(params, 'sinceHours');
    const limit = this.objectNumber(params, 'limit');
    const includeSystemApps = this.objectBool(params, 'includeSystemApps');
    const endTime = Date.now();
    const startTime = endTime - Math.max(0, sinceHours) * 60 * 60 * 1000;
    const events = await bundleState.queryCurrentBundleActiveStates(startTime, endTime);
    const selfPackage = this.context.abilityInfo.bundleName;
    if (packageName.length > 0 && packageName !== selfPackage) {
      throw new Error(`OpenHarmony API9 exposes current-bundle usage only: ${packageName}`);
    }
    const usedEvents = events.slice(0, Math.max(0, limit));
    return {
      startTime: startTime,
      endTime: endTime,
      sinceHours: sinceHours,
      requestedPackageName: packageName.length === 0 ? null : packageName,
      includesSystemApps: includeSystemApps,
      totalEntries: usedEvents.length,
      entries: [{
        packageName: selfPackage,
        appName: selfPackage,
        totalForegroundTimeMs: usedEvents.length,
        lastTimeUsed: endTime,
        isSystemApp: false
      }]
    };
  }

  /// Reads the current OpenHarmony device location.
  private async systemGetDeviceLocation(params: JsonPayload): Promise<JsonPayload> {
    const timeout = this.objectNumber(params, 'timeout');
    const highAccuracy = this.objectBool(params, 'highAccuracy');
    const includeAddress = this.objectBool(params, 'includeAddress');
    const location = await geoLocationManager.getCurrentLocation({
      priority: highAccuracy ? geoLocationManager.LocationRequestPriority.ACCURACY : geoLocationManager.LocationRequestPriority.LOW_POWER,
      scenario: highAccuracy ? geoLocationManager.LocationRequestScenario.NAVIGATION : geoLocationManager.LocationRequestScenario.DAILY_LIFE_SERVICE,
      timeoutMs: timeout
    });
    let address = '';
    let city = '';
    let province = '';
    let country = '';
    if (includeAddress) {
      const addresses = await geoLocationManager.getAddressesFromLocation({
        latitude: location.latitude,
        longitude: location.longitude,
        maxItems: 1
      });
      if (addresses.length > 0) {
        const first = addresses[0];
        address = `${first.countryName ?? ''}${first.administrativeArea ?? ''}${first.locality ?? ''}${first.roadName ?? ''}`;
        city = first.locality ?? '';
        province = first.administrativeArea ?? '';
        country = first.countryName ?? '';
      }
    }
    return {
      latitude: location.latitude,
      longitude: location.longitude,
      accuracy: location.accuracy,
      provider: 'ohos.geoLocationManager',
      timestamp: location.timeStamp,
      rawData: JSON.stringify(location),
      address: address,
      city: city,
      province: province,
      country: country
    };
  }

  /// Reads OpenHarmony device information.
  private systemDeviceInfo(): JsonPayload {
    const currentDisplay = display.getDefaultDisplaySync();
    const charging = batteryInfo.chargingStatus === batteryInfo.BatteryChargeState.ENABLE ||
      batteryInfo.chargingStatus === batteryInfo.BatteryChargeState.FULL;
    return {
      deviceId: deviceInfo.udid,
      model: deviceInfo.productModel,
      manufacturer: deviceInfo.manufacture,
      androidVersion: deviceInfo.osFullName,
      sdkVersion: deviceInfo.sdkApiVersion,
      screenResolution: `${currentDisplay.width}x${currentDisplay.height}`,
      screenDensity: currentDisplay.densityPixels,
      totalMemory: '',
      availableMemory: '',
      totalStorage: '',
      availableStorage: '',
      batteryLevel: batteryInfo.batterySOC,
      batteryCharging: charging,
      cpuInfo: deviceInfo.abiList,
      networkType: '',
      additionalInfo: {
        deviceType: deviceInfo.deviceType,
        brand: deviceInfo.brand,
        marketName: deviceInfo.marketName,
        displayVersion: deviceInfo.displayVersion,
        buildType: deviceInfo.buildType,
        displayName: currentDisplay.name
      }
    };
  }

  /// Plays one local audio file with OpenHarmony AVPlayer.
  private async ownerAudioPlay(call: MethodCall): Promise<AudioPlayResponse> {
    const path = this.mapString(call, 'path');
    this.requireAccessibleFile(path);
    const player = await media.createAVPlayer();
    player.on('stateChange', (state: media.AVPlayerState) => {
      if (state === 'completed' || state === 'stopped' || state === 'error') {
        player.release();
      }
    });
    player.url = fileUri.getUriFromPath(path);
    await player.prepare();
    await player.play();
    return {
      path: path,
      started: true,
      details: 'ohos_av_player_started'
    };
  }

  /// Handles one OpenHarmony TTS playback command.
  private async ownerTtsPlayback(call: MethodCall): Promise<TtsPlaybackResponse> {
    const command = this.mapString(call, 'command');
    switch (command) {
      case 'play':
        return this.ttsPlay(call);
      case 'pause':
        return this.ttsPause();
      case 'resume':
        return this.ttsResume();
      case 'stop':
        return this.ttsStop();
      case 'state':
        return this.ttsStatus('ohos tts player status');
      default:
        throw new Error(`unsupported TTS playback command: ${command}`);
    }
  }

  /// Starts one synthesized WAV with OpenHarmony AVPlayer.
  private async ttsPlay(call: MethodCall): Promise<TtsPlaybackResponse> {
    const path = this.mapString(call, 'audioPath');
    this.requireAccessibleFile(path);
    await this.releaseTtsPlayer();
    const player = await media.createAVPlayer();
    this.ttsPlayer = player;
    this.ttsPath = path;
    this.ttsState = 'preparing';
    player.on('stateChange', (state: media.AVPlayerState) => {
      if (this.ttsPlayer !== player) {
        return;
      }
      this.ttsState = state;
      if (state === 'completed' || state === 'stopped' || state === 'error') {
        this.ttsPlayer = null;
        player.release();
      }
    });
    try {
      player.url = fileUri.getUriFromPath(path);
      await player.prepare();
      await player.play();
      this.ttsState = 'playing';
      return this.ttsStatus('ohos tts playback started');
    } catch (error) {
      if (this.ttsPlayer === player) {
        this.ttsPlayer = null;
        await player.release();
      }
      this.ttsState = 'error';
      throw new Error(`${error}`);
    }
  }

  /// Pauses the active OpenHarmony TTS player.
  private async ttsPause(): Promise<TtsPlaybackResponse> {
    const player = this.ttsPlayer;
    if (player === null || this.ttsState === 'completed' || this.ttsState === 'stopped') {
      return this.ttsStatus('ohos tts playback is not active');
    }
    if (this.ttsState === 'paused') {
      return this.ttsStatus('ohos tts playback is already paused');
    }
    await player.pause();
    this.ttsState = 'paused';
    return this.ttsStatus('ohos tts playback paused');
  }

  /// Resumes the paused OpenHarmony TTS player.
  private async ttsResume(): Promise<TtsPlaybackResponse> {
    const player = this.ttsPlayer;
    if (player === null || this.ttsState !== 'paused') {
      return this.ttsStatus('ohos tts playback is not paused');
    }
    await player.play();
    this.ttsState = 'playing';
    return this.ttsStatus('ohos tts playback resumed');
  }

  /// Stops and releases the active OpenHarmony TTS player.
  private async ttsStop(): Promise<TtsPlaybackResponse> {
    await this.releaseTtsPlayer();
    this.ttsState = 'stopped';
    return this.ttsStatus('ohos tts playback stopped');
  }

  /// Builds the authoritative OpenHarmony TTS playback status.
  private ttsStatus(details: string): TtsPlaybackResponse {
    const active = this.ttsPlayer !== null &&
      (this.ttsState === 'playing' || this.ttsState === 'paused' || this.ttsState === 'preparing');
    return {
      path: this.ttsPath,
      active: active,
      paused: active && this.ttsState === 'paused',
      details: details
    };
  }

  /// Releases the owned OpenHarmony TTS player.
  private async releaseTtsPlayer(): Promise<void> {
    const player = this.ttsPlayer;
    this.ttsPlayer = null;
    if (player !== null) {
      await player.release();
    }
  }

  /// Handles one OpenHarmony music playback command.
  private async ownerMusicPlayback(call: MethodCall): Promise<MusicPlaybackResponse> {
    const command = this.mapString(call, 'command');
    switch (command) {
      case 'play':
        return this.musicPlay(call);
      case 'pause':
        return this.musicPause();
      case 'resume':
        return this.musicResume();
      case 'stop':
        return this.musicStop();
      case 'seek':
        return this.musicSeek(this.mapNumber(call, 'positionMs'));
      case 'set_volume':
        return this.musicSetVolume(this.mapNumber(call, 'volume'));
      case 'status':
        return this.musicStatus('ohos music player status');
      default:
        throw new Error(`unsupported music command: ${command}`);
    }
  }

  /// Starts OpenHarmony music playback with AVPlayer.
  private async musicPlay(call: MethodCall): Promise<MusicPlaybackResponse> {
    const source = this.mapString(call, 'source');
    const sourceType = this.mapString(call, 'sourceType');
    const player = await media.createAVPlayer();
    const volume = this.normalizedVolume(this.mapNumber(call, 'volume'));
    const loopPlayback = this.mapBool(call, 'loopPlayback');
    switch (sourceType) {
      case 'path':
        this.requireAccessibleFile(source);
        player.url = fileUri.getUriFromPath(source);
        break;
      case 'uri':
      case 'url':
        player.url = source;
        break;
      default:
        throw new Error(`unsupported music sourceType: ${sourceType}`);
    }
    player.loop = loopPlayback;
    player.setVolume(volume);
    player.on('stateChange', (state: media.AVPlayerState) => {
      if (state === 'completed') {
        this.musicState = 'completed';
      }
      if (state === 'error') {
        this.musicState = 'error';
      }
    });
    await player.prepare();
    const startPositionMs = this.mapNumber(call, 'positionMs');
    if (startPositionMs > 0) {
      player.seek(startPositionMs);
    }
    await this.releaseMusicPlayer();
    this.musicPlayer = player;
    this.musicState = 'playing';
    this.musicSource = source;
    this.musicSourceType = sourceType;
    this.musicTitle = this.mapOptionalString(call, 'title');
    this.musicArtist = this.mapOptionalString(call, 'artist');
    this.musicVolume = volume;
    this.musicLoopPlayback = loopPlayback;
    await player.play();
    return this.musicStatus('ohos music playback started');
  }

  /// Pauses OpenHarmony music playback.
  private async musicPause(): Promise<MusicPlaybackResponse> {
    const player = this.requireMusicPlayer();
    await player.pause();
    this.musicState = 'paused';
    return this.musicStatus('ohos music playback paused');
  }

  /// Resumes OpenHarmony music playback.
  private async musicResume(): Promise<MusicPlaybackResponse> {
    const player = this.requireMusicPlayer();
    await player.play();
    this.musicState = 'playing';
    return this.musicStatus('ohos music playback resumed');
  }

  /// Stops OpenHarmony music playback.
  private async musicStop(): Promise<MusicPlaybackResponse> {
    await this.releaseMusicPlayer();
    this.musicState = 'stopped';
    return this.musicStatus('ohos music playback stopped');
  }

  /// Seeks OpenHarmony music playback.
  private async musicSeek(positionMs: number): Promise<MusicPlaybackResponse> {
    const player = this.requireMusicPlayer();
    player.seek(Math.max(0, positionMs));
    return this.musicStatus('ohos music playback seeked');
  }

  /// Updates OpenHarmony music playback volume.
  private async musicSetVolume(volume: number): Promise<MusicPlaybackResponse> {
    const player = this.requireMusicPlayer();
    this.musicVolume = this.normalizedVolume(volume);
    player.setVolume(this.musicVolume);
    return this.musicStatus('ohos music playback volume changed');
  }

  /// Builds the current OpenHarmony music playback status.
  private musicStatus(message: string): MusicPlaybackResponse {
    const player = this.musicPlayer;
    const state = player === null ? this.musicState : player.state;
    return {
      state: state,
      source: this.musicSource,
      sourceType: this.musicSourceType,
      title: this.musicTitle,
      artist: this.musicArtist,
      durationMs: player === null || player.duration < 0 ? null : player.duration,
      positionMs: player === null ? 0 : player.currentTime,
      bufferedPositionMs: player === null ? 0 : player.currentTime,
      volume: this.musicVolume,
      loopPlayback: this.musicLoopPlayback,
      message: message
    };
  }

  /// Returns the active OpenHarmony music player.
  private requireMusicPlayer(): media.AVPlayer {
    if (this.musicPlayer === null) {
      throw new Error('OpenHarmony music player is not initialized');
    }
    return this.musicPlayer;
  }

  /// Releases the active OpenHarmony music player.
  private async releaseMusicPlayer(): Promise<void> {
    const player = this.musicPlayer;
    this.musicPlayer = null;
    if (player !== null) {
      await player.release();
    }
  }

  /// Normalizes one audio volume value.
  private normalizedVolume(volume: number): number {
    return Math.min(1.0, Math.max(0.0, volume));
  }

  /// Returns the current OpenHarmony onboarding permission statuses.
  private hostOnboardingPermissionSnapshot(call: MethodCall): OnboardingSnapshot {
    this.requireOhosOnboardingHost(call);
    return {
      'ohos.location': this.onboardingRequirement('ohos.location', this.hasAllPermissions(this.locationPermissions())),
      'ohos.bluetooth': this.onboardingRequirement('ohos.bluetooth', this.hasAllPermissions(this.bluetoothPermissions()))
    };
  }

  /// Requests the OpenHarmony permissions attached to one onboarding requirement.
  private async hostOnboardingRequestPermission(call: MethodCall): Promise<void> {
    this.requireOhosOnboardingHost(call);
    const requirementId = this.mapString(call, 'requirementId');
    const permissions = this.onboardingRequirementPermissions(requirementId);
    const atManager = abilityAccessCtrl.createAtManager();
    await atManager.requestPermissionsFromUser(this.context, permissions);
  }

  /// Validates that the onboarding call targets the OpenHarmony host.
  private requireOhosOnboardingHost(call: MethodCall): void {
    const hostId = this.mapString(call, 'hostId');
    if (hostId !== 'ohos') {
      throw new Error('Invalid onboarding host');
    }
  }

  /// Maps one OpenHarmony onboarding requirement to its runtime permissions.
  private onboardingRequirementPermissions(requirementId: string): Array<Permissions> {
    switch (requirementId) {
      case 'ohos.location':
        return this.locationPermissions();
      case 'ohos.bluetooth':
        return this.bluetoothPermissions();
      default:
        throw new Error(`Invalid OpenHarmony onboarding requirement: ${requirementId}`);
    }
  }

  /// Builds one onboarding requirement status payload for Dart.
  private onboardingRequirement(id: string, satisfied: boolean): OnboardingRequirementStatus {
    return {
      id: id,
      status: satisfied ? 'Satisfied' : 'Missing'
    };
  }

  /// Checks that every listed OpenHarmony permission is granted.
  private hasAllPermissions(permissions: Array<Permissions>): boolean {
    const atManager = abilityAccessCtrl.createAtManager();
    const tokenId = this.context.applicationInfo.accessTokenId;
    return permissions.every((permission: Permissions): boolean => {
      return atManager.verifyAccessTokenSync(tokenId, permission) === abilityAccessCtrl.GrantStatus.PERMISSION_GRANTED;
    });
  }

  /// Returns OpenHarmony permissions required for location operations.
  private locationPermissions(): Array<Permissions> {
    return [
      'ohos.permission.LOCATION',
      'ohos.permission.APPROXIMATELY_LOCATION'
    ];
  }

  /// Returns OpenHarmony permissions required for Bluetooth operations.
  private bluetoothPermissions(): Array<Permissions> {
    return [
      'ohos.permission.USE_BLUETOOTH',
      'ohos.permission.DISCOVER_BLUETOOTH',
      'ohos.permission.MANAGE_BLUETOOTH'
    ];
  }

  /// Handles one OpenHarmony Bluetooth command.
  private async ownerBluetooth(call: MethodCall): Promise<JsonStringPayload> {
    const command = this.mapString(call, 'command');
    const paramsJson = this.mapString(call, 'paramsJson');
    const params = JSON.parse(paramsJson) as JsonPayload;
    let value: Object;
    switch (command) {
      case 'request_permission':
        value = 'ohos_bluetooth_permissions_declared';
        break;
      case 'state':
        value = this.bluetoothState();
        break;
      case 'request_enable':
        bluetoothManager.enableBluetooth();
        value = 'ohos_bluetooth_enable_requested';
        break;
      case 'bonded_devices':
        value = this.bluetoothBondedDevices();
        break;
      case 'scan':
        value = await this.bluetoothScan(params);
        break;
      case 'classic_connect':
        value = await this.bluetoothClassicConnect(params);
        break;
      case 'classic_listen':
        value = await this.bluetoothClassicListen(params);
        break;
      case 'classic_accept':
        value = await this.bluetoothClassicAccept(params);
        break;
      case 'classic_send':
        value = this.bluetoothClassicSend(params);
        break;
      case 'classic_read':
        value = await this.bluetoothClassicRead(params);
        break;
      case 'classic_send_and_read':
        value = await this.bluetoothClassicSendAndRead(params);
        break;
      case 'close':
        value = this.bluetoothClose(params);
        break;
      case 'ble_connect':
        value = await this.bluetoothBleConnect(params);
        break;
      case 'ble_discover_services':
        value = await this.bluetoothBleDiscoverServices(params);
        break;
      case 'ble_read_characteristic':
        value = await this.bluetoothBleReadCharacteristic(params);
        break;
      case 'ble_write_characteristic':
        value = this.bluetoothBleWriteCharacteristic(params);
        break;
      case 'ble_write_and_read_characteristic':
        value = await this.bluetoothBleWriteAndReadCharacteristic(params);
        break;
      case 'ble_subscribe_characteristic':
        value = this.bluetoothBleSubscribeCharacteristic(params);
        break;
      case 'ble_read_notifications':
        value = this.bluetoothBleReadNotifications(params);
        break;
      default:
        throw new Error(`unsupported OpenHarmony Bluetooth command: ${command}`);
    }
    return {
      resultJson: JSON.stringify(value)
    };
  }

  /// Reads the current OpenHarmony Bluetooth state.
  private bluetoothState(): JsonPayload {
    const state = bluetoothManager.getState();
    const enabled = state === bluetoothManager.BluetoothState.STATE_ON ||
      state === bluetoothManager.BluetoothState.STATE_BLE_ON;
    return {
      supported: true,
      enabled: enabled,
      state: `${state}`
    };
  }

  /// Lists paired OpenHarmony Bluetooth devices.
  private bluetoothBondedDevices(): JsonPayload {
    const devices = bluetoothManager.getPairedDevices().map((address: string): JsonPayload => {
      return {
        name: bluetoothManager.getRemoteDeviceName(address),
        address: address,
        type: 'classic',
        bondState: 'bonded'
      };
    });
    return {
      devices: devices
    };
  }

  /// Scans OpenHarmony Bluetooth devices for the requested duration.
  private bluetoothScan(params: JsonPayload): Promise<JsonPayload> {
    const durationMs = this.objectNumber(params, 'durationMs');
    const includeBle = this.objectBool(params, 'includeBle');
    const devices = new Map<string, JsonPayload>();
    const classicCallback = (addresses: Array<string>): void => {
      addresses.forEach((address: string) => {
        devices.set(address, {
          name: bluetoothManager.getRemoteDeviceName(address),
          address: address,
          type: 'classic',
          bondState: 'unknown',
          source: 'classic',
          rssi: null
        });
      });
    };
    const bleCallback = (results: Array<bluetoothManager.ScanResult>): void => {
      results.forEach((entry: bluetoothManager.ScanResult) => {
        devices.set(entry.deviceId, {
          name: null,
          address: entry.deviceId,
          type: 'ble',
          bondState: 'unknown',
          source: 'ble',
          rssi: entry.rssi
        });
      });
    };
    bluetoothManager.on('bluetoothDeviceFind', classicCallback);
    bluetoothManager.startBluetoothDiscovery();
    if (includeBle) {
      bluetoothManager.BLE.on('BLEDeviceFind', bleCallback);
      bluetoothManager.BLE.startBLEScan([]);
    }
    return new Promise((resolve: (value: JsonPayload) => void) => {
      setTimeout(() => {
        bluetoothManager.stopBluetoothDiscovery();
        bluetoothManager.off('bluetoothDeviceFind', classicCallback);
        if (includeBle) {
          bluetoothManager.BLE.stopBLEScan();
          bluetoothManager.BLE.off('BLEDeviceFind', bleCallback);
        }
        resolve({
          devices: Array.from(devices.values()),
          durationMs: durationMs,
          includesBle: includeBle
        });
      }, durationMs);
    });
  }

  /// Connects to one classic Bluetooth SPP device.
  private async bluetoothClassicConnect(params: JsonPayload): Promise<JsonPayload> {
    const address = this.objectString(params, 'address');
    const uuid = this.objectString(params, 'uuid');
    const socket = await this.sppConnect(address, uuid);
    const sessionId = this.nextBluetoothSessionId('classic');
    this.registerClassicClientSession(sessionId, address, 'classic_client', socket);
    return {
      sessionId: sessionId,
      address: address,
      mode: 'classic_client'
    };
  }

  /// Starts one classic Bluetooth SPP listening socket.
  private async bluetoothClassicListen(params: JsonPayload): Promise<JsonPayload> {
    const name = this.objectString(params, 'name');
    const uuid = this.objectString(params, 'uuid');
    const serverSocket = await this.sppListen(name, uuid);
    const sessionId = this.nextBluetoothSessionId('classic-listener');
    this.classicBluetoothSessions.set(sessionId, {
      address: name,
      mode: 'classic_listener',
      clientSocket: null,
      serverSocket: serverSocket,
      readQueue: [],
      readCallback: null
    });
    return {
      sessionId: sessionId,
      address: name,
      mode: 'classic_listener'
    };
  }

  /// Accepts one incoming classic Bluetooth SPP client.
  private async bluetoothClassicAccept(params: JsonPayload): Promise<JsonPayload> {
    const listenerSessionId = this.objectString(params, 'listenerSessionId');
    const timeoutMs = this.objectNumber(params, 'timeoutMs');
    const listener = this.requireClassicSession(listenerSessionId);
    if (listener.serverSocket === null) {
      throw new Error(`OpenHarmony Bluetooth session is not a listener: ${listenerSessionId}`);
    }
    const socket = await this.sppAccept(listener.serverSocket, timeoutMs);
    const sessionId = this.nextBluetoothSessionId('classic');
    this.registerClassicClientSession(sessionId, listener.address, 'classic_server_client', socket);
    return {
      sessionId: sessionId,
      address: listener.address,
      mode: 'classic_server_client'
    };
  }

  /// Writes bytes to one classic Bluetooth SPP session.
  private bluetoothClassicSend(params: JsonPayload): JsonPayload {
    const sessionId = this.objectString(params, 'sessionId');
    const session = this.requireClassicClientSession(sessionId);
    const payload = this.bluetoothPayload(params);
    bluetoothManager.sppWrite(session.clientSocket as number, payload);
    return {
      sessionId: sessionId,
      bytesWritten: payload.byteLength
    };
  }

  /// Reads bytes from one classic Bluetooth SPP session.
  private async bluetoothClassicRead(params: JsonPayload): Promise<JsonPayload> {
    const sessionId = this.objectString(params, 'sessionId');
    const maxBytes = this.objectNumber(params, 'maxBytes');
    const timeoutMs = this.objectNumber(params, 'timeoutMs');
    const session = this.requireClassicClientSession(sessionId);
    const data = await this.readClassicSession(session, maxBytes, timeoutMs);
    return this.bluetoothReadData(sessionId, data);
  }

  /// Writes and then reads one classic Bluetooth SPP session.
  private async bluetoothClassicSendAndRead(params: JsonPayload): Promise<JsonPayload> {
    this.bluetoothClassicSend(params);
    return this.bluetoothClassicRead(params);
  }

  /// Closes one classic or BLE Bluetooth session.
  private bluetoothClose(params: JsonPayload): string {
    const sessionId = this.objectString(params, 'sessionId');
    const classic = this.classicBluetoothSessions.get(sessionId);
    if (classic !== undefined) {
      if (classic.clientSocket !== null) {
        if (classic.readCallback !== null) {
          bluetoothManager.off('sppRead', classic.clientSocket, classic.readCallback);
        }
        bluetoothManager.sppCloseClientSocket(classic.clientSocket);
      }
      if (classic.serverSocket !== null) {
        bluetoothManager.sppCloseServerSocket(classic.serverSocket);
      }
      this.classicBluetoothSessions.delete(sessionId);
      return 'ohos_bluetooth_session_closed';
    }
    const ble = this.bleBluetoothSessions.get(sessionId);
    if (ble !== undefined) {
      if (ble.notifyCallback !== null) {
        ble.client.off('BLECharacteristicChange', ble.notifyCallback);
      }
      ble.client.disconnect();
      ble.client.close();
      this.bleBluetoothSessions.delete(sessionId);
      return 'ohos_bluetooth_session_closed';
    }
    throw new Error(`OpenHarmony Bluetooth session is unknown: ${sessionId}`);
  }

  /// Connects to one BLE device.
  private async bluetoothBleConnect(params: JsonPayload): Promise<JsonPayload> {
    const address = this.objectString(params, 'address');
    const client = bluetoothManager.BLE.createGattClientDevice(address);
    await this.waitBleConnect(client, address);
    const sessionId = this.nextBluetoothSessionId('ble');
    this.bleBluetoothSessions.set(sessionId, {
      address: address,
      client: client,
      notifications: [],
      notifyCallback: null
    });
    return {
      sessionId: sessionId,
      address: address,
      mode: 'ble_client'
    };
  }

  /// Discovers BLE services for one connected session.
  private async bluetoothBleDiscoverServices(params: JsonPayload): Promise<JsonPayload> {
    const sessionId = this.objectString(params, 'sessionId');
    const session = this.requireBleSession(sessionId);
    const services = await session.client.getServices();
    return {
      sessionId: sessionId,
      services: services.map((service: bluetoothManager.GattService): BleServicePayload => {
        return {
          uuid: service.serviceUuid,
          characteristics: service.characteristics.map(
            (characteristic: bluetoothManager.BLECharacteristic): BleCharacteristicPayload => {
              return {
                uuid: characteristic.characteristicUuid,
                properties: [] as Array<string>
              };
            }
          )
        };
      })
    };
  }

  /// Reads one BLE characteristic.
  private async bluetoothBleReadCharacteristic(params: JsonPayload): Promise<JsonPayload> {
    const sessionId = this.objectString(params, 'sessionId');
    const serviceUuid = this.objectString(params, 'serviceUuid');
    const characteristicUuid = this.objectString(params, 'characteristicUuid');
    const session = this.requireBleSession(sessionId);
    const characteristic = await session.client.readCharacteristicValue({
      serviceUuid: serviceUuid,
      characteristicUuid: characteristicUuid,
      characteristicValue: new ArrayBuffer(0),
      descriptors: []
    });
    return this.bluetoothReadData(sessionId, characteristic.characteristicValue);
  }

  /// Writes one BLE characteristic.
  private bluetoothBleWriteCharacteristic(params: JsonPayload): JsonPayload {
    const sessionId = this.objectString(params, 'sessionId');
    const serviceUuid = this.objectString(params, 'serviceUuid');
    const characteristicUuid = this.objectString(params, 'characteristicUuid');
    const session = this.requireBleSession(sessionId);
    const payload = this.bluetoothPayload(params);
    session.client.writeCharacteristicValue({
      serviceUuid: serviceUuid,
      characteristicUuid: characteristicUuid,
      characteristicValue: payload,
      descriptors: []
    });
    return {
      sessionId: sessionId,
      bytesWritten: payload.byteLength
    };
  }

  /// Writes one BLE characteristic and reads another characteristic.
  private async bluetoothBleWriteAndReadCharacteristic(params: JsonPayload): Promise<JsonPayload> {
    const sessionId = this.objectString(params, 'sessionId');
    const session = this.requireBleSession(sessionId);
    const payload = this.bluetoothPayload(params);
    session.client.writeCharacteristicValue({
      serviceUuid: this.objectString(params, 'writeServiceUuid'),
      characteristicUuid: this.objectString(params, 'writeCharacteristicUuid'),
      characteristicValue: payload,
      descriptors: []
    });
    const readCharacteristic = await session.client.readCharacteristicValue({
      serviceUuid: this.objectString(params, 'readServiceUuid'),
      characteristicUuid: this.objectString(params, 'readCharacteristicUuid'),
      characteristicValue: new ArrayBuffer(0),
      descriptors: []
    });
    return this.bluetoothReadData(sessionId, readCharacteristic.characteristicValue);
  }

  /// Enables or disables notifications for one BLE characteristic.
  private bluetoothBleSubscribeCharacteristic(params: JsonPayload): JsonPayload {
    const sessionId = this.objectString(params, 'sessionId');
    const serviceUuid = this.objectString(params, 'serviceUuid');
    const characteristicUuid = this.objectString(params, 'characteristicUuid');
    const enable = this.objectBool(params, 'enable');
    const session = this.requireBleSession(sessionId);
    if (enable && session.notifyCallback === null) {
      session.notifyCallback = (characteristic: bluetoothManager.BLECharacteristic): void => {
        const data = characteristic.characteristicValue;
        session.notifications.push({
          characteristicUuid: characteristic.characteristicUuid,
          bytesRead: data.byteLength,
          text: this.arrayBufferToUtf8(data),
          dataBase64: this.arrayBufferToBase64(data),
          timestamp: Date.now()
        });
      };
      session.client.on('BLECharacteristicChange', session.notifyCallback);
    }
    session.client.setNotifyCharacteristicChanged({
      serviceUuid: serviceUuid,
      characteristicUuid: characteristicUuid,
      characteristicValue: new ArrayBuffer(0),
      descriptors: []
    }, enable);
    return {
      sessionId: sessionId,
      bytesWritten: 0
    };
  }

  /// Reads buffered BLE characteristic notifications.
  private bluetoothBleReadNotifications(params: JsonPayload): JsonPayload {
    const sessionId = this.objectString(params, 'sessionId');
    const limit = Math.max(0, this.objectNumber(params, 'limit'));
    const session = this.requireBleSession(sessionId);
    const notifications = session.notifications.splice(0, limit);
    return {
      sessionId: sessionId,
      notifications: notifications
    };
  }

  /// Connects one SPP client socket.
  private sppConnect(address: string, uuid: string): Promise<number> {
    return new Promise((resolve: (value: number) => void, reject: (error: Error) => void) => {
      bluetoothManager.sppConnect(address, {
        uuid: uuid,
        secure: true,
        type: bluetoothManager.SppType.SPP_RFCOMM
      }, (error: Error, socket: number) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(socket);
      });
    });
  }

  /// Opens one SPP listening socket.
  private sppListen(name: string, uuid: string): Promise<number> {
    return new Promise((resolve: (value: number) => void, reject: (error: Error) => void) => {
      bluetoothManager.sppListen(name, {
        uuid: uuid,
        secure: true,
        type: bluetoothManager.SppType.SPP_RFCOMM
      }, (error: Error, socket: number) => {
        if (error) {
          reject(error);
          return;
        }
        resolve(socket);
      });
    });
  }

  /// Accepts one SPP client socket with a timeout.
  private sppAccept(serverSocket: number, timeoutMs: number): Promise<number> {
    return new Promise((resolve: (value: number) => void, reject: (error: Error) => void) => {
      const timer = setTimeout(() => {
        reject(new Error(`OpenHarmony Bluetooth SPP accept timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      bluetoothManager.sppAccept(serverSocket, (error: Error, socket: number) => {
        clearTimeout(timer);
        if (error) {
          reject(error);
          return;
        }
        resolve(socket);
      });
    });
  }

  /// Waits for one BLE connection state change.
  private waitBleConnect(client: bluetoothManager.GattClientDevice, address: string): Promise<void> {
    return new Promise((resolve: () => void, reject: (error: Error) => void) => {
      let timer = -1;
      const callback = (state: bluetoothManager.BLEConnectChangedState): void => {
        if (state.deviceId !== address) {
          return;
        }
        if (state.state === bluetoothManager.ProfileConnectionState.STATE_CONNECTED) {
          clearTimeout(timer);
          client.off('BLEConnectionStateChange', callback);
          resolve();
        }
        if (state.state === bluetoothManager.ProfileConnectionState.STATE_DISCONNECTED) {
          clearTimeout(timer);
          client.off('BLEConnectionStateChange', callback);
          reject(new Error(`OpenHarmony BLE disconnected during connect: ${address}`));
        }
      };
      timer = setTimeout(() => {
        client.off('BLEConnectionStateChange', callback);
        reject(new Error(`OpenHarmony BLE connect timed out: ${address}`));
      }, 15000);
      client.on('BLEConnectionStateChange', callback);
      client.connect();
    });
  }

  /// Registers one classic SPP client session.
  private registerClassicClientSession(sessionId: string, address: string, mode: string, socket: number): void {
    const session: ClassicBluetoothSession = {
      address: address,
      mode: mode,
      clientSocket: socket,
      serverSocket: null,
      readQueue: [],
      readCallback: null
    };
    session.readCallback = (data: ArrayBuffer): void => {
      session.readQueue.push(data);
    };
    bluetoothManager.on('sppRead', socket, session.readCallback);
    this.classicBluetoothSessions.set(sessionId, session);
  }

  /// Reads one queued SPP payload or waits for the next payload.
  private readClassicSession(session: ClassicBluetoothSession, maxBytes: number, timeoutMs: number): Promise<ArrayBuffer> {
    if (session.readQueue.length > 0) {
      const data = session.readQueue.shift() as ArrayBuffer;
      return Promise.resolve(this.sliceArrayBuffer(data, maxBytes));
    }
    return new Promise((resolve: (value: ArrayBuffer) => void, reject: (error: Error) => void) => {
      const socket = session.clientSocket as number;
      let timer = -1;
      const callback = (data: ArrayBuffer): void => {
        clearTimeout(timer);
        bluetoothManager.off('sppRead', socket, callback);
        if (session.readCallback !== null) {
          bluetoothManager.on('sppRead', socket, session.readCallback);
        }
        resolve(this.sliceArrayBuffer(data, maxBytes));
      };
      timer = setTimeout(() => {
        bluetoothManager.off('sppRead', socket, callback);
        if (session.readCallback !== null) {
          bluetoothManager.on('sppRead', socket, session.readCallback);
        }
        reject(new Error(`OpenHarmony Bluetooth read timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      if (session.readCallback !== null) {
        bluetoothManager.off('sppRead', socket, session.readCallback);
      }
      bluetoothManager.on('sppRead', socket, callback);
    });
  }

  /// Returns one classic Bluetooth session.
  private requireClassicSession(sessionId: string): ClassicBluetoothSession {
    const session = this.classicBluetoothSessions.get(sessionId);
    if (session === undefined) {
      throw new Error(`OpenHarmony classic Bluetooth session is unknown: ${sessionId}`);
    }
    return session;
  }

  /// Returns one classic Bluetooth client session.
  private requireClassicClientSession(sessionId: string): ClassicBluetoothSession {
    const session = this.requireClassicSession(sessionId);
    if (session.clientSocket === null) {
      throw new Error(`OpenHarmony classic Bluetooth session has no client socket: ${sessionId}`);
    }
    return session;
  }

  /// Returns one BLE session.
  private requireBleSession(sessionId: string): BleBluetoothSession {
    const session = this.bleBluetoothSessions.get(sessionId);
    if (session === undefined) {
      throw new Error(`OpenHarmony BLE session is unknown: ${sessionId}`);
    }
    return session;
  }

  /// Builds one Bluetooth read result.
  private bluetoothReadData(sessionId: string, data: ArrayBuffer): JsonPayload {
    return {
      sessionId: sessionId,
      bytesRead: data.byteLength,
      text: this.arrayBufferToUtf8(data),
      dataBase64: this.arrayBufferToBase64(data)
    };
  }

  /// Builds one ArrayBuffer payload from text or base64 bytes.
  private bluetoothPayload(params: JsonPayload): ArrayBuffer {
    const text = this.objectOptionalString(params, 'text');
    const dataBase64 = this.objectOptionalString(params, 'dataBase64');
    if (text !== null) {
      return this.utf8ToArrayBuffer(text);
    }
    if (dataBase64 !== null) {
      const bytes = new util.Base64Helper().decodeSync(dataBase64);
      return bytes.buffer;
    }
    throw new Error('Bluetooth payload requires text or dataBase64');
  }

  /// Returns the next Bluetooth session id.
  private nextBluetoothSessionId(prefix: string): string {
    const value = `${prefix}-${this.bluetoothSessionSeq}`;
    this.bluetoothSessionSeq += 1;
    return value;
  }

  /// Encodes UTF-8 text as an ArrayBuffer.
  private utf8ToArrayBuffer(text: string): ArrayBuffer {
    return new util.TextEncoder('utf-8').encodeInto(text).buffer;
  }

  /// Decodes an ArrayBuffer as UTF-8 text.
  private arrayBufferToUtf8(data: ArrayBuffer): string {
    return new util.TextDecoder('utf-8').decode(new Uint8Array(data));
  }

  /// Encodes an ArrayBuffer as base64 text.
  private arrayBufferToBase64(data: ArrayBuffer): string {
    return new util.Base64Helper().encodeToStringSync(new Uint8Array(data));
  }

  /// Slices an ArrayBuffer to the requested byte count.
  private sliceArrayBuffer(data: ArrayBuffer, maxBytes: number): ArrayBuffer {
    const count = Math.min(Math.max(0, maxBytes), data.byteLength);
    return new Uint8Array(data).slice(0, count).buffer;
  }

  /// Opens one readable file through the OpenHarmony ability resolver.
  private async ownerFileOpen(call: MethodCall): Promise<FileOperationResponse> {
    const path = this.mapString(call, 'path');
    this.requireAccessibleFile(path);
    await this.context.startAbility({
      action: wantConstant.Action.ACTION_VIEW_DATA,
      uri: fileUri.getUriFromPath(path),
      type: '*/*',
      flags: wantConstant.Flags.FLAG_AUTH_READ_URI_PERMISSION
    } as Want);
    return this.fileOperationSuccess();
  }

  /// Shares one readable file through the OpenHarmony ability resolver.
  private async ownerFileShare(call: MethodCall): Promise<FileOperationResponse> {
    const path = this.mapString(call, 'path');
    const title = this.mapString(call, 'title');
    const uri = fileUri.getUriFromPath(path);
    this.requireAccessibleFile(path);
    await this.context.startAbility({
      action: wantConstant.Action.ACTION_SEND_DATA,
      uri: uri,
      type: '*/*',
      flags: wantConstant.Flags.FLAG_AUTH_READ_URI_PERMISSION,
      parameters: {
        [wantConstant.Action.PARAMS_STREAM]: uri,
        [wantConstant.Action.INTENT_PARAMS_TITLE]: title
      }
    } as Want);
    return this.fileOperationSuccess();
  }

  /// Requires that one file path is accessible to the OpenHarmony app.
  private requireAccessibleFile(path: string): void {
    if (!fs.accessSync(path)) {
      throw new Error(`File is not accessible: ${path}`);
    }
  }

  /// Builds a successful file-operation response for Dart.
  private fileOperationSuccess(): FileOperationResponse {
    return {
      success: true,
      error: null
    };
  }

  /// Builds a failed file-operation response for Dart.
  private fileOperationError(error: Object): FileOperationResponse {
    return {
      success: false,
      error: `${error}`
    };
  }

  /// Reads one required ArrayBuffer argument.
  private requiredArrayBuffer(call: MethodCall, method: string): ArrayBuffer {
    const value: Object | null = call.args as Object | null;
    if (!(value instanceof ArrayBuffer)) {
      throw new Error(`${method} expects MessagePack bytes`);
    }
    return value;
  }

  /// Reads one required string argument.
  private requiredStringArg(call: MethodCall, method: string): string {
    const value: Object | null = call.args as Object | null;
    if (typeof value !== 'string' || value.length === 0) {
      throw new Error(`${method} expects a non-empty string`);
    }
    return value;
  }

  /// Reads one required numeric argument.
  private requiredNumberArg(call: MethodCall, method: string): number {
    const value: Object | null = call.args as Object | null;
    if (typeof value !== 'number') {
      throw new Error(`${method} expects a number`);
    }
    return value;
  }

  /// Reads one required string value from a map argument.
  private mapString(call: MethodCall, key: string): string {
    const value = call.argument(key) as string | null;
    if (typeof value !== 'string' || value.length === 0) {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one optional string value from a map argument.
  private mapOptionalString(call: MethodCall, key: string): string | null {
    const value = call.argument(key) as string | null;
    if (typeof value !== 'string' || value.length === 0) {
      return null;
    }
    return value;
  }

  /// Reads one required numeric value from a map argument.
  private mapNumber(call: MethodCall, key: string): number {
    const value = call.argument(key) as number | null;
    if (typeof value !== 'number') {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one required boolean value from a map argument.
  private mapBool(call: MethodCall, key: string): boolean {
    const value = call.argument(key) as boolean | null;
    if (typeof value !== 'boolean') {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one required number from an object map.
  private objectNumber(values: JsonPayload, key: string): number {
    const value = this.objectValue(values, key);
    if (typeof value !== 'number') {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one required boolean from an object map.
  private objectBool(values: JsonPayload, key: string): boolean {
    const value = this.objectValue(values, key);
    if (typeof value !== 'boolean') {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one required string from an object map.
  private objectString(values: JsonPayload, key: string): string {
    const value = this.objectValue(values, key);
    if (typeof value !== 'string' || value.length === 0) {
      throw new Error(`${key} is required`);
    }
    return value;
  }

  /// Reads one optional string from an object map.
  private objectOptionalString(values: JsonPayload, key: string): string | null {
    const value = this.objectValue(values, key);
    if (typeof value !== 'string' || value.length === 0) {
      return null;
    }
    return value;
  }

  /// Reads one known field from a JSON payload.
  private objectValue(values: JsonPayload, key: string): Object | null {
    switch (key) {
      case 'message':
        return values.message ?? null;
      case 'title':
        return values.title ?? null;
      case 'namespace':
        return values.namespace ?? null;
      case 'setting':
        return values.setting ?? null;
      case 'value':
        return values.value ?? null;
      case 'path':
        return values.path ?? null;
      case 'packageName':
        return values.packageName ?? null;
      case 'includeSystemApps':
        return values.includeSystemApps ?? null;
      case 'limit':
        return values.limit ?? null;
      case 'includeOngoing':
        return values.includeOngoing ?? null;
      case 'sinceHours':
        return values.sinceHours ?? null;
      case 'timeout':
        return values.timeout ?? null;
      case 'highAccuracy':
        return values.highAccuracy ?? null;
      case 'includeAddress':
        return values.includeAddress ?? null;
      case 'durationMs':
        return values.durationMs ?? null;
      case 'includeBle':
        return values.includeBle ?? null;
      case 'address':
        return values.address ?? null;
      case 'uuid':
        return values.uuid ?? null;
      case 'name':
        return values.name ?? null;
      case 'listenerSessionId':
        return values.listenerSessionId ?? null;
      case 'sessionId':
        return values.sessionId ?? null;
      case 'maxBytes':
        return values.maxBytes ?? null;
      case 'serviceUuid':
        return values.serviceUuid ?? null;
      case 'characteristicUuid':
        return values.characteristicUuid ?? null;
      case 'enable':
        return values.enable ?? null;
      case 'text':
        return values.text ?? null;
      case 'dataBase64':
        return values.dataBase64 ?? null;
      case 'writeServiceUuid':
        return values.writeServiceUuid ?? null;
      case 'writeCharacteristicUuid':
        return values.writeCharacteristicUuid ?? null;
      case 'readServiceUuid':
        return values.readServiceUuid ?? null;
      case 'readCharacteristicUuid':
        return values.readCharacteristicUuid ?? null;
      default:
        throw new Error(`Unsupported OpenHarmony JSON payload field: ${key}`);
    }
  }

  /// Validates one OpenHarmony storage root path.
  private requiredRoot(path: string | null, label: string): string {
    if (typeof path !== 'string' || path.trim().length === 0) {
      throw new Error(`${label} is required`);
    }
    const value = path.trim();
    if (!value.startsWith('/')) {
      throw new Error(`${label} must be an absolute OpenHarmony path`);
    }
    return value;
  }
}
