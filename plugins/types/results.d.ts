// Generated from operit-plugin-sdk Rust declarations.

/**
 * Contains every concrete payload returned by the built-in tool runtime.
 */
export type ToolResultData = BooleanResultData | StringResultData | SleepResultData | EnvironmentVariableReadResultData | EnvironmentVariableWriteResultData | IntResultData | BinaryResultData | FilePartContentData | DirectoryListingData | FileContentData | BinaryFileContentData | FileExistsData | FileInfoData | FileOperationData | FileApplyResultData | HttpResponseData | HttpStreamEventData | SystemSettingData | AppOperationData | AppListData | AppUsageTimeResultData | NotificationData | LocationData | DeviceInfoResultData | MemoryQueryResultData | ChatServiceStartResultData | ChatCreationResultData | ChatListResultData | ChatFindResultData | AgentStatusResultData | ChatSwitchResultData | ChatTitleUpdateResultData | ChatDeleteResultData | MessageSendResultData | ChatMessagesResultData | CharacterCardListResultData | VisitWebResultData | TerminalInfoResultData | TerminalCommandResultData | TerminalStreamEventData | HiddenTerminalCommandResultData | TerminalSessionCreationResultData | TerminalSessionCloseResultData | TerminalSessionScreenResultData | MusicPlaybackResultData | BluetoothStateData | BluetoothBondedDevicesData | BluetoothScanResultData | BluetoothSessionData | BluetoothTransferData | BluetoothReadData | BluetoothBleServicesData | BluetoothBleNotificationData | FindFilesResultData | GrepResultData | MemoryLinkResultData | MemoryLinkQueryResultData;

/**
 * Captures the UI node and Android surface reached when an automation run finishes.
 */
export interface AutomationExecutionFinalState {
  /**
   * Identifies the final automation node.
   */
  nodeId: string;
  /**
   * Identifies the package shown in the final state.
   */
  packageName: string;
  /**
   * Identifies the activity shown in the final state.
   */
  activityName: string;
}

/**
 * Records an automation or UI subagent invocation, its session, outcome, logs, and final state.
 */
export interface AutomationExecutionResultData {
  /**
   * Function name of the automation or subagent
   */
  functionName: string;
  /**
   * Parameters provided to the automation
   */
  providedParameters: Record<string, string>;
  /**
   * Optional agent id used for this run (can be reused to keep operating on the same virtual screen session)
   */
  agentId?: string | null;
  /**
   * Optional virtual display id associated with the agent session
   */
  displayId?: number | null;
  /**
   * Whether the execution succeeded
   */
  executionSuccess: boolean;
  /**
   * Detailed execution message and action logs
   */
  executionMessage: string;
  /**
   * Optional error message when execution fails
   */
  executionError?: string | null;
  /**
   * Final UI state information, if available
   */
  finalState?: AutomationExecutionFinalState | null;
  /**
   * Number of steps executed
   */
  executionSteps: number;
  /**
   * Formats the automation result for tool output.
   */
  toString(): string;
}

/**
 * Represents one serializable UI hierarchy node with accessibility metadata and child nodes.
 */
export interface SimplifiedUINode {
  className?: string;
  text?: string;
  contentDesc?: string;
  resourceId?: string;
  bounds?: string;
  isClickable: boolean;
  children: SimplifiedUINode[];
  /**
   * Formats the node as a compact JSON value.
   */
  toString(): string;
  /**
   * Formats this node and its descendants as an indented tree.
   */
  toTreeString(indent?: string): string;
  /**
   * Reports whether the node contains information relevant to UI automation.
   */
  shouldKeepNode(): boolean;
}

/**
 * Captures the active Android package, activity, and simplified UI hierarchy.
 */
export interface UIPageResultData {
  packageName: string;
  activityName: string;
  uiElements: SimplifiedUINode;
  /**
   * Formats the current UI page for tool output.
   */
  toString(): string;
}

/**
 * Describes an executed UI action and its optional target coordinates or element identifier.
 */
