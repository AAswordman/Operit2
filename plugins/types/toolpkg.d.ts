// Generated from operit-plugin-sdk Rust declarations.

import type { ComposeDslScreen } from "./compose-dsl";
import type { ToolParams } from "./core";

export namespace ToolPkg {
  /**
   * Stores localized strings keyed by language tag.
   */
  export interface LocalizedTextVariant2 {
    [key: string]: string;
  }

  /**
   * Stores a JSON object whose properties may contain any ToolPkg JSON value.
   */
  export interface JsonValueVariant3 {
    [key: string]: JsonValue;
  }

  /**
   * Carries the message-processing hook discriminator.
   */
  export type HookEventNameVariant2 = "message_processing";

  /**
   * Carries the XML-render hook discriminator.
   */
  export type HookEventNameVariant3 = "xml_render";

  /**
   * Carries the input-menu-toggle hook discriminator.
   */
  export type HookEventNameVariant4 = "input_menu_toggle";

  /**
   * Carries the navigation-entry-action hook discriminator.
   */
  export type HookEventNameVariant7 = "navigation_entry_action";

  /**
   * Enumerates values to which an asynchronous generic hook may resolve.
   */
  export type HookReturnVariant3Output = JsonValue | void;

  /**
   * Enumerates values to which an asynchronous application lifecycle hook may resolve.
   */
  export type AppLifecycleHookReturnVariant3Output = JsonValue | void;

  /**
   * Supplies a Compose DSL screen and optional rendering state for an XML render hook.
   */
  export interface XmlRenderHookObjectResultComposeDsl {
    /**
     * Contains the Compose DSL screen rendered by the host.
     */
    screen: ComposeDslScreen;
    /**
     * Carries mutable state used by the Compose DSL screen.
     */
    state?: JsonObject;
    /**
     * Carries memoized Compose DSL data across renders.
     */
    memo?: JsonObject;
    /**
     * Carries module metadata consumed by the Compose DSL renderer.
     */
    moduleSpec?: JsonObject;
  }

  /**
   * Enumerates values to which an asynchronous XML render hook may resolve.
   */
  export type XmlRenderHookReturnVariant5Output = string | XmlRenderHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous input menu toggle hook may resolve.
   */
  export type InputMenuToggleHookReturnVariant5Output = InputMenuToggleDefinitionResult[] | InputMenuToggleObjectResult | null | void;

  /**
   * Controls whether submitted chat input is allowed, blocked, replaced, or consumed.
   */
  export type ChatInputHookObjectResultAction = "Allow" | "Block" | "Replace" | "Consume";

  /**
   * Enumerates values to which an asynchronous chat input hook may resolve.
   */
  export type ChatInputHookReturnVariant5Output = string | ChatInputHookObjectResult | null | void;

  /**
   * Stores tool-call parameters as strings keyed by parameter name.
   */
  export interface ToolLifecycleEventPayloadParameters {
    [key: string]: string;
  }

  /**
   * Enumerates values to which an asynchronous prompt input hook may resolve.
   */
  export type PromptInputHookReturnVariant5Output = string | PromptHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous prompt history hook may resolve.
   */
  export type PromptHistoryHookReturnVariant5Output = PromptTurn[] | PromptHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous system prompt compose hook may resolve.
   */
  export type SystemPromptComposeHookReturnVariant5Output = string | PromptHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous tool prompt compose hook may resolve.
   */
  export type ToolPromptComposeHookReturnVariant5Output = string | PromptHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous prompt finalize hook may resolve.
   */
  export type PromptFinalizeHookReturnVariant6Output = string | PromptTurn[] | PromptHookObjectResult | null | void;

  /**
   * Enumerates values to which an asynchronous summary generate hook may resolve.
   */
  export type SummaryGenerateHookReturnVariant5Output = string | SummaryHookObjectResult | null | void;

  /**
   * Allows message processing to complete immediately or asynchronously.
   */
  export type MessageProcessingHookHandlerOutput = MessageProcessingHookReturnValue | Promise<MessageProcessingHookReturnValue>;

  /**
   * Identifies the request to create input-menu toggle definitions.
   */
  export type InputMenuToggleEventPayloadActionVariant1 = "Create";

  /**
   * Identifies a change to an existing input-menu toggle.
   */
  export type InputMenuToggleEventPayloadActionVariant2 = "Toggle";

  /**
   * Identifies whether toggle definitions are requested or a toggle is changed.
   */
  export type InputMenuToggleEventPayloadAction = InputMenuToggleEventPayloadActionVariant1 | InputMenuToggleEventPayloadActionVariant2 | string;

  /**
   * Identifies the classic chat input style.
   */
  export type ChatInputEventPayloadInputStyleVariant1 = "Classic";

  /**
   * Identifies the agent chat input style.
   */
  export type ChatInputEventPayloadInputStyleVariant2 = "Agent";

  /**
   * Identifies the active chat input style while preserving host-defined styles.
   */
  export type ChatInputEventPayloadInputStyle = ChatInputEventPayloadInputStyleVariant1 | ChatInputEventPayloadInputStyleVariant2 | string;

  /**
   * Identifies input from the classic chat surface.
   */
  export type ChatInputEventPayloadSourceVariant1 = "Classic";

  /**
   * Identifies input from the agent chat surface.
   */
  export type ChatInputEventPayloadSourceVariant2 = "Agent";

  /**
   * Identifies input from the fullscreen chat surface.
   */
  export type ChatInputEventPayloadSourceVariant3 = "Fullscreen";

  /**
   * Identifies input dispatched from the message queue.
   */
  export type ChatInputEventPayloadSourceVariant4 = "Queue";

  /**
   * Identifies where chat input originated while preserving host-defined sources.
   */
  export type ChatInputEventPayloadSource = ChatInputEventPayloadSourceVariant1 | ChatInputEventPayloadSourceVariant2 | ChatInputEventPayloadSourceVariant3 | ChatInputEventPayloadSourceVariant4 | string;

  /**
   * Identifies submission through the send action.
   */
  export type ChatInputEventPayloadSubmitSourceVariant1 = "Send";

  /**
   * Identifies submission through a UI button.
   */
  export type ChatInputEventPayloadSubmitSourceVariant2 = "Button";

  /**
   * Identifies submission through the input method send action.
   */
  export type ChatInputEventPayloadSubmitSourceVariant3 = "ImeSend";

  /**
   * Identifies submission through the Enter key.
   */
  export type ChatInputEventPayloadSubmitSourceVariant4 = "Enter";

  /**
   * Identifies submission by the queued-message flow.
   */
  export type ChatInputEventPayloadSubmitSourceVariant5 = "Queue";

  /**
   * Identifies the action that submitted chat input while preserving host-defined actions.
   */
  export type ChatInputEventPayloadSubmitSource = ChatInputEventPayloadSubmitSourceVariant1 | ChatInputEventPayloadSubmitSourceVariant2 | ChatInputEventPayloadSubmitSourceVariant3 | ChatInputEventPayloadSubmitSourceVariant4 | ChatInputEventPayloadSubmitSourceVariant5 | string;

  /**
   * Carries the fixed discriminator for message processing hook events.
   */
  export type MessageProcessingHookEventBaseType1 = "MessageProcessing";

  /**
   * Carries the fixed discriminator for XML render hook events.
   */
  export type XmlRenderHookEventBaseType1 = "XmlRender";

  /**
   * Carries the fixed discriminator for input menu toggle hook events.
   */
  export type InputMenuToggleHookEventBaseType1 = "InputMenuToggle";

  /**
   * Carries the fixed discriminator for navigation entry action hook events.
   */
  export type NavigationEntryActionHookEventBaseType1 = "NavigationEntryAction";

  /**
   * Carries the fixed discriminator for AI provider list models events.
   */
  export type AiProviderListModelsEventBaseType1 = "ToolpkgAiProviderListModels";

  /**
   * Carries the fixed discriminator for AI provider send message events.
   */
  export type AiProviderSendMessageEventBaseType1 = "ToolpkgAiProviderSendMessage";

  /**
   * Carries the fixed discriminator for AI provider test connection events.
   */
  export type AiProviderTestConnectionEventBaseType1 = "ToolpkgAiProviderTestConnection";

  /**
   * Carries the fixed discriminator for AI provider calculate input tokens events.
   */
  export type AiProviderCalculateInputTokensEventBaseType1 = "ToolpkgAiProviderCalculateInputTokens";

  /**
   * Selects one of the callbacks supported by an AI provider registration.
   */
  export type AiProviderHandlerRegistrationFunction = AiProviderListModelsHandler | AiProviderSendMessageHandler | AiProviderTestConnectionHandler | AiProviderCalculateInputTokensHandler;

  /**
   * Maps every standard broadcast topic to its one canonical payload type.
   */
  export interface BroadcastDataTypeMap {
    /**
     * Maps application resume events to lifecycle data.
     */
    "app.lifecycle.resumed": BroadcastLifecycleData;
    /**
     * Maps application inactive events to lifecycle data.
     */
    "app.lifecycle.inactive": BroadcastLifecycleData;
    /**
     * Maps application pause events to lifecycle data.
     */
    "app.lifecycle.paused": BroadcastLifecycleData;
    /**
     * Maps application detach events to lifecycle data.
     */
    "app.lifecycle.detached": BroadcastLifecycleData;
    /**
     * Maps application hidden events to lifecycle data.
     */
    "app.lifecycle.hidden": BroadcastLifecycleData;
    /**
     * Maps host boot completion events to boot data.
     */
    "system.boot.completed": BroadcastBootData;
    /**
     * Maps external power connection events to power data.
     */
    "system.power.connected": BroadcastPowerConnectionData;
    /**
     * Maps external power disconnection events to power data.
     */
    "system.power.disconnected": BroadcastPowerConnectionData;
    /**
     * Maps host sleep events to sleep-state data.
     */
    "system.power.sleep": BroadcastPowerSleepData;
    /**
     * Maps host wake events to sleep-state data.
     */
    "system.power.wake": BroadcastPowerSleepData;
    /**
     * Maps low-battery events to battery data.
     */
    "system.battery.low": BroadcastBatteryData;
    /**
     * Maps recovered-battery events to battery data.
     */
    "system.battery.okay": BroadcastBatteryData;
    /**
     * Maps display-on events to screen data.
     */
    "system.screen.on": BroadcastScreenData;
    /**
     * Maps display-off events to screen data.
     */
    "system.screen.off": BroadcastScreenData;
    /**
     * Maps user-presence events to presence data.
     */
    "system.user.present": BroadcastUserPresenceData;
    /**
     * Maps clock tick events to time data.
     */
    "system.time.tick": BroadcastTimeData;
    /**
     * Maps date changes to time data.
     */
    "system.date.changed": BroadcastTimeData;
    /**
     * Maps timezone changes to time data.
     */
    "system.timezone.changed": BroadcastTimeData;
    /**
     * Maps airplane-mode changes to airplane-mode data.
     */
    "system.airplane_mode.changed": BroadcastAirplaneModeData;
    /**
     * Maps headset route changes to headset data.
     */
    "system.headset.plug": BroadcastHeadsetData;
    /**
     * Maps session lock events to session data.
     */
    "system.session.lock": BroadcastSessionData;
    /**
     * Maps session unlock events to session data.
     */
    "system.session.unlock": BroadcastSessionData;
    /**
     * Maps network changes to network data.
     */
    "system.network.changed": BroadcastNetworkChangedData;
    /**
     * Maps Bluetooth discovery events to device data.
     */
    "bluetooth.device.found": BroadcastBluetoothDeviceData;
    /**
     * Maps Bluetooth name changes to device data.
     */
    "bluetooth.device.name_changed": BroadcastBluetoothDeviceData;
    /**
     * Maps Bluetooth device connections to device data.
     */
    "bluetooth.device.connected": BroadcastBluetoothDeviceData;
    /**
     * Maps Bluetooth device disconnections to device data.
     */
    "bluetooth.device.disconnected": BroadcastBluetoothDeviceData;
    /**
     * Maps Bluetooth bond-state changes to device data.
     */
    "bluetooth.device.bond_state_changed": BroadcastBluetoothDeviceData;
    /**
     * Maps Bluetooth adapter connection changes to adapter data.
     */
    "bluetooth.adapter.connection_state_changed": BroadcastAdapterData;
    /**
     * Maps Bluetooth adapter power changes to adapter data.
     */
    "bluetooth.adapter.powered_changed": BroadcastAdapterData;
  }

