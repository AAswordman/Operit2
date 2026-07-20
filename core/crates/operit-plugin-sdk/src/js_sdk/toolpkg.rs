//! ToolPkg data contracts, hooks, host events, UI contributions, IPC, and AI provider registration.
use super::compose_dsl::*;
use super::core::*;
use super::{JsDate, JsFuture, JsOptional};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
/// Stores localized strings keyed by language tag.
pub struct ToolPkgLocalizedTextVariant2 {
    /// Maps language tags to localized text.
    pub additional_properties: BTreeMap<String, String>,
}
/// Stores a JSON object whose properties may contain any ToolPkg JSON value.
pub struct ToolPkgJsonValueVariant3 {
    /// Stores arbitrary JSON properties keyed by name.
    pub additional_properties: BTreeMap<String, ToolPkgJsonValue>,
}
/// Carries the message-processing hook discriminator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHookEventNameVariant2 {
    /// Identifies the message-processing hook.
    #[serde(rename = "message_processing")]
    MessageProcessing,
}
/// Carries the XML-render hook discriminator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHookEventNameVariant3 {
    /// Identifies the XML-render hook.
    #[serde(rename = "xml_render")]
    XmlRender,
}
/// Carries the input-menu-toggle hook discriminator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHookEventNameVariant4 {
    /// Identifies the input-menu-toggle hook.
    #[serde(rename = "input_menu_toggle")]
    InputMenuToggle,
}
/// Carries the navigation-entry-action hook discriminator.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHookEventNameVariant7 {
    /// Identifies the navigation-entry-action hook.
    #[serde(rename = "navigation_entry_action")]
    NavigationEntryAction,
}
/// Enumerates values to which an asynchronous generic hook may resolve.
pub enum ToolPkgHookReturnVariant3Output {
    Variant1(ToolPkgJsonValue),
    Variant2(()),
}
/// Enumerates values to which an asynchronous application lifecycle hook may resolve.
pub enum ToolPkgAppLifecycleHookReturnVariant3Output {
    Variant1(ToolPkgJsonValue),
    Variant2(()),
}
/// Supplies a Compose DSL screen and optional rendering state for an XML render hook.
pub struct ToolPkgXmlRenderHookObjectResultComposeDsl {
    /// Contains the Compose DSL screen rendered by the host.
    pub screen: ComposeDslScreen,
    /// Carries mutable state used by the Compose DSL screen.
    pub state: Option<ToolPkgJsonObject>,
    /// Carries memoized Compose DSL data across renders.
    pub memo: Option<ToolPkgJsonObject>,
    /// Carries module metadata consumed by the Compose DSL renderer.
    pub moduleSpec: Option<ToolPkgJsonObject>,
}
/// Enumerates values to which an asynchronous XML render hook may resolve.
pub enum ToolPkgXmlRenderHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgXmlRenderHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous input menu toggle hook may resolve.
pub enum ToolPkgInputMenuToggleHookReturnVariant5Output {
    Variant1(Vec<ToolPkgInputMenuToggleDefinitionResult>),
    Variant2(ToolPkgInputMenuToggleObjectResult),
    Null,
    Void,
}
/// Controls whether submitted chat input is allowed, blocked, replaced, or consumed.
pub enum ToolPkgChatInputHookObjectResultAction {
    Allow,
    Block,
    Replace,
    Consume,
}
/// Enumerates values to which an asynchronous chat input hook may resolve.
pub enum ToolPkgChatInputHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgChatInputHookObjectResult),
    Null,
    Void,
}
/// Stores tool-call parameters as strings keyed by parameter name.
pub struct ToolPkgToolLifecycleEventPayloadParameters {
    /// Maps tool parameter names to serialized values.
    pub additional_properties: BTreeMap<String, String>,
}
/// Enumerates values to which an asynchronous prompt input hook may resolve.
pub enum ToolPkgPromptInputHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous prompt history hook may resolve.
pub enum ToolPkgPromptHistoryHookReturnVariant5Output {
    Variant1(Vec<ToolPkgPromptTurn>),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous system prompt compose hook may resolve.
pub enum ToolPkgSystemPromptComposeHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous tool prompt compose hook may resolve.
pub enum ToolPkgToolPromptComposeHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous prompt finalize hook may resolve.
pub enum ToolPkgPromptFinalizeHookReturnVariant6Output {
    Variant1(String),
    Variant2(Vec<ToolPkgPromptTurn>),
    Variant3(ToolPkgPromptHookObjectResult),
    Null,
    Void,
}
/// Enumerates values to which an asynchronous summary generate hook may resolve.
pub enum ToolPkgSummaryGenerateHookReturnVariant5Output {
    Variant1(String),
    Variant2(ToolPkgSummaryHookObjectResult),
    Null,
    Void,
}
/// Allows message processing to complete immediately or asynchronously.
pub enum ToolPkgMessageProcessingHookHandlerOutput {
    Variant1(ToolPkgMessageProcessingHookReturnValue),
    Variant2(JsFuture<ToolPkgMessageProcessingHookReturnValue>),
}
/// Identifies the request to create input-menu toggle definitions.
pub enum ToolPkgInputMenuToggleEventPayloadActionVariant1 {
    Create,
}
/// Identifies a change to an existing input-menu toggle.
pub enum ToolPkgInputMenuToggleEventPayloadActionVariant2 {
    Toggle,
}
/// Identifies whether toggle definitions are requested or a toggle is changed.
pub enum ToolPkgInputMenuToggleEventPayloadAction {
    Variant1(ToolPkgInputMenuToggleEventPayloadActionVariant1),
    Variant2(ToolPkgInputMenuToggleEventPayloadActionVariant2),
    Variant3(String),
}
/// Identifies the classic chat input style.
pub enum ToolPkgChatInputEventPayloadInputStyleVariant1 {
    Classic,
}
/// Identifies the agent chat input style.
pub enum ToolPkgChatInputEventPayloadInputStyleVariant2 {
    Agent,
}
/// Identifies the active chat input style while preserving host-defined styles.
pub enum ToolPkgChatInputEventPayloadInputStyle {
    Variant1(ToolPkgChatInputEventPayloadInputStyleVariant1),
    Variant2(ToolPkgChatInputEventPayloadInputStyleVariant2),
    Variant3(String),
}
/// Identifies input from the classic chat surface.
pub enum ToolPkgChatInputEventPayloadSourceVariant1 {
    Classic,
}
/// Identifies input from the agent chat surface.
pub enum ToolPkgChatInputEventPayloadSourceVariant2 {
    Agent,
}
/// Identifies input from the fullscreen chat surface.
pub enum ToolPkgChatInputEventPayloadSourceVariant3 {
    Fullscreen,
}
/// Identifies input dispatched from the message queue.
pub enum ToolPkgChatInputEventPayloadSourceVariant4 {
    Queue,
}
/// Identifies where chat input originated while preserving host-defined sources.
pub enum ToolPkgChatInputEventPayloadSource {
    Variant1(ToolPkgChatInputEventPayloadSourceVariant1),
    Variant2(ToolPkgChatInputEventPayloadSourceVariant2),
    Variant3(ToolPkgChatInputEventPayloadSourceVariant3),
    Variant4(ToolPkgChatInputEventPayloadSourceVariant4),
    Variant5(String),
}
/// Identifies submission through the send action.
pub enum ToolPkgChatInputEventPayloadSubmitSourceVariant1 {
    Send,
}
/// Identifies submission through a UI button.
pub enum ToolPkgChatInputEventPayloadSubmitSourceVariant2 {
    Button,
}
/// Identifies submission through the input method send action.
pub enum ToolPkgChatInputEventPayloadSubmitSourceVariant3 {
    ImeSend,
}
/// Identifies submission through the Enter key.
pub enum ToolPkgChatInputEventPayloadSubmitSourceVariant4 {
    Enter,
}
/// Identifies submission by the queued-message flow.
pub enum ToolPkgChatInputEventPayloadSubmitSourceVariant5 {
    Queue,
}
/// Identifies the action that submitted chat input while preserving host-defined actions.
pub enum ToolPkgChatInputEventPayloadSubmitSource {
    Variant1(ToolPkgChatInputEventPayloadSubmitSourceVariant1),
    Variant2(ToolPkgChatInputEventPayloadSubmitSourceVariant2),
    Variant3(ToolPkgChatInputEventPayloadSubmitSourceVariant3),
    Variant4(ToolPkgChatInputEventPayloadSubmitSourceVariant4),
    Variant5(ToolPkgChatInputEventPayloadSubmitSourceVariant5),
    Variant6(String),
}
/// Carries the fixed discriminator for message processing hook events.
pub enum ToolPkgMessageProcessingHookEventBaseType1 {
    MessageProcessing,
}
/// Carries the fixed discriminator for XML render hook events.
pub enum ToolPkgXmlRenderHookEventBaseType1 {
    XmlRender,
}
/// Carries the fixed discriminator for input menu toggle hook events.
pub enum ToolPkgInputMenuToggleHookEventBaseType1 {
    InputMenuToggle,
}
/// Carries the fixed discriminator for navigation entry action hook events.
pub enum ToolPkgNavigationEntryActionHookEventBaseType1 {
    NavigationEntryAction,
}
/// Carries the fixed discriminator for AI provider list models events.
pub enum ToolPkgAiProviderListModelsEventBaseType1 {
    ToolpkgAiProviderListModels,
}
/// Carries the fixed discriminator for AI provider send message events.
pub enum ToolPkgAiProviderSendMessageEventBaseType1 {
    ToolpkgAiProviderSendMessage,
}
/// Carries the fixed discriminator for AI provider test connection events.
pub enum ToolPkgAiProviderTestConnectionEventBaseType1 {
    ToolpkgAiProviderTestConnection,
}
/// Carries the fixed discriminator for AI provider calculate input tokens events.
pub enum ToolPkgAiProviderCalculateInputTokensEventBaseType1 {
    ToolpkgAiProviderCalculateInputTokens,
}
/// Selects one of the callbacks supported by an AI provider registration.
pub enum ToolPkgAiProviderHandlerRegistrationFunction {
    Variant1(ToolPkgAiProviderListModelsHandler),
    Variant2(ToolPkgAiProviderSendMessageHandler),
    Variant3(ToolPkgAiProviderTestConnectionHandler),
    Variant4(ToolPkgAiProviderCalculateInputTokensHandler),
}
/// Maps every standard broadcast topic to its one canonical payload type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastDataTypeMap {
    /// Maps application resume events to lifecycle data.
    #[serde(rename = "app.lifecycle.resumed")]
    AppLifecycleResumed(ToolPkgBroadcastLifecycleData),
    /// Maps application inactive events to lifecycle data.
    #[serde(rename = "app.lifecycle.inactive")]
    AppLifecycleInactive(ToolPkgBroadcastLifecycleData),
    /// Maps application pause events to lifecycle data.
    #[serde(rename = "app.lifecycle.paused")]
    AppLifecyclePaused(ToolPkgBroadcastLifecycleData),
    /// Maps application detach events to lifecycle data.
    #[serde(rename = "app.lifecycle.detached")]
    AppLifecycleDetached(ToolPkgBroadcastLifecycleData),
    /// Maps application hidden events to lifecycle data.
    #[serde(rename = "app.lifecycle.hidden")]
    AppLifecycleHidden(ToolPkgBroadcastLifecycleData),
    /// Maps host boot completion events to boot data.
    #[serde(rename = "system.boot.completed")]
    SystemBootCompleted(ToolPkgBroadcastBootData),
    /// Maps external power connection events to power data.
    #[serde(rename = "system.power.connected")]
    SystemPowerConnected(ToolPkgBroadcastPowerConnectionData),
    /// Maps external power disconnection events to power data.
    #[serde(rename = "system.power.disconnected")]
    SystemPowerDisconnected(ToolPkgBroadcastPowerConnectionData),
    /// Maps host sleep events to sleep-state data.
    #[serde(rename = "system.power.sleep")]
    SystemPowerSleep(ToolPkgBroadcastPowerSleepData),
    /// Maps host wake events to sleep-state data.
    #[serde(rename = "system.power.wake")]
    SystemPowerWake(ToolPkgBroadcastPowerSleepData),
    /// Maps low-battery events to battery data.
    #[serde(rename = "system.battery.low")]
    SystemBatteryLow(ToolPkgBroadcastBatteryData),
    /// Maps recovered-battery events to battery data.
    #[serde(rename = "system.battery.okay")]
    SystemBatteryOkay(ToolPkgBroadcastBatteryData),
    /// Maps display-on events to screen data.
    #[serde(rename = "system.screen.on")]
    SystemScreenOn(ToolPkgBroadcastScreenData),
    /// Maps display-off events to screen data.
    #[serde(rename = "system.screen.off")]
    SystemScreenOff(ToolPkgBroadcastScreenData),
    /// Maps user-presence events to presence data.
    #[serde(rename = "system.user.present")]
    SystemUserPresent(ToolPkgBroadcastUserPresenceData),
    /// Maps clock tick events to time data.
    #[serde(rename = "system.time.tick")]
    SystemTimeTick(ToolPkgBroadcastTimeData),
    /// Maps date changes to time data.
    #[serde(rename = "system.date.changed")]
    SystemDateChanged(ToolPkgBroadcastTimeData),
    /// Maps timezone changes to time data.
    #[serde(rename = "system.timezone.changed")]
    SystemTimezoneChanged(ToolPkgBroadcastTimeData),
    /// Maps airplane-mode changes to airplane-mode data.
    #[serde(rename = "system.airplane_mode.changed")]
    SystemAirplaneModeChanged(ToolPkgBroadcastAirplaneModeData),
    /// Maps headset route changes to headset data.
    #[serde(rename = "system.headset.plug")]
    SystemHeadsetPlug(ToolPkgBroadcastHeadsetData),
    /// Maps session lock events to session data.
    #[serde(rename = "system.session.lock")]
    SystemSessionLock(ToolPkgBroadcastSessionData),
    /// Maps session unlock events to session data.
    #[serde(rename = "system.session.unlock")]
    SystemSessionUnlock(ToolPkgBroadcastSessionData),
    /// Maps network changes to network data.
    #[serde(rename = "system.network.changed")]
    SystemNetworkChanged(ToolPkgBroadcastNetworkChangedData),
    /// Maps Bluetooth discovery events to device data.
    #[serde(rename = "bluetooth.device.found")]
    BluetoothDeviceFound(ToolPkgBroadcastBluetoothDeviceData),
    /// Maps Bluetooth name changes to device data.
    #[serde(rename = "bluetooth.device.name_changed")]
    BluetoothDeviceNameChanged(ToolPkgBroadcastBluetoothDeviceData),
    /// Maps Bluetooth device connections to device data.
    #[serde(rename = "bluetooth.device.connected")]
    BluetoothDeviceConnected(ToolPkgBroadcastBluetoothDeviceData),
    /// Maps Bluetooth device disconnections to device data.
    #[serde(rename = "bluetooth.device.disconnected")]
    BluetoothDeviceDisconnected(ToolPkgBroadcastBluetoothDeviceData),
    /// Maps Bluetooth bond-state changes to device data.
    #[serde(rename = "bluetooth.device.bond_state_changed")]
    BluetoothDeviceBondStateChanged(ToolPkgBroadcastBluetoothDeviceData),
    /// Maps Bluetooth adapter connection changes to adapter data.
    #[serde(rename = "bluetooth.adapter.connection_state_changed")]
    BluetoothAdapterConnectionStateChanged(ToolPkgBroadcastAdapterData),
    /// Maps Bluetooth adapter power changes to adapter data.
    #[serde(rename = "bluetooth.adapter.powered_changed")]
    BluetoothAdapterPoweredChanged(ToolPkgBroadcastAdapterData),
}
/// Carries metadata and topic-specific data for a host broadcast.
pub struct ToolPkgHostEventBroadcastPayloadWithData<TTopic> {
    /// Identifies the broadcast topic.
    pub topic: TTopic,
    /// Identifies the source platform.
    pub platform: ToolPkgBroadcastPlatform,
    /// Contains topic-specific broadcast data.
    pub data: ToolPkgBroadcastDataForTopic<TTopic>,
    /// Records when the host event occurred, in epoch milliseconds.
    pub occurredAtMillis: f64,
}
/// Subscribes a host event hook to one broadcast topic.
pub struct ToolPkgHostEventBroadcastTriggerVariant1<TTopic> {
    /// Identifies the concrete kind of trigger or prompt value.
    pub kind: ToolPkgHostEventBroadcastSource,
    /// Selects the broadcast topic observed by this trigger.
    pub topic: TTopic,
    /// Restricts delivery to one host platform when present.
    pub platform: Option<ToolPkgBroadcastPlatform>,
    /// Restricts delivery to the listed host platforms when present.
    pub platforms: Option<Vec<ToolPkgBroadcastPlatform>>,
    /// Prevents a topic list from being supplied for a single-topic trigger.
    pub topics: Option<super::JsNever>,
}
/// Subscribes a host event hook to multiple broadcast topics.
pub struct ToolPkgHostEventBroadcastTriggerVariant2<TTopic> {
    /// Identifies the concrete kind of trigger or prompt value.
    pub kind: ToolPkgHostEventBroadcastSource,
    /// Prevents a single topic from being supplied for a multi-topic trigger.
    pub topic: Option<super::JsNever>,
    /// Selects broadcast topics observed by this trigger.
    pub topics: Vec<TTopic>,
    /// Restricts delivery to one host platform when present.
    pub platform: Option<ToolPkgBroadcastPlatform>,
    /// Restricts delivery to the listed host platforms when present.
    pub platforms: Option<Vec<ToolPkgBroadcastPlatform>>,
}
/// Maps each host event source to its matching trigger configuration.
#[allow(non_camel_case_types)]
pub enum ToolPkgHostEventTriggerTypeMap {
    /// Maps timer sources to timer trigger configuration.
    timer(ToolPkgHostEventTimerTrigger),
    /// Maps interval sources to interval trigger configuration.
    interval(ToolPkgHostEventIntervalTrigger),
    /// Maps broadcast sources to broadcast trigger configuration.
    broadcast(ToolPkgHostEventBroadcastTrigger),
}
/// Maps each host event source to the payload delivered to its handler.
#[allow(non_camel_case_types)]
pub enum ToolPkgHostEventPayloadTypeMap {
    /// Maps timer sources to timer event payloads.
    timer(ToolPkgHostEventTimerPayload<ToolPkgJsonObject>),
    /// Maps interval sources to interval event payloads.
    interval(ToolPkgHostEventIntervalPayload<ToolPkgJsonObject>),
    /// Maps broadcast sources to broadcast event payloads.
    broadcast(ToolPkgHostEventBroadcastPayload<ToolPkgBroadcastTopic>),
}
/// Associates an AI provider with its model-listing callback.
pub struct ToolPkgAiProviderRegistrationListModels {
    /// Provides the callback invoked for this AI provider registration.
    pub function: ToolPkgAiProviderListModelsHandler,
}
/// Associates an AI provider with its message-generation callback.
pub struct ToolPkgAiProviderRegistrationSendMessage {
    /// Provides the callback invoked for this AI provider registration.
    pub function: ToolPkgAiProviderSendMessageHandler,
}
/// Associates an AI provider with its connection-test callback.
pub struct ToolPkgAiProviderRegistrationTestConnection {
    /// Provides the callback invoked for this AI provider registration.
    pub function: ToolPkgAiProviderTestConnectionHandler,
}
/// Associates an AI provider with its input-token-count callback.
pub struct ToolPkgAiProviderRegistrationCalculateInputTokens {
    /// Provides the callback invoked for this AI provider registration.
    pub function: ToolPkgAiProviderCalculateInputTokensHandler,
}
/// Allows a IPC API on callback to complete immediately or asynchronously.
pub enum ToolPkgIpcApiOnHandlerOutput<TResult> {
    Variant1(TResult),
    Variant2(JsFuture<TResult>),
}
/// Allows a IPC API off callback to complete immediately or asynchronously.
pub enum ToolPkgIpcApiOffHandlerOutput<TResult> {
    Variant1(TResult),
    Variant2(JsFuture<TResult>),
}
/// Accepts either plain text or translations keyed by language tag.
pub enum ToolPkgLocalizedText {
    Variant1(String),
    Variant2(ToolPkgLocalizedTextVariant2),
}
/// Enumerates scalar values supported by ToolPkg JSON payloads.
pub enum ToolPkgJsonPrimitive {
    Variant1(String),
    Variant2(f64),
    Variant3(bool),
    Null,
    Undefined,
}
/// Represents recursively nested JSON exchanged by hooks, providers, and IPC calls.
pub enum ToolPkgJsonValue {
    Variant1(ToolPkgJsonPrimitive),
    Variant2(Vec<ToolPkgJsonValue>),
    Variant3(ToolPkgJsonValueVariant3),
}
/// Stores a ToolPkg JSON object as values keyed by property name.
pub struct ToolPkgJsonObject {
    /// Stores arbitrary JSON properties keyed by name.
    pub additional_properties: BTreeMap<String, ToolPkgJsonValue>,
}
/// Names application and activity lifecycle callbacks exposed to plugins.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgAppLifecycleEvent {
    #[serde(rename = "application_on_create")]
    ApplicationOnCreate,
    #[serde(rename = "application_on_foreground")]
    ApplicationOnForeground,
    #[serde(rename = "application_on_background")]
    ApplicationOnBackground,
    #[serde(rename = "application_on_low_memory")]
    ApplicationOnLowMemory,
    #[serde(rename = "application_on_trim_memory")]
    ApplicationOnTrimMemory,
    #[serde(rename = "application_on_terminate")]
    ApplicationOnTerminate,
    #[serde(rename = "activity_on_create")]
    ActivityOnCreate,
    #[serde(rename = "activity_on_start")]
    ActivityOnStart,
    #[serde(rename = "activity_on_resume")]
    ActivityOnResume,
    #[serde(rename = "activity_on_pause")]
    ActivityOnPause,
    #[serde(rename = "activity_on_stop")]
    ActivityOnStop,
    #[serde(rename = "activity_on_destroy")]
    ActivityOnDestroy,
}
/// Enumerates every hook event that a ToolPkg plugin may register.
pub enum ToolPkgHookEventName {
    Variant1(ToolPkgAppLifecycleEvent),
    Variant2(ToolPkgHookEventNameVariant2),
    Variant3(ToolPkgHookEventNameVariant3),
    Variant4(ToolPkgHookEventNameVariant4),
    Variant5(ToolPkgChatInputEventName),
    Variant6(ToolPkgChatViewEventName),
    Variant7(ToolPkgHookEventNameVariant7),
    Variant8(ToolPkgToolLifecycleEventName),
    Variant9(ToolPkgPromptInputEventName),
    Variant10(ToolPkgPromptHistoryEventName),
    Variant11(ToolPkgSystemPromptComposeEventName),
    Variant12(ToolPkgToolPromptComposeEventName),
    Variant13(ToolPkgPromptFinalizeEventName),
    Variant14(ToolPkgSummaryGenerateEventName),
    Variant15(ToolPkgHostEventName),
}
/// Accepts a JSON result, no result, or asynchronous completion from a generic hook.
pub enum ToolPkgHookReturn {
    Variant1(ToolPkgJsonValue),
    Variant2(()),
    Variant3(JsFuture<ToolPkgHookReturnVariant3Output>),
}
/// Callback invoked when a  event is dispatched.
pub type ToolPkgHookHandler<TEvent> = Arc<dyn Fn(TEvent) -> ToolPkgHookReturn + Send + Sync>;
/// Enumerates immediate and asynchronous results accepted from an application lifecycle hook.
pub enum ToolPkgAppLifecycleHookReturn {
    Variant1(ToolPkgJsonValue),
    Variant2(()),
    Variant3(JsFuture<ToolPkgAppLifecycleHookReturnVariant3Output>),
}
/// Controls how message processing reports a match and replacement content.
pub struct ToolPkgMessageProcessingHookObjectResult {
    /// Preserves additional JSON properties supplied with this message processing hook object result.
    pub base_json_object: ToolPkgJsonObject,
    /// Reports whether message processing matched this plugin.
    pub matched: Option<bool>,
    /// Contains text produced, replaced, or inspected by this operation.
    pub text: Option<String>,
    /// Supplies content produced or transformed by the hook.
    pub content: Option<String>,
    /// Supplies replacement message chunks produced by processing.
    pub chunks: Option<Vec<String>>,
}
/// Enumerates supported message processing hook return value values.
pub enum ToolPkgMessageProcessingHookReturnValue {
    Variant1(bool),
    Variant2(String),
    Variant3(ToolPkgMessageProcessingHookObjectResult),
    Null,
    Void,
}
/// Enumerates immediate and asynchronous results accepted from a message processing hook.
pub enum ToolPkgMessageProcessingHookReturn {
    Variant1(ToolPkgMessageProcessingHookReturnValue),
    Variant2(JsFuture<ToolPkgMessageProcessingHookReturnValue>),
}
/// Describes text, content, or Compose DSL produced while handling an XML element.
pub struct ToolPkgXmlRenderHookObjectResult {
    /// Reports whether the XML hook handled the supplied element.
    pub handled: Option<bool>,
    /// Contains text produced, replaced, or inspected by this operation.
    pub text: Option<String>,
    /// Supplies content produced or transformed by the hook.
    pub content: Option<String>,
    /// Supplies a Compose DSL screen produced by the XML renderer.
    pub composeDsl: Option<ToolPkgXmlRenderHookObjectResultComposeDsl>,
}
/// Enumerates immediate and asynchronous results accepted from an XML-render hook.
pub enum ToolPkgXmlRenderHookReturn {
    Variant1(String),
    Variant2(ToolPkgXmlRenderHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgXmlRenderHookReturnVariant5Output>),
}
/// Selects the input-menu section in which a toggle is displayed.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgInputMenuToggleSlot {
    #[serde(rename = "thinking")]
    Thinking,
    #[serde(rename = "memory")]
    Memory,
    #[serde(rename = "model")]
    Model,
    #[serde(rename = "tools")]
    Tools,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "default")]
    Default,
}
/// Describes one toggle contributed to the chat input menu.
pub struct ToolPkgInputMenuToggleDefinitionResult {
    /// Preserves additional JSON properties supplied with this input menu toggle definition result.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies this input menu toggle definition result within its owning package.
    pub id: String,
    /// Provides primary text displayed by the host UI.
    pub title: String,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: Option<String>,
    /// Provides the icon name displayed by the host UI.
    pub icon: Option<String>,
    /// Sets the current checked state of an input-menu toggle.
    pub isChecked: Option<bool>,
    /// Selects the chat input menu section containing this toggle.
    pub slot: Option<String>,
}
/// Wraps toggle definitions returned by an input-menu hook.
pub struct ToolPkgInputMenuToggleObjectResult {
    /// Preserves additional JSON properties supplied with this input menu toggle object result.
    pub base_json_object: ToolPkgJsonObject,
    /// Contains toggle definitions contributed to the input menu.
    pub toggles: Option<Vec<ToolPkgInputMenuToggleDefinitionResult>>,
}
/// Enumerates immediate and asynchronous results accepted from an input-menu-toggle hook.
pub enum ToolPkgInputMenuToggleHookReturn {
    Variant1(Vec<ToolPkgInputMenuToggleDefinitionResult>),
    Variant2(ToolPkgInputMenuToggleObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgInputMenuToggleHookReturnVariant5Output>),
}
/// Names the stages at which chat input hooks run.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgChatInputEventName {
    #[serde(rename = "input_changed")]
    InputChanged,
    #[serde(rename = "submit_requested")]
    SubmitRequested,
    #[serde(rename = "submitted")]
    Submitted,
}
/// Names the open, update, and close stages of a chat view.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgChatViewEventName {
    #[serde(rename = "view_opened")]
    ViewOpened,
    #[serde(rename = "view_updated")]
    ViewUpdated,
    #[serde(rename = "view_closed")]
    ViewClosed,
}
/// Controls chat input handling and optionally supplies replacement text or metadata.
pub struct ToolPkgChatInputHookObjectResult {
    /// Preserves additional JSON properties supplied with this chat input hook object result.
    pub base_json_object: ToolPkgJsonObject,
    /// Controls whether the host allows, blocks, replaces, or consumes chat input.
    pub action: Option<ToolPkgChatInputHookObjectResultAction>,
    /// Contains text produced, replaced, or inspected by this operation.
    pub text: Option<String>,
    /// Provides a user-facing explanation for the hook decision.
    pub message: Option<String>,
    /// Requests that the host clear chat input after handling.
    pub clearInput: Option<bool>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgJsonObject>,
}
/// Enumerates immediate and asynchronous results accepted from a chat input hook.
pub enum ToolPkgChatInputHookReturn {
    Variant1(String),
    Variant2(ToolPkgChatInputHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgChatInputHookReturnVariant5Output>),
}
/// Names permission, execution, result, and completion stages of a tool call.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgToolLifecycleEventName {
    /// Runs before a tool call is accepted by the execution pipeline.
    #[serde(rename = "tool_call_intercept")]
    ToolCallIntercept,
    #[serde(rename = "tool_call_requested")]
    ToolCallRequested,
    #[serde(rename = "tool_permission_checked")]
    ToolPermissionChecked,
    #[serde(rename = "tool_execution_started")]
    ToolExecutionStarted,
    #[serde(rename = "tool_execution_result")]
    ToolExecutionResult,
    #[serde(rename = "tool_execution_error")]
    ToolExecutionError,
    #[serde(rename = "tool_execution_finished")]
    ToolExecutionFinished,
}
/// Names the stages before and after user-input processing.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgPromptInputEventName {
    #[serde(rename = "before_process")]
    BeforeProcess,
    #[serde(rename = "after_process")]
    AfterProcess,
}
/// Names the stages before and after prompt-history preparation.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgPromptHistoryEventName {
    #[serde(rename = "before_prepare_history")]
    BeforePrepareHistory,
    #[serde(rename = "after_prepare_history")]
    AfterPrepareHistory,
}
/// Names the stages used to assemble a system prompt.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgSystemPromptComposeEventName {
    #[serde(rename = "before_compose_system_prompt")]
    BeforeComposeSystemPrompt,
    #[serde(rename = "compose_system_prompt_sections")]
    ComposeSystemPromptSections,
    #[serde(rename = "after_compose_system_prompt")]
    AfterComposeSystemPrompt,
}
/// Names the stages used to filter and assemble tool prompts.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgToolPromptComposeEventName {
    #[serde(rename = "before_compose_tool_prompt")]
    BeforeComposeToolPrompt,
    #[serde(rename = "filter_tool_prompt_items")]
    FilterToolPromptItems,
    #[serde(rename = "filter_tool_call_tools")]
    FilterToolCallTools,
    #[serde(rename = "after_compose_tool_prompt")]
    AfterComposeToolPrompt,
}
/// Names the final prompt stages before a model request.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgPromptFinalizeEventName {
    #[serde(rename = "before_finalize_prompt")]
    BeforeFinalizePrompt,
    #[serde(rename = "before_send_to_model")]
    BeforeSendToModel,
}
/// Names the stages used to prepare and generate a conversation summary.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgSummaryGenerateEventName {
    #[serde(rename = "before_prepare_summary_prompt")]
    BeforePrepareSummaryPrompt,
    #[serde(rename = "before_send_to_model")]
    BeforeSendToModel,
    #[serde(rename = "after_generate_summary")]
    AfterGenerateSummary,
}
/// Identifies the role and purpose of one prompt-history turn.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgPromptTurnKind {
    #[serde(rename = "SYSTEM")]
    SYSTEM,
    #[serde(rename = "USER")]
    USER,
    #[serde(rename = "ASSISTANT")]
    ASSISTANT,
    #[serde(rename = "TOOL_CALL")]
    TOOLCALL,
    #[serde(rename = "TOOL_RESULT")]
    TOOLRESULT,
    #[serde(rename = "SUMMARY")]
    SUMMARY,
}
/// Contains one typed turn in prepared prompt history.
pub struct ToolPkgPromptTurn {
    /// Preserves additional JSON properties supplied with this prompt turn.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the concrete kind of trigger or prompt value.
    pub kind: ToolPkgPromptTurnKind,
    /// Supplies content produced or transformed by the hook.
    pub content: String,
    /// Identifies the tool associated with this event or prompt entry.
    pub toolName: Option<String>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgJsonObject>,
}
/// Enumerates supported active prompt type values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgActivePromptType {
    /// Identifies a character-card prompt.
    #[serde(rename = "character_card")]
    CharacterCard,
    /// Identifies a character-group prompt.
    #[serde(rename = "character_group")]
    CharacterGroup,
}
/// Captures the identity of the character prompt active for a hook.
pub struct ToolPkgActivePromptSnapshot {
    /// Preserves additional JSON properties supplied with this active prompt snapshot.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the semantic kind of this value.
    pub r#type: ToolPkgActivePromptType,
    /// Identifies this active prompt snapshot within its owning package.
    pub id: String,
    /// Provides the stable or user-facing name of this active prompt snapshot.
    pub name: String,
}
/// Carries contextual metadata shared across prompt hooks.
pub struct ToolPkgHookMetadata {
    /// Preserves additional JSON properties supplied with this hook metadata.
    pub base_json_object: ToolPkgJsonObject,
    /// Captures the character prompt currently active for this hook.
    pub activePrompt: Option<ToolPkgActivePromptSnapshot>,
}
/// Carries tool request, permission, execution, and result data.
pub struct ToolPkgToolLifecycleEventPayload {
    /// Preserves additional JSON properties supplied with this tool lifecycle event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the tool associated with this event or prompt entry.
    pub toolName: String,
    /// Contains parameters associated with this tool or route.
    pub parameters: Option<ToolPkgToolLifecycleEventPayloadParameters>,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: Option<String>,
    /// Reports whether tool execution permission was granted.
    pub granted: Option<bool>,
    /// Explains a permission or lifecycle decision.
    pub reason: Option<String>,
    /// Reports whether the operation completed successfully.
    pub success: Option<bool>,
    /// Provides the tool execution error reported by the host.
    pub errorMessage: Option<String>,
    /// Contains the text result returned by the tool.
    pub resultText: Option<String>,
    /// Contains the structured result returned by the tool.
    pub resultJson: Option<ToolPkgJsonValue>,
}
/// Describes one parameter exposed in a model-facing tool prompt.
pub struct ToolPkgToolPromptParameter {
    /// Preserves additional JSON properties supplied with this tool prompt parameter.
    pub base_json_object: ToolPkgJsonObject,
    /// Provides the stable or user-facing name of this tool prompt parameter.
    pub name: String,
    /// Identifies the semantic kind of this value.
    pub r#type: Option<String>,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: String,
    /// Marks whether the model must supply this parameter.
    pub required: Option<bool>,
    /// Provides the parameter value used when none is supplied.
    pub default: Option<ToolPkgJsonPrimitive>,
}
/// Describes one tool entry made available to the model.
pub struct ToolPkgToolPromptItem {
    /// Preserves additional JSON properties supplied with this tool prompt item.
    pub base_json_object: ToolPkgJsonObject,
    /// Groups this tool under a model-facing category.
    pub categoryName: String,
    /// Provides optional text shown before this tool category.
    pub categoryHeader: Option<String>,
    /// Provides optional text shown after this tool category.
    pub categoryFooter: Option<String>,
    /// Provides the stable or user-facing name of this tool prompt item.
    pub name: String,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: String,
    /// Contains parameters associated with this tool or route.
    pub parameters: Option<String>,
    /// Provides extended model-facing details for the tool.
    pub details: Option<String>,
    /// Provides additional model-facing usage guidance.
    pub notes: Option<String>,
    /// Provides machine-readable tool parameter definitions.
    pub parametersStructured: Option<Vec<ToolPkgToolPromptParameter>>,
}
/// Contains prompt fields that a hook may replace for subsequent stages.
pub struct ToolPkgPromptHookObjectResult {
    /// Preserves additional JSON properties supplied with this prompt hook object result.
    pub base_json_object: ToolPkgJsonObject,
    /// Contains user input before prompt processing.
    pub rawInput: Option<String>,
    /// Contains user input after the current processing stage.
    pub processedInput: Option<String>,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains conversation turns after host preparation.
    pub preparedHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains the system prompt assembled at this hook stage.
    pub systemPrompt: Option<String>,
    /// Contains the model-facing tool prompt at this hook stage.
    pub toolPrompt: Option<String>,
    /// Lists the tools currently available for model invocation.
    pub availableTools: Option<Vec<ToolPkgToolPromptItem>>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgHookMetadata>,
}
/// Contains summary inputs and output that a hook may replace.
pub struct ToolPkgSummaryHookObjectResult {
    /// Preserves additional JSON properties supplied with this summary hook object result.
    pub base_json_object: ToolPkgJsonObject,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains conversation turns after host preparation.
    pub preparedHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains the system prompt assembled at this hook stage.
    pub systemPrompt: Option<String>,
    /// Contains the prompt sent to the model for summarization.
    pub summaryPrompt: Option<String>,
    /// Contains the summary generated at this stage.
    pub summaryResult: Option<String>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgHookMetadata>,
}
/// Carries the current prompt-building state to prompt hooks.
pub struct ToolPkgPromptHookEventPayload {
    /// Preserves additional JSON properties supplied with this prompt hook event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Names the current prompt or summary processing stage.
    pub stage: Option<String>,
    /// Identifies the conversation associated with the event.
    pub chatId: Option<String>,
    /// Identifies the host function participating in prompt construction.
    pub functionType: Option<String>,
    /// Identifies the prompt-building function active for this hook.
    pub promptFunctionType: Option<String>,
    /// Requests English prompt text from the host pipeline.
    pub useEnglish: Option<bool>,
    /// Contains user input before prompt processing.
    pub rawInput: Option<String>,
    /// Contains user input after the current processing stage.
    pub processedInput: Option<String>,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains conversation turns after host preparation.
    pub preparedHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains the system prompt assembled at this hook stage.
    pub systemPrompt: Option<String>,
    /// Contains the model-facing tool prompt at this hook stage.
    pub toolPrompt: Option<String>,
    /// Contains model configuration active for this request.
    pub modelParameters: Option<Vec<ToolPkgJsonObject>>,
    /// Lists the tools currently available for model invocation.
    pub availableTools: Option<Vec<ToolPkgToolPromptItem>>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgHookMetadata>,
}
/// Carries the current summary-generation state to summary hooks.
pub struct ToolPkgSummaryGenerateEventPayload {
    /// Preserves additional JSON properties supplied with this summary generate event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Names the current prompt or summary processing stage.
    pub stage: Option<String>,
    /// Identifies the host function participating in prompt construction.
    pub functionType: Option<String>,
    /// Requests English prompt text from the host pipeline.
    pub useEnglish: Option<bool>,
    /// Contains the summary from the preceding summarization cycle.
    pub previousSummary: Option<String>,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains conversation turns after host preparation.
    pub preparedHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Contains the system prompt assembled at this hook stage.
    pub systemPrompt: Option<String>,
    /// Contains the prompt sent to the model for summarization.
    pub summaryPrompt: Option<String>,
    /// Contains the summary generated at this stage.
    pub summaryResult: Option<String>,
    /// Contains model configuration active for this request.
    pub modelParameters: Option<Vec<ToolPkgJsonObject>>,
    /// Carries structured context for later hook stages.
    pub metadata: Option<ToolPkgHookMetadata>,
}
/// Enumerates immediate and asynchronous results accepted from a tool lifecycle hook.
pub enum ToolPkgToolLifecycleHookReturn {
    Variant1(()),
    Variant2(JsFuture<()>),
}
/// Enumerates immediate and asynchronous results accepted from a prompt input hook.
pub enum ToolPkgPromptInputHookReturn {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgPromptInputHookReturnVariant5Output>),
}
/// Enumerates immediate and asynchronous results accepted from a prompt history hook.
pub enum ToolPkgPromptHistoryHookReturn {
    Variant1(Vec<ToolPkgPromptTurn>),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgPromptHistoryHookReturnVariant5Output>),
}
/// Enumerates immediate and asynchronous results accepted from a system prompt compose hook.
pub enum ToolPkgSystemPromptComposeHookReturn {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgSystemPromptComposeHookReturnVariant5Output>),
}
/// Enumerates immediate and asynchronous results accepted from a tool prompt compose hook.
pub enum ToolPkgToolPromptComposeHookReturn {
    Variant1(String),
    Variant2(ToolPkgPromptHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgToolPromptComposeHookReturnVariant5Output>),
}
/// Enumerates immediate and asynchronous results accepted from a prompt finalize hook.
pub enum ToolPkgPromptFinalizeHookReturn {
    Variant1(String),
    Variant2(Vec<ToolPkgPromptTurn>),
    Variant3(ToolPkgPromptHookObjectResult),
    Null,
    Void,
    Variant6(JsFuture<ToolPkgPromptFinalizeHookReturnVariant6Output>),
}
/// Enumerates immediate and asynchronous results accepted from a summary generate hook.
pub enum ToolPkgSummaryGenerateHookReturn {
    Variant1(String),
    Variant2(ToolPkgSummaryHookObjectResult),
    Null,
    Void,
    Variant5(JsFuture<ToolPkgSummaryGenerateHookReturnVariant5Output>),
}
/// Callback invoked when an application or activity lifecycle event is dispatched.
pub type ToolPkgAppLifecycleHookHandler =
    Arc<dyn Fn(ToolPkgAppLifecycleHookEvent) -> ToolPkgAppLifecycleHookReturn + Send + Sync>;