export interface UIActionResultData {
  actionType: string;
  actionDescription: string;
  coordinates?: [number, number];
  elementId?: string;
  /**
   * Formats the UI action for tool output.
   */
  toString(): string;
}

/**
 * Identifies the command interpreter backing a terminal session.
 */
export type TerminalType = "powershell" | "bash" | "linux" | "shell";

/**
 * Selects the command interpreter requested when creating a terminal session.
 */
export type TerminalCreateType = TerminalType;

/**
 * Reports the start or an incremental chunk of a streamed chat-message response.
 */
export interface MessageSendStreamEventData {
  /**
   * Event type, currently "start" or "chunk"
   */
  type: string;
  /**
   * The ID of the chat receiving the streamed reply
   */
  chatId: string;
  /**
   * The original message content that was sent
   */
  message: string;
  /**
   * Whether waifu-style chunk aggregation is enabled for this stream
   */
  waifu: boolean;
  /**
   * Incremental chunk content for "chunk" events
   */
  chunk?: string | null;
  /**
   * Zero-based chunk index
   */
  chunkIndex?: number | null;
  /**
   * Total received character count so far
   */
  receivedChars?: number | null;
  /**
   * Returns the original message associated with this stream event.
   */
  toString(): string;
}

/**
 * Wraps a boolean value returned by a built-in tool.
 */
export interface BooleanResultData {
  value: boolean;
}

/**
 * Wraps a string inside the Rust tool runtime before the JavaScript bridge emits a primitive string.
 */
export interface StringResultData {
  /**
   * Contains the returned string value.
   */
  value: string;
}

/**
 * Records the requested and actual duration of a completed sleep operation.
 */
export interface SleepResultData {
  requestedMs: number;
  sleptMs: number;
  /**
   * Formats the actual sleep duration in milliseconds.
   */
  toString(): string;
}

/**
 * Reports the current value and existence state of an environment variable.
 */
export interface EnvironmentVariableReadResultData {
  key: string;
  value?: string | null;
  exists: boolean;
  /**
   * Formats the variable as a key-value pair or reports that it is unset.
   */
  toString(): string;
}

/**
 * Reports the requested and resulting state of an environment-variable update.
 */
export interface EnvironmentVariableWriteResultData {
  key: string;
  requestedValue: string;
  value?: string | null;
  exists: boolean;
  cleared: boolean;
  /**
   * Formats the variable's resulting value or reports that it was cleared.
   */
  toString(): string;
}

/**
 * Wraps an integer value returned by a built-in tool.
 */
export interface IntResultData {
  value: number;
}

/**
 * Carries raw bytes returned by a built-in tool.
 */
export interface BinaryResultData {
  value: number[];
}

/**
 * Contains one numbered line segment read from a text file.
 */
export interface FilePartContentData {
  path: string;
  content: string;
  partIndex: number;
  totalParts: number;
  startLine: number;
  endLine: number;
  totalLines: number;
  /**
   * Formats the segment number and line range followed by the segment content.
   */
  toString(): string;
}

/**
 * Describes one file-system entry returned in a directory listing.
 */
export interface FileEntry {
  name: string;
  isDirectory: boolean;
  size: number;
  permissions: string;
  lastModified: string;
}

/**
 * Contains the entries found at a virtual file-system directory path.
 */
export interface DirectoryListingData {
  path: string;
  entries: FileEntry[];
  /**
   * Formats directory entries with type, permissions, size, modification time, and name.
   */
  toString(): string;
}

/**
 * Contains the complete text and byte size read from a file.
 */
export interface FileContentData {
  path: string;
  content: string;
  size: number;
  /**
   * Formats the file path followed by its text content.
   */
  toString(): string;
}

/**
 * Contains file bytes encoded as Base64 together with the original byte size.
 */
export interface BinaryFileContentData {
  path: string;
  /**
   * Base64 encoded content of the file
   */
  contentBase64: string;
  /**
   * File size in bytes
   */
  size: number;
  /**
   * Summarizes the binary file path, byte size, and Base64 character count.
   */
  toString(): string;
}