  /**
   * Carries metadata and topic-specific data for a host broadcast.
   */
  export interface HostEventBroadcastPayloadWithData<TTopic> {
    /**
     * Identifies the broadcast topic.
     */
    topic: TTopic;
    /**
     * Identifies the source platform.
     */
    platform: BroadcastPlatform;
    /**
     * Contains topic-specific broadcast data.
     */
    data: BroadcastDataForTopic<TTopic>;
    /**
     * Records when the host event occurred, in epoch milliseconds.
     */
    occurredAtMillis: number;
  }

  /**
   * Subscribes a host event hook to one broadcast topic.
   */
  export interface HostEventBroadcastTriggerVariant1<TTopic> {
    /**
     * Identifies the concrete kind of trigger or prompt value.
     */
    kind: HostEventBroadcastSource;
    /**
     * Selects the broadcast topic observed by this trigger.
     */
    topic: TTopic;
    /**
     * Restricts delivery to one host platform when present.
     */
    platform?: BroadcastPlatform;
    /**
     * Restricts delivery to the listed host platforms when present.
     */
    platforms?: BroadcastPlatform[];
    /**
     * Prevents a topic list from being supplied for a single-topic trigger.
     */
    topics?: never;
  }

  /**
   * Subscribes a host event hook to multiple broadcast topics.
   */
  export interface HostEventBroadcastTriggerVariant2<TTopic> {
    /**
     * Identifies the concrete kind of trigger or prompt value.
     */
    kind: HostEventBroadcastSource;
    /**
     * Prevents a single topic from being supplied for a multi-topic trigger.
     */
    topic?: never;
    /**
     * Selects broadcast topics observed by this trigger.
     */
    topics: TTopic[];
    /**
     * Restricts delivery to one host platform when present.
     */
    platform?: BroadcastPlatform;
    /**
     * Restricts delivery to the listed host platforms when present.
     */
    platforms?: BroadcastPlatform[];
  }

  /**
   * Maps each host event source to its matching trigger configuration.
   */
  export interface HostEventTriggerTypeMap {
    /**
     * Maps timer sources to timer trigger configuration.
     */
    "timer": HostEventTimerTrigger;
    /**
     * Maps interval sources to interval trigger configuration.
     */
    "interval": HostEventIntervalTrigger;
    /**
     * Maps broadcast sources to broadcast trigger configuration.
     */
    "broadcast": HostEventBroadcastTrigger;
  }

  /**
   * Maps each host event source to the payload delivered to its handler.
   */
  export interface HostEventPayloadTypeMap {
    /**
     * Maps timer sources to timer event payloads.
     */
    "timer": HostEventTimerPayload<JsonObject>;
    /**
     * Maps interval sources to interval event payloads.
     */
    "interval": HostEventIntervalPayload<JsonObject>;
    /**
     * Maps broadcast sources to broadcast event payloads.
     */
    "broadcast": HostEventBroadcastPayload<BroadcastTopic>;
  }

  /**
   * Associates an AI provider with its model-listing callback.
   */
  export interface AiProviderRegistrationListModels {
    /**
     * Provides the callback invoked for this AI provider registration.
     */
    function: AiProviderListModelsHandler;
  }

  /**
   * Associates an AI provider with its message-generation callback.
   */
  export interface AiProviderRegistrationSendMessage {
    /**
     * Provides the callback invoked for this AI provider registration.
     */
    function: AiProviderSendMessageHandler;
  }

  /**
   * Associates an AI provider with its connection-test callback.
   */
  export interface AiProviderRegistrationTestConnection {
    /**
     * Provides the callback invoked for this AI provider registration.
     */
    function: AiProviderTestConnectionHandler;
  }

  /**
   * Associates an AI provider with its input-token-count callback.
   */
  export interface AiProviderRegistrationCalculateInputTokens {
    /**
     * Provides the callback invoked for this AI provider registration.
     */
    function: AiProviderCalculateInputTokensHandler;
  }

  /**
   * Allows a IPC API on callback to complete immediately or asynchronously.
   */
  export type IpcApiOnHandlerOutput<TResult> = TResult | Promise<TResult>;

  /**
   * Allows a IPC API off callback to complete immediately or asynchronously.
   */
  export type IpcApiOffHandlerOutput<TResult> = TResult | Promise<TResult>;

  /**
   * Accepts either plain text or translations keyed by language tag.
   */
  export type LocalizedText = string | LocalizedTextVariant2;

  /**
   * Enumerates scalar values supported by ToolPkg JSON payloads.
   */
  export type JsonPrimitive = string | number | boolean | null | undefined;

  /**
   * Represents recursively nested JSON exchanged by hooks, providers, and IPC calls.
   */
  export type JsonValue = JsonPrimitive | JsonValue[] | JsonValueVariant3;

  /**
   * Stores a ToolPkg JSON object as values keyed by property name.
   */
  export interface JsonObject {
    [key: string]: JsonValue;
  }

  /**
   * Names application and activity lifecycle callbacks exposed to plugins.
   */
  export type AppLifecycleEvent = "application_on_create" | "application_on_foreground" | "application_on_background" | "application_on_low_memory" | "application_on_trim_memory" | "application_on_terminate" | "activity_on_create" | "activity_on_start" | "activity_on_resume" | "activity_on_pause" | "activity_on_stop" | "activity_on_destroy";

  /**
   * Enumerates every hook event that a ToolPkg plugin may register.
   */
  export type HookEventName = AppLifecycleEvent | HookEventNameVariant2 | HookEventNameVariant3 | HookEventNameVariant4 | ChatInputEventName | ChatViewEventName | HookEventNameVariant7 | ToolLifecycleEventName | PromptInputEventName | PromptHistoryEventName | SystemPromptComposeEventName | ToolPromptComposeEventName | PromptFinalizeEventName | SummaryGenerateEventName | HostEventName;

  /**
   * Accepts a JSON result, no result, or asynchronous completion from a generic hook.
   */
  export type HookReturn = JsonValue | void | Promise<HookReturnVariant3Output>;

  /**
   * Callback invoked when a  event is dispatched.
   */
  export type HookHandler<TEvent> = (arg0: TEvent) => HookReturn;

  /**
   * Enumerates immediate and asynchronous results accepted from an application lifecycle hook.
   */
  export type AppLifecycleHookReturn = JsonValue | void | Promise<AppLifecycleHookReturnVariant3Output>;

  /**
   * Controls how message processing reports a match and replacement content.
   */
  export interface MessageProcessingHookObjectResult extends JsonObject {
    /**
     * Reports whether message processing matched this plugin.
     */
    matched?: boolean;
    /**
     * Contains text produced, replaced, or inspected by this operation.
     */
    text?: string;
    /**
     * Supplies content produced or transformed by the hook.
     */
    content?: string;
    /**
     * Supplies replacement message chunks produced by processing.
     */
    chunks?: string[];
  }

  /**
   * Enumerates supported message processing hook return value values.
   */
  export type MessageProcessingHookReturnValue = boolean | string | MessageProcessingHookObjectResult | null | void;

  /**
   * Enumerates immediate and asynchronous results accepted from a message processing hook.
   */
  export type MessageProcessingHookReturn = MessageProcessingHookReturnValue | Promise<MessageProcessingHookReturnValue>;

  /**
   * Describes text, content, or Compose DSL produced while handling an XML element.
   */
  export interface XmlRenderHookObjectResult {
    /**
     * Reports whether the XML hook handled the supplied element.
     */
    handled?: boolean;
    /**
     * Contains text produced, replaced, or inspected by this operation.
     */
    text?: string;
    /**
     * Supplies content produced or transformed by the hook.
     */
    content?: string;
    /**
     * Supplies a Compose DSL screen produced by the XML renderer.
     */
    composeDsl?: XmlRenderHookObjectResultComposeDsl;
  }

  /**
   * Enumerates immediate and asynchronous results accepted from an XML-render hook.
   */
  export type XmlRenderHookReturn = string | XmlRenderHookObjectResult | null | void | Promise<XmlRenderHookReturnVariant5Output>;

  /**
   * Selects the input-menu section in which a toggle is displayed.
   */
  export type InputMenuToggleSlot = "thinking" | "memory" | "model" | "tools" | "general" | "default";

  /**
   * Describes one toggle contributed to the chat input menu.
   */
  export interface InputMenuToggleDefinitionResult extends JsonObject {
    /**
     * Identifies this input menu toggle definition result within its owning package.
     */
    id: string;
    /**
     * Provides primary text displayed by the host UI.
     */
    title: string;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description?: string;
    /**
     * Provides the icon name displayed by the host UI.
     */
    icon?: string;
    /**
     * Sets the current checked state of an input-menu toggle.
     */
    isChecked?: boolean;
    /**
     * Selects the chat input menu section containing this toggle.
     */
    slot?: string;
  }

