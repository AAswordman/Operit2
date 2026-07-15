// Generated from operit-plugin-sdk Rust declarations.

import type { AppListData, AppOperationData, AppUsageTimeResultData, BluetoothBleNotificationData, BluetoothBleServicesData, BluetoothBondedDevicesData, BluetoothReadData, BluetoothScanResultData, BluetoothSessionData, BluetoothStateData, BluetoothTransferData, DeviceInfoResultData, HiddenTerminalCommandResultData, LocationData, MusicPlaybackResultData, NotificationData, SleepResultData, SystemSettingData, TerminalCommandResultData, TerminalCreateType, TerminalInfoResultData, TerminalSessionCloseResultData, TerminalSessionCreationResultData, TerminalSessionScreenResultData, TerminalStreamEventData } from "./results";

/**
 * Provides device settings, application control, notifications, usage, and location services.
 */
export namespace System {
  /**
   * Identifies a BLE peripheral and controls connection establishment behavior.
   */
  export interface BluetoothBleHostConnectOptions {
    /**
     * Contains the platform Bluetooth address of the peripheral.
     */
    address: string;
    /**
     * Allows the platform to establish the connection when the peripheral becomes available.
     */
    autoConnect?: boolean;
  }

  /**
   * Accepts a numeric or string-encoded service-discovery timeout in milliseconds.
   */
  export type BluetoothBleHostDiscoverServicesTimeoutMs = number | string;

  /**
   * Accepts a numeric or string-encoded characteristic-read timeout in milliseconds.
   */
  export type BluetoothBleHostReadCharacteristicOptionsTimeoutMs = number | string;

  /**
   * Selects a BLE characteristic and limits how long its value may take to read.
   */
  export interface BluetoothBleHostReadCharacteristicOptions {
    /**
     * Identifies the service containing the characteristic.
     */
    serviceUuid: string;
    /**
     * Identifies the characteristic whose value is read.
     */
    characteristicUuid: string;
    /**
     * Sets the maximum read wait in milliseconds.
     */
    timeoutMs?: BluetoothBleHostReadCharacteristicOptionsTimeoutMs;
  }

  /**
   * Selects a BLE characteristic and supplies the textual or binary value written to it.
   */
  export interface BluetoothBleHostWriteCharacteristicOptions {
    /**
     * Identifies the service containing the target characteristic.
     */
    serviceUuid: string;
    /**
     * Identifies the characteristic that receives the value.
     */
    characteristicUuid: string;
    /**
     * Supplies a UTF-8 text payload for the write.
     */
    text?: string;
    /**
     * Supplies arbitrary write bytes encoded as Base64.
     */
    dataBase64?: string;
  }

  /**
   * Accepts a numeric or string-encoded BLE response timeout in milliseconds.
   */
  export type BluetoothBleHostWriteAndReadCharacteristicOptionsTimeoutMs = number | string;

  /**
   * Configures a BLE request written to one characteristic and response read from another.
   */
  export interface BluetoothBleHostWriteAndReadCharacteristicOptions {
    /**
     * Identifies the service containing the request characteristic.
     */
    writeServiceUuid: string;
    /**
     * Identifies the characteristic that receives the request payload.
     */
    writeCharacteristicUuid: string;
    /**
     * Identifies the service containing the response characteristic.
     */
    readServiceUuid: string;
    /**
     * Identifies the characteristic from which the response is read.
     */
    readCharacteristicUuid: string;
    /**
     * Supplies a UTF-8 request payload.
     */
    text?: string;
    /**
     * Supplies arbitrary request bytes encoded as Base64.
     */
    dataBase64?: string;
    /**
     * Sets the maximum response wait in milliseconds.
     */
    timeoutMs?: BluetoothBleHostWriteAndReadCharacteristicOptionsTimeoutMs;
  }

  /**
   * Selects a BLE characteristic and enables or disables value notifications.
   */
  export interface BluetoothBleHostSubscribeOptions {
    /**
     * Identifies the service containing the notification characteristic.
     */
    serviceUuid: string;
    /**
     * Identifies the characteristic whose notifications are controlled.
     */
    characteristicUuid: string;
    /**
     * Enables notifications when true and disables them when false.
     */
    enable?: boolean;
  }

  /**
   * Accepts a numeric or string-encoded limit on queued BLE notifications.
   */
  export type BluetoothBleHostReadNotificationsLimit = number | string;

  /**
   * Accepts a numeric or string-encoded Bluetooth scan duration in milliseconds.
   */
  export type BluetoothHostScanOptionsDurationMs = number | string;