/**
 * Reports whether a virtual file-system path exists and what it contains.
 */
export interface FileExistsData {
  path: string;
  exists: boolean;
  isDirectory: boolean;
  size: number;
  /**
   * Reports whether the path exists, its kind, and its byte size.
   */
  toString(): string;
}

/**
 * Reports virtual file-system stat metadata, including ownership, permissions, and raw output.
 */
export interface FileInfoData {
  path: string;
  exists: boolean;
  fileType: string;
  size: number;
  permissions: string;
  owner: string;
  group: string;
  lastModified: string;
  rawStatOutput: string;
  /**
   * Formats the path's type, size, permissions, ownership, and modification time.
   */
  toString(): string;
}

/**
 * Reports the target, success state, and details of a virtual file-system mutation.
 */
export interface FileOperationData {
  operation: string;
  path: string;
  successful: boolean;
  details: string;
  /**
   * Returns the operation details supplied by the file-system host.
   */
  toString(): string;
}

/**
 * Combines a file operation outcome with its generated diff and patch instructions.
 */
export interface FileApplyResultData {
  operation: FileOperationData;
  aiDiffInstructions: string;
  diffContent?: string;
  /**
   * Formats the file operation with diff markup, request context, and generated instructions.
   */
  toString(): string;
}

/**
 * Contains an HTTP response status, headers, cookies, media type, and decoded or Base64 body.
 */
export interface HttpResponseData {
  url: string;
  statusCode: number;
  statusMessage: string;
  headers: Record<string, string>;
  contentType: string;
  content: string;
  contentBase64?: string;
  size: number;
  cookies: Record<string, string>;
  /**
   * Formats response metadata, a bounded cookie preview, and the decoded body content.
   */
  toString(): string;
}

/**
 * Describes one metadata or body-chunk event from a streaming HTTP response.
 */
export interface HttpStreamEventData {
  type: string;
  url: string;
  statusCode?: number;
  statusMessage?: string;
  headers: Record<string, string>;
  contentType?: string;
  chunk?: string;
  chunkIndex?: number;
  receivedBytes?: number;
  /**
   * Returns chunk content directly or formats the stream-start status and other event kinds.
   */
  toString(): string;
}

/**
 * Identifies a system-setting namespace, key, and current value.
 */
export interface SystemSettingData {
  namespace: string;
  setting: string;
  value: string;
  /**
   * Formats the namespaced setting name and its current value.
   */
  toString(): string;
}

/**
 * Reports the outcome of installing, uninstalling, starting, or stopping an application package.
 */
export interface AppOperationData {
  operationType: string;
  packageName: string;
  success: boolean;
  details: string;
  /**
   * Formats a successful package operation or returns the host-provided operation details.
   */
  toString(): string;
}

/**
 * Lists installed application packages and whether system applications were included.
 */
export interface AppListData {
  includesSystemApps: boolean;
  packages: string[];
  /**
   * Formats installed package names and indicates whether the list includes system apps.
   */
  toString(): string;
}

/**
 * Records one application's foreground duration, last use, and system-app classification.
 */
export interface AppUsageTimeEntry {
  packageName: string;
  appName: string;
  totalForegroundTimeMs: number;
  lastTimeUsed: number;
  isSystemApp: boolean;
}

/**
 * Contains application foreground usage over a requested time window and package filter.
 */
export interface AppUsageTimeResultData {
  startTime: number;
  endTime: number;
  sinceHours: number;
  requestedPackageName?: string;
  includesSystemApps: boolean;
  totalEntries: number;
  entries: AppUsageTimeEntry[];
  /**
   * Formats the query window and each application's human-readable foreground duration.
   */
  toString(): string;
}

/**
 * Captures the source, text, and arrival time of one system notification.
 */
export interface Notification {
  packageName: string;
  text: string;
  timestamp: number;
}