/// Callback invoked when a message processing event is dispatched.
pub type ToolPkgMessageProcessingHookHandler = Arc<
    dyn Fn(ToolPkgMessageProcessingHookEvent) -> ToolPkgMessageProcessingHookHandlerOutput
        + Send
        + Sync,
>;
/// Callback invoked when the host requests rendering for a registered XML tag.
pub type ToolPkgXmlRenderHookHandler =
    Arc<dyn Fn(ToolPkgXmlRenderHookEvent) -> ToolPkgXmlRenderHookReturn + Send + Sync>;
/// Callback invoked when input-menu toggles are requested or changed.
pub type ToolPkgInputMenuToggleHookHandler =
    Arc<dyn Fn(ToolPkgInputMenuToggleHookEvent) -> ToolPkgInputMenuToggleHookReturn + Send + Sync>;
/// Callback invoked when a chat input event is dispatched.
pub type ToolPkgChatInputHookHandler =
    Arc<dyn Fn(ToolPkgChatInputHookEvent) -> ToolPkgChatInputHookReturn + Send + Sync>;
/// Callback invoked when a navigation entry action event is dispatched.
pub type ToolPkgNavigationEntryActionHookHandler =
    Arc<dyn Fn(ToolPkgNavigationEntryActionHookEvent) -> ToolPkgHookReturn + Send + Sync>;
