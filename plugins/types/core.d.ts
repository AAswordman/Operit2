// Generated from operit-plugin-sdk Rust declarations.

/**
 * Stores a scalar or JSON value assigned to an arbitrary tool parameter.
 */
export type ToolParamsAdditionalValue = string | number | boolean | unknown;

/**
 * Selects the native algorithm used to decompress plugin data.
 */
export type NativeInterfaceHostDecompressAlgorithm = "deflate";

/**
 * Selects the native cryptographic operation family requested by a plugin.
 */
export type NativeInterfaceHostCryptoAlgorithm = "md5" | "aes";

/**
 * Stores the named arguments passed to a tool invocation.
 */
export interface ToolParams extends Record<string, ToolParamsAdditionalValue> {
}

/**
 * Configures an object-style tool invocation and its streaming callback.
 */
export interface ToolConfig {
  /**
   * Selects the tool category when the runtime requires one.
   */
  type?: string;
  /**
   * Identifies the tool to invoke.
   */
  name: string;
  /**
   * Contains the tool arguments.
   */
  params?: ToolParams;
  /**
   * Receives intermediate values produced by a streaming tool.
   */
  onIntermediateResult?: (arg0: any) => void;
}

/**
 * Configures callbacks for a global tool call.
 */
export interface ToolCallOptions<TIntermediate = any> {
  /**
   * Receives an intermediate tool result.
   */
  onIntermediateResult?: (arg0: TIntermediate) => void;
}

/**
 * Reports whether an operation succeeded and carries its error when present.
 */
export interface BaseResult {
  /**
   * Reports whether the tool operation succeeded.
   */
  success: boolean;
  /**
   * Contains the operation error message.
   */
  error?: string;
}

/**
 * Returns a string together with the operation status.
 */
export interface StringResult extends BaseResult {
  /**
   * Contains the returned string.
   */
  data: string;
  /**
   * Returns the string stored in this result.
   */
  toString(): string;
}

/**
 * Contains a boolean tool result.
 */
export interface BooleanResult extends BaseResult {
  /**
   * Contains the returned boolean.
   */
  data: boolean;
  /**
   * Formats the boolean stored in this result.
   */
  toString(): string;
}

/**
 * Contains a numeric tool result.
 */
export interface NumberResult extends BaseResult {
  /**
   * Contains the returned number.
   */
  data: number;
  /**
   * Formats the number stored in this result.
   */
  toString(): string;
}

/**
 * Contains a dynamically typed structured tool result.
 */
export interface DynamicToolResult extends BaseResult {
  /**
   * Contains the tool-specific result value.
   */
  data: any;
}

/**
 * Holds any scalar or JSON-compatible result returned by a tool invocation.
 */
export type ToolResult = StringResult | BooleanResult | NumberResult | DynamicToolResult;

/**
 * Resolves a statically known tool name to its declared result type.
 */
export type ToolReturnType<T> = T extends keyof import("./tool-types").ToolResultMap ? import("./tool-types").ToolResultMap[T] : any;

/**
 * Configures an object-style call whose tool name remains statically typed.
 */
export interface NamedToolConfig<T extends string> {
  /**
   * Selects the tool category when the runtime requires one.
   */
  type?: string;
  /**
   * Contains the statically known tool name.
   */
  name: T;
  /**
   * Contains the tool arguments.
   */
  params?: ToolParams;
  /**
   * Receives intermediate values produced by a streaming tool.
   */
  onIntermediateResult?: (arg0: any) => void;
}

/**
 * Supplies either an array or an object to a collection utility.
 */
export type LodashCollection<T> = T[] | object;

/**
 * Provides collection iteration and dynamic value predicates to plugin scripts.
 */
export interface LodashApi {
  /**
   * Reports whether a dynamic value is empty.
   */
  isEmpty(value: any): boolean;
  /**
   * Reports whether a dynamic value is a string.
   */
  isString(value: any): boolean;
  /**
   * Reports whether a dynamic value is a number.
   */
  isNumber(value: any): boolean;
  /**
   * Reports whether a dynamic value is a boolean.
   */
  isBoolean(value: any): boolean;
  /**
   * Reports whether a dynamic value is an object.
   */
  isObject(value: any): boolean;
  /**
   * Reports whether a dynamic value is an array.
   */
  isArray(value: any): boolean;
  /**
   * Invokes a callback for every collection entry.
   */
  forEach<T>(collection: LodashCollection<T>, iteratee: (arg0: any, arg1: any, arg2: any) => void): any;
  /**
   * Maps every collection entry to a new result value.
   */
  map<T, R>(collection: LodashCollection<T>, iteratee: (arg0: any, arg1: any, arg2: any) => R): R[];
}

/**
 * Exposes the lodash-like utility service as a plugin global.
 */
export declare const _: LodashApi;

/**
 * Accepts a date value or date string for formatting.
 */
export type DataUtilsDateInput = string;

/**
 * Parses, serializes, and formats values used by plugin scripts.
 */
export interface DataUtilsApi {
  /**
   * Parses a JSON string into a dynamic JavaScript value.
   */
  parseJson(jsonString: string): any;
  /**
   * Serializes a dynamic JavaScript value as JSON.
   */
  stringifyJson(obj: any): string;
  /**
   * Formats an optional date or string value.
   */
  formatDate(date?: DataUtilsDateInput): string;
}

/**
 * Exposes data conversion utilities as a plugin global.
 */
export declare const dataUtils: DataUtilsApi;

/**
 * Stores the named values assigned to CommonJS module exports.
 */
export declare var exports: Record<string, any>;

/**
 * Provides synchronous Android runtime services used by plugin scripts.
 */