/**
 * Contains the system notifications visible at a specific retrieval time.
 */
export interface NotificationData {
  /**
   * List of notification objects
   */
  notifications: Notification[];
  /**
   * Timestamp when the notifications were retrieved
   */
  timestamp: number;
  /**
   * Formats a numbered list of notification package names and text content.
   */
  toString(): string;
}

/**
 * Reports a device location with provider accuracy, raw data, and reverse-geocoded address fields.
 */
export interface LocationData {
  /**
   * Latitude coordinate in decimal degrees
   */
  latitude: number;
  /**
   * Longitude coordinate in decimal degrees
   */
  longitude: number;
  /**
   * Accuracy of the location in meters
   */
  accuracy: number;
  /**
   * Location provider (e.g., "gps", "network", etc.)
   */
  provider: string;
  /**
   * Timestamp when the location was retrieved
   */
  timestamp: number;
  /**
   * Raw location data from the system
   */
  rawData: string;
  /**
   * Street address determined from coordinates
   */
  address: string;
  /**
   * City name determined from coordinates
   */
  city: string;
  /**
   * Province/state name determined from coordinates
   */
  province: string;
  /**
   * Country name determined from coordinates
   */
  country: string;
  /**
   * Formats coordinates, accuracy, provider, local timestamp, and available address fields.
   */
  toString(): string;
}

/**
 * Summarizes device identity, Android version, display, memory, storage, power, CPU, and network state.
 */
export interface DeviceInfoResultData {
  deviceId: string;
  model: string;
  manufacturer: string;
  androidVersion: string;
  sdkVersion: number;
  screenResolution: string;
  screenDensity: number;
  totalMemory: string;
  availableMemory: string;
  totalStorage: string;
  availableStorage: string;
  batteryLevel: number;
  batteryCharging: boolean;
  cpuInfo: string;
  networkType: string;
  additionalInfo: Record<string, string>;
  /**
   * Formats a labeled report of device hardware, Android, storage, power, and network details.
   */
  toString(): string;
}

/**
 * Describes one stored memory together with ownership, provenance, tags, and chunk metadata.
 */
export interface MemoryInfo {
  ownerKey: string;
  title: string;
  content: string;
  source: string;
  tags: string[];
  createdAt: string;
  chunkInfo?: string | null;
  chunkIndices?: number[] | null;
}

/**
 * Contains matched memories and snapshot metadata used to suppress previously returned matches.
 */
export interface MemoryQueryResultData {
  /**
   * Queried memories
   */
  memories: MemoryInfo[];
  /**
   * Snapshot id for de-duplicated follow-up or parallel queries; may be auto-generated or caller-specified
   */
  snapshotId?: string | null;
  /**
   * Whether this call created a new snapshot, including when a caller-specified id was created on first use
   */
  snapshotCreated?: boolean;
  /**
   * Number of matched memories excluded because they were already seen in the snapshot
   */
  excludedBySnapshotCount?: number;
  /**
   * Formats snapshot de-duplication metadata followed by the matched memory records.
   */
  toString(): string;
}

/**
 * Reports whether the chat service connected and when the connection was established.
 */
export interface ChatServiceStartResultData {
  /**
   * Whether the service is connected
   */
  isConnected: boolean;
  /**
   * Connection timestamp
   */
  connectionTime: number;
}

/**
 * Identifies a newly created chat and its creation time.
 */
export interface ChatCreationResultData {
  /**
   * The ID of the newly created chat
   */
  chatId: string;
  /**
   * Creation timestamp
   */
  createdAt: number;
}

/**
 * Summarizes a chat, its activity, token use, current status, and bound character card.
 */