/// Callback invoked when a tool lifecycle event is dispatched.
pub type ToolPkgToolLifecycleHookHandler =
    Arc<dyn Fn(ToolPkgToolLifecycleHookEvent) -> ToolPkgToolLifecycleHookReturn + Send + Sync>;
/// Callback invoked when a prompt input event is dispatched.
pub type ToolPkgPromptInputHookHandler =
    Arc<dyn Fn(ToolPkgPromptInputHookEvent) -> ToolPkgPromptInputHookReturn + Send + Sync>;
/// Callback invoked when a prompt history event is dispatched.
pub type ToolPkgPromptHistoryHookHandler =
    Arc<dyn Fn(ToolPkgPromptHistoryHookEvent) -> ToolPkgPromptHistoryHookReturn + Send + Sync>;
/// Callback invoked when a prompt estimate history event is dispatched.
pub type ToolPkgPromptEstimateHistoryHookHandler = Arc<
    dyn Fn(ToolPkgPromptEstimateHistoryHookEvent) -> ToolPkgPromptHistoryHookReturn + Send + Sync,
>;
/// Callback invoked when a system prompt compose event is dispatched.
pub type ToolPkgSystemPromptComposeHookHandler = Arc<
    dyn Fn(ToolPkgSystemPromptComposeHookEvent) -> ToolPkgSystemPromptComposeHookReturn
        + Send
        + Sync,