export namespace NativeInterface {
  /**
   * Call a tool synchronously (legacy method)
   * @param toolType - Tool type
   * @param toolName - Tool name
   * @param paramsJson - Parameters as JSON string
   * @returns A JSON string representing a ToolResult object
   */
  function callTool(toolType: string, toolName: string, paramsJson: string): string;
  /**
   * Call a tool asynchronously
   * @param callbackId - Unique callback ID
   * @param toolType - Tool type
   * @param toolName - Tool name
   * @param paramsJson - Parameters as JSON string
   * The callback will receive a ToolResult object
   */
  function callToolAsync(callbackId: string, toolType: string, toolName: string, paramsJson: string): void;
  /**
   * Starts an asynchronous tool call and routes both intermediate and final results to callbacks.
   */
  function callToolAsyncStreaming(callbackId: string, intermediateCallbackId: string, toolType: string, toolName: string, paramsJson: string): void;
  /**
   * Execute native crypto operations used by the CryptoJS bridge.
   */
  function crypto(algorithm: NativeInterfaceHostCryptoAlgorithm, operation: string, argsJson: string): string;
  /**
   * Decompress native deflate data from a base64 string or binary handle.
   */
  function decompress(data: string, algorithm: NativeInterfaceHostDecompressAlgorithm): string;
  /**
   * Resolve the persistent config directory for a package or toolpkg.
   * Returns an absolute path under `/sdcard/Download/Operit/plugins/<id>`.
   */
  function getPluginConfigDir(pluginId: string): string;
  /**
   * Execute native image operations used by the Jimp bridge.
   */
  function image_processing(callbackId: string, operation: string, argsJson: string): void;
  /**
   * Log debug message with data
   * @param message - Debug message
   * @param data - Debug data
   */
  function logDebug(message: string, data: string): void;
  /**
   * Log error message
   * @param message - Error message to log
   */
  function logError(message: string): void;
  /**
   * Log informational message
   * @param message - Message to log
   */
  function logInfo(message: string): void;
  /**
   * Register an image from base64-encoded data into the global image pool
   * and return a `<link type="image" id="...">` tag string that can be
   * embedded into tool results or messages.
   */
  function registerImageFromBase64(base64: string, mimeType: string): string;
  /**
   * Register an image from a file path on the device into the global image
   * pool and return a `<link type="image" id="...">` tag string that can
   * be embedded into tool results or messages.
   */
  function registerImageFromPath(path: string): string;
  /**
   * Register an app lifecycle hook for current toolpkg main registration session.
   * @param specJson - JSON object string describing an app lifecycle hook
   */
  function registerToolPkgAppLifecycleHook(specJson: string): void;
  /**
   * Register a chat input hook for current toolpkg main registration session.
   * @param specJson - JSON object string describing a chat input hook
   */
  function registerToolPkgChatInputHook(specJson: string): void;
  /**
   * Register an input menu toggle plugin for current toolpkg main registration session.
   * @param specJson - JSON object string describing an input menu toggle plugin
   */
  function registerToolPkgInputMenuTogglePlugin(specJson: string): void;
  /**
   * Register a message processing plugin for current toolpkg main registration session.
   * @param specJson - JSON object string describing a message processing plugin
   */
  function registerToolPkgMessageProcessingPlugin(specJson: string): void;
  /**
   * Register a toolbox UI module for current toolpkg main registration session.
   * @param specJson - JSON object string describing a toolbox UI module
   */
  function registerToolPkgToolboxUiModule(specJson: string): void;
  /**
   * Register an XML render plugin for current toolpkg main registration session.
   * @param specJson - JSON object string describing an XML render plugin
   */
  function registerToolPkgXmlRenderPlugin(specJson: string): void;
  /**
   * Report a script error with its source line and stack details.
   * @param errorType - Error type
   * @param errorMessage - Error message
   * @param errorLine - Line number where error occurred
   * @param errorStack - Error stack trace
   */
  function reportError(errorType: string, errorMessage: string, errorLine: number, errorStack: string): void;
  /**
   * Set an error for script execution
   * @param error - Error message
   */
  function setError(error: string): void;
  /**
   * Set the result of script execution
   * @param result - Result string
   */
  function setResult(result: string): void;
}

/**
 * Global function to complete tool execution with a result
 * Result values must be JSON-serializable.
 * @param result - The result to return
 */
export declare function complete<T>(result: T): void;
/**
 * Global function to call a tool and get a result
 * Note: Promise-based waiting does not guarantee the underlying tool work is truly parallel.
 * @returns A Promise with the tool result data of the appropriate type
 */
export declare function toolCall<T extends string>(toolType: string, toolName: T, toolParams?: ToolParams): Promise<ToolReturnType<T>>;
/**
 * Calls a tool by its globally registered name.
 */
export declare function toolCall<T extends string>(toolName: T, toolParams?: ToolParams): Promise<ToolReturnType<T>>;
/**
 * Calls a tool with object-style configuration.
 */
export declare function toolCall<T extends string>(config: NamedToolConfig<T>): Promise<ToolReturnType<T>>;
/**
 * Calls a categorized tool and receives intermediate results.
 */
export declare function toolCall<T extends string, TIntermediate>(toolType: string, toolName: T, toolParams: ToolParams | undefined, options: ToolCallOptions<TIntermediate>): Promise<ToolReturnType<T>>;
/**
 * Calls a globally named tool and receives intermediate results.
 */
export declare function toolCall<T extends string, TIntermediate>(toolName: T, toolParams: ToolParams | undefined, options: ToolCallOptions<TIntermediate>): Promise<ToolReturnType<T>>;
/**
 * Calls a dynamically named tool.
 */
export declare function toolCall(toolName: string): Promise<any>;