export interface ChatInfo {
  /**
   * Chat ID
   */
  id: string;
  /**
   * Chat title
   */
  title: string;
  /**
   * Number of messages in the chat
   */
  messageCount: number;
  /**
   * Creation timestamp
   */
  createdAt: string;
  /**
   * Last updated timestamp
   */
  updatedAt: string;
  /**
   * Whether this is the current active chat
   */
  isCurrent: boolean;
  /**
   * Total input tokens used
   */
  inputTokens: number;
  /**
   * Total output tokens used
   */
  outputTokens: number;
  /**
   * Bound character card name (if any)
   */
  characterCardName?: string | null;
}

/**
 * Contains the available chats and identifies the currently active chat.
 */
export interface ChatListResultData {
  /**
   * Total number of chats
   */
  totalCount: number;
  /**
   * The ID of the current active chat
   */
  currentChatId: string | null;
  /**
   * List of chat information
   */
  chats: ChatInfo[];
  /**
   * Formats chat summaries, marks the current chat, and includes token and card metadata.
   */
  toString(): string;
}

/**
 * Reports the number of matching chats and the selected match, when one exists.
 */
export interface ChatFindResultData {
  /**
   * Total matched chats
   */
  matchedCount: number;
  /**
   * The selected chat (if any)
   */
  chat: ChatInfo | null;
  /**
   * Reports the selected chat identifier and total match count, or that no chat was found.
   */
  toString(): string;
}

/**
 * Reports a chat agent's current state and whether it is idle or processing.
 */
export interface AgentStatusResultData {
  /**
   * Target chat id
   */
  chatId: string;
  /**
   * Current state key
   */
  state: string;
  /**
   * Optional detail message
   */
  message?: string | null;
  /**
   * Whether the chat is idle
   */
  isIdle: boolean;
  /**
   * Whether the chat is processing
   */
  isProcessing: boolean;
  /**
   * Formats the chat agent's state with its optional detail message.
   */
  toString(): string;
}

/**
 * Identifies the chat selected as active and the time of the switch.
 */
export interface ChatSwitchResultData {
  /**
   * The ID of the chat switched to
   */
  chatId: string;
  /**
   * The title of the chat
   */
  chatTitle: string;
  /**
   * Switch timestamp
   */
  switchedAt: number;
  /**
   * Formats the selected chat title and identifier.
   */
  toString(): string;
}

/**
 * Reports a chat's updated title and update time.
 */
export interface ChatTitleUpdateResultData {
  /**
   * Target chat ID
   */
  chatId: string;
  /**
   * Updated title
   */
  title: string;
  /**
   * Update timestamp
   */
  updatedAt: number;
}

/**
 * Identifies a deleted chat and the time it was deleted.
 */
export interface ChatDeleteResultData {
  /**
   * Deleted chat ID
   */
  chatId: string;
  /**
   * Delete timestamp
   */
  deletedAt: number;
}

/**
 * Records a sent chat message and its optional final AI reply.
 */
export interface MessageSendResultData {
  /**
   * The ID of the chat the message was sent to
   */
  chatId: string;
  /**
   * The message content that was sent
   */
  message: string;
  /**
   * Final AI response content when available
   */
  aiResponse?: string | null;
  /**
   * Reply receive timestamp when available
   */
  receivedAt?: number | null;
  /**
   * Sent timestamp
   */
  sentAt: number;
  /**
   * Formats bounded previews of the sent message and optional AI reply.
   */
  toString(): string;
}

/**
 * Describes one chat message together with its role, provider, model, and timestamp.
 */
export interface ChatMessageInfo {
  sender: string;
  content: string;
  timestamp: number;
  roleName: string;
  provider: string;
  modelName: string;
}

/**
 * Contains an ordered, limited message history for a chat.
 */
export interface ChatMessagesResultData {
  chatId: string;
  order: string;
  limit: number;
  messages: ChatMessageInfo[];
}

/**
 * Describes a character card, including its default status and lifecycle timestamps.
 */
export interface CharacterCardInfo {
  id: string;
  name: string;
  description: string;
  isDefault: boolean;
  createdAt: number;
  updatedAt: number;
}

/**
 * Contains the character cards available to the chat service.
 */