>;
/// Callback invoked when a tool prompt compose event is dispatched.
pub type ToolPkgToolPromptComposeHookHandler = Arc<
    dyn Fn(ToolPkgToolPromptComposeHookEvent) -> ToolPkgToolPromptComposeHookReturn + Send + Sync,
>;
/// Callback invoked when a prompt finalize event is dispatched.
pub type ToolPkgPromptFinalizeHookHandler =
    Arc<dyn Fn(ToolPkgPromptFinalizeHookEvent) -> ToolPkgPromptFinalizeHookReturn + Send + Sync>;
/// Callback invoked when a prompt estimate finalize event is dispatched.
pub type ToolPkgPromptEstimateFinalizeHookHandler = Arc<
    dyn Fn(ToolPkgPromptEstimateFinalizeHookEvent) -> ToolPkgPromptFinalizeHookReturn + Send + Sync,
>;
/// Callback invoked when a summary generate event is dispatched.
pub type ToolPkgSummaryGenerateHookHandler =
    Arc<dyn Fn(ToolPkgSummaryGenerateHookEvent) -> ToolPkgSummaryGenerateHookReturn + Send + Sync>;
/// Carries a hook discriminator, typed payload, package identity, and dispatch metadata.
pub struct ToolPkgHookEventBase<TEventName, TPayload> {
    /// Identifies the hook event being dispatched.
    pub event: TEventName,
    /// Repeats the typed event name for handler-friendly access.
    pub eventName: TEventName,
    /// Contains data specific to the dispatched event.
    pub eventPayload: TPayload,
    /// Identifies the ToolPkg package that dispatched the hook.
    pub toolPkgId: Option<String>,
    /// Identifies the ToolPkg container that dispatched the hook.
    pub containerPackageName: Option<String>,
    /// Identifies the plugin function selected for dispatch.
    pub functionName: Option<String>,
    /// Identifies the plugin that owns the hook.
    pub pluginId: Option<String>,
    /// Identifies the registered hook that received the event.
    pub hookId: Option<String>,
    /// Records when the hook was dispatched, in epoch milliseconds.
    pub timestampMs: Option<f64>,
}
/// Carries app lifecycle data supplied when the event is dispatched.
pub struct ToolPkgAppLifecycleEventPayload {
    /// Preserves additional JSON properties supplied with this app lifecycle event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Carries host-specific data without a dedicated field.
    pub extras: Option<ToolPkgJsonObject>,
}
/// Carries message processing data supplied when the event is dispatched.
pub struct ToolPkgMessageProcessingEventPayload {
    /// Preserves additional JSON properties supplied with this message processing event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the conversation associated with the event.
    pub chatId: Option<String>,
    /// Contains the message text being processed.
    pub messageContent: Option<String>,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    /// Provides the workspace path associated with the conversation.
    pub workspacePath: Option<String>,
    /// Provides the maximum token budget for message processing.
    pub maxTokens: Option<f64>,
    /// Provides the token-usage threshold for message processing.
    pub tokenUsageThreshold: Option<f64>,
    /// Requests match detection without applying message changes.
    pub probeOnly: Option<bool>,
    /// Correlates this event with a specific execution attempt.
    pub executionId: Option<String>,
}
/// Carries XML render data supplied when the event is dispatched.
pub struct ToolPkgXmlRenderEventPayload {
    /// Preserves additional JSON properties supplied with this XML render event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Contains the XML fragment supplied to the renderer.
    pub xmlContent: Option<String>,
    /// Identifies the XML tag currently being rendered.
    pub tagName: Option<String>,
}
/// Carries input menu toggle data supplied when the event is dispatched.
pub struct ToolPkgInputMenuToggleEventPayload {
    /// Preserves additional JSON properties supplied with this input menu toggle event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Selects whether toggle definitions are requested or a toggle is changed.
    pub action: Option<ToolPkgInputMenuToggleEventPayloadAction>,
    /// Identifies the input-menu toggle being changed.
    pub toggleId: Option<String>,
    /// Identifies the conversation associated with the event.
    pub chatId: Option<String>,
    /// Identifies the runtime that owns or emitted this value.
    pub runtime: Option<String>,
}
/// Carries chat input data supplied when the event is dispatched.
pub struct ToolPkgChatInputEventPayload {
    /// Preserves additional JSON properties supplied with this chat input event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the conversation associated with the event.
    pub chatId: Option<String>,
    /// Contains text produced, replaced, or inspected by this operation.
    pub text: Option<String>,
    /// Provides the start offset of the text selection.
    pub selectionStart: Option<f64>,
    /// Provides the exclusive end offset of the text selection.
    pub selectionEnd: Option<f64>,
    /// Reports whether chat input includes attachments.
    pub hasAttachments: Option<bool>,
    /// Reports how many attachments accompany the chat input.
    pub attachmentCount: Option<f64>,
    /// Reports whether the chat is processing a request.
    pub isProcessing: Option<bool>,
    /// Identifies the active chat input mode.
    pub inputStyle: Option<ToolPkgChatInputEventPayloadInputStyle>,
    /// Identifies where this value originated.
    pub source: Option<ToolPkgChatInputEventPayloadSource>,
    /// Identifies the action that submitted chat input.
    pub submitSource: Option<ToolPkgChatInputEventPayloadSubmitSource>,
}
/// Carries chat view data supplied when the event is dispatched.
pub struct ToolPkgChatViewEventPayload {
    /// Preserves additional JSON properties supplied with this chat view event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the chat view that emitted the event.
    pub viewId: Option<String>,
    /// Identifies the conversation associated with the event.
    pub chatId: Option<String>,
    /// Provides the workspace path associated with the conversation.
    pub workspacePath: Option<String>,
    /// Contains serialized workspace environment data.
    pub workspaceEnv: Option<String>,
    /// Identifies the runtime that owns or emitted this value.
    pub runtime: Option<String>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<String>,
}
/// Carries navigation entry action data supplied when the event is dispatched.
pub struct ToolPkgNavigationEntryActionEventPayload {
    /// Preserves additional JSON properties supplied with this navigation entry action event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the navigation entry that emitted the action.
    pub entryId: Option<String>,
    /// Identifies the route opened by this contribution.
    pub routeId: Option<String>,
    /// Selects the host navigation surface containing this entry.
    pub surface: Option<String>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<String>,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: Option<String>,
}
/// Combines shared dispatch metadata with an application lifecycle payload.
pub struct ToolPkgAppLifecycleHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this app lifecycle hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgAppLifecycleEvent, ToolPkgAppLifecycleEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a message processing hook.
pub struct ToolPkgMessageProcessingHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this message processing hook event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgMessageProcessingHookEventBaseType1,
        ToolPkgMessageProcessingEventPayload,
    >,
}
/// Combines shared dispatch metadata with an XML-render payload.
pub struct ToolPkgXmlRenderHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this XML render hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgXmlRenderHookEventBaseType1, ToolPkgXmlRenderEventPayload>,
}
/// Combines shared dispatch metadata with an input-menu-toggle payload.
pub struct ToolPkgInputMenuToggleHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this input menu toggle hook event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgInputMenuToggleHookEventBaseType1,
        ToolPkgInputMenuToggleEventPayload,
    >,
}
/// Combines shared dispatch metadata with the typed payload for a chat input hook.
pub struct ToolPkgChatInputHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this chat input hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgChatInputEventName, ToolPkgChatInputEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a chat view hook.
pub struct ToolPkgChatViewHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this chat view hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgChatViewEventName, ToolPkgChatViewEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a navigation entry action hook.
pub struct ToolPkgNavigationEntryActionHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this navigation entry action hook event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgNavigationEntryActionHookEventBaseType1,
        ToolPkgNavigationEntryActionEventPayload,
    >,
}
/// Combines shared dispatch metadata with the typed payload for a tool lifecycle hook.
pub struct ToolPkgToolLifecycleHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this tool lifecycle hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgToolLifecycleEventName, ToolPkgToolLifecycleEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a prompt input hook.
pub struct ToolPkgPromptInputHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this prompt input hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgPromptInputEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a prompt history hook.
pub struct ToolPkgPromptHistoryHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this prompt history hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgPromptHistoryEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a prompt estimate history hook.
pub struct ToolPkgPromptEstimateHistoryHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this prompt estimate history hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgPromptHistoryEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a system prompt compose hook.
pub struct ToolPkgSystemPromptComposeHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this system prompt compose hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgSystemPromptComposeEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a tool prompt compose hook.
pub struct ToolPkgToolPromptComposeHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this tool prompt compose hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgToolPromptComposeEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a prompt finalize hook.
pub struct ToolPkgPromptFinalizeHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this prompt finalize hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgPromptFinalizeEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a prompt estimate finalize hook.
pub struct ToolPkgPromptEstimateFinalizeHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this prompt estimate finalize hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgPromptFinalizeEventName, ToolPkgPromptHookEventPayload>,
}
/// Combines shared dispatch metadata with the typed payload for a summary generate hook.
pub struct ToolPkgSummaryGenerateHookEvent {
    /// Carries shared dispatch metadata and the typed payload for this summary generate hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgSummaryGenerateEventName, ToolPkgSummaryGenerateEventPayload>,
}
/// Contains host configuration supplied to registered AI provider callbacks.
pub struct ToolPkgAiProviderConfig {
    /// Preserves additional JSON properties supplied with this AI provider config.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies this AI provider config within its owning package.
    pub id: String,
    /// Provides the stable or user-facing name of this AI provider config.
    pub name: String,
    /// Names the provider implementation family.
    pub apiProviderType: String,
    /// Identifies the configured provider implementation.
    pub apiProviderTypeId: String,
    /// Provides the credential used to authenticate provider requests.
    pub apiKey: String,
    /// Provides the base endpoint for provider API requests.
    pub apiEndpoint: String,
    /// Identifies the model selected for this provider request.
    pub modelName: String,
    /// Adds provider-specific HTTP headers to requests.
    pub customHeaders: ToolPkgJsonObject,
    /// Adds provider-specific request parameters.
    pub customParameters: Vec<ToolPkgJsonValue>,
    /// Enables direct image processing for provider requests.
    pub enableDirectImageProcessing: bool,
    /// Enables direct audio processing for provider requests.
    pub enableDirectAudioProcessing: bool,
    /// Enables direct video processing for provider requests.
    pub enableDirectVideoProcessing: bool,
    /// Enables Google Search augmentation for provider requests.
    pub enableGoogleSearch: bool,
    /// Enables Claude's one-hour prompt-cache policy.
    pub enableClaude1hPromptCache: bool,
    /// Allows the provider to issue tool calls.
    pub enableToolCall: bool,
    /// Limits provider requests issued per minute.
    pub requestLimitPerMinute: f64,
    /// Sets the maximum concurrent requests allowed for this provider.
    pub maxConcurrentRequests: f64,
    /// Provides the locale requested by the host.
    pub locale: Option<String>,
}
/// Carries provider configuration and locale shared by AI provider operations.
pub struct ToolPkgAiProviderBaseEventPayload {
    /// Preserves additional JSON properties supplied with this AI provider base event payload.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies the provider registration handling the event.
    pub providerId: String,
    /// Provides the provider name displayed by the host.
    pub providerDisplayName: Option<String>,
    /// Provides the host-facing provider description.
    pub providerDescription: Option<String>,
    /// Contains active host configuration for the provider.
    pub config: ToolPkgAiProviderConfig,
}
/// Carries a request to list the models available from an AI provider.
pub struct ToolPkgAiProviderListModelsEvent {
    /// Carries shared dispatch metadata and the typed payload for this AI provider list models event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgAiProviderListModelsEventBaseType1,
        ToolPkgAiProviderBaseEventPayload,
    >,
}
/// Carries AI provider send message data supplied when the event is dispatched.
pub struct ToolPkgAiProviderSendMessageEventPayload {
    /// Carries provider configuration and locale shared by this AI provider request.
    pub base_ai_provider_base_event_payload: ToolPkgAiProviderBaseEventPayload,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Vec<ToolPkgPromptTurn>,
    /// Contains model configuration active for this request.
    pub modelParameters: Option<Vec<ToolPkgJsonObject>>,
    /// Lists the tools currently available for model invocation.
    pub availableTools: Option<Vec<ToolPkgJsonObject>>,
    /// Enables the provider's extended reasoning mode.
    pub enableThinking: Option<bool>,
    /// Controls whether the provider streams generated output.
    pub stream: Option<bool>,
    /// Keeps provider reasoning content in conversation history.
    pub preserveThinkInHistory: Option<bool>,
    /// Allows a failed provider request to be retried.
    pub enableRetry: Option<bool>,
}
/// Carries a message-generation request dispatched to an AI provider.
pub struct ToolPkgAiProviderSendMessageEvent {
    /// Carries shared dispatch metadata and the typed payload for this AI provider send message event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgAiProviderSendMessageEventBaseType1,
        ToolPkgAiProviderSendMessageEventPayload,
    >,
}
/// Carries a connection-test request dispatched to an AI provider.
pub struct ToolPkgAiProviderTestConnectionEvent {
    /// Carries shared dispatch metadata and the typed payload for this AI provider test connection event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgAiProviderTestConnectionEventBaseType1,
        ToolPkgAiProviderBaseEventPayload,
    >,
}
/// Carries AI provider calculate input tokens data supplied when the event is dispatched.
pub struct ToolPkgAiProviderCalculateInputTokensEventPayload {
    /// Carries provider configuration and locale shared by this AI provider request.
    pub base_ai_provider_base_event_payload: ToolPkgAiProviderBaseEventPayload,
    /// Contains conversation turns available at this hook stage.
    pub chatHistory: Vec<ToolPkgPromptTurn>,
    /// Lists the tools currently available for model invocation.
    pub availableTools: Option<Vec<ToolPkgJsonObject>>,
}
/// Carries an input-token-count request dispatched to an AI provider.
pub struct ToolPkgAiProviderCalculateInputTokensEvent {
    /// Carries shared dispatch metadata and the typed payload for this AI provider calculate input tokens event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgAiProviderCalculateInputTokensEventBaseType1,
        ToolPkgAiProviderCalculateInputTokensEventPayload,
    >,
}
/// Describes one model exposed by a registered AI provider.
pub struct ToolPkgAiProviderModelOption {
    /// Preserves additional JSON properties supplied with this AI provider model option.
    pub base_json_object: ToolPkgJsonObject,
    /// Identifies this AI provider model option within its owning package.
    pub id: String,
    /// Provides the stable or user-facing name of this AI provider model option.
    pub name: String,
}
/// Reports input, output, and cached token usage for a provider response.
pub struct ToolPkgAiProviderUsage {
    /// Preserves additional JSON properties supplied with this AI provider usage.
    pub base_json_object: ToolPkgJsonObject,
    /// Reports the number of input tokens consumed.
    pub input: Option<f64>,
    /// Reports the number of cached input tokens consumed.
    pub cachedInput: Option<f64>,
    /// Reports the number of output tokens generated.
    pub output: Option<f64>,
}
/// Returns models available from a registered AI provider.
pub struct ToolPkgAiProviderListModelsResult {
    /// Preserves additional JSON properties supplied with this AI provider list models result.
    pub base_json_object: ToolPkgJsonObject,
    /// Lists models discovered from the provider.
    pub models: Vec<ToolPkgAiProviderModelOption>,
}
/// Returns generated text and token usage from an AI provider.
pub struct ToolPkgAiProviderSendMessageResult {
    /// Preserves additional JSON properties supplied with this AI provider send message result.
    pub base_json_object: ToolPkgJsonObject,
    /// Contains text produced, replaced, or inspected by this operation.
    pub text: String,
    /// Reports token consumption for the generated response.
    pub usage: Option<ToolPkgAiProviderUsage>,
}
/// Reports whether an AI provider connection test succeeded.
pub struct ToolPkgAiProviderTestConnectionResult {
    /// Preserves additional JSON properties supplied with this AI provider test connection result.
    pub base_json_object: ToolPkgJsonObject,
    /// Reports whether the operation completed successfully.
    pub success: bool,
    /// Provides the provider's connection-test status message.
    pub message: Option<String>,
    /// Provides an error message when the operation fails.
    pub error: Option<String>,
}
/// Reports the input-token count calculated by an AI provider.
pub struct ToolPkgAiProviderCalculateInputTokensResult {
    /// Preserves additional JSON properties supplied with this AI provider calculate input tokens result.
    pub base_json_object: ToolPkgJsonObject,
    /// Reports the calculated number of input tokens.
    pub tokens: f64,
}
/// Allows model listing to complete immediately or asynchronously.
pub enum ToolPkgAiProviderListModelsReturn {
    Variant1(ToolPkgAiProviderListModelsResult),
    Variant2(JsFuture<ToolPkgAiProviderListModelsResult>),
}
/// Allows message generation to complete immediately or asynchronously.
pub enum ToolPkgAiProviderSendMessageReturn {
    Variant1(ToolPkgAiProviderSendMessageResult),
    Variant2(JsFuture<ToolPkgAiProviderSendMessageResult>),
}
/// Allows a provider connection test to complete immediately or asynchronously.
pub enum ToolPkgAiProviderTestConnectionReturn {
    Variant1(ToolPkgAiProviderTestConnectionResult),
    Variant2(JsFuture<ToolPkgAiProviderTestConnectionResult>),
}
/// Allows input-token calculation to complete immediately or asynchronously.
pub enum ToolPkgAiProviderCalculateInputTokensReturn {
    Variant1(ToolPkgAiProviderCalculateInputTokensResult),
    Variant2(JsFuture<ToolPkgAiProviderCalculateInputTokensResult>),
}
/// Callback that lists models exposed by an AI provider.
pub type ToolPkgAiProviderListModelsHandler = Arc<
    dyn Fn(ToolPkgAiProviderListModelsEvent) -> ToolPkgAiProviderListModelsReturn + Send + Sync,