  /**
   * Configures the duration and protocol coverage of a nearby-device scan.
   */
  export interface BluetoothHostScanOptions {
    /**
     * Sets how long nearby devices are discovered.
     */
    durationMs?: BluetoothHostScanOptionsDurationMs;
    /**
     * Includes Bluetooth Low Energy advertisements in scan results.
     */
    includeBle?: boolean;
  }

  /**
   * Identifies a classic Bluetooth device and service to connect to.
   */
  export interface BluetoothHostConnectOptions {
    /**
     * Contains the platform Bluetooth address of the remote device.
     */
    address: string;
    /**
     * Selects the remote RFCOMM service UUID.
     */
    uuid?: string;
  }

  /**
   * Configures a classic Bluetooth RFCOMM listener advertised by this device.
   */
  export interface BluetoothHostListenOptions {
    /**
     * Sets the human-readable service name advertised to peers.
     */
    name?: string;
    /**
     * Sets the RFCOMM service UUID accepted by the listener.
     */
    uuid?: string;
  }

  /**
   * Accepts a numeric or string-encoded incoming-connection timeout in milliseconds.
   */
  export type BluetoothHostAcceptTimeoutMs = number | string;

  /**
   * Supplies textual or binary bytes sent over a classic Bluetooth session.
   */
  export interface BluetoothHostSendOptions {
    /**
     * Supplies a UTF-8 text payload for transmission.
     */
    text?: string;
    /**
     * Supplies arbitrary transmitted bytes encoded as Base64.
     */
    dataBase64?: string;
  }

  /**
   * Accepts a numeric or string-encoded maximum Bluetooth read size.
   */
  export type BluetoothHostReadOptionsMaxBytes = number | string;

  /**
   * Accepts a numeric or string-encoded Bluetooth read timeout in milliseconds.
   */
  export type BluetoothHostReadOptionsTimeoutMs = number | string;

  /**
   * Limits the size and duration of a classic Bluetooth read operation.
   */
  export interface BluetoothHostReadOptions {
    /**
     * Caps the number of bytes returned by the read.
     */
    maxBytes?: BluetoothHostReadOptionsMaxBytes;
    /**
     * Sets the maximum time to wait for incoming data.
     */
    timeoutMs?: BluetoothHostReadOptionsTimeoutMs;
  }

  /**
   * Accepts a numeric or string-encoded maximum Bluetooth response size.
   */
  export type BluetoothHostSendAndReadOptionsMaxBytes = number | string;

  /**
   * Accepts a numeric or string-encoded Bluetooth response timeout in milliseconds.
   */
  export type BluetoothHostSendAndReadOptionsTimeoutMs = number | string;

  /**
   * Configures a classic Bluetooth request payload and bounded response read.
   */
  export interface BluetoothHostSendAndReadOptions {
    /**
     * Supplies a UTF-8 request payload.
     */
    text?: string;
    /**
     * Supplies arbitrary request bytes encoded as Base64.
     */
    dataBase64?: string;
    /**
     * Caps the number of response bytes returned.
     */
    maxBytes?: BluetoothHostSendAndReadOptionsMaxBytes;
    /**
     * Sets the maximum time to wait for the response.
     */
    timeoutMs?: BluetoothHostSendAndReadOptionsTimeoutMs;
  }

  /**
   * Accepts a numeric or string-encoded terminal command timeout in milliseconds.
   */
  export type TerminalHostExecTimeoutMs = number | string;

  /**
   * Accepts a numeric or string-encoded streaming command timeout in milliseconds.
   */
  export type TerminalHostExecStreamingOptionsTimeoutMs = number | string;

  /**
   * Configures timeout and incremental output delivery for a terminal command.
   */
  export interface TerminalHostExecStreamingOptions {
    /**
     * Sets the maximum command execution time in milliseconds.
     */
    timeout_ms?: TerminalHostExecStreamingOptionsTimeoutMs;
    /**
     * Receives output events while the command is still running.
     */
    on_intermediate_result?: (arg0: TerminalStreamEventData) => void;
  }

  /**
   * Accepts a numeric or string-encoded hidden command timeout in milliseconds.
   */
  export type TerminalHostHiddenExecOptionsTimeoutMs = number | string;

  /**
   * Configures a reusable hidden command executor and its command interpreter.
   */
  export interface TerminalHostHiddenExecOptions {
    /**
     * Selects the hidden login context reused across related commands.
     */
    executorKey?: string;
    /**
     * Selects the command interpreter used by the hidden executor.
     */
    type: TerminalCreateType;
    /**
     * Sets the maximum command execution time in milliseconds.
     */
    timeoutMs?: TerminalHostHiddenExecOptionsTimeoutMs;
  }