export interface CharacterCardListResultData {
  totalCount: number;
  cards: CharacterCardInfo[];
  /**
   * Formats character card metadata and marks the default card.
   */
  toString(): string;
}

/**
 * Reports the runtime platform, default terminal type, and available interpreters.
 */
export interface TerminalInfoResultData {
  /**
   * Current runtime platform, such as windows, linux, or android
   */
  platform: string;
  /**
   * Default terminal type for this platform
   */
  defaultType: TerminalType;
  /**
   * Terminal types known to this host
   */
  types: TerminalTypeInfoData[];
  /**
   * Formats the platform, default terminal, and availability of each known terminal type.
   */
  toString(): string;
}

/**
 * Reports whether one terminal interpreter is available and describes its purpose.
 */
export interface TerminalTypeInfoData {
  /**
   * Terminal type id supported by a terminal host
   */
  terminalType: TerminalType;
  /**
   * Whether this terminal type is available on the current platform
   */
  available: boolean;
  /**
   * Human-readable terminal type description
   */
  description: string;
}

/**
 * Records a terminal command's session, interpreter, output, exit code, and timeout state.
 */
export interface TerminalCommandResultData {
  /**
   * The command that was executed
   */
  command: string;
  /**
   * The output from the command execution
   */
  output: string;
  /**
   * Exit code from the command (0 typically means success)
   */
  exitCode: number;
  /**
   * ID of the terminal session used for execution
   */
  sessionId: string;
  /**
   * Actual terminal type used for execution
   */
  terminalType: TerminalType;
  /**
   * Whether this execution ended due to timeout. On timeout, the current command is cancelled and the terminal session is kept.
   */
  timedOut: boolean;
  /**
   * Formats command, session, interpreter, exit, timeout, and captured output details.
   */
  toString(): string;
}

/**
 * Reports the start or an incremental output chunk of a terminal command stream.
 */
export interface TerminalStreamEventData {
  /**
   * Event type, currently "start" or "chunk"
   */
  type: string;
  /**
   * The command being executed
   */
  command: string;
  /**
   * ID of the terminal session used for execution
   */
  sessionId: string;
  /**
   * Incremental output chunk for "chunk" events
   */
  chunk?: string | null;
  /**
   * Zero-based chunk index
   */
  chunkIndex?: number | null;
  /**
   * Total received character count so far
   */
  receivedChars?: number | null;
  /**
   * Returns an output chunk directly or labels the stream lifecycle event.
   */
  toString(): string;
}

/**
 * Records a hidden executor command's interpreter, output, exit code, and timeout state.
 */
export interface HiddenTerminalCommandResultData {
  /**
   * The command that was executed
   */
  command: string;
  /**
   * The output from the command execution
   */
  output: string;
  /**
   * Exit code from the command (0 typically means success)
   */
  exitCode: number;
  /**
   * Hidden executor key used for execution
   */
  executorKey: string;
  /**
   * Actual terminal type used for execution
   */
  terminalType: TerminalType;
  /**
   * Whether this execution ended due to timeout. On timeout, the current command is cancelled and the hidden executor session is kept.
   */
  timedOut: boolean;
  /**
   * Formats command, executor, interpreter, exit, timeout, and captured output details.
   */
  toString(): string;
}

/**
 * Identifies a created or reused named terminal session and its interpreter.
 */
export interface TerminalSessionCreationResultData {
  /**
   * ID of the created or retrieved session
   */
  sessionId: string;
  /**
   * Name of the session
   */
  sessionName: string;
  /**
   * Actual terminal type for the session
   */
  terminalType: TerminalType;
  /**
   * Whether a new session was created
   */
  isNewSession: boolean;
  /**
   * Reports whether the named terminal session was created or reused and identifies it.
   */
  toString(): string;
}

/**
 * Reports whether a terminal session closed successfully and provides the host message.
 */
export interface TerminalSessionCloseResultData {
  /**
   * ID of the closed session
   */
  sessionId: string;
  /**
   * Whether the session was closed successfully
   */
  success: boolean;
  /**
   * A message describing the result
   */
  message: string;
}