  /**
   * Wraps toggle definitions returned by an input-menu hook.
   */
  export interface InputMenuToggleObjectResult extends JsonObject {
    /**
     * Contains toggle definitions contributed to the input menu.
     */
    toggles?: InputMenuToggleDefinitionResult[];
  }

  /**
   * Enumerates immediate and asynchronous results accepted from an input-menu-toggle hook.
   */
  export type InputMenuToggleHookReturn = InputMenuToggleDefinitionResult[] | InputMenuToggleObjectResult | null | void | Promise<InputMenuToggleHookReturnVariant5Output>;

  /**
   * Names the stages at which chat input hooks run.
   */
  export type ChatInputEventName = "input_changed" | "submit_requested" | "submitted";

  /**
   * Names the open, update, and close stages of a chat view.
   */
  export type ChatViewEventName = "view_opened" | "view_updated" | "view_closed";

  /**
   * Controls chat input handling and optionally supplies replacement text or metadata.
   */
  export interface ChatInputHookObjectResult extends JsonObject {
    /**
     * Controls whether the host allows, blocks, replaces, or consumes chat input.
     */
    action?: ChatInputHookObjectResultAction;
    /**
     * Contains text produced, replaced, or inspected by this operation.
     */
    text?: string;
    /**
     * Provides a user-facing explanation for the hook decision.
     */
    message?: string;
    /**
     * Requests that the host clear chat input after handling.
     */
    clearInput?: boolean;
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: JsonObject;
  }

  /**
   * Enumerates immediate and asynchronous results accepted from a chat input hook.
   */
  export type ChatInputHookReturn = string | ChatInputHookObjectResult | null | void | Promise<ChatInputHookReturnVariant5Output>;

  /**
   * Names permission, execution, result, and completion stages of a tool call.
   */
  export type ToolLifecycleEventName = "tool_call_intercept" | "tool_call_requested" | "tool_permission_checked" | "tool_execution_started" | "tool_execution_result" | "tool_execution_error" | "tool_execution_finished";

  /**
   * Names the stages before and after user-input processing.
   */
  export type PromptInputEventName = "before_process" | "after_process";

  /**
   * Names the stages before and after prompt-history preparation.
   */
  export type PromptHistoryEventName = "before_prepare_history" | "after_prepare_history";

  /**
   * Names the stages used to assemble a system prompt.
   */
  export type SystemPromptComposeEventName = "before_compose_system_prompt" | "compose_system_prompt_sections" | "after_compose_system_prompt";

  /**
   * Names the stages used to filter and assemble tool prompts.
   */
  export type ToolPromptComposeEventName = "before_compose_tool_prompt" | "filter_tool_prompt_items" | "filter_tool_call_tools" | "after_compose_tool_prompt";

  /**
   * Names the final prompt stages before a model request.
   */
  export type PromptFinalizeEventName = "before_finalize_prompt" | "before_send_to_model";

  /**
   * Names the stages used to prepare and generate a conversation summary.
   */
  export type SummaryGenerateEventName = "before_prepare_summary_prompt" | "before_send_to_model" | "after_generate_summary";

  /**
   * Identifies the role and purpose of one prompt-history turn.
   */
  export type PromptTurnKind = "SYSTEM" | "USER" | "ASSISTANT" | "TOOL_CALL" | "TOOL_RESULT" | "SUMMARY";

  /**
   * Contains one typed turn in prepared prompt history.
   */
  export interface PromptTurn extends JsonObject {
    /**
     * Identifies the concrete kind of trigger or prompt value.
     */
    kind: PromptTurnKind;
    /**
     * Supplies content produced or transformed by the hook.
     */
    content: string;
    /**
     * Identifies the tool associated with this event or prompt entry.
     */
    toolName?: string;
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: JsonObject;
  }

  /**
   * Enumerates supported active prompt type values.
   */
  export type ActivePromptType = "character_card" | "character_group";

  /**
   * Captures the identity of the character prompt active for a hook.
   */
  export interface ActivePromptSnapshot extends JsonObject {
    /**
     * Identifies the semantic kind of this value.
     */
    type: ActivePromptType;
    /**
     * Identifies this active prompt snapshot within its owning package.
     */
    id: string;
    /**
     * Provides the stable or user-facing name of this active prompt snapshot.
     */
    name: string;
  }

  /**
   * Carries contextual metadata shared across prompt hooks.
   */
  export interface HookMetadata extends JsonObject {
    /**
     * Captures the character prompt currently active for this hook.
     */
    activePrompt?: ActivePromptSnapshot;
  }

  /**
   * Carries tool request, permission, execution, and result data.
   */
  export interface ToolLifecycleEventPayload extends JsonObject {
    /**
     * Identifies the tool associated with this event or prompt entry.
     */
    toolName: string;
    /**
     * Contains parameters associated with this tool or route.
     */
    parameters?: ToolLifecycleEventPayloadParameters;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description?: string;
    /**
     * Reports whether tool execution permission was granted.
     */
    granted?: boolean;
    /**
     * Explains a permission or lifecycle decision.
     */
    reason?: string;
    /**
     * Reports whether the operation completed successfully.
     */
    success?: boolean;
    /**
     * Provides the tool execution error reported by the host.
     */
    errorMessage?: string;
    /**
     * Contains the text result returned by the tool.
     */
    resultText?: string;
    /**
     * Contains the structured result returned by the tool.
     */
    resultJson?: JsonValue;
  }

  /**
   * Describes one parameter exposed in a model-facing tool prompt.
   */
  export interface ToolPromptParameter extends JsonObject {
    /**
     * Provides the stable or user-facing name of this tool prompt parameter.
     */
    name: string;
    /**
     * Identifies the semantic kind of this value.
     */
    type?: string;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description: string;
    /**
     * Marks whether the model must supply this parameter.
     */
    required?: boolean;
    /**
     * Provides the parameter value used when none is supplied.
     */
    default?: JsonPrimitive;
  }

  /**
   * Describes one tool entry made available to the model.
   */
  export interface ToolPromptItem extends JsonObject {
    /**
     * Groups this tool under a model-facing category.
     */
    categoryName: string;
    /**
     * Provides optional text shown before this tool category.
     */
    categoryHeader?: string;
    /**
     * Provides optional text shown after this tool category.
     */
    categoryFooter?: string;
    /**
     * Provides the stable or user-facing name of this tool prompt item.
     */
    name: string;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description: string;
    /**
     * Contains parameters associated with this tool or route.
     */
    parameters?: string;
    /**
     * Provides extended model-facing details for the tool.
     */
    details?: string;
    /**
     * Provides additional model-facing usage guidance.
     */
    notes?: string;
    /**
     * Provides machine-readable tool parameter definitions.
     */
    parametersStructured?: ToolPromptParameter[];
  }

  /**
   * Contains prompt fields that a hook may replace for subsequent stages.
   */
  export interface PromptHookObjectResult extends JsonObject {
    /**
     * Contains user input before prompt processing.
     */
    rawInput?: string;
    /**
     * Contains user input after the current processing stage.
     */
    processedInput?: string;
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory?: PromptTurn[];
    /**
     * Contains conversation turns after host preparation.
     */
    preparedHistory?: PromptTurn[];
    /**
     * Contains the system prompt assembled at this hook stage.
     */
    systemPrompt?: string;
    /**
     * Contains the model-facing tool prompt at this hook stage.
     */
    toolPrompt?: string;
    /**
     * Lists the tools currently available for model invocation.
     */
    availableTools?: ToolPromptItem[];
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: HookMetadata;
  }

  /**
   * Contains summary inputs and output that a hook may replace.
   */
  export interface SummaryHookObjectResult extends JsonObject {
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory?: PromptTurn[];
    /**
     * Contains conversation turns after host preparation.
     */
    preparedHistory?: PromptTurn[];
    /**
     * Contains the system prompt assembled at this hook stage.
     */
    systemPrompt?: string;
    /**
     * Contains the prompt sent to the model for summarization.
     */
    summaryPrompt?: string;
    /**
     * Contains the summary generated at this stage.
     */
    summaryResult?: string;
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: HookMetadata;
  }

  /**
   * Carries the current prompt-building state to prompt hooks.
   */
  export interface PromptHookEventPayload extends JsonObject {
    /**
     * Names the current prompt or summary processing stage.
     */
    stage?: string;
    /**
     * Identifies the conversation associated with the event.
     */
    chatId?: string;
    /**
     * Identifies the host function participating in prompt construction.
     */
    functionType?: string;
    /**
     * Identifies the prompt-building function active for this hook.
     */
    promptFunctionType?: string;
    /**
     * Requests English prompt text from the host pipeline.
     */
    useEnglish?: boolean;
    /**
     * Contains user input before prompt processing.
     */
    rawInput?: string;
    /**
     * Contains user input after the current processing stage.
     */
    processedInput?: string;
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory?: PromptTurn[];
    /**
     * Contains conversation turns after host preparation.
     */
    preparedHistory?: PromptTurn[];
    /**
     * Contains the system prompt assembled at this hook stage.
     */
    systemPrompt?: string;
    /**
     * Contains the model-facing tool prompt at this hook stage.
     */
    toolPrompt?: string;
    /**
     * Contains model configuration active for this request.
     */
    modelParameters?: JsonObject[];
    /**
     * Lists the tools currently available for model invocation.
     */
    availableTools?: ToolPromptItem[];
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: HookMetadata;
  }

  /**
   * Carries the current summary-generation state to summary hooks.
   */
  export interface SummaryGenerateEventPayload extends JsonObject {
    /**
     * Names the current prompt or summary processing stage.
     */
    stage?: string;
    /**
     * Identifies the host function participating in prompt construction.
     */
    functionType?: string;
    /**
     * Requests English prompt text from the host pipeline.
     */
    useEnglish?: boolean;
    /**
     * Contains the summary from the preceding summarization cycle.
     */
    previousSummary?: string;
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory?: PromptTurn[];
    /**
     * Contains conversation turns after host preparation.
     */
    preparedHistory?: PromptTurn[];
    /**
     * Contains the system prompt assembled at this hook stage.
     */
    systemPrompt?: string;
    /**
     * Contains the prompt sent to the model for summarization.
     */
    summaryPrompt?: string;
    /**
     * Contains the summary generated at this stage.
     */
    summaryResult?: string;
    /**
     * Contains model configuration active for this request.
     */
    modelParameters?: JsonObject[];
    /**
     * Carries structured context for later hook stages.
     */
    metadata?: HookMetadata;
  }

  /**
   * Enumerates immediate and asynchronous results accepted from a tool lifecycle hook.
   */
  export type ToolLifecycleHookReturn = void | Promise<void>;

  /**
   * Enumerates immediate and asynchronous results accepted from a prompt input hook.
   */
  export type PromptInputHookReturn = string | PromptHookObjectResult | null | void | Promise<PromptInputHookReturnVariant5Output>;