>;
/// Callback that generates a response through an AI provider.
pub type ToolPkgAiProviderSendMessageHandler = Arc<
    dyn Fn(ToolPkgAiProviderSendMessageEvent) -> ToolPkgAiProviderSendMessageReturn + Send + Sync,
>;
/// Callback that verifies connectivity to an AI provider.
pub type ToolPkgAiProviderTestConnectionHandler = Arc<
    dyn Fn(ToolPkgAiProviderTestConnectionEvent) -> ToolPkgAiProviderTestConnectionReturn
        + Send
        + Sync,
>;
/// Callback that calculates token usage for an AI provider input.
pub type ToolPkgAiProviderCalculateInputTokensHandler = Arc<
    dyn Fn(
            ToolPkgAiProviderCalculateInputTokensEvent,
        ) -> ToolPkgAiProviderCalculateInputTokensReturn
        + Send
        + Sync,
>;
/// Collects settings and callbacks used to register AI provider handler.
pub struct ToolPkgAiProviderHandlerRegistration {
    /// Provides the callback invoked for this AI provider handler registration.
    pub function: ToolPkgAiProviderHandlerRegistrationFunction,
}
/// Describes a Compose DSL screen contributed to the toolbox UI.
pub struct ToolPkgToolboxUiModuleRegistration {
    /// Uniquely identifies this toolbox UI module registration within the package.
    pub id: String,
    /// Identifies the runtime that owns or emitted this value.
    pub runtime: Option<String>,
    /// Contains the Compose DSL screen rendered by the host.
    pub screen: ComposeDslScreen,
    /// Supplies initial parameters to the Compose DSL screen.
    pub params: Option<ToolParams>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<ToolPkgLocalizedText>,
    /// Controls whether the host retains the UI instance between visits.
    pub keepAlive: Option<bool>,
}
/// Describes a routable Compose DSL screen contributed by a plugin.
pub struct ToolPkgUiRouteRegistration {
    /// Uniquely identifies this UI route registration within the package.
    pub id: String,
    /// Provides the path used to open this UI contribution.
    pub route: Option<String>,
    /// Identifies the route opened by this contribution.
    pub routeId: Option<String>,
    /// Identifies the runtime that owns or emitted this value.
    pub runtime: Option<String>,
    /// Contains the Compose DSL screen rendered by the host.
    pub screen: ComposeDslScreen,
    /// Supplies initial parameters to the Compose DSL screen.
    pub params: Option<ToolParams>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<ToolPkgLocalizedText>,
    /// Controls whether the host retains the UI instance between visits.
    pub keepAlive: Option<bool>,
}
/// Enumerates supported navigation surface values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgNavigationSurface {
    /// Places the entry in the toolbox.
    #[serde(rename = "toolbox")]
    Toolbox,
    /// Places the entry in the main sidebar plugin section.
    #[serde(rename = "main_sidebar_plugins")]
    MainSidebarPlugins,
    /// Places an icon button in the application top bar.
    #[serde(rename = "app_bar")]
    AppBar,
}
/// Describes a plugin action exposed through a host navigation surface.
pub struct ToolPkgNavigationEntryRegistration {
    /// Uniquely identifies this navigation entry registration within the package.
    pub id: String,
    /// Provides the path used to open this UI contribution.
    pub route: Option<String>,
    /// Selects the host navigation surface containing this entry.
    pub surface: ToolPkgNavigationSurface,
    /// Identifies the operation associated with this event or hook result.
    pub action: Option<ToolPkgNavigationEntryActionHookHandler>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<ToolPkgLocalizedText>,
    /// Provides the icon name displayed by the host UI.
    pub icon: Option<String>,
    /// Controls relative placement in the host UI.
    pub order: Option<f64>,
}
/// Describes a plugin widget shown on the desktop surface.
pub struct ToolPkgDesktopWidgetRegistration {
    /// Uniquely identifies this desktop widget registration within the package.
    pub id: String,
    /// Provides the path used to open this UI contribution.
    pub route: Option<String>,
    /// Identifies the route opened by this contribution.
    pub routeId: Option<String>,
    /// Supplies the callback used to render a desktop widget.
    pub render: Option<String>,
    /// Identifies the route used to render a desktop widget.
    pub renderRouteId: Option<String>,
    /// Provides primary text displayed by the host UI.
    pub title: Option<ToolPkgLocalizedText>,
    /// Provides secondary text displayed with a desktop widget.
    pub subtitle: Option<ToolPkgLocalizedText>,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: Option<ToolPkgLocalizedText>,
    /// Provides the icon name displayed by the host UI.
    pub icon: Option<String>,
    /// Controls relative placement in the host UI.
    pub order: Option<f64>,
}
/// Binds an application lifecycle event to a plugin callback.
pub struct ToolPkgAppLifecycleHookRegistration {
    /// Uniquely identifies this app lifecycle hook registration within the package.
    pub id: String,
    /// Identifies the hook event being dispatched.
    pub event: ToolPkgAppLifecycleEvent,
    /// Provides the callback invoked for this app lifecycle hook registration.
    pub function: ToolPkgAppLifecycleHookHandler,
}
/// Collects settings and callbacks used to register message processing plugin.
pub struct ToolPkgMessageProcessingPluginRegistration {
    /// Uniquely identifies this message processing plugin registration within the package.
    pub id: String,
    /// Provides the callback invoked for this message processing plugin registration.
    pub function: ToolPkgMessageProcessingHookHandler,
}
/// Collects settings and callbacks used to register XML render plugin.
pub struct ToolPkgXmlRenderPluginRegistration {
    /// Uniquely identifies this XML render plugin registration within the package.
    pub id: String,
    /// Selects the XML tag handled by this renderer.
    pub tag: String,
    /// Provides the callback invoked for this XML render plugin registration.
    pub function: ToolPkgXmlRenderHookHandler,
}
/// Collects settings and callbacks used to register input menu toggle plugin.
pub struct ToolPkgInputMenuTogglePluginRegistration {
    /// Uniquely identifies this input menu toggle plugin registration within the package.
    pub id: String,
    /// Provides the callback invoked for this input menu toggle plugin registration.
    pub function: ToolPkgInputMenuToggleHookHandler,
}
/// Configures and identifies a chat input hook registration.
pub struct ToolPkgChatInputHookRegistration {
    /// Uniquely identifies this chat input hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this chat input hook registration.
    pub function: ToolPkgChatInputHookHandler,
}
/// Configures and identifies a chat view hook registration.
pub struct ToolPkgChatViewHookRegistration {
    /// Uniquely identifies this chat view hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this chat view hook registration.
    pub function: ToolPkgHookHandler<ToolPkgChatViewHookEvent>,
}
/// Identifies a supported host event timer source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHostEventTimerSource {
    /// Selects a one-shot runtime timer.
    #[serde(rename = "timer")]
    Timer,
}
/// Identifies a supported host event interval source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHostEventIntervalSource {
    /// Selects a repeating runtime interval.
    #[serde(rename = "interval")]
    Interval,
}
/// Identifies a supported host event broadcast source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHostEventBroadcastSource {
    /// Selects an event emitted by a host platform.
    #[serde(rename = "broadcast")]
    Broadcast,
}
/// Identifies a supported host event source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHostEventSource {
    /// Selects a one-shot runtime timer.
    #[serde(rename = "timer")]
    Timer,
    /// Selects a repeating runtime interval.
    #[serde(rename = "interval")]
    Interval,
    /// Selects an event emitted by a host platform.
    #[serde(rename = "broadcast")]
    Broadcast,
}
/// Names the hook event used for host-originated runtime events.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgHostEventName {
    /// Identifies a host-originated runtime event.
    #[serde(rename = "host_event")]
    HostEvent,
}
/// Enumerates supported broadcast platform values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastPlatform {
    /// Identifies Android host events.
    #[serde(rename = "android")]
    Android,
    /// Identifies Windows host events.
    #[serde(rename = "windows")]
    Windows,
    /// Identifies Linux host events.
    #[serde(rename = "linux")]
    Linux,
    /// Identifies macOS host events.
    #[serde(rename = "macos")]
    Macos,
    /// Identifies iOS host events.
    #[serde(rename = "ios")]
    Ios,
    /// Identifies OpenHarmony host events.
    #[serde(rename = "ohos")]
    Ohos,
    /// Identifies browser host events.
    #[serde(rename = "web")]
    Web,
}
/// Identifies a supported broadcast topic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastTopic {
    #[serde(rename = "app.lifecycle.resumed")]
    AppLifecycleResumed,
    #[serde(rename = "app.lifecycle.inactive")]
    AppLifecycleInactive,
    #[serde(rename = "app.lifecycle.paused")]
    AppLifecyclePaused,
    #[serde(rename = "app.lifecycle.detached")]
    AppLifecycleDetached,
    #[serde(rename = "app.lifecycle.hidden")]
    AppLifecycleHidden,
    #[serde(rename = "system.boot.completed")]
    SystemBootCompleted,
    #[serde(rename = "system.power.connected")]
    SystemPowerConnected,
    #[serde(rename = "system.power.disconnected")]
    SystemPowerDisconnected,
    #[serde(rename = "system.power.sleep")]
    SystemPowerSleep,
    #[serde(rename = "system.power.wake")]
    SystemPowerWake,
    #[serde(rename = "system.battery.low")]
    SystemBatteryLow,
    #[serde(rename = "system.battery.okay")]
    SystemBatteryOkay,
    #[serde(rename = "system.screen.on")]
    SystemScreenOn,
    #[serde(rename = "system.screen.off")]
    SystemScreenOff,
    #[serde(rename = "system.user.present")]
    SystemUserPresent,
    #[serde(rename = "system.time.tick")]
    SystemTimeTick,
    #[serde(rename = "system.date.changed")]
    SystemDateChanged,
    #[serde(rename = "system.timezone.changed")]
    SystemTimezoneChanged,
    #[serde(rename = "system.airplane_mode.changed")]
    SystemAirplaneModeChanged,
    #[serde(rename = "system.headset.plug")]
    SystemHeadsetPlug,
    #[serde(rename = "system.session.lock")]
    SystemSessionLock,
    #[serde(rename = "system.session.unlock")]
    SystemSessionUnlock,
    #[serde(rename = "system.network.changed")]
    SystemNetworkChanged,
    #[serde(rename = "bluetooth.device.found")]
    BluetoothDeviceFound,
    #[serde(rename = "bluetooth.device.name_changed")]
    BluetoothDeviceNameChanged,
    #[serde(rename = "bluetooth.device.connected")]
    BluetoothDeviceConnected,
    #[serde(rename = "bluetooth.device.disconnected")]
    BluetoothDeviceDisconnected,
    #[serde(rename = "bluetooth.device.bond_state_changed")]
    BluetoothDeviceBondStateChanged,
    #[serde(rename = "bluetooth.adapter.connection_state_changed")]
    BluetoothAdapterConnectionStateChanged,
    #[serde(rename = "bluetooth.adapter.powered_changed")]
    BluetoothAdapterPoweredChanged,
}
/// Marks Rust types that represent one published ToolPkg broadcast topic key.
pub trait ToolPkgBroadcastTopicKey {}