  /**
   * Supplies text or a control-key action to a live terminal session.
   */
  export interface TerminalHostInputOptions {
    /**
     * Contains text or the key combined with a control modifier.
     */
    input?: string;
    /**
     * Names a control action such as `enter`, or a modifier such as `ctrl`.
     */
    control?: string;
  }

  /**
   * Identifies whether an audio source is a local path, network URL, or platform URI.
   */
  export type MusicHostPlayOptionsSourceType = "path" | "url" | "uri";

  /**
   * Accepts a numeric or string-encoded initial playback volume.
   */
  export type MusicHostPlayOptionsVolume = number | string;

  /**
   * Accepts a numeric or string-encoded initial playback position in milliseconds.
   */
  export type MusicHostPlayOptionsStartPositionMs = number | string;

  /**
   * Configures the source, metadata, looping, volume, and start position of audio playback.
   */
  export interface MusicHostPlayOptions {
    /**
     * Contains the path, URL, or URI of the audio resource.
     */
    source: string;
    /**
     * Determines how the audio source string is resolved.
     */
    sourceType: MusicHostPlayOptionsSourceType;
    /**
     * Sets the track title shown in playback UI.
     */
    title?: string;
    /**
     * Sets the artist shown in playback UI.
     */
    artist?: string;
    /**
     * Restarts playback automatically when the source ends.
     */
    loop?: boolean;
    /**
     * Sets the initial playback volume from zero to one.
     */
    volume?: MusicHostPlayOptionsVolume;
    /**
     * Starts playback at this offset in milliseconds.
     */
    startPositionMs?: MusicHostPlayOptionsStartPositionMs;
  }

  /**
   * Accepts a numeric or string-encoded playback position in milliseconds.
   */
  export type MusicHostSeekPositionMs = number | string;

  /**
   * Accepts a numeric or string-encoded playback volume from zero to one.
   */
  export type MusicHostSetVolumeVolume = number | string;

  /**
   * Accepts a numeric or string-encoded sleep duration in milliseconds.
   */
  export type HostSleepMilliseconds = string | number;

  /**
   * Accepts a numeric or string-encoded lookback period in hours.
   */
  export type HostGetAppUsageTimeOptionsSinceHours = number | string;

  /**
   * Accepts a numeric or string-encoded maximum number of usage records.
   */
  export type HostGetAppUsageTimeOptionsLimit = number | string;

  /**
   * Filters Android foreground-usage statistics by application, time window, and count.
   */
  export interface HostGetAppUsageTimeOptions {
    /**
     * Restricts usage records to one application package.
     */
    packageName?: string;
    /**
     * Limits records to the preceding number of hours.
     */
    sinceHours?: HostGetAppUsageTimeOptionsSinceHours;
    /**
     * Caps the number of application usage records returned.
     */
    limit?: HostGetAppUsageTimeOptionsLimit;
    /**
     * Includes packages installed as system applications.
     */
    includeSystemApps?: boolean;
  }