  /**
   * Enumerates immediate and asynchronous results accepted from a prompt history hook.
   */
  export type PromptHistoryHookReturn = PromptTurn[] | PromptHookObjectResult | null | void | Promise<PromptHistoryHookReturnVariant5Output>;

  /**
   * Enumerates immediate and asynchronous results accepted from a system prompt compose hook.
   */
  export type SystemPromptComposeHookReturn = string | PromptHookObjectResult | null | void | Promise<SystemPromptComposeHookReturnVariant5Output>;

  /**
   * Enumerates immediate and asynchronous results accepted from a tool prompt compose hook.
   */
  export type ToolPromptComposeHookReturn = string | PromptHookObjectResult | null | void | Promise<ToolPromptComposeHookReturnVariant5Output>;

  /**
   * Enumerates immediate and asynchronous results accepted from a prompt finalize hook.
   */
  export type PromptFinalizeHookReturn = string | PromptTurn[] | PromptHookObjectResult | null | void | Promise<PromptFinalizeHookReturnVariant6Output>;

  /**
   * Enumerates immediate and asynchronous results accepted from a summary generate hook.
   */
  export type SummaryGenerateHookReturn = string | SummaryHookObjectResult | null | void | Promise<SummaryGenerateHookReturnVariant5Output>;

  /**
   * Callback invoked when an application or activity lifecycle event is dispatched.
   */
  export type AppLifecycleHookHandler = (arg0: AppLifecycleHookEvent) => AppLifecycleHookReturn;

  /**
   * Callback invoked when a message processing event is dispatched.
   */
  export type MessageProcessingHookHandler = (arg0: MessageProcessingHookEvent) => MessageProcessingHookHandlerOutput;

  /**
   * Callback invoked when the host requests rendering for a registered XML tag.
   */
  export type XmlRenderHookHandler = (arg0: XmlRenderHookEvent) => XmlRenderHookReturn;

  /**
   * Callback invoked when input-menu toggles are requested or changed.
   */
  export type InputMenuToggleHookHandler = (arg0: InputMenuToggleHookEvent) => InputMenuToggleHookReturn;

  /**
   * Callback invoked when a chat input event is dispatched.
   */
  export type ChatInputHookHandler = (arg0: ChatInputHookEvent) => ChatInputHookReturn;

  /**
   * Callback invoked when a navigation entry action event is dispatched.
   */
  export type NavigationEntryActionHookHandler = (arg0: NavigationEntryActionHookEvent) => HookReturn;

  /**
   * Callback invoked when a tool lifecycle event is dispatched.
   */
  export type ToolLifecycleHookHandler = (arg0: ToolLifecycleHookEvent) => ToolLifecycleHookReturn;

  /**
   * Callback invoked when a prompt input event is dispatched.
   */
  export type PromptInputHookHandler = (arg0: PromptInputHookEvent) => PromptInputHookReturn;

  /**
   * Callback invoked when a prompt history event is dispatched.
   */
  export type PromptHistoryHookHandler = (arg0: PromptHistoryHookEvent) => PromptHistoryHookReturn;

  /**
   * Callback invoked when a prompt estimate history event is dispatched.
   */
  export type PromptEstimateHistoryHookHandler = (arg0: PromptEstimateHistoryHookEvent) => PromptHistoryHookReturn;

  /**
   * Callback invoked when a system prompt compose event is dispatched.
   */
  export type SystemPromptComposeHookHandler = (arg0: SystemPromptComposeHookEvent) => SystemPromptComposeHookReturn;

  /**
   * Callback invoked when a tool prompt compose event is dispatched.
   */
  export type ToolPromptComposeHookHandler = (arg0: ToolPromptComposeHookEvent) => ToolPromptComposeHookReturn;

  /**
   * Callback invoked when a prompt finalize event is dispatched.
   */
  export type PromptFinalizeHookHandler = (arg0: PromptFinalizeHookEvent) => PromptFinalizeHookReturn;

  /**
   * Callback invoked when a prompt estimate finalize event is dispatched.
   */
  export type PromptEstimateFinalizeHookHandler = (arg0: PromptEstimateFinalizeHookEvent) => PromptFinalizeHookReturn;

  /**
   * Callback invoked when a summary generate event is dispatched.
   */
  export type SummaryGenerateHookHandler = (arg0: SummaryGenerateHookEvent) => SummaryGenerateHookReturn;

  /**
   * Carries a hook discriminator, typed payload, package identity, and dispatch metadata.
   */
  export interface HookEventBase<TEventName, TPayload> {
    /**
     * Identifies the hook event being dispatched.
     */
    event: TEventName;
    /**
     * Repeats the typed event name for handler-friendly access.
     */
    eventName: TEventName;
    /**
     * Contains data specific to the dispatched event.
     */
    eventPayload: TPayload;
    /**
     * Identifies the ToolPkg package that dispatched the hook.
     */
    toolPkgId?: string;
    /**
     * Identifies the ToolPkg container that dispatched the hook.
     */
    containerPackageName?: string;
    /**
     * Identifies the plugin function selected for dispatch.
     */
    functionName?: string;
    /**
     * Identifies the plugin that owns the hook.
     */
    pluginId?: string;
    /**
     * Identifies the registered hook that received the event.
     */
    hookId?: string;
    /**
     * Records when the hook was dispatched, in epoch milliseconds.
     */
    timestampMs?: number;
  }

  /**
   * Carries app lifecycle data supplied when the event is dispatched.
   */
  export interface AppLifecycleEventPayload extends JsonObject {
    /**
     * Carries host-specific data without a dedicated field.
     */
    extras?: JsonObject;
  }

  /**
   * Carries message processing data supplied when the event is dispatched.
   */
  export interface MessageProcessingEventPayload extends JsonObject {
    /**
     * Identifies the conversation associated with the event.
     */
    chatId?: string;
    /**
     * Contains the message text being processed.
     */
    messageContent?: string;
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory?: PromptTurn[];
    /**
     * Provides the workspace path associated with the conversation.
     */
    workspacePath?: string;
    /**
     * Provides the maximum token budget for message processing.
     */
    maxTokens?: number;
    /**
     * Provides the token-usage threshold for message processing.
     */
    tokenUsageThreshold?: number;
    /**
     * Requests match detection without applying message changes.
     */
    probeOnly?: boolean;
    /**
     * Correlates this event with a specific execution attempt.
     */
    executionId?: string;
  }

  /**
   * Carries XML render data supplied when the event is dispatched.
   */
  export interface XmlRenderEventPayload extends JsonObject {
    /**
     * Contains the XML fragment supplied to the renderer.
     */
    xmlContent?: string;
    /**
     * Identifies the XML tag currently being rendered.
     */
    tagName?: string;
  }

  /**
   * Carries input menu toggle data supplied when the event is dispatched.
   */
  export interface InputMenuToggleEventPayload extends JsonObject {
    /**
     * Selects whether toggle definitions are requested or a toggle is changed.
     */
    action?: InputMenuToggleEventPayloadAction;
    /**
     * Identifies the input-menu toggle being changed.
     */
    toggleId?: string;
    /**
     * Identifies the conversation associated with the event.
     */
    chatId?: string;
    /**
     * Identifies the runtime that owns or emitted this value.
     */
    runtime?: string;
  }

  /**
   * Carries chat input data supplied when the event is dispatched.
   */
  export interface ChatInputEventPayload extends JsonObject {
    /**
     * Identifies the conversation associated with the event.
     */
    chatId?: string;
    /**
     * Contains text produced, replaced, or inspected by this operation.
     */
    text?: string;
    /**
     * Provides the start offset of the text selection.
     */
    selectionStart?: number;
    /**
     * Provides the exclusive end offset of the text selection.
     */
    selectionEnd?: number;
    /**
     * Reports whether chat input includes attachments.
     */
    hasAttachments?: boolean;
    /**
     * Reports how many attachments accompany the chat input.
     */
    attachmentCount?: number;
    /**
     * Reports whether the chat is processing a request.
     */
    isProcessing?: boolean;
    /**
     * Identifies the active chat input mode.
     */
    inputStyle?: ChatInputEventPayloadInputStyle;
    /**
     * Identifies where this value originated.
     */
    source?: ChatInputEventPayloadSource;
    /**
     * Identifies the action that submitted chat input.
     */
    submitSource?: ChatInputEventPayloadSubmitSource;
  }

  /**
   * Carries chat view data supplied when the event is dispatched.
   */
  export interface ChatViewEventPayload extends JsonObject {
    /**
     * Identifies the chat view that emitted the event.
     */
    viewId?: string;
    /**
     * Identifies the conversation associated with the event.
     */
    chatId?: string;
    /**
     * Provides the workspace path associated with the conversation.
     */
    workspacePath?: string;
    /**
     * Contains serialized workspace environment data.
     */
    workspaceEnv?: string;
    /**
     * Identifies the runtime that owns or emitted this value.
     */
    runtime?: string;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: string;
  }

  /**
   * Carries navigation entry action data supplied when the event is dispatched.
   */
  export interface NavigationEntryActionEventPayload extends JsonObject {
    /**
     * Identifies the navigation entry that emitted the action.
     */
    entryId?: string;
    /**
     * Identifies the route opened by this contribution.
     */
    routeId?: string;
    /**
     * Selects the host navigation surface containing this entry.
     */
    surface?: string;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: string;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description?: string;
  }

  /**
   * Combines shared dispatch metadata with an application lifecycle payload.
   */
  export interface AppLifecycleHookEvent extends HookEventBase<AppLifecycleEvent, AppLifecycleEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a message processing hook.
   */
  export interface MessageProcessingHookEvent extends HookEventBase<MessageProcessingHookEventBaseType1, MessageProcessingEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with an XML-render payload.
   */
  export interface XmlRenderHookEvent extends HookEventBase<XmlRenderHookEventBaseType1, XmlRenderEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with an input-menu-toggle payload.
   */
  export interface InputMenuToggleHookEvent extends HookEventBase<InputMenuToggleHookEventBaseType1, InputMenuToggleEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a chat input hook.
   */
  export interface ChatInputHookEvent extends HookEventBase<ChatInputEventName, ChatInputEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a chat view hook.
   */
  export interface ChatViewHookEvent extends HookEventBase<ChatViewEventName, ChatViewEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a navigation entry action hook.
   */
  export interface NavigationEntryActionHookEvent extends HookEventBase<NavigationEntryActionHookEventBaseType1, NavigationEntryActionEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a tool lifecycle hook.
   */
  export interface ToolLifecycleHookEvent extends HookEventBase<ToolLifecycleEventName, ToolLifecycleEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a prompt input hook.
   */
  export interface PromptInputHookEvent extends HookEventBase<PromptInputEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a prompt history hook.
   */
  export interface PromptHistoryHookEvent extends HookEventBase<PromptHistoryEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a prompt estimate history hook.
   */
  export interface PromptEstimateHistoryHookEvent extends HookEventBase<PromptHistoryEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a system prompt compose hook.
   */
  export interface SystemPromptComposeHookEvent extends HookEventBase<SystemPromptComposeEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a tool prompt compose hook.
   */
  export interface ToolPromptComposeHookEvent extends HookEventBase<ToolPromptComposeEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a prompt finalize hook.
   */
  export interface PromptFinalizeHookEvent extends HookEventBase<PromptFinalizeEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a prompt estimate finalize hook.
   */
  export interface PromptEstimateFinalizeHookEvent extends HookEventBase<PromptFinalizeEventName, PromptHookEventPayload> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a summary generate hook.
   */
  export interface SummaryGenerateHookEvent extends HookEventBase<SummaryGenerateEventName, SummaryGenerateEventPayload> {
  }