impl ToolPkgBroadcastTopicKey for ToolPkgBroadcastTopic {}

/// Names one normalized application lifecycle state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastLifecycleState {
    #[serde(rename = "resumed")]
    Resumed,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "detached")]
    Detached,
    #[serde(rename = "hidden")]
    Hidden,
}
/// Carries one application lifecycle transition on every supported host platform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastLifecycleData {
    /// Identifies the normalized lifecycle state.
    pub state: ToolPkgBroadcastLifecycleState,
}
/// Reports that host startup completed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastBootData {
    /// Confirms that the host completed its boot sequence.
    pub bootCompleted: bool,
}
/// Names a normalized source supplying host power.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastPowerSource {
    #[serde(rename = "ac")]
    Ac,
    #[serde(rename = "usb")]
    Usb,
    #[serde(rename = "wireless")]
    Wireless,
    #[serde(rename = "battery")]
    Battery,
    #[serde(rename = "unknown")]
    Unknown,
}
/// Carries a normalized external-power connection change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastPowerConnectionData {
    /// Reports whether the host is connected to external power.
    pub connected: bool,
    /// Identifies the normalized power source when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ToolPkgBroadcastPowerSource>,
    /// Reports the battery percentage when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batteryLevel: Option<f64>,
}
/// Reports whether a power-state event is entering sleep or waking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastPowerSleepData {
    /// Reports whether the host is entering a suspended state.
    pub sleeping: bool,
}
/// Carries a normalized battery threshold change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastBatteryData {
    /// Reports whether the battery is currently below the host low threshold.
    pub low: bool,
    /// Reports the battery percentage when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f64>,
    /// Reports whether the battery is charging when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charging: Option<bool>,
}
/// Carries a normalized display power change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastScreenData {
    /// Reports whether the primary display is on.
    pub screenOn: bool,
}
/// Carries a normalized user presence change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastUserPresenceData {
    /// Reports whether the host considers its user present and unlocked.
    pub present: bool,
}
/// Carries a normalized clock, date, or timezone change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastTimeData {
    /// Records the platform timestamp at which the change was observed.
    pub timestampMillis: f64,
    /// Identifies the active timezone when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}
/// Carries a normalized airplane-mode change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastAirplaneModeData {
    /// Reports whether airplane mode is enabled.
    pub enabled: bool,
}
/// Carries a normalized wired or wireless headset connection change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastHeadsetData {
    /// Reports whether a headset is connected.
    pub connected: bool,
    /// Provides the headset name when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviceName: Option<String>,
    /// Reports microphone availability when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hasMicrophone: Option<bool>,
}
/// Carries a normalized desktop session lock change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastSessionData {
    /// Reports whether the active user session is locked.
    pub locked: bool,
}
/// Names the normalized active network transport.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgBroadcastNetworkType {
    #[serde(rename = "wifi")]
    Wifi,
    #[serde(rename = "cellular")]
    Cellular,
    #[serde(rename = "ethernet")]
    Ethernet,
    #[serde(rename = "vpn")]
    Vpn,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "none")]
    None,
}
/// Carries a normalized network connectivity change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastNetworkChangedData {
    /// Reports whether the host has an active network.
    pub connected: bool,
    /// Identifies the normalized active network transport.
    pub networkType: ToolPkgBroadcastNetworkType,
    /// Reports whether the active network is metered when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metered: Option<bool>,
    /// Identifies the changed interface when the platform reports it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interfaceName: Option<String>,
}
/// Carries a normalized Bluetooth device change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastBluetoothDeviceData {
    /// Identifies the Bluetooth device address when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviceAddress: Option<String>,
    /// Provides the Bluetooth device name when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deviceName: Option<String>,
    /// Reports the connection state when the topic describes or includes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    /// Reports the bond state when the topic describes or includes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonded: Option<bool>,
    /// Reports received signal strength when the platform provides it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rssi: Option<f64>,
}
/// Carries a normalized Bluetooth adapter change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolPkgBroadcastAdapterData {
    /// Reports whether the Bluetooth adapter is powered when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powered: Option<bool>,
    /// Reports whether the adapter has an active device connection when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
}
/// Resolves a broadcast topic to the data shape delivered for that topic.
pub type ToolPkgBroadcastDataForTopic<TTopic> =
    super::JsTypeIndex<ToolPkgBroadcastDataTypeMap, TTopic>;
/// Carries runtime data for a host event broadcast event.
pub type ToolPkgHostEventBroadcastPayload<TTopic> =
    ToolPkgHostEventBroadcastPayloadWithData<TTopic>;