/**
 * Captures one terminal session's visible screen, dimensions, interpreter, and running state.
 */
export interface TerminalSessionScreenResultData {
  /**
   * ID of the session
   */
  sessionId: string;
  /**
   * Actual terminal type for the session
   */
  terminalType: TerminalType;
  /**
   * Screen row count
   */
  rows: number;
  /**
   * Screen column count
   */
  cols: number;
  /**
   * Current visible screen text content
   */
  content: string;
  /**
   * Whether a command is currently running in this session
   */
  commandRunning: boolean;
  /**
   * Formats screen dimensions and session state followed by the visible terminal content.
   */
  toString(): string;
}

/**
 * Reports the active music source, transport state, position, buffering, and volume.
 */
export interface MusicPlaybackResultData {
  /**
   * Playback state
   */
  state: string;
  /**
   * Current audio source
   */
  source?: string | null;
  /**
   * Current audio source type
   */
  sourceType?: string | null;
  /**
   * Display title
   */
  title?: string | null;
  /**
   * Display artist
   */
  artist?: string | null;
  /**
   * Duration in milliseconds, when known
   */
  durationMs?: number | null;
  /**
   * Current playback position in milliseconds
   */
  positionMs: number;
  /**
   * Buffered playback position in milliseconds
   */
  bufferedPositionMs: number;
  /**
   * Playback volume from 0 to 1
   */
  volume: number;
  /**
   * Whether current track loops
   */
  loop: boolean;
  /**
   * Operation message
   */
  message: string;
  /**
   * Formats playback state, track metadata, timing, buffering, volume, looping, and message.
   */
  toString(): string;
}

/**
 * Reports whether Bluetooth is supported, enabled, and its current adapter state.
 */
export interface BluetoothStateData {
  supported: boolean;
  enabled: boolean;
  state: string;
  /**
   * Formats Bluetooth support, enablement, and adapter state.
   */
  toString(): string;
}

/**
 * Identifies a bonded Bluetooth device and its device and bond classifications.
 */
export interface BluetoothDeviceData {
  name?: string;
  address: string;
  type: string;
  bondState: string;
}

/**
 * Contains the devices currently bonded with the Bluetooth adapter.
 */
export interface BluetoothBondedDevicesData {
  devices: BluetoothDeviceData[];
  /**
   * Formats each bonded device's name, address, type, and bond state.
   */
  toString(): string;
}

/**
 * Describes a discovered Bluetooth device, including discovery source and optional signal strength.
 */
export interface BluetoothScannedDeviceData {
  name?: string;
  address: string;
  type: string;
  bondState: string;
  source: string;
  rssi?: number;
}

/**
 * Contains devices discovered during a timed Bluetooth scan and indicates BLE coverage.
 */
export interface BluetoothScanResultData {
  devices: BluetoothScannedDeviceData[];
  durationMs: number;
  includesBle: boolean;
  /**
   * Formats scan duration and BLE coverage followed by every discovered device.
   */
  toString(): string;
}

/**
 * Identifies an open Bluetooth session, its remote address, and connection mode.
 */
export interface BluetoothSessionData {
  sessionId: string;
  address: string;
  mode: string;
  /**
   * Formats the Bluetooth session identifier, remote address, and connection mode.
   */
  toString(): string;
}

/**
 * Reports the number of bytes written through a Bluetooth session.
 */
export interface BluetoothTransferData {
  sessionId: string;
  bytesWritten: number;
  /**
   * Formats the Bluetooth session identifier and number of bytes written.
   */
  toString(): string;
}

/**
 * Contains bytes read from a Bluetooth session as optional text and Base64 data.
 */
export interface BluetoothReadData {
  sessionId: string;
  bytesRead: number;
  text?: string;
  dataBase64?: string;
  /**
   * Formats the byte count and available text or Base64 payload read from the session.
   */
  toString(): string;
}