  /**
   * Contains host configuration supplied to registered AI provider callbacks.
   */
  export interface AiProviderConfig extends JsonObject {
    /**
     * Identifies this AI provider config within its owning package.
     */
    id: string;
    /**
     * Provides the stable or user-facing name of this AI provider config.
     */
    name: string;
    /**
     * Names the provider implementation family.
     */
    apiProviderType: string;
    /**
     * Identifies the configured provider implementation.
     */
    apiProviderTypeId: string;
    /**
     * Provides the credential used to authenticate provider requests.
     */
    apiKey: string;
    /**
     * Provides the base endpoint for provider API requests.
     */
    apiEndpoint: string;
    /**
     * Identifies the model selected for this provider request.
     */
    modelName: string;
    /**
     * Adds provider-specific HTTP headers to requests.
     */
    customHeaders: JsonObject;
    /**
     * Adds provider-specific request parameters.
     */
    customParameters: JsonValue[];
    /**
     * Enables direct image processing for provider requests.
     */
    enableDirectImageProcessing: boolean;
    /**
     * Enables direct audio processing for provider requests.
     */
    enableDirectAudioProcessing: boolean;
    /**
     * Enables direct video processing for provider requests.
     */
    enableDirectVideoProcessing: boolean;
    /**
     * Enables Google Search augmentation for provider requests.
     */
    enableGoogleSearch: boolean;
    /**
     * Enables Claude's one-hour prompt-cache policy.
     */
    enableClaude1hPromptCache: boolean;
    /**
     * Allows the provider to issue tool calls.
     */
    enableToolCall: boolean;
    /**
     * Limits provider requests issued per minute.
     */
    requestLimitPerMinute: number;
    /**
     * Sets the maximum concurrent requests allowed for this provider.
     */
    maxConcurrentRequests: number;
    /**
     * Provides the locale requested by the host.
     */
    locale?: string;
  }

  /**
   * Carries provider configuration and locale shared by AI provider operations.
   */
  export interface AiProviderBaseEventPayload extends JsonObject {
    /**
     * Identifies the provider registration handling the event.
     */
    providerId: string;
    /**
     * Provides the provider name displayed by the host.
     */
    providerDisplayName?: string;
    /**
     * Provides the host-facing provider description.
     */
    providerDescription?: string;
    /**
     * Contains active host configuration for the provider.
     */
    config: AiProviderConfig;
  }

  /**
   * Carries a request to list the models available from an AI provider.
   */
  export interface AiProviderListModelsEvent extends HookEventBase<AiProviderListModelsEventBaseType1, AiProviderBaseEventPayload> {
  }

  /**
   * Carries AI provider send message data supplied when the event is dispatched.
   */
  export interface AiProviderSendMessageEventPayload extends AiProviderBaseEventPayload {
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory: PromptTurn[];
    /**
     * Contains model configuration active for this request.
     */
    modelParameters?: JsonObject[];
    /**
     * Lists the tools currently available for model invocation.
     */
    availableTools?: JsonObject[];
    /**
     * Enables the provider's extended reasoning mode.
     */
    enableThinking?: boolean;
    /**
     * Controls whether the provider streams generated output.
     */
    stream?: boolean;
    /**
     * Keeps provider reasoning content in conversation history.
     */
    preserveThinkInHistory?: boolean;
    /**
     * Allows a failed provider request to be retried.
     */
    enableRetry?: boolean;
  }

  /**
   * Carries a message-generation request dispatched to an AI provider.
   */
  export interface AiProviderSendMessageEvent extends HookEventBase<AiProviderSendMessageEventBaseType1, AiProviderSendMessageEventPayload> {
  }

  /**
   * Carries a connection-test request dispatched to an AI provider.
   */
  export interface AiProviderTestConnectionEvent extends HookEventBase<AiProviderTestConnectionEventBaseType1, AiProviderBaseEventPayload> {
  }

  /**
   * Carries AI provider calculate input tokens data supplied when the event is dispatched.
   */
  export interface AiProviderCalculateInputTokensEventPayload extends AiProviderBaseEventPayload {
    /**
     * Contains conversation turns available at this hook stage.
     */
    chatHistory: PromptTurn[];
    /**
     * Lists the tools currently available for model invocation.
     */
    availableTools?: JsonObject[];
  }

  /**
   * Carries an input-token-count request dispatched to an AI provider.
   */
  export interface AiProviderCalculateInputTokensEvent extends HookEventBase<AiProviderCalculateInputTokensEventBaseType1, AiProviderCalculateInputTokensEventPayload> {
  }

  /**
   * Describes one model exposed by a registered AI provider.
   */
  export interface AiProviderModelOption extends JsonObject {
    /**
     * Identifies this AI provider model option within its owning package.
     */
    id: string;
    /**
     * Provides the stable or user-facing name of this AI provider model option.
     */
    name: string;
  }

  /**
   * Reports input, output, and cached token usage for a provider response.
   */
  export interface AiProviderUsage extends JsonObject {
    /**
     * Reports the number of input tokens consumed.
     */
    input?: number;
    /**
     * Reports the number of cached input tokens consumed.
     */
    cachedInput?: number;
    /**
     * Reports the number of output tokens generated.
     */
    output?: number;
  }

  /**
   * Returns models available from a registered AI provider.
   */
  export interface AiProviderListModelsResult extends JsonObject {
    /**
     * Lists models discovered from the provider.
     */
    models: AiProviderModelOption[];
  }

  /**
   * Returns generated text and token usage from an AI provider.
   */
  export interface AiProviderSendMessageResult extends JsonObject {
    /**
     * Contains text produced, replaced, or inspected by this operation.
     */
    text: string;
    /**
     * Reports token consumption for the generated response.
     */
    usage?: AiProviderUsage;
  }

  /**
   * Reports whether an AI provider connection test succeeded.
   */
  export interface AiProviderTestConnectionResult extends JsonObject {
    /**
     * Reports whether the operation completed successfully.
     */
    success: boolean;
    /**
     * Provides the provider's connection-test status message.
     */
    message?: string;
    /**
     * Provides an error message when the operation fails.
     */
    error?: string;
  }

  /**
   * Reports the input-token count calculated by an AI provider.
   */
  export interface AiProviderCalculateInputTokensResult extends JsonObject {
    /**
     * Reports the calculated number of input tokens.
     */
    tokens: number;
  }

  /**
   * Allows model listing to complete immediately or asynchronously.
   */
  export type AiProviderListModelsReturn = AiProviderListModelsResult | Promise<AiProviderListModelsResult>;

  /**
   * Allows message generation to complete immediately or asynchronously.
   */
  export type AiProviderSendMessageReturn = AiProviderSendMessageResult | Promise<AiProviderSendMessageResult>;

  /**
   * Allows a provider connection test to complete immediately or asynchronously.
   */
  export type AiProviderTestConnectionReturn = AiProviderTestConnectionResult | Promise<AiProviderTestConnectionResult>;

  /**
   * Allows input-token calculation to complete immediately or asynchronously.
   */
  export type AiProviderCalculateInputTokensReturn = AiProviderCalculateInputTokensResult | Promise<AiProviderCalculateInputTokensResult>;

  /**
   * Callback that lists models exposed by an AI provider.
   */
  export type AiProviderListModelsHandler = (arg0: AiProviderListModelsEvent) => AiProviderListModelsReturn;

  /**
   * Callback that generates a response through an AI provider.
   */
  export type AiProviderSendMessageHandler = (arg0: AiProviderSendMessageEvent) => AiProviderSendMessageReturn;

  /**
   * Callback that verifies connectivity to an AI provider.
   */
  export type AiProviderTestConnectionHandler = (arg0: AiProviderTestConnectionEvent) => AiProviderTestConnectionReturn;

  /**
   * Callback that calculates token usage for an AI provider input.
   */
  export type AiProviderCalculateInputTokensHandler = (arg0: AiProviderCalculateInputTokensEvent) => AiProviderCalculateInputTokensReturn;

  /**
   * Collects settings and callbacks used to register AI provider handler.
   */
  export interface AiProviderHandlerRegistration {
    /**
     * Provides the callback invoked for this AI provider handler registration.
     */
    function: AiProviderHandlerRegistrationFunction;
  }

  /**
   * Describes a Compose DSL screen contributed to the toolbox UI.
   */
  export interface ToolboxUiModuleRegistration {
    /**
     * Uniquely identifies this toolbox UI module registration within the package.
     */
    id: string;
    /**
     * Identifies the runtime that owns or emitted this value.
     */
    runtime?: string;
    /**
     * Contains the Compose DSL screen rendered by the host.
     */
    screen: ComposeDslScreen;
    /**
     * Supplies initial parameters to the Compose DSL screen.
     */
    params?: ToolParams;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: LocalizedText;
    /**
     * Controls whether the host retains the UI instance between visits.
     */
    keepAlive?: boolean;
  }

  /**
   * Describes a routable Compose DSL screen contributed by a plugin.
   */
  export interface UiRouteRegistration {
    /**
     * Uniquely identifies this UI route registration within the package.
     */
    id: string;
    /**
     * Provides the path used to open this UI contribution.
     */
    route?: string;
    /**
     * Identifies the route opened by this contribution.
     */
    routeId?: string;
    /**
     * Identifies the runtime that owns or emitted this value.
     */
    runtime?: string;
    /**
     * Contains the Compose DSL screen rendered by the host.
     */
    screen: ComposeDslScreen;
    /**
     * Supplies initial parameters to the Compose DSL screen.
     */
    params?: ToolParams;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: LocalizedText;
    /**
     * Controls whether the host retains the UI instance between visits.
     */
    keepAlive?: boolean;
  }

