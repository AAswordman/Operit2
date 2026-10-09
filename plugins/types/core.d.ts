// Generated from operit-plugin-sdk Rust declarations.

/**
 * Stores a scalar or JSON value assigned to an arbitrary tool parameter.
 */
export type ToolParamsAdditionalValue = string | number | boolean | unknown;

/**
 * Stores the named arguments passed to a tool invocation.
 */
export interface ToolParams {
  [key: string]: ToolParamsAdditionalValue;
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