/**
 * Describes a BLE characteristic UUID and the operations it supports.
 */
export interface BluetoothBleCharacteristicData {
  uuid: string;
  properties: string[];
}

/**
 * Describes a discovered BLE service and its characteristics.
 */
export interface BluetoothBleServiceData {
  uuid: string;
  characteristics: BluetoothBleCharacteristicData[];
}

/**
 * Contains the BLE services and characteristics discovered for a session.
 */
export interface BluetoothBleServicesData {
  sessionId: string;
  services: BluetoothBleServiceData[];
  /**
   * Formats discovered BLE services and the properties of each characteristic.
   */
  toString(): string;
}

/**
 * Carries one timestamped value received from a subscribed BLE characteristic.
 */
export interface BluetoothBleNotificationEntry {
  characteristicUuid: string;
  bytesRead: number;
  text?: string;
  dataBase64?: string;
  timestamp: number;
}

/**
 * Contains timestamped characteristic notifications received by a BLE session.
 */
export interface BluetoothBleNotificationData {
  sessionId: string;
  notifications: BluetoothBleNotificationEntry[];
  /**
   * Formats each received BLE notification with its source, timestamp, and available payload.
   */
  toString(): string;
}

/**
 * Contains extracted web-page content, links, images, metadata, and truncation storage details.
 */
export interface VisitWebResultData {
  url: string;
  title: string;
  content: string;
  metadata: Record<string, string>;
  links: LinkData[];
  imageLinks: string[];
  visitKey?: string;
  contentSavedTo?: string;
  contentTruncated: boolean;
  originalContentLength?: number;
  /**
   * Formats bounded link and image previews, persisted-content metadata, and page content.
   */
  toString(): string;
}

/**
 * Describes a URL and the human-readable text associated with it.
 */
export interface LinkData {
  url: string;
  text: string;
}

/**
 * Contains file paths matching a pattern beneath a requested search path.
 */
export interface FindFilesResultData {
  path: string;
  pattern: string;
  files: string[];
  /**
   * Formats search parameters and a bounded preview of matched file paths.
   */
  toString(): string;
}

/**
 * Identifies one matching line and its optional surrounding grep context.
 */
export interface GrepLineMatch {
  lineNumber: number;
  lineContent: string;
  matchContext?: string;
}

/**
 * Groups grep line matches by source file.
 */
export interface GrepFileMatch {
  filePath: string;
  lineMatches: GrepLineMatch[];
}

/**
 * Summarizes a text search across files and groups all reported line matches by file.
 */
export interface GrepResultData {
  searchPath: string;
  pattern: string;
  matches: GrepFileMatch[];
  totalMatches: number;
  filesSearched: number;
  /**
   * Formats search statistics and bounded, line-aware match context grouped by file.
   */
  toString(): string;
}

/**
 * Describes a newly created weighted relationship between two memories.
 */
export interface MemoryLinkResultData {
  /**
   * The title of the source memory
   */
  sourceTitle: string;
  /**
   * The title of the target memory
   */
  targetTitle: string;
  /**
   * The type of link (e.g., "related", "causes", "explains", "part_of")
   */
  linkType: string;
  /**
   * The strength of the link (0.0-1.0)
   */
  weight: number;
  /**
   * Optional description of the link
   */
  description: string;
  /**
   * Formats the source, target, relationship type, and strength of the created memory link.
   */
  toString(): string;
}

/**
 * Describes a weighted relationship between two stored memories.
 */
export interface LinkInfo {
  linkId: number;
  sourceTitle: string;
  targetTitle: string;
  linkType: string;
  weight: number;
  description: string;
}

/**
 * Contains the weighted memory relationships returned by a link query.
 */
export interface MemoryLinkQueryResultData {
  /**
   * Number of links returned
   */
  totalCount: number;
  /**
   * Queried links
   */
  links: LinkInfo[];
  /**
   * Formats each queried memory link with identifiers, relationship metadata, and description.
   */
  toString(): string;
}