  /**
   * Enumerates supported navigation surface values.
   */
  export type NavigationSurface = "toolbox" | "main_sidebar_plugins";

  /**
   * Describes a plugin action exposed through a host navigation surface.
   */
  export interface NavigationEntryRegistration {
    /**
     * Uniquely identifies this navigation entry registration within the package.
     */
    id: string;
    /**
     * Provides the path used to open this UI contribution.
     */
    route?: string;
    /**
     * Selects the host navigation surface containing this entry.
     */
    surface: NavigationSurface;
    /**
     * Identifies the operation associated with this event or hook result.
     */
    action?: NavigationEntryActionHookHandler;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: LocalizedText;
    /**
     * Provides the icon name displayed by the host UI.
     */
    icon?: string;
    /**
     * Controls relative placement in the host UI.
     */
    order?: number;
  }

  /**
   * Describes a plugin widget shown on the desktop surface.
   */
  export interface DesktopWidgetRegistration {
    /**
     * Uniquely identifies this desktop widget registration within the package.
     */
    id: string;
    /**
     * Provides the path used to open this UI contribution.
     */
    route?: string;
    /**
     * Identifies the route opened by this contribution.
     */
    routeId?: string;
    /**
     * Supplies the callback used to render a desktop widget.
     */
    render?: string;
    /**
     * Identifies the route used to render a desktop widget.
     */
    renderRouteId?: string;
    /**
     * Provides primary text displayed by the host UI.
     */
    title?: LocalizedText;
    /**
     * Provides secondary text displayed with a desktop widget.
     */
    subtitle?: LocalizedText;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description?: LocalizedText;
    /**
     * Provides the icon name displayed by the host UI.
     */
    icon?: string;
    /**
     * Controls relative placement in the host UI.
     */
    order?: number;
  }

  /**
   * Binds an application lifecycle event to a plugin callback.
   */
  export interface AppLifecycleHookRegistration {
    /**
     * Uniquely identifies this app lifecycle hook registration within the package.
     */
    id: string;
    /**
     * Identifies the hook event being dispatched.
     */
    event: AppLifecycleEvent;
    /**
     * Provides the callback invoked for this app lifecycle hook registration.
     */
    function: AppLifecycleHookHandler;
  }

  /**
   * Collects settings and callbacks used to register message processing plugin.
   */
  export interface MessageProcessingPluginRegistration {
    /**
     * Uniquely identifies this message processing plugin registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this message processing plugin registration.
     */
    function: MessageProcessingHookHandler;
  }

  /**
   * Collects settings and callbacks used to register XML render plugin.
   */
  export interface XmlRenderPluginRegistration {
    /**
     * Uniquely identifies this XML render plugin registration within the package.
     */
    id: string;
    /**
     * Selects the XML tag handled by this renderer.
     */
    tag: string;
    /**
     * Provides the callback invoked for this XML render plugin registration.
     */
    function: XmlRenderHookHandler;
  }

  /**
   * Collects settings and callbacks used to register input menu toggle plugin.
   */
  export interface InputMenuTogglePluginRegistration {
    /**
     * Uniquely identifies this input menu toggle plugin registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this input menu toggle plugin registration.
     */
    function: InputMenuToggleHookHandler;
  }

  /**
   * Configures and identifies a chat input hook registration.
   */
  export interface ChatInputHookRegistration {
    /**
     * Uniquely identifies this chat input hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this chat input hook registration.
     */
    function: ChatInputHookHandler;
  }

  /**
   * Configures and identifies a chat view hook registration.
   */
  export interface ChatViewHookRegistration {
    /**
     * Uniquely identifies this chat view hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this chat view hook registration.
     */
    function: HookHandler<ChatViewHookEvent>;
  }

  /**
   * Identifies a supported host event timer source.
   */
  export type HostEventTimerSource = "timer";

  /**
   * Identifies a supported host event interval source.
   */
  export type HostEventIntervalSource = "interval";

  /**
   * Identifies a supported host event broadcast source.
   */
  export type HostEventBroadcastSource = "broadcast";

  /**
   * Identifies a supported host event source.
   */
  export type HostEventSource = "timer" | "interval" | "broadcast";

  /**
   * Names the hook event used for host-originated runtime events.
   */
  export type HostEventName = "host_event";

  /**
   * Enumerates supported broadcast platform values.
   */
  export type BroadcastPlatform = "android" | "windows" | "linux" | "macos" | "ios" | "ohos" | "web";

  /**
   * Identifies a supported broadcast topic.
   */
  export type BroadcastTopic = "app.lifecycle.resumed" | "app.lifecycle.inactive" | "app.lifecycle.paused" | "app.lifecycle.detached" | "app.lifecycle.hidden" | "system.boot.completed" | "system.power.connected" | "system.power.disconnected" | "system.power.sleep" | "system.power.wake" | "system.battery.low" | "system.battery.okay" | "system.screen.on" | "system.screen.off" | "system.user.present" | "system.time.tick" | "system.date.changed" | "system.timezone.changed" | "system.airplane_mode.changed" | "system.headset.plug" | "system.session.lock" | "system.session.unlock" | "system.network.changed" | "bluetooth.device.found" | "bluetooth.device.name_changed" | "bluetooth.device.connected" | "bluetooth.device.disconnected" | "bluetooth.device.bond_state_changed" | "bluetooth.adapter.connection_state_changed" | "bluetooth.adapter.powered_changed";

  /**
   * Names one normalized application lifecycle state.
   */
  export type BroadcastLifecycleState = "resumed" | "inactive" | "paused" | "detached" | "hidden";

  /**
   * Carries one application lifecycle transition on every supported host platform.
   */
  export interface BroadcastLifecycleData {
    /**
     * Identifies the normalized lifecycle state.
     */
    state: BroadcastLifecycleState;
  }

  /**
   * Reports that host startup completed.
   */
  export interface BroadcastBootData {
    /**
     * Confirms that the host completed its boot sequence.
     */
    bootCompleted: boolean;
  }

  /**
   * Names a normalized source supplying host power.
   */
  export type BroadcastPowerSource = "ac" | "usb" | "wireless" | "battery" | "unknown";

  /**
   * Carries a normalized external-power connection change.
   */
  export interface BroadcastPowerConnectionData {
    /**
     * Reports whether the host is connected to external power.
     */
    connected: boolean;
    /**
     * Identifies the normalized power source when the platform reports it.
     */
    source?: BroadcastPowerSource;
    /**
     * Reports the battery percentage when the platform reports it.
     */
    batteryLevel?: number;
  }

  /**
   * Reports whether a power-state event is entering sleep or waking.
   */
  export interface BroadcastPowerSleepData {
    /**
     * Reports whether the host is entering a suspended state.
     */
    sleeping: boolean;
  }

  /**
   * Carries a normalized battery threshold change.
   */
  export interface BroadcastBatteryData {
    /**
     * Reports whether the battery is currently below the host low threshold.
     */
    low: boolean;
    /**
     * Reports the battery percentage when the platform reports it.
     */
    level?: number;
    /**
     * Reports whether the battery is charging when the platform reports it.
     */
    charging?: boolean;
  }

  /**
   * Carries a normalized display power change.
   */
  export interface BroadcastScreenData {
    /**
     * Reports whether the primary display is on.
     */
    screenOn: boolean;
  }

  /**
   * Carries a normalized user presence change.
   */
  export interface BroadcastUserPresenceData {
    /**
     * Reports whether the host considers its user present and unlocked.
     */
    present: boolean;
  }

  /**
   * Carries a normalized clock, date, or timezone change.
   */
  export interface BroadcastTimeData {
    /**
     * Records the platform timestamp at which the change was observed.
     */
    timestampMillis: number;
    /**
     * Identifies the active timezone when the platform reports it.
     */
    timezone?: string;
  }

  /**
   * Carries a normalized airplane-mode change.
   */
  export interface BroadcastAirplaneModeData {
    /**
     * Reports whether airplane mode is enabled.
     */
    enabled: boolean;
  }

  /**
   * Carries a normalized wired or wireless headset connection change.
   */
  export interface BroadcastHeadsetData {
    /**
     * Reports whether a headset is connected.
     */
    connected: boolean;
    /**
     * Provides the headset name when the platform reports it.
     */
    deviceName?: string;
    /**
     * Reports microphone availability when the platform reports it.
     */
    hasMicrophone?: boolean;
  }

  /**
   * Carries a normalized desktop session lock change.
   */
  export interface BroadcastSessionData {
    /**
     * Reports whether the active user session is locked.
     */
    locked: boolean;
  }

  /**
   * Names the normalized active network transport.
   */
  export type BroadcastNetworkType = "wifi" | "cellular" | "ethernet" | "vpn" | "other" | "none";

  /**
   * Carries a normalized network connectivity change.
   */
  export interface BroadcastNetworkChangedData {
    /**
     * Reports whether the host has an active network.
     */
    connected: boolean;
    /**
     * Identifies the normalized active network transport.
     */
    networkType: BroadcastNetworkType;
    /**
     * Reports whether the active network is metered when known.
     */
    metered?: boolean;
    /**
     * Identifies the changed interface when the platform reports it.
     */
    interfaceName?: string;
  }

  /**
   * Carries a normalized Bluetooth device change.
   */
  export interface BroadcastBluetoothDeviceData {
    /**
     * Identifies the Bluetooth device address when available.
     */
    deviceAddress?: string;
    /**
     * Provides the Bluetooth device name when available.
     */
    deviceName?: string;
    /**
     * Reports the connection state when the topic describes or includes it.
     */
    connected?: boolean;
    /**
     * Reports the bond state when the topic describes or includes it.
     */
    bonded?: boolean;
    /**
     * Reports received signal strength when the platform provides it.
     */
    rssi?: number;
  }

  /**
   * Carries a normalized Bluetooth adapter change.
   */
  export interface BroadcastAdapterData {
    /**
     * Reports whether the Bluetooth adapter is powered when known.
     */
    powered?: boolean;
    /**
     * Reports whether the adapter has an active device connection when known.
     */
    connected?: boolean;
  }

  /**
   * Resolves a broadcast topic to the data shape delivered for that topic.
   */
  export type BroadcastDataForTopic<TTopic> = BroadcastDataTypeMap[TTopic & keyof BroadcastDataTypeMap];

  /**
   * Carries runtime data for a host event broadcast event.
   */
  export type HostEventBroadcastPayload<TTopic> = HostEventBroadcastPayloadWithData<TTopic>;

  /**
   * Configures when a host event timer event is emitted.
   */
  export interface HostEventTimerTrigger<TPayload = JsonObject> {
    /**
     * Identifies the concrete kind of trigger or prompt value.
     */
    kind: HostEventTimerSource;
    /**
     * Sets the delay before a timer fires, in milliseconds.
     */
    delayMs: number;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload?: TPayload;
  }