  /**
   * Get app foreground usage time from Android Usage Access.
   * @param options Query options
   */
  function getAppUsageTime(options?: HostGetAppUsageTimeOptions): Promise<AppUsageTimeResultData>;
  /**
   * Get device information
   */
  function getDeviceInfo(): Promise<DeviceInfoResultData>;
  /**
   * Get device location
   * @param highAccuracy - Whether to use high accuracy mode (default: false)
   * @param timeout - Timeout in seconds (default: 10)
   * @returns Promise resolving to location data
   */
  function getLocation(highAccuracy?: boolean, timeout?: number): Promise<LocationData>;
  /**
   * Get device notifications
   * @param limit - Maximum number of notifications to return (default: 10)
   * @param includeOngoing - Whether to include ongoing notifications (default: false)
   * @returns Promise resolving to notification data
   */
  function getNotifications(limit?: number, includeOngoing?: boolean): Promise<NotificationData>;
  /**
   * Get a system setting
   * @param setting - Setting name
   * @param namespace - Setting namespace
   */
  function getSetting(setting: string, namespace?: string): Promise<SystemSettingData>;
  /**
   * Install an application
   * @param path - Path to the APK file
   */
  function installApp(path: string): Promise<AppOperationData>;
  /**
   * List installed apps
   * @param includeSystem - Whether to include system apps
   */
  function listApps(includeSystem?: boolean): Promise<AppListData>;
  /**
   * Send a notification using the same channel as AI reply completion.
   * @param message - Notification content
   * @param title - Optional notification title
   */
  function sendNotification(message: string, title?: string): Promise<string>;
  /**
   * Modify a system setting
   * @param setting - Setting name
   * @param value - New value
   * @param namespace - Setting namespace
   */
  function setSetting(setting: string, value: string, namespace?: string): Promise<SystemSettingData>;
  /**
   * Sleep for specified milliseconds
   * @param milliseconds - Milliseconds to sleep
   */
  function sleep(milliseconds: HostSleepMilliseconds): Promise<SleepResultData>;
  /**
   * Start an app by package name
   * @param packageName - Package name
   * @param activity - Optional specific activity to launch
   */
  function startApp(packageName: string, activity?: string): Promise<AppOperationData>;
  /**
   * Stop a running app
   * @param packageName - Package name
   */
  function stopApp(packageName: string): Promise<AppOperationData>;
  /**
   * Show a toast message on device.
   * @param message - The message to show
   */
  function toast(message: string): Promise<string>;
  /**
   * Uninstall an application
   * @param packageName - Package name of the app to uninstall
   */
  function uninstallApp(packageName: string): Promise<AppOperationData>;
  /**
   * Use a tool package
   * @param packageName - Package name
   */
  function usePackage(packageName: string): Promise<string>;
  /**
   * Bluetooth operations.
   */
  export namespace bluetooth {
    /**
     * Accept one incoming Bluetooth classic connection.
     */
    function accept(listenerSessionId: string, timeoutMs?: System.BluetoothHostAcceptTimeoutMs): Promise<BluetoothSessionData>;
    /**
     * Close a Bluetooth classic, listener, or BLE session.
     */
    function close(sessionId: string): Promise<string>;
    /**
     * Connect to a Bluetooth classic device.
     */
    function connect(options: System.BluetoothHostConnectOptions): Promise<BluetoothSessionData>;
    /**
     * Get Bluetooth adapter state.
     */
    function getState(): Promise<BluetoothStateData>;
    /**
     * List bonded Bluetooth devices.
     */
    function listBondedDevices(): Promise<BluetoothBondedDevicesData>;
    /**
     * Listen for another device connecting to this phone over Bluetooth classic.
     */
    function listen(options?: System.BluetoothHostListenOptions): Promise<BluetoothSessionData>;
    /**
     * Read text or bytes from a Bluetooth classic session.
     */
    function read(sessionId: string, options?: System.BluetoothHostReadOptions): Promise<BluetoothReadData>;
    /**
     * Open the system dialog to enable Bluetooth.
     */
    function requestEnable(): Promise<string>;
    /**
     * Request Bluetooth nearby devices permission.
     */
    function requestPermission(): Promise<string>;
    /**
     * Scan nearby Bluetooth classic and BLE devices.
     */
    function scan(options?: System.BluetoothHostScanOptions): Promise<BluetoothScanResultData>;
    /**
     * Send text or bytes to a Bluetooth classic session.
     */
    function send(sessionId: string, options: System.BluetoothHostSendOptions): Promise<BluetoothTransferData>;
    /**
     * Send text or bytes and read the response from a Bluetooth classic session.
     */
    function sendAndRead(sessionId: string, options: System.BluetoothHostSendAndReadOptions): Promise<BluetoothReadData>;
    /**
     * Connects to BLE peripherals and accesses their services, characteristics, and notifications.
     */
    export namespace ble {
      /**
       * Connect to a BLE device.
       */
      function connect(options: System.BluetoothBleHostConnectOptions): Promise<BluetoothSessionData>;
      /**
       * Discover BLE services and characteristics.
       */
      function discoverServices(sessionId: string, timeoutMs?: System.BluetoothBleHostDiscoverServicesTimeoutMs): Promise<BluetoothBleServicesData>;
      /**
       * Read a BLE characteristic.
       */
      function readCharacteristic(sessionId: string, options: System.BluetoothBleHostReadCharacteristicOptions): Promise<BluetoothReadData>;
      /**
       * Read received BLE notifications.
       */
      function readNotifications(sessionId: string, limit?: System.BluetoothBleHostReadNotificationsLimit): Promise<BluetoothBleNotificationData>;
      /**
       * Subscribe or unsubscribe BLE characteristic notifications.
       */
      function subscribe(sessionId: string, options: System.BluetoothBleHostSubscribeOptions): Promise<BluetoothTransferData>;
      /**
       * Write text or bytes to one BLE characteristic and read another characteristic response.
       */
      function writeAndReadCharacteristic(sessionId: string, options: System.BluetoothBleHostWriteAndReadCharacteristicOptions): Promise<BluetoothReadData>;
      /**
       * Write text or bytes to a BLE characteristic.
       */
      function writeCharacteristic(sessionId: string, options: System.BluetoothBleHostWriteCharacteristicOptions): Promise<BluetoothTransferData>;
    }

  }