/// Configures when a host event timer event is emitted.
pub struct ToolPkgHostEventTimerTrigger<TPayload = ToolPkgJsonObject> {
    /// Identifies the concrete kind of trigger or prompt value.
    pub kind: ToolPkgHostEventTimerSource,
    /// Sets the delay before a timer fires, in milliseconds.
    pub delayMs: f64,
    /// Contains data delivered when the trigger fires.
    pub payload: Option<TPayload>,
}
/// Configures when a host event interval event is emitted.
pub struct ToolPkgHostEventIntervalTrigger<TPayload = ToolPkgJsonObject> {
    /// Identifies the concrete kind of trigger or prompt value.
    pub kind: ToolPkgHostEventIntervalSource,
    /// Sets or reports the interval duration in milliseconds.
    pub intervalMs: f64,
    /// Contains data delivered when the trigger fires.
    pub payload: Option<TPayload>,
}
/// Configures when a host event broadcast event is emitted.
pub enum ToolPkgHostEventBroadcastTrigger<TTopic = ToolPkgBroadcastTopic> {
    Variant1(ToolPkgHostEventBroadcastTriggerVariant1<TTopic>),
    Variant2(ToolPkgHostEventBroadcastTriggerVariant2<TTopic>),
}
/// Contains a timer, interval, or broadcast trigger configuration.
pub enum ToolPkgHostEventTrigger {
    Variant1(ToolPkgHostEventTimerTrigger),
    Variant2(ToolPkgHostEventIntervalTrigger),
    Variant3(ToolPkgHostEventBroadcastTrigger),
}
/// Resolves a host event source to its matching trigger configuration.
pub type ToolPkgHostEventTriggerForSource<TSource> =
    super::JsTypeIndex<ToolPkgHostEventTriggerTypeMap, TSource>;
/// Carries runtime data for a host event timer event.
pub struct ToolPkgHostEventTimerPayload<TPayload> {
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Identifies where this value originated.
    pub source: ToolPkgHostEventTimerSource,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventTimerTrigger<TPayload>,
    /// Contains data delivered when the trigger fires.
    pub payload: Option<TPayload>,
    /// Records when the host scheduled the event, in epoch milliseconds.
    pub scheduledAtMillis: f64,
    /// Records when the platform delivered the timer event, in epoch milliseconds.
    pub firedAtMillis: f64,
    /// Sets the delay before a timer fires, in milliseconds.
    pub delayMs: Option<f64>,
    /// Sets or reports the interval duration in milliseconds.
    pub intervalMs: Option<f64>,
}
/// Carries runtime data for a host event interval event.
pub struct ToolPkgHostEventIntervalPayload<TPayload> {
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Identifies where this value originated.
    pub source: ToolPkgHostEventIntervalSource,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventIntervalTrigger<TPayload>,
    /// Contains data delivered when the trigger fires.
    pub payload: Option<TPayload>,
    /// Records when the host scheduled the event, in epoch milliseconds.
    pub scheduledAtMillis: f64,
    /// Records when the platform delivered the interval event, in epoch milliseconds.
    pub firedAtMillis: f64,
    /// Sets or reports the interval duration in milliseconds.
    pub intervalMs: f64,
}
/// Resolves a host event source to the payload delivered by that source.
pub type ToolPkgHostEventPayloadForSource<TSource> =
    super::JsTypeIndex<ToolPkgHostEventPayloadTypeMap, TSource>;
/// Configures and identifies a host event timer hook registration.
pub struct ToolPkgHostEventTimerHookRegistration<TPayload = ToolPkgJsonObject> {
    /// Uniquely identifies this host event timer hook registration within the package.
    pub id: String,
    /// Identifies where this value originated.
    pub source: ToolPkgHostEventTimerSource,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventTimerTrigger<TPayload>,
    /// Provides the callback invoked for this host event timer hook registration.
    pub function: ToolPkgHostEventTimerHookHandler<TPayload>,
    /// Enables d for provider requests.
    pub enabled: Option<bool>,
}
/// Configures and identifies a host event interval hook registration.
pub struct ToolPkgHostEventIntervalHookRegistration<TPayload = ToolPkgJsonObject> {
    /// Uniquely identifies this host event interval hook registration within the package.
    pub id: String,
    /// Identifies where this value originated.
    pub source: ToolPkgHostEventIntervalSource,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventIntervalTrigger<TPayload>,
    /// Provides the callback invoked for this host event interval hook registration.
    pub function: ToolPkgHostEventIntervalHookHandler<TPayload>,
    /// Enables d for provider requests.
    pub enabled: Option<bool>,
}
/// Configures and identifies a host event broadcast hook registration.
pub struct ToolPkgHostEventBroadcastHookRegistration<TTopic = ToolPkgBroadcastTopic> {
    /// Uniquely identifies this host event broadcast hook registration within the package.
    pub id: String,
    /// Identifies where this value originated.
    pub source: ToolPkgHostEventBroadcastSource,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventBroadcastTrigger<TTopic>,
    /// Provides the callback invoked for this host event broadcast hook registration.
    pub function: ToolPkgHostEventBroadcastHookHandler<TTopic>,
    /// Enables d for provider requests.
    pub enabled: Option<bool>,
}
/// Contains a timer, interval, or broadcast hook registration.
pub enum ToolPkgHostEventHookRegistration {
    Variant1(ToolPkgHostEventTimerHookRegistration),
    Variant2(ToolPkgHostEventIntervalHookRegistration),
    Variant3(ToolPkgHostEventBroadcastHookRegistration),
}
/// Combines shared dispatch metadata with the typed payload for a host event hook.
pub struct ToolPkgHostEventHookEvent<TSource> {
    /// Carries shared dispatch metadata and the typed payload for this host event hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgHostEventName, ToolPkgHostEventHookEventPayload<TSource>>,
}
/// Combines shared dispatch metadata with the typed payload for a host event timer hook.
pub struct ToolPkgHostEventTimerHookEvent<TPayload> {
    /// Carries shared dispatch metadata and the typed payload for this host event timer hook event.
    pub base_hook_event_base:
        ToolPkgHookEventBase<ToolPkgHostEventName, ToolPkgHostEventTimerHookEventPayload<TPayload>>,
}
/// Combines shared dispatch metadata with the typed payload for a host event interval hook.
pub struct ToolPkgHostEventIntervalHookEvent<TPayload> {
    /// Carries shared dispatch metadata and the typed payload for this host event interval hook event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgHostEventName,
        ToolPkgHostEventIntervalHookEventPayload<TPayload>,
    >,
}
/// Combines shared dispatch metadata with the typed payload for a host event broadcast hook.
pub struct ToolPkgHostEventBroadcastHookEvent<TTopic> {
    /// Carries shared dispatch metadata and the typed payload for this host event broadcast hook event.
    pub base_hook_event_base: ToolPkgHookEventBase<
        ToolPkgHostEventName,
        ToolPkgHostEventBroadcastHookEventPayload<TTopic>,
    >,
}
/// Carries source, trigger, and runtime data delivered to a host event hook.
pub struct ToolPkgHostEventHookEventPayload<TSource> {
    /// Identifies the timer, interval, or broadcast source that fired.
    pub eventSource: TSource,
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventTriggerForSource<TSource>,
    /// Contains data delivered when the trigger fires.
    pub payload: ToolPkgHostEventPayloadForSource<TSource>,
}
/// Carries source, trigger, and runtime data delivered to a host event timer hook.
pub struct ToolPkgHostEventTimerHookEventPayload<TPayload> {
    /// Identifies the timer, interval, or broadcast source that fired.
    pub eventSource: ToolPkgHostEventTimerSource,
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventTimerTrigger<TPayload>,
    /// Contains data delivered when the trigger fires.
    pub payload: ToolPkgHostEventTimerPayload<TPayload>,
}
/// Carries source, trigger, and runtime data delivered to a host event interval hook.
pub struct ToolPkgHostEventIntervalHookEventPayload<TPayload> {
    /// Identifies the timer, interval, or broadcast source that fired.
    pub eventSource: ToolPkgHostEventIntervalSource,
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventIntervalTrigger<TPayload>,
    /// Contains data delivered when the trigger fires.
    pub payload: ToolPkgHostEventIntervalPayload<TPayload>,
}
/// Carries source, trigger, and runtime data delivered to a host event broadcast hook.
pub struct ToolPkgHostEventBroadcastHookEventPayload<TTopic> {
    /// Identifies the timer, interval, or broadcast source that fired.
    pub eventSource: ToolPkgHostEventBroadcastSource,
    /// Identifies the registered hook that received the event.
    pub hookId: String,
    /// Contains the configuration that scheduled or subscribed to the event.
    pub trigger: ToolPkgHostEventBroadcastTrigger<TTopic>,
    /// Contains data delivered when the trigger fires.
    pub payload: ToolPkgHostEventBroadcastPayload<TTopic>,
}
/// Callback invoked when a timer, interval, or broadcast host event is dispatched.
pub type ToolPkgHostEventHookHandler<TSource> =
    Arc<dyn Fn(ToolPkgHostEventHookEvent<TSource>) -> ToolPkgHookReturn + Send + Sync>;
/// Callback invoked when a host event timer event is dispatched.
pub type ToolPkgHostEventTimerHookHandler<TPayload> =
    Arc<dyn Fn(ToolPkgHostEventTimerHookEvent<TPayload>) -> ToolPkgHookReturn + Send + Sync>;
/// Callback invoked when a host event interval event is dispatched.
pub type ToolPkgHostEventIntervalHookHandler<TPayload> =
    Arc<dyn Fn(ToolPkgHostEventIntervalHookEvent<TPayload>) -> ToolPkgHookReturn + Send + Sync>;
/// Callback invoked when a host event broadcast event is dispatched.
pub type ToolPkgHostEventBroadcastHookHandler<TTopic> =
    Arc<dyn Fn(ToolPkgHostEventBroadcastHookEvent<TTopic>) -> ToolPkgHookReturn + Send + Sync>;
/// Configures and identifies a tool lifecycle hook registration.
pub struct ToolPkgToolLifecycleHookRegistration {
    /// Uniquely identifies this tool lifecycle hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this tool lifecycle hook registration.
    pub function: ToolPkgToolLifecycleHookHandler,
}
/// Configures and identifies a prompt input hook registration.
pub struct ToolPkgPromptInputHookRegistration {
    /// Uniquely identifies this prompt input hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this prompt input hook registration.
    pub function: ToolPkgPromptInputHookHandler,
}
/// Configures and identifies a prompt history hook registration.
pub struct ToolPkgPromptHistoryHookRegistration {
    /// Uniquely identifies this prompt history hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this prompt history hook registration.
    pub function: ToolPkgPromptHistoryHookHandler,
}
/// Configures and identifies a prompt estimate history hook registration.
pub struct ToolPkgPromptEstimateHistoryHookRegistration {
    /// Uniquely identifies this prompt estimate history hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this prompt estimate history hook registration.
    pub function: ToolPkgPromptEstimateHistoryHookHandler,
}
/// Configures and identifies a system prompt compose hook registration.
pub struct ToolPkgSystemPromptComposeHookRegistration {
    /// Uniquely identifies this system prompt compose hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this system prompt compose hook registration.
    pub function: ToolPkgSystemPromptComposeHookHandler,
}
/// Configures and identifies a tool prompt compose hook registration.
pub struct ToolPkgToolPromptComposeHookRegistration {
    /// Uniquely identifies this tool prompt compose hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this tool prompt compose hook registration.
    pub function: ToolPkgToolPromptComposeHookHandler,
}
/// Configures and identifies a prompt finalize hook registration.
pub struct ToolPkgPromptFinalizeHookRegistration {
    /// Uniquely identifies this prompt finalize hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this prompt finalize hook registration.
    pub function: ToolPkgPromptFinalizeHookHandler,
}
/// Configures and identifies a prompt estimate finalize hook registration.
pub struct ToolPkgPromptEstimateFinalizeHookRegistration {
    /// Uniquely identifies this prompt estimate finalize hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this prompt estimate finalize hook registration.
    pub function: ToolPkgPromptEstimateFinalizeHookHandler,
}
/// Configures and identifies a summary generate hook registration.
pub struct ToolPkgSummaryGenerateHookRegistration {
    /// Uniquely identifies this summary generate hook registration within the package.
    pub id: String,
    /// Provides the callback invoked for this summary generate hook registration.
    pub function: ToolPkgSummaryGenerateHookHandler,
}
/// Describes an AI provider and every callback required to operate it.
pub struct ToolPkgAiProviderRegistration {
    /// Uniquely identifies this AI provider registration within the package.
    pub id: String,
    /// Provides the user-facing name for this registration.
    pub displayName: Option<String>,
    /// Provides explanatory text for users or model-facing metadata.
    pub description: Option<String>,
    /// Supplies the callback used to enumerate provider models.
    pub listModels: ToolPkgAiProviderRegistrationListModels,
    /// Supplies the callback used to generate a provider response.
    pub sendMessage: ToolPkgAiProviderRegistrationSendMessage,
    /// Supplies the callback used to verify provider connectivity.
    pub testConnection: ToolPkgAiProviderRegistrationTestConnection,
    /// Supplies the callback used to count input tokens.
    pub calculateInputTokens: ToolPkgAiProviderRegistrationCalculateInputTokens,
}
///Runtime area that owns a ToolPkg JavaScript context.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgRuntimeKind {
    /// Identifies the package's main runtime.
    #[serde(rename = "main")]
    Main,
    /// Identifies a package UI runtime.
    #[serde(rename = "ui")]
    Ui,
    /// Identifies a sandboxed package runtime.
    #[serde(rename = "sandbox")]
    Sandbox,
    /// Identifies an AI provider runtime.
    #[serde(rename = "provider")]
    Provider,
}
/// Identifies the scalar ABI value type used by ToolPkg WASM exports.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToolPkgWasmValueType {
    /// Identifies a signed 32-bit integer value.
    #[serde(rename = "i32")]
    I32,
    /// Identifies a signed 64-bit integer value.
    #[serde(rename = "i64")]
    I64,
    /// Identifies a 32-bit floating-point value.
    #[serde(rename = "f32")]
    F32,
    /// Identifies a 64-bit floating-point value.
    #[serde(rename = "f64")]
    F64,
}
/// Stores one scalar value passed into or returned from a ToolPkg WASM export.
pub enum ToolPkgWasmScalarValue {
    Variant1(f64),
    Variant2(String),
    Null,
}
/// Describes one scalar ABI argument for a ToolPkg WASM export.
pub struct ToolPkgWasmArg {
    /// Identifies the scalar ABI type.
    pub r#type: ToolPkgWasmValueType,
    /// Stores the scalar argument value.
    pub value: ToolPkgWasmScalarValue,
}
///Metadata passed to a handler registered with {@link IpcApi.on}.
pub struct ToolPkgIpcMeta {
    ///Channel name used for this call.
    pub channel: String,
    ///Context key of the runtime that initiated the call.
    pub callerContextKey: Option<String>,
    ///Context key of the runtime currently executing the handler.
    pub currentContextKey: Option<String>,
    ///Runtime kind currently executing the handler.
    pub currentRuntime: Option<ToolPkgRuntimeKind>,
    ///ToolPkg container package name for this call.
    pub packageTarget: Option<String>,
}
///Options used by {@link IpcApi.call} to select the target runtime.
pub struct ToolPkgIpcCallOptions {
    ///Runtime kind to call. Main is selected when no explicit target is given.
    pub targetRuntime: Option<ToolPkgRuntimeKind>,
    ///Exact target context key, required for non-main runtime calls.
    pub targetContextKey: Option<String>,
}
/// Represents the IPC API exposed on a ToolPkg registry.
pub struct ToolPkgIpcApi;