  /**
   * Configures when a host event interval event is emitted.
   */
  export interface HostEventIntervalTrigger<TPayload = JsonObject> {
    /**
     * Identifies the concrete kind of trigger or prompt value.
     */
    kind: HostEventIntervalSource;
    /**
     * Sets or reports the interval duration in milliseconds.
     */
    intervalMs: number;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload?: TPayload;
  }

  /**
   * Configures when a host event broadcast event is emitted.
   */
  export type HostEventBroadcastTrigger<TTopic = BroadcastTopic> = HostEventBroadcastTriggerVariant1<TTopic> | HostEventBroadcastTriggerVariant2<TTopic>;

  /**
   * Contains a timer, interval, or broadcast trigger configuration.
   */
  export type HostEventTrigger = HostEventTimerTrigger | HostEventIntervalTrigger | HostEventBroadcastTrigger;

  /**
   * Resolves a host event source to its matching trigger configuration.
   */
  export type HostEventTriggerForSource<TSource> = HostEventTriggerTypeMap[TSource & keyof HostEventTriggerTypeMap];

  /**
   * Carries runtime data for a host event timer event.
   */
  export interface HostEventTimerPayload<TPayload> {
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Identifies where this value originated.
     */
    source: HostEventTimerSource;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventTimerTrigger<TPayload>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload?: TPayload;
    /**
     * Records when the host scheduled the event, in epoch milliseconds.
     */
    scheduledAtMillis: number;
    /**
     * Records when the platform delivered the timer event, in epoch milliseconds.
     */
    firedAtMillis: number;
    /**
     * Sets the delay before a timer fires, in milliseconds.
     */
    delayMs?: number;
    /**
     * Sets or reports the interval duration in milliseconds.
     */
    intervalMs?: number;
  }

  /**
   * Carries runtime data for a host event interval event.
   */
  export interface HostEventIntervalPayload<TPayload> {
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Identifies where this value originated.
     */
    source: HostEventIntervalSource;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventIntervalTrigger<TPayload>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload?: TPayload;
    /**
     * Records when the host scheduled the event, in epoch milliseconds.
     */
    scheduledAtMillis: number;
    /**
     * Records when the platform delivered the interval event, in epoch milliseconds.
     */
    firedAtMillis: number;
    /**
     * Sets or reports the interval duration in milliseconds.
     */
    intervalMs: number;
  }

  /**
   * Resolves a host event source to the payload delivered by that source.
   */
  export type HostEventPayloadForSource<TSource> = HostEventPayloadTypeMap[TSource & keyof HostEventPayloadTypeMap];

  /**
   * Configures and identifies a host event timer hook registration.
   */
  export interface HostEventTimerHookRegistration<TPayload = JsonObject> {
    /**
     * Uniquely identifies this host event timer hook registration within the package.
     */
    id: string;
    /**
     * Identifies where this value originated.
     */
    source: HostEventTimerSource;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventTimerTrigger<TPayload>;
    /**
     * Provides the callback invoked for this host event timer hook registration.
     */
    function: HostEventTimerHookHandler<TPayload>;
    /**
     * Enables d for provider requests.
     */
    enabled?: boolean;
  }

  /**
   * Configures and identifies a host event interval hook registration.
   */
  export interface HostEventIntervalHookRegistration<TPayload = JsonObject> {
    /**
     * Uniquely identifies this host event interval hook registration within the package.
     */
    id: string;
    /**
     * Identifies where this value originated.
     */
    source: HostEventIntervalSource;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventIntervalTrigger<TPayload>;
    /**
     * Provides the callback invoked for this host event interval hook registration.
     */
    function: HostEventIntervalHookHandler<TPayload>;
    /**
     * Enables d for provider requests.
     */
    enabled?: boolean;
  }

  /**
   * Configures and identifies a host event broadcast hook registration.
   */
  export interface HostEventBroadcastHookRegistration<TTopic = BroadcastTopic> {
    /**
     * Uniquely identifies this host event broadcast hook registration within the package.
     */
    id: string;
    /**
     * Identifies where this value originated.
     */
    source: HostEventBroadcastSource;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventBroadcastTrigger<TTopic>;
    /**
     * Provides the callback invoked for this host event broadcast hook registration.
     */
    function: HostEventBroadcastHookHandler<TTopic>;
    /**
     * Enables d for provider requests.
     */
    enabled?: boolean;
  }

  /**
   * Contains a timer, interval, or broadcast hook registration.
   */
  export type HostEventHookRegistration = HostEventTimerHookRegistration | HostEventIntervalHookRegistration | HostEventBroadcastHookRegistration;

  /**
   * Combines shared dispatch metadata with the typed payload for a host event hook.
   */
  export interface HostEventHookEvent<TSource> extends HookEventBase<HostEventName, HostEventHookEventPayload<TSource>> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a host event timer hook.
   */
  export interface HostEventTimerHookEvent<TPayload> extends HookEventBase<HostEventName, HostEventTimerHookEventPayload<TPayload>> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a host event interval hook.
   */
  export interface HostEventIntervalHookEvent<TPayload> extends HookEventBase<HostEventName, HostEventIntervalHookEventPayload<TPayload>> {
  }

  /**
   * Combines shared dispatch metadata with the typed payload for a host event broadcast hook.
   */
  export interface HostEventBroadcastHookEvent<TTopic> extends HookEventBase<HostEventName, HostEventBroadcastHookEventPayload<TTopic>> {
  }

  /**
   * Carries source, trigger, and runtime data delivered to a host event hook.
   */
  export interface HostEventHookEventPayload<TSource> {
    /**
     * Identifies the timer, interval, or broadcast source that fired.
     */
    eventSource: TSource;
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventTriggerForSource<TSource>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload: HostEventPayloadForSource<TSource>;
  }

  /**
   * Carries source, trigger, and runtime data delivered to a host event timer hook.
   */
  export interface HostEventTimerHookEventPayload<TPayload> {
    /**
     * Identifies the timer, interval, or broadcast source that fired.
     */
    eventSource: HostEventTimerSource;
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventTimerTrigger<TPayload>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload: HostEventTimerPayload<TPayload>;
  }

  /**
   * Carries source, trigger, and runtime data delivered to a host event interval hook.
   */
  export interface HostEventIntervalHookEventPayload<TPayload> {
    /**
     * Identifies the timer, interval, or broadcast source that fired.
     */
    eventSource: HostEventIntervalSource;
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventIntervalTrigger<TPayload>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload: HostEventIntervalPayload<TPayload>;
  }

  /**
   * Carries source, trigger, and runtime data delivered to a host event broadcast hook.
   */
  export interface HostEventBroadcastHookEventPayload<TTopic> {
    /**
     * Identifies the timer, interval, or broadcast source that fired.
     */
    eventSource: HostEventBroadcastSource;
    /**
     * Identifies the registered hook that received the event.
     */
    hookId: string;
    /**
     * Contains the configuration that scheduled or subscribed to the event.
     */
    trigger: HostEventBroadcastTrigger<TTopic>;
    /**
     * Contains data delivered when the trigger fires.
     */
    payload: HostEventBroadcastPayload<TTopic>;
  }

  /**
   * Callback invoked when a timer, interval, or broadcast host event is dispatched.
   */
  export type HostEventHookHandler<TSource> = (arg0: HostEventHookEvent<TSource>) => HookReturn;

  /**
   * Callback invoked when a host event timer event is dispatched.
   */
  export type HostEventTimerHookHandler<TPayload> = (arg0: HostEventTimerHookEvent<TPayload>) => HookReturn;

  /**
   * Callback invoked when a host event interval event is dispatched.
   */
  export type HostEventIntervalHookHandler<TPayload> = (arg0: HostEventIntervalHookEvent<TPayload>) => HookReturn;

  /**
   * Callback invoked when a host event broadcast event is dispatched.
   */
  export type HostEventBroadcastHookHandler<TTopic> = (arg0: HostEventBroadcastHookEvent<TTopic>) => HookReturn;

  /**
   * Configures and identifies a tool lifecycle hook registration.
   */
  export interface ToolLifecycleHookRegistration {
    /**
     * Uniquely identifies this tool lifecycle hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this tool lifecycle hook registration.
     */
    function: ToolLifecycleHookHandler;
  }

  /**
   * Configures and identifies a prompt input hook registration.
   */
  export interface PromptInputHookRegistration {
    /**
     * Uniquely identifies this prompt input hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this prompt input hook registration.
     */
    function: PromptInputHookHandler;
  }

  /**
   * Configures and identifies a prompt history hook registration.
   */
  export interface PromptHistoryHookRegistration {
    /**
     * Uniquely identifies this prompt history hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this prompt history hook registration.
     */
    function: PromptHistoryHookHandler;
  }

  /**
   * Configures and identifies a prompt estimate history hook registration.
   */
  export interface PromptEstimateHistoryHookRegistration {
    /**
     * Uniquely identifies this prompt estimate history hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this prompt estimate history hook registration.
     */
    function: PromptEstimateHistoryHookHandler;
  }

  /**
   * Configures and identifies a system prompt compose hook registration.
   */
  export interface SystemPromptComposeHookRegistration {
    /**
     * Uniquely identifies this system prompt compose hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this system prompt compose hook registration.
     */
    function: SystemPromptComposeHookHandler;
  }

  /**
   * Configures and identifies a tool prompt compose hook registration.
   */
  export interface ToolPromptComposeHookRegistration {
    /**
     * Uniquely identifies this tool prompt compose hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this tool prompt compose hook registration.
     */
    function: ToolPromptComposeHookHandler;
  }

  /**
   * Configures and identifies a prompt finalize hook registration.
   */
  export interface PromptFinalizeHookRegistration {
    /**
     * Uniquely identifies this prompt finalize hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this prompt finalize hook registration.
     */
    function: PromptFinalizeHookHandler;
  }

  /**
   * Configures and identifies a prompt estimate finalize hook registration.
   */
  export interface PromptEstimateFinalizeHookRegistration {
    /**
     * Uniquely identifies this prompt estimate finalize hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this prompt estimate finalize hook registration.
     */
    function: PromptEstimateFinalizeHookHandler;
  }

  /**
   * Configures and identifies a summary generate hook registration.
   */
  export interface SummaryGenerateHookRegistration {
    /**
     * Uniquely identifies this summary generate hook registration within the package.
     */
    id: string;
    /**
     * Provides the callback invoked for this summary generate hook registration.
     */
    function: SummaryGenerateHookHandler;
  }