  /**
   * Terminal operations.
   */
  export namespace terminal {
    /**
     * Close a terminal session.
     * @param sessionId The ID of the session to close.
     * @returns Promise resolving to the session close result.
     */
    function close(sessionId: string): Promise<TerminalSessionCloseResultData>;
    /**
     * Create or get a terminal session.
     * @param sessionName The name for the session.
     * @param type Terminal type. Windows supports powershell and bash; Linux supports linux; Android supports bash and shell.
     * @returns Promise resolving to the session creation result.
     */
    function create(sessionName: string, type: TerminalCreateType): Promise<TerminalSessionCreationResultData>;
    /**
     * Execute a command in a terminal session.
     * @param sessionId The ID of the session.
     * @param command The command to execute.
     * @param timeoutMs Optional timeout in milliseconds. Strongly recommended to always pass explicitly.
     * @returns Promise resolving to the command execution result. On timeout, the current command is cancelled, the terminal session is kept, and the returned result has `timedOut === true`.
     */
    function exec(sessionId: string, command: string, timeoutMs?: System.TerminalHostExecTimeoutMs): Promise<TerminalCommandResultData>;
    /**
     * Execute a command in a terminal session and receive incremental output chunks.
     * Final resolution still returns the complete terminal command result.
     * @param sessionId The ID of the session.
     * @param command The command to execute.
     * @param options Streaming execution options.
     * @returns Promise resolving to the final command execution result.
     */
    function execStreaming(sessionId: string, command: string, options?: System.TerminalHostExecStreamingOptions): Promise<TerminalCommandResultData>;
    /**
     * Execute a command in a hidden non-PTY executor.
     * Commands using the same executorKey reuse the same hidden login context and are not shown in the visible terminal UI.
     * @param command The command to execute.
     * @param options Hidden executor options.
     * @returns Promise resolving to the hidden command execution result. On timeout, the current command is cancelled, the hidden executor session is kept, and the returned result has `timedOut === true`.
     */
    function hiddenExec(command: string, options: System.TerminalHostHiddenExecOptions): Promise<HiddenTerminalCommandResultData>;
    /**
     * Get terminal environment information for the current platform.
     */
    function info(): Promise<TerminalInfoResultData>;
    /**
     * Write input to a terminal session.
     * At least one of `input` or `control` should be provided.
     * - Typical usage: send input first, then send control=`enter` to submit.
     * - If `control` and `input` are provided together, it is treated as a key combo
     * (for example, control=`ctrl`, input=`c` means Ctrl+C).
     * @param sessionId The ID of the session.
     * @param options Input options for this write.
     * @returns Promise resolving to the write result message.
     */
    function input(sessionId: string, options?: System.TerminalHostInputOptions): Promise<string>;
    /**
     * Get the current visible terminal screen content for a session (single screen only, no history).
     * @param sessionId The ID of the session.
     * @returns Promise resolving to the current screen snapshot result.
     */
    function screen(sessionId: string): Promise<TerminalSessionScreenResultData>;
  }

  /**
   * App music playback operations.
   */
  export namespace music {
    /**
     * Pause current music playback.
     */
    function pause(): Promise<MusicPlaybackResultData>;
    /**
     * Play audio inside the app.
     * @param options Playback options
     */
    function play(options: System.MusicHostPlayOptions): Promise<MusicPlaybackResultData>;
    /**
     * Resume current music playback.
     */
    function resume(): Promise<MusicPlaybackResultData>;
    /**
     * Seek current music playback.
     * @param positionMs Target position in milliseconds
     */
    function seek(positionMs: System.MusicHostSeekPositionMs): Promise<MusicPlaybackResultData>;
    /**
     * Set playback volume.
     * @param volume Volume from 0 to 1
     */
    function setVolume(volume: System.MusicHostSetVolumeVolume): Promise<MusicPlaybackResultData>;
    /**
     * Get current music playback status.
     */
    function status(): Promise<MusicPlaybackResultData>;
    /**
     * Stop current music playback.
     */
    function stop(): Promise<MusicPlaybackResultData>;
  }

}