///Low-level message API between ToolPkg runtime contexts.
pub trait ToolPkgIpcApiMethods: Send + Sync {
    ///Registers a handler for a channel and returns a function that removes it.
    fn on<TPayload, TResult>(
        &self,
        channel: String,
        handler: Arc<
            dyn Fn(TPayload, ToolPkgIpcMeta) -> ToolPkgIpcApiOnHandlerOutput<TResult> + Send + Sync,
        >,
    ) -> Arc<dyn Fn() -> () + Send + Sync>;
    ///Removes a channel handler. Passing the handler checks that the same function is still registered.
    fn off<TPayload, TResult>(
        &self,
        channel: String,
        handler: Option<
            Arc<
                dyn Fn(TPayload, ToolPkgIpcMeta) -> ToolPkgIpcApiOffHandlerOutput<TResult>
                    + Send
                    + Sync,
            >,
        >,
    ) -> bool;
    ///Calls a handler in the selected runtime context and resolves with its result.
    fn call<TPayload, TResult>(
        &self,
        channel: String,
        payload: Option<TPayload>,
        options: Option<ToolPkgIpcCallOptions>,
    ) -> JsFuture<TResult>;
}
/// Represents the WASM API exposed on a ToolPkg registry.
pub struct ToolPkgWasmApi;

/// Calls scalar ToolPkg WASM exports declared by the current package manifest.
pub trait ToolPkgWasmApiMethods: Send + Sync {
    /// Calls one WASM export and resolves with its scalar result.
    fn call(
        &self,
        moduleId: String,
        exportName: String,
        args: Option<Vec<ToolPkgWasmArg>>,
    ) -> JsFuture<ToolPkgWasmScalarValue>;
}
/// Provides IPC and registration services for the current ToolPkg package.
pub struct ToolPkgRegistry {
    /// Exposes inter-context messaging for this ToolPkg registry.
    pub ipc: ToolPkgIpcApi,
    /// Exposes declared WASM exports for this ToolPkg registry.
    pub wasm: ToolPkgWasmApi,
}
/// Requires the host to implement every registry methods operation.
pub trait ToolPkgRegistryMethods: Send + Sync {
    /// Registers a Compose DSL screen in the toolbox UI.
    fn registerToolboxUiModule(&self, definition: ToolPkgToolboxUiModuleRegistration) -> ();
    /// Registers a routable Compose DSL screen for the current plugin.
    fn registerUiRoute(&self, definition: ToolPkgUiRouteRegistration) -> ();
    /// Adds a plugin action to a host navigation surface.
    fn registerNavigationEntry(&self, definition: ToolPkgNavigationEntryRegistration) -> ();
    /// Registers a plugin widget on the desktop surface.
    fn registerDesktopWidget(&self, definition: ToolPkgDesktopWidgetRegistration) -> ();
    /// Registers a callback for an application or activity lifecycle event.
    fn registerAppLifecycleHook(&self, definition: ToolPkgAppLifecycleHookRegistration) -> ();
    /// Registers a callback that inspects or transforms messages.
    fn registerMessageProcessingPlugin(
        &self,
        definition: ToolPkgMessageProcessingPluginRegistration,
    ) -> ();
    /// Registers a callback that renders a selected XML tag.
    fn registerXmlRenderPlugin(&self, definition: ToolPkgXmlRenderPluginRegistration) -> ();
    /// Registers a callback that supplies chat input menu toggles.
    fn registerInputMenuTogglePlugin(
        &self,
        definition: ToolPkgInputMenuTogglePluginRegistration,
    ) -> ();
    /// Registers a callback for chat input changes and submissions.
    fn registerChatInputHook(&self, definition: ToolPkgChatInputHookRegistration) -> ();
    /// Registers a callback for chat view lifecycle changes.
    fn registerChatViewHook(&self, definition: ToolPkgChatViewHookRegistration) -> ();
    /// Registers a typed timer, interval, or broadcast hook.
    fn registerHostEventHook_overload_1<TPayload>(
        &self,
        definition: ToolPkgHostEventTimerHookRegistration<TPayload>,
    ) -> ();
    /// Registers a typed timer, interval, or broadcast hook.
    fn registerHostEventHook_overload_2<TPayload>(
        &self,
        definition: ToolPkgHostEventIntervalHookRegistration<TPayload>,
    ) -> ();
    /// Registers a typed timer, interval, or broadcast hook.
    fn registerHostEventHook_overload_3<TTopic: ToolPkgBroadcastTopicKey>(
        &self,
        definition: ToolPkgHostEventBroadcastHookRegistration<TTopic>,
    ) -> ();
    /// Registers a callback for tool permission and execution stages.
    fn registerToolLifecycleHook(&self, definition: ToolPkgToolLifecycleHookRegistration) -> ();
    /// Registers a callback around user-input processing.
    fn registerPromptInputHook(&self, definition: ToolPkgPromptInputHookRegistration) -> ();
    /// Registers a callback around prompt-history preparation.
    fn registerPromptHistoryHook(&self, definition: ToolPkgPromptHistoryHookRegistration) -> ();
    /// Registers a prompt-history callback used during token estimation.
    fn registerPromptEstimateHistoryHook(
        &self,
        definition: ToolPkgPromptEstimateHistoryHookRegistration,
    ) -> ();
    /// Registers a callback for system-prompt assembly.
    fn registerSystemPromptComposeHook(
        &self,
        definition: ToolPkgSystemPromptComposeHookRegistration,
    ) -> ();
    /// Registers a callback for tool-prompt assembly and filtering.
    fn registerToolPromptComposeHook(
        &self,
        definition: ToolPkgToolPromptComposeHookRegistration,
    ) -> ();
    /// Registers a callback before a prompt is sent to the model.
    fn registerPromptFinalizeHook(&self, definition: ToolPkgPromptFinalizeHookRegistration) -> ();
    /// Registers a prompt-finalization callback used during token estimation.
    fn registerPromptEstimateFinalizeHook(
        &self,
        definition: ToolPkgPromptEstimateFinalizeHookRegistration,
    ) -> ();
    /// Registers a callback for summary preparation and generation.
    fn registerSummaryGenerateHook(&self, definition: ToolPkgSummaryGenerateHookRegistration)
        -> ();
    /// Registers an AI provider and its required operation callbacks.
    fn registerAiProvider(&self, definition: ToolPkgAiProviderRegistration) -> ();
    /// Extracts a packaged plugin resource and resolves to its readable path.
    fn readResource(
        &self,
        key: String,
        outputFileName: Option<String>,
        internal: Option<bool>,
    ) -> JsFuture<String>;
    /// Returns the configuration directory for the selected plugin.
    fn getConfigDir(&self, pluginId: Option<String>) -> String;
}
/// Requires the host to implement every global host operation.
pub trait GlobalHost: Send + Sync {
    /// Registers a Compose DSL screen in the toolbox UI. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgToolboxUiModule(&self, definition: ToolPkgToolboxUiModuleRegistration) -> ();
    /// Registers a routable Compose DSL screen for the current plugin. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgUiRoute(&self, definition: ToolPkgUiRouteRegistration) -> ();
    /// Adds a plugin action to a host navigation surface. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgNavigationEntry(&self, definition: ToolPkgNavigationEntryRegistration) -> ();
    /// Registers a plugin widget on the desktop surface. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgDesktopWidget(&self, definition: ToolPkgDesktopWidgetRegistration) -> ();
    /// Registers a callback for an application or activity lifecycle event. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgAppLifecycleHook(
        &self,
        definition: ToolPkgAppLifecycleHookRegistration,
    ) -> ();
    /// Registers a callback that inspects or transforms messages. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgMessageProcessingPlugin(
        &self,
        definition: ToolPkgMessageProcessingPluginRegistration,
    ) -> ();
    /// Registers a callback that renders a selected XML tag. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgXmlRenderPlugin(&self, definition: ToolPkgXmlRenderPluginRegistration) -> ();
    /// Registers a callback that supplies chat input menu toggles. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgInputMenuTogglePlugin(
        &self,
        definition: ToolPkgInputMenuTogglePluginRegistration,
    ) -> ();
    /// Registers a callback for chat input changes and submissions. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgChatInputHook(&self, definition: ToolPkgChatInputHookRegistration) -> ();
    /// Registers a callback for chat view lifecycle changes. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgChatViewHook(&self, definition: ToolPkgChatViewHookRegistration) -> ();
    /// Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgHostEventHook_overload_1<TPayload>(
        &self,
        definition: ToolPkgHostEventTimerHookRegistration<TPayload>,
    ) -> ();
    /// Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgHostEventHook_overload_2<TPayload>(
        &self,
        definition: ToolPkgHostEventIntervalHookRegistration<TPayload>,
    ) -> ();
    /// Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgHostEventHook_overload_3<TTopic: ToolPkgBroadcastTopicKey>(
        &self,
        definition: ToolPkgHostEventBroadcastHookRegistration<TTopic>,
    ) -> ();
    /// Registers a callback for tool permission and execution stages. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgToolLifecycleHook(
        &self,
        definition: ToolPkgToolLifecycleHookRegistration,
    ) -> ();
    /// Registers a callback around user-input processing. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgPromptInputHook(&self, definition: ToolPkgPromptInputHookRegistration) -> ();
    /// Registers a callback around prompt-history preparation. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgPromptHistoryHook(
        &self,
        definition: ToolPkgPromptHistoryHookRegistration,
    ) -> ();
    /// Registers a prompt-history callback used during token estimation. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgPromptEstimateHistoryHook(
        &self,
        definition: ToolPkgPromptEstimateHistoryHookRegistration,
    ) -> ();
    /// Registers a callback for system-prompt assembly. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgSystemPromptComposeHook(
        &self,
        definition: ToolPkgSystemPromptComposeHookRegistration,
    ) -> ();
    /// Registers a callback for tool-prompt assembly and filtering. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgToolPromptComposeHook(
        &self,
        definition: ToolPkgToolPromptComposeHookRegistration,
    ) -> ();
    /// Registers a callback before a prompt is sent to the model. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgPromptFinalizeHook(
        &self,
        definition: ToolPkgPromptFinalizeHookRegistration,
    ) -> ();
    /// Registers a prompt-finalization callback used during token estimation. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgPromptEstimateFinalizeHook(
        &self,
        definition: ToolPkgPromptEstimateFinalizeHookRegistration,
    ) -> ();
    /// Registers a callback for summary preparation and generation. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgSummaryGenerateHook(
        &self,
        definition: ToolPkgSummaryGenerateHookRegistration,
    ) -> ();
    /// Registers an AI provider and its required operation callbacks. The global binding delegates to the active ToolPkg registry.
    fn registerToolPkgAiProvider(&self, definition: ToolPkgAiProviderRegistration) -> ();
}
/// Binds the complete registry API to the JavaScript `ToolPkg` global.
pub struct ToolPkgGlobalBinding;
