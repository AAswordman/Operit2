//! Device, application, Bluetooth, terminal, and media controls exposed to plugins.
use super::results::*;
use super::{JsDate, JsFuture};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a BLE peripheral and controls connection establishment behavior.
pub struct SystemBluetoothBleHostConnectOptions {
    /// Contains the platform Bluetooth address of the peripheral.
    pub address: String,
    /// Allows the platform to establish the connection when the peripheral becomes available.
    #[serde(rename = "autoConnect")]
    pub auto_connect: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded service-discovery timeout in milliseconds.
pub enum SystemBluetoothBleHostDiscoverServicesTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded characteristic-read timeout in milliseconds.
pub enum SystemBluetoothBleHostReadCharacteristicOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects a BLE characteristic and limits how long its value may take to read.
pub struct SystemBluetoothBleHostReadCharacteristicOptions {
    /// Identifies the service containing the characteristic.
    #[serde(rename = "serviceUuid")]
    pub service_uuid: String,
    /// Identifies the characteristic whose value is read.
    #[serde(rename = "characteristicUuid")]
    pub characteristic_uuid: String,
    /// Sets the maximum read wait in milliseconds.
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<SystemBluetoothBleHostReadCharacteristicOptionsTimeoutMs>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects a BLE characteristic and supplies the textual or binary value written to it.
pub struct SystemBluetoothBleHostWriteCharacteristicOptions {
    /// Identifies the service containing the target characteristic.
    #[serde(rename = "serviceUuid")]
    pub service_uuid: String,
    /// Identifies the characteristic that receives the value.
    #[serde(rename = "characteristicUuid")]
    pub characteristic_uuid: String,
    /// Supplies a UTF-8 text payload for the write.
    pub text: Option<String>,
    /// Supplies arbitrary write bytes encoded as Base64.
    #[serde(rename = "dataBase64")]
    pub data_base64: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded BLE response timeout in milliseconds.
pub enum SystemBluetoothBleHostWriteAndReadCharacteristicOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a BLE request written to one characteristic and response read from another.
pub struct SystemBluetoothBleHostWriteAndReadCharacteristicOptions {
    /// Identifies the service containing the request characteristic.
    #[serde(rename = "writeServiceUuid")]
    pub write_service_uuid: String,
    /// Identifies the characteristic that receives the request payload.
    #[serde(rename = "writeCharacteristicUuid")]
    pub write_characteristic_uuid: String,
    /// Identifies the service containing the response characteristic.
    #[serde(rename = "readServiceUuid")]
    pub read_service_uuid: String,
    /// Identifies the characteristic from which the response is read.
    #[serde(rename = "readCharacteristicUuid")]
    pub read_characteristic_uuid: String,
    /// Supplies a UTF-8 request payload.
    pub text: Option<String>,
    /// Supplies arbitrary request bytes encoded as Base64.
    #[serde(rename = "dataBase64")]
    pub data_base64: Option<String>,
    /// Sets the maximum response wait in milliseconds.
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<
        SystemBluetoothBleHostWriteAndReadCharacteristicOptionsTimeoutMs,
    >,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Selects a BLE characteristic and enables or disables value notifications.
pub struct SystemBluetoothBleHostSubscribeOptions {
    /// Identifies the service containing the notification characteristic.
    #[serde(rename = "serviceUuid")]
    pub service_uuid: String,
    /// Identifies the characteristic whose notifications are controlled.
    #[serde(rename = "characteristicUuid")]
    pub characteristic_uuid: String,
    /// Enables notifications when true and disables them when false.
    pub enable: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded limit on queued BLE notifications.
pub enum SystemBluetoothBleHostReadNotificationsLimit {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded Bluetooth scan duration in milliseconds.
pub enum SystemBluetoothHostScanOptionsDurationMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures the duration and protocol coverage of a nearby-device scan.
pub struct SystemBluetoothHostScanOptions {
    /// Sets how long nearby devices are discovered.
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<SystemBluetoothHostScanOptionsDurationMs>,
    /// Includes Bluetooth Low Energy advertisements in scan results.
    #[serde(rename = "includeBle")]
    pub include_ble: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies a classic Bluetooth device and service to connect to.
pub struct SystemBluetoothHostConnectOptions {
    /// Contains the platform Bluetooth address of the remote device.
    pub address: String,
    /// Selects the remote RFCOMM service UUID.
    pub uuid: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a classic Bluetooth RFCOMM listener advertised by this device.
pub struct SystemBluetoothHostListenOptions {
    /// Sets the human-readable service name advertised to peers.
    pub name: Option<String>,
    /// Sets the RFCOMM service UUID accepted by the listener.
    pub uuid: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded incoming-connection timeout in milliseconds.
pub enum SystemBluetoothHostAcceptTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies textual or binary bytes sent over a classic Bluetooth session.
pub struct SystemBluetoothHostSendOptions {
    /// Supplies a UTF-8 text payload for transmission.
    pub text: Option<String>,
    /// Supplies arbitrary transmitted bytes encoded as Base64.
    #[serde(rename = "dataBase64")]
    pub data_base64: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded maximum Bluetooth read size.
pub enum SystemBluetoothHostReadOptionsMaxBytes {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded Bluetooth read timeout in milliseconds.
pub enum SystemBluetoothHostReadOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Limits the size and duration of a classic Bluetooth read operation.
pub struct SystemBluetoothHostReadOptions {
    /// Caps the number of bytes returned by the read.
    #[serde(rename = "maxBytes")]
    pub max_bytes: Option<SystemBluetoothHostReadOptionsMaxBytes>,
    /// Sets the maximum time to wait for incoming data.
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<SystemBluetoothHostReadOptionsTimeoutMs>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded maximum Bluetooth response size.
pub enum SystemBluetoothHostSendAndReadOptionsMaxBytes {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded Bluetooth response timeout in milliseconds.
pub enum SystemBluetoothHostSendAndReadOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a classic Bluetooth request payload and bounded response read.
pub struct SystemBluetoothHostSendAndReadOptions {
    /// Supplies a UTF-8 request payload.
    pub text: Option<String>,
    /// Supplies arbitrary request bytes encoded as Base64.
    #[serde(rename = "dataBase64")]
    pub data_base64: Option<String>,
    /// Caps the number of response bytes returned.
    #[serde(rename = "maxBytes")]
    pub max_bytes: Option<SystemBluetoothHostSendAndReadOptionsMaxBytes>,
    /// Sets the maximum time to wait for the response.
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<SystemBluetoothHostSendAndReadOptionsTimeoutMs>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded terminal command timeout in milliseconds.
pub enum SystemTerminalHostExecTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded streaming command timeout in milliseconds.
pub enum SystemTerminalHostExecStreamingOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
/// Configures timeout and incremental output delivery for a terminal command.
pub struct SystemTerminalHostExecStreamingOptions {
    /// Sets the maximum command execution time in milliseconds.
    pub timeout_ms: Option<SystemTerminalHostExecStreamingOptionsTimeoutMs>,
    /// Receives output events while the command is still running.
    pub on_intermediate_result: Option<
        Arc<dyn Fn(TerminalStreamEventData) -> () + Send + Sync>,
    >,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded hidden command timeout in milliseconds.
pub enum SystemTerminalHostHiddenExecOptionsTimeoutMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures a reusable hidden command executor and its command interpreter.
pub struct SystemTerminalHostHiddenExecOptions {
    /// Selects the hidden login context reused across related commands.
    #[serde(rename = "executorKey")]
    pub executor_key: Option<String>,
    /// Selects the command interpreter used by the hidden executor.
    pub r#type: TerminalCreateType,
    /// Sets the maximum command execution time in milliseconds.
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<SystemTerminalHostHiddenExecOptionsTimeoutMs>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Supplies text or a control-key action to a live terminal session.
pub struct SystemTerminalHostInputOptions {
    /// Contains text or the key combined with a control modifier.
    pub input: Option<String>,
    /// Names a control action such as `enter`, or a modifier such as `ctrl`.
    pub control: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Identifies whether an audio source is a local path, network URL, or platform URI.
pub enum SystemMusicHostPlayOptionsSourceType {
    #[serde(rename = "path")]
    Path,
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "uri")]
    Uri,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded initial playback volume.
pub enum SystemMusicHostPlayOptionsVolume {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded initial playback position in milliseconds.
pub enum SystemMusicHostPlayOptionsStartPositionMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Configures the source, metadata, looping, volume, and start position of audio playback.
pub struct SystemMusicHostPlayOptions {
    /// Contains the path, URL, or URI of the audio resource.
    pub source: String,
    /// Determines how the audio source string is resolved.
    #[serde(rename = "sourceType")]
    pub source_type: SystemMusicHostPlayOptionsSourceType,
    /// Sets the track title shown in playback UI.
    pub title: Option<String>,
    /// Sets the artist shown in playback UI.
    pub artist: Option<String>,
    /// Restarts playback automatically when the source ends.
    pub r#loop: Option<bool>,
    /// Sets the initial playback volume from zero to one.
    pub volume: Option<SystemMusicHostPlayOptionsVolume>,
    /// Starts playback at this offset in milliseconds.
    #[serde(rename = "startPositionMs")]
    pub start_position_ms: Option<SystemMusicHostPlayOptionsStartPositionMs>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded playback position in milliseconds.
pub enum SystemMusicHostSeekPositionMs {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded playback volume from zero to one.
pub enum SystemMusicHostSetVolumeVolume {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded sleep duration in milliseconds.
pub enum SystemHostSleepMilliseconds {
    Variant1(String),
    Variant2(f64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded lookback period in hours.
pub enum SystemHostGetAppUsageTimeOptionsSinceHours {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
/// Accepts a numeric or string-encoded maximum number of usage records.
pub enum SystemHostGetAppUsageTimeOptionsLimit {
    Variant1(f64),
    Variant2(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
/// Filters Android foreground-usage statistics by application, time window, and count.
pub struct SystemHostGetAppUsageTimeOptions {
    /// Restricts usage records to one application package.
    #[serde(rename = "packageName")]
    pub package_name: Option<String>,
    /// Limits records to the preceding number of hours.
    #[serde(rename = "sinceHours")]
    pub since_hours: Option<SystemHostGetAppUsageTimeOptionsSinceHours>,
    /// Caps the number of application usage records returned.
    pub limit: Option<SystemHostGetAppUsageTimeOptionsLimit>,
    /// Includes packages installed as system applications.
    #[serde(rename = "includeSystemApps")]
    pub include_system_apps: Option<bool>,
}
/// Provides device settings, application control, notifications, usage, and location services.
pub trait SystemHost: Send + Sync {
    ///
    ///Sleep for specified milliseconds
    ///@param milliseconds - Milliseconds to sleep
    ///
    fn sleep(
        &self,
        milliseconds: SystemHostSleepMilliseconds,
    ) -> JsFuture<SleepResultData>;
    ///
    ///Get a system setting
    ///@param setting - Setting name
    ///@param namespace - Setting namespace
    ///
    fn getSetting(
        &self,
        setting: String,
        namespace: Option<String>,
    ) -> JsFuture<SystemSettingData>;
    ///
    ///Modify a system setting
    ///@param setting - Setting name
    ///@param value - New value
    ///@param namespace - Setting namespace
    ///
    fn setSetting(
        &self,
        setting: String,
        value: String,
        namespace: Option<String>,
    ) -> JsFuture<SystemSettingData>;
    ///
    ///Get device information
    ///
    fn getDeviceInfo(&self) -> JsFuture<DeviceInfoResultData>;
    ///
    ///Show a toast message on device.
    ///@param message - The message to show
    ///
    fn toast(&self, message: String) -> JsFuture<StringResultData>;
    ///
    ///Send a notification using the same channel as AI reply completion.
    ///@param message - Notification content
    ///@param title - Optional notification title
    ///
    fn sendNotification(
        &self,
        message: String,
        title: Option<String>,
    ) -> JsFuture<StringResultData>;
    ///
    ///Use a tool package
    ///@param packageName - Package name
    ///
    fn usePackage(&self, packageName: String) -> JsFuture<String>;
    ///
    ///Install an application
    ///@param path - Path to the APK file
    ///
    fn installApp(&self, path: String) -> JsFuture<AppOperationData>;
    ///
    ///Uninstall an application
    ///@param packageName - Package name of the app to uninstall
    ///
    fn uninstallApp(&self, packageName: String) -> JsFuture<AppOperationData>;
    ///
    ///Stop a running app
    ///@param packageName - Package name
    ///
    fn stopApp(&self, packageName: String) -> JsFuture<AppOperationData>;
    ///
    ///List installed apps
    ///@param includeSystem - Whether to include system apps
    ///
    fn listApps(&self, includeSystem: Option<bool>) -> JsFuture<AppListData>;
    ///
    ///Start an app by package name
    ///@param packageName - Package name
    ///@param activity - Optional specific activity to launch
    ///
    fn startApp(
        &self,
        packageName: String,
        activity: Option<String>,
    ) -> JsFuture<AppOperationData>;
    ///
    ///Get device notifications
    ///@param limit - Maximum number of notifications to return (default: 10)
    ///@param includeOngoing - Whether to include ongoing notifications (default: false)
    ///@returns Promise resolving to notification data
    ///
    fn getNotifications(
        &self,
        limit: Option<f64>,
        includeOngoing: Option<bool>,
    ) -> JsFuture<NotificationData>;
    ///
    ///Get app foreground usage time from Android Usage Access.
    ///@param options Query options
    ///
    fn getAppUsageTime(
        &self,
        options: Option<SystemHostGetAppUsageTimeOptions>,
    ) -> JsFuture<AppUsageTimeResultData>;
    ///
    ///Get device location
    ///@param highAccuracy - Whether to use high accuracy mode (default: false)
    ///@param timeout - Timeout in seconds (default: 10)
    ///@returns Promise resolving to location data
    ///
    fn getLocation(
        &self,
        highAccuracy: Option<bool>,
        timeout: Option<f64>,
    ) -> JsFuture<LocationData>;
}
///
///Bluetooth operations.
///
pub trait SystemBluetoothHost: Send + Sync {
    ///Request Bluetooth nearby devices permission.
    fn requestPermission(&self) -> JsFuture<StringResultData>;
    ///Get Bluetooth adapter state.
    fn getState(&self) -> JsFuture<BluetoothStateData>;
    ///Open the system dialog to enable Bluetooth.
    fn requestEnable(&self) -> JsFuture<StringResultData>;
    ///List bonded Bluetooth devices.
    fn listBondedDevices(&self) -> JsFuture<BluetoothBondedDevicesData>;
    ///Scan nearby Bluetooth classic and BLE devices.
    fn scan(
        &self,
        options: Option<SystemBluetoothHostScanOptions>,
    ) -> JsFuture<BluetoothScanResultData>;
    ///Connect to a Bluetooth classic device.
    fn connect(
        &self,
        options: SystemBluetoothHostConnectOptions,
    ) -> JsFuture<BluetoothSessionData>;
    ///Listen for another device connecting to this phone over Bluetooth classic.
    fn listen(
        &self,
        options: Option<SystemBluetoothHostListenOptions>,
    ) -> JsFuture<BluetoothSessionData>;
    ///Accept one incoming Bluetooth classic connection.
    fn accept(
        &self,
        listenerSessionId: String,
        timeoutMs: Option<SystemBluetoothHostAcceptTimeoutMs>,
    ) -> JsFuture<BluetoothSessionData>;
    ///Send text or bytes to a Bluetooth classic session.
    fn send(
        &self,
        sessionId: String,
        options: SystemBluetoothHostSendOptions,
    ) -> JsFuture<BluetoothTransferData>;
    ///Read text or bytes from a Bluetooth classic session.
    fn read(
        &self,
        sessionId: String,
        options: Option<SystemBluetoothHostReadOptions>,
    ) -> JsFuture<BluetoothReadData>;
    ///Send text or bytes and read the response from a Bluetooth classic session.
    fn sendAndRead(
        &self,
        sessionId: String,
        options: SystemBluetoothHostSendAndReadOptions,
    ) -> JsFuture<BluetoothReadData>;
    ///Close a Bluetooth classic, listener, or BLE session.
    fn close(&self, sessionId: String) -> JsFuture<StringResultData>;
}
/// Connects to BLE peripherals and accesses their services, characteristics, and notifications.
pub trait SystemBluetoothBleHost: Send + Sync {
    ///Connect to a BLE device.
    fn connect(
        &self,
        options: SystemBluetoothBleHostConnectOptions,
    ) -> JsFuture<BluetoothSessionData>;
    ///Discover BLE services and characteristics.
    fn discoverServices(
        &self,
        sessionId: String,
        timeoutMs: Option<SystemBluetoothBleHostDiscoverServicesTimeoutMs>,
    ) -> JsFuture<BluetoothBleServicesData>;
    ///Read a BLE characteristic.
    fn readCharacteristic(
        &self,
        sessionId: String,
        options: SystemBluetoothBleHostReadCharacteristicOptions,
    ) -> JsFuture<BluetoothReadData>;
    ///Write text or bytes to a BLE characteristic.
    fn writeCharacteristic(
        &self,
        sessionId: String,
        options: SystemBluetoothBleHostWriteCharacteristicOptions,
    ) -> JsFuture<BluetoothTransferData>;
    ///Write text or bytes to one BLE characteristic and read another characteristic response.
    fn writeAndReadCharacteristic(
        &self,
        sessionId: String,
        options: SystemBluetoothBleHostWriteAndReadCharacteristicOptions,
    ) -> JsFuture<BluetoothReadData>;
    ///Subscribe or unsubscribe BLE characteristic notifications.
    fn subscribe(
        &self,
        sessionId: String,
        options: SystemBluetoothBleHostSubscribeOptions,
    ) -> JsFuture<BluetoothTransferData>;
    ///Read received BLE notifications.
    fn readNotifications(
        &self,
        sessionId: String,
        limit: Option<SystemBluetoothBleHostReadNotificationsLimit>,
    ) -> JsFuture<BluetoothBleNotificationData>;
}
///
///Terminal operations.
///
pub trait SystemTerminalHost: Send + Sync {
    ///
    ///Get terminal environment information for the current platform.
    ///
    fn info(&self) -> JsFuture<TerminalInfoResultData>;
    ///
    ///Create or get a terminal session.
    ///@param sessionName The name for the session.
    ///@param type Terminal type. Windows supports powershell and bash; Linux supports linux; Android supports bash and shell.
    ///@returns Promise resolving to the session creation result.
    ///
    fn create(
        &self,
        sessionName: String,
        r#type: TerminalCreateType,
    ) -> JsFuture<TerminalSessionCreationResultData>;
    ///
    ///Execute a command in a terminal session.
    ///@param sessionId The ID of the session.
    ///@param command The command to execute.
    ///@param timeoutMs Optional timeout in milliseconds. Strongly recommended to always pass explicitly.
    ///@returns Promise resolving to the command execution result. On timeout, the current command is cancelled, the terminal session is kept, and the returned result has `timedOut === true`.
    ///
    fn exec(
        &self,
        sessionId: String,
        command: String,
        timeoutMs: Option<SystemTerminalHostExecTimeoutMs>,
    ) -> JsFuture<TerminalCommandResultData>;
    ///
    ///Execute a command in a terminal session and receive incremental output chunks.
    ///Final resolution still returns the complete terminal command result.
    ///@param sessionId The ID of the session.
    ///@param command The command to execute.
    ///@param options Streaming execution options.
    ///@returns Promise resolving to the final command execution result.
    ///
    fn execStreaming(
        &self,
        sessionId: String,
        command: String,
        options: Option<SystemTerminalHostExecStreamingOptions>,
    ) -> JsFuture<TerminalCommandResultData>;
    ///
    ///Execute a command in a hidden non-PTY executor.
    ///Commands using the same executorKey reuse the same hidden login context and are not shown in the visible terminal UI.
    ///@param command The command to execute.
    ///@param options Hidden executor options.
    ///@returns Promise resolving to the hidden command execution result. On timeout, the current command is cancelled, the hidden executor session is kept, and the returned result has `timedOut === true`.
    ///
    fn hiddenExec(
        &self,
        command: String,
        options: SystemTerminalHostHiddenExecOptions,
    ) -> JsFuture<HiddenTerminalCommandResultData>;
    ///
    ///Close a terminal session.
    ///@param sessionId The ID of the session to close.
    ///@returns Promise resolving to the session close result.
    ///
    fn close(&self, sessionId: String) -> JsFuture<TerminalSessionCloseResultData>;
    ///
    ///Get the current visible terminal screen content for a session (single screen only, no history).
    ///@param sessionId The ID of the session.
    ///@returns Promise resolving to the current screen snapshot result.
    ///
    fn screen(&self, sessionId: String) -> JsFuture<TerminalSessionScreenResultData>;
    ///
    ///Write input to a terminal session.
    ///At least one of `input` or `control` should be provided.
    ///- Typical usage: send input first, then send control=`enter` to submit.
    ///- If `control` and `input` are provided together, it is treated as a key combo
    ///  (for example, control=`ctrl`, input=`c` means Ctrl+C).
    ///@param sessionId The ID of the session.
    ///@param options Input options for this write.
    ///@returns Promise resolving to the write result message.
    ///
    fn input(
        &self,
        sessionId: String,
        options: Option<SystemTerminalHostInputOptions>,
    ) -> JsFuture<StringResultData>;
}
///
///App music playback operations.
///
pub trait SystemMusicHost: Send + Sync {
    ///
    ///Play audio inside the app.
    ///@param options Playback options
    ///
    fn play(
        &self,
        options: SystemMusicHostPlayOptions,
    ) -> JsFuture<MusicPlaybackResultData>;
    ///Pause current music playback.
    fn pause(&self) -> JsFuture<MusicPlaybackResultData>;
    ///Resume current music playback.
    fn resume(&self) -> JsFuture<MusicPlaybackResultData>;
    ///Stop current music playback.
    fn stop(&self) -> JsFuture<MusicPlaybackResultData>;
    ///
    ///Seek current music playback.
    ///@param positionMs Target position in milliseconds
    ///
    fn seek(
        &self,
        positionMs: SystemMusicHostSeekPositionMs,
    ) -> JsFuture<MusicPlaybackResultData>;
    ///
    ///Set playback volume.
    ///@param volume Volume from 0 to 1
    ///
    fn setVolume(
        &self,
        volume: SystemMusicHostSetVolumeVolume,
    ) -> JsFuture<MusicPlaybackResultData>;
    ///Get current music playback status.
    fn status(&self) -> JsFuture<MusicPlaybackResultData>;
}