  /**
   * Describes an AI provider and every callback required to operate it.
   */
  export interface AiProviderRegistration {
    /**
     * Uniquely identifies this AI provider registration within the package.
     */
    id: string;
    /**
     * Provides the user-facing name for this registration.
     */
    displayName?: string;
    /**
     * Provides explanatory text for users or model-facing metadata.
     */
    description?: string;
    /**
     * Supplies the callback used to enumerate provider models.
     */
    listModels: AiProviderRegistrationListModels;
    /**
     * Supplies the callback used to generate a provider response.
     */
    sendMessage: AiProviderRegistrationSendMessage;
    /**
     * Supplies the callback used to verify provider connectivity.
     */
    testConnection: AiProviderRegistrationTestConnection;
    /**
     * Supplies the callback used to count input tokens.
     */
    calculateInputTokens: AiProviderRegistrationCalculateInputTokens;
  }

  /**
   * Runtime area that owns a ToolPkg JavaScript context.
   */
  export type RuntimeKind = "main" | "ui" | "sandbox" | "provider";

  /**
   * Metadata passed to a handler registered with {@link IpcApi.on}.
   */
  export interface IpcMeta {
    /**
     * Channel name used for this call.
     */
    channel: string;
    /**
     * Context key of the runtime that initiated the call.
     */
    callerContextKey?: string;
    /**
     * Context key of the runtime currently executing the handler.
     */
    currentContextKey?: string;
    /**
     * Runtime kind currently executing the handler.
     */
    currentRuntime?: RuntimeKind;
    /**
     * ToolPkg container package name for this call.
     */
    packageTarget?: string;
  }

  /**
   * Options used by {@link IpcApi.call} to select the target runtime.
   */
  export interface IpcCallOptions {
    /**
     * Runtime kind to call. Main is selected when no explicit target is given.
     */
    targetRuntime?: RuntimeKind;
    /**
     * Exact target context key, required for non-main runtime calls.
     */
    targetContextKey?: string;
  }

  /**
   * Represents the IPC API exposed on a ToolPkg registry.
   */
  export interface IpcApi {
    /**
     * Registers a handler for a channel and returns a function that removes it.
     */
    on<TPayload, TResult>(channel: string, handler: (arg0: TPayload, arg1: IpcMeta) => IpcApiOnHandlerOutput<TResult>): () => void;
    /**
     * Removes a channel handler. Passing the handler checks that the same function is still registered.
     */
    off<TPayload, TResult>(channel: string, handler?: (arg0: TPayload, arg1: IpcMeta) => IpcApiOffHandlerOutput<TResult>): boolean;
    /**
     * Calls a handler in the selected runtime context and resolves with its result.
     */
    call<TPayload, TResult>(channel: string, payload?: TPayload, options?: IpcCallOptions): Promise<TResult>;
  }

  /**
   * Provides IPC and registration services for the current ToolPkg package.
   */
  export interface Registry {
    /**
     * Exposes inter-context messaging for this ToolPkg registry.
     */
    ipc: IpcApi;
    /**
     * Registers a Compose DSL screen in the toolbox UI.
     */
    registerToolboxUiModule(definition: ToolboxUiModuleRegistration): void;
    /**
     * Registers a routable Compose DSL screen for the current plugin.
     */
    registerUiRoute(definition: UiRouteRegistration): void;
    /**
     * Adds a plugin action to a host navigation surface.
     */
    registerNavigationEntry(definition: NavigationEntryRegistration): void;
    /**
     * Registers a plugin widget on the desktop surface.
     */
    registerDesktopWidget(definition: DesktopWidgetRegistration): void;
    /**
     * Registers a callback for an application or activity lifecycle event.
     */
    registerAppLifecycleHook(definition: AppLifecycleHookRegistration): void;
    /**
     * Registers a callback that inspects or transforms messages.
     */
    registerMessageProcessingPlugin(definition: MessageProcessingPluginRegistration): void;
    /**
     * Registers a callback that renders a selected XML tag.
     */
    registerXmlRenderPlugin(definition: XmlRenderPluginRegistration): void;
    /**
     * Registers a callback that supplies chat input menu toggles.
     */
    registerInputMenuTogglePlugin(definition: InputMenuTogglePluginRegistration): void;
    /**
     * Registers a callback for chat input changes and submissions.
     */
    registerChatInputHook(definition: ChatInputHookRegistration): void;
    /**
     * Registers a callback for chat view lifecycle changes.
     */
    registerChatViewHook(definition: ChatViewHookRegistration): void;
    /**
     * Registers a typed timer, interval, or broadcast hook.
     */
    registerHostEventHook<TPayload>(definition: HostEventTimerHookRegistration<TPayload>): void;
    /**
     * Registers a typed timer, interval, or broadcast hook.
     */
    registerHostEventHook<TPayload>(definition: HostEventIntervalHookRegistration<TPayload>): void;
    /**
     * Registers a typed timer, interval, or broadcast hook.
     */
    registerHostEventHook<TTopic extends ToolPkg.BroadcastTopic>(definition: HostEventBroadcastHookRegistration<TTopic>): void;
    /**
     * Registers a callback for tool permission and execution stages.
     */
    registerToolLifecycleHook(definition: ToolLifecycleHookRegistration): void;
    /**
     * Registers a callback around user-input processing.
     */
    registerPromptInputHook(definition: PromptInputHookRegistration): void;
    /**
     * Registers a callback around prompt-history preparation.
     */
    registerPromptHistoryHook(definition: PromptHistoryHookRegistration): void;
    /**
     * Registers a prompt-history callback used during token estimation.
     */
    registerPromptEstimateHistoryHook(definition: PromptEstimateHistoryHookRegistration): void;
    /**
     * Registers a callback for system-prompt assembly.
     */
    registerSystemPromptComposeHook(definition: SystemPromptComposeHookRegistration): void;
    /**
     * Registers a callback for tool-prompt assembly and filtering.
     */
    registerToolPromptComposeHook(definition: ToolPromptComposeHookRegistration): void;
    /**
     * Registers a callback before a prompt is sent to the model.
     */
    registerPromptFinalizeHook(definition: PromptFinalizeHookRegistration): void;
    /**
     * Registers a prompt-finalization callback used during token estimation.
     */
    registerPromptEstimateFinalizeHook(definition: PromptEstimateFinalizeHookRegistration): void;
    /**
     * Registers a callback for summary preparation and generation.
     */
    registerSummaryGenerateHook(definition: SummaryGenerateHookRegistration): void;
    /**
     * Registers an AI provider and its required operation callbacks.
     */
    registerAiProvider(definition: AiProviderRegistration): void;
    /**
     * Extracts a packaged plugin resource and resolves to its readable path.
     */
    readResource(key: string, outputFileName?: string, internal?: boolean): Promise<string>;
    /**
     * Returns the configuration directory for the selected plugin.
     */
    getConfigDir(pluginId?: string): string;
  }

}

/**
 * Requires the host to implement every global host operation.
 */
declare global {
  /**
   * Registers an AI provider and its required operation callbacks. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgAiProvider(definition: ToolPkg.AiProviderRegistration): void;
  /**
   * Registers a callback for an application or activity lifecycle event. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgAppLifecycleHook(definition: ToolPkg.AppLifecycleHookRegistration): void;
  /**
   * Registers a callback for chat input changes and submissions. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgChatInputHook(definition: ToolPkg.ChatInputHookRegistration): void;
  /**
   * Registers a callback for chat view lifecycle changes. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgChatViewHook(definition: ToolPkg.ChatViewHookRegistration): void;
  /**
   * Registers a plugin widget on the desktop surface. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgDesktopWidget(definition: ToolPkg.DesktopWidgetRegistration): void;
  /**
   * Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgHostEventHook<TPayload>(definition: ToolPkg.HostEventTimerHookRegistration<TPayload>): void;
  /**
   * Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgHostEventHook<TPayload>(definition: ToolPkg.HostEventIntervalHookRegistration<TPayload>): void;
  /**
   * Registers a typed timer, interval, or broadcast hook. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgHostEventHook<TTopic extends ToolPkg.BroadcastTopic>(definition: ToolPkg.HostEventBroadcastHookRegistration<TTopic>): void;
  /**
   * Registers a callback that supplies chat input menu toggles. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgInputMenuTogglePlugin(definition: ToolPkg.InputMenuTogglePluginRegistration): void;
  /**
   * Registers a callback that inspects or transforms messages. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgMessageProcessingPlugin(definition: ToolPkg.MessageProcessingPluginRegistration): void;
  /**
   * Adds a plugin action to a host navigation surface. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgNavigationEntry(definition: ToolPkg.NavigationEntryRegistration): void;
  /**
   * Registers a prompt-finalization callback used during token estimation. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgPromptEstimateFinalizeHook(definition: ToolPkg.PromptEstimateFinalizeHookRegistration): void;
  /**
   * Registers a prompt-history callback used during token estimation. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgPromptEstimateHistoryHook(definition: ToolPkg.PromptEstimateHistoryHookRegistration): void;
  /**
   * Registers a callback before a prompt is sent to the model. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgPromptFinalizeHook(definition: ToolPkg.PromptFinalizeHookRegistration): void;
  /**
   * Registers a callback around prompt-history preparation. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgPromptHistoryHook(definition: ToolPkg.PromptHistoryHookRegistration): void;
  /**
   * Registers a callback around user-input processing. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgPromptInputHook(definition: ToolPkg.PromptInputHookRegistration): void;
  /**
   * Registers a callback for summary preparation and generation. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgSummaryGenerateHook(definition: ToolPkg.SummaryGenerateHookRegistration): void;
  /**
   * Registers a callback for system-prompt assembly. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgSystemPromptComposeHook(definition: ToolPkg.SystemPromptComposeHookRegistration): void;
  /**
   * Registers a callback for tool permission and execution stages. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgToolLifecycleHook(definition: ToolPkg.ToolLifecycleHookRegistration): void;
  /**
   * Registers a callback for tool-prompt assembly and filtering. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgToolPromptComposeHook(definition: ToolPkg.ToolPromptComposeHookRegistration): void;
  /**
   * Registers a Compose DSL screen in the toolbox UI. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgToolboxUiModule(definition: ToolPkg.ToolboxUiModuleRegistration): void;
  /**
   * Registers a routable Compose DSL screen for the current plugin. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgUiRoute(definition: ToolPkg.UiRouteRegistration): void;
  /**
   * Registers a callback that renders a selected XML tag. The global binding delegates to the active ToolPkg registry.
   */
  function registerToolPkgXmlRenderPlugin(definition: ToolPkg.XmlRenderPluginRegistration): void;
  /**
   * Binds the complete registry API to the JavaScript `ToolPkg` global.
   */
  const ToolPkg: ToolPkg.Registry;

}
