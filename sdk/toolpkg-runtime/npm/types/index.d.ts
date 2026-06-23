export interface ToolPkgRuntimeClientOptions {
  runtimeBin: string;
  languageCode: string;
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  host?: ToolPkgRuntimeHost;
}

export interface ToolPkgRuntimeHost {
  invokeTool(tool: ToolCallRequest): Promise<ToolResult[]> | ToolResult[];
}

export interface ToolCallRequest {
  name: string;
  parameters: ToolParameter[];
}

export interface ToolParameter {
  name: string;
  value: string;
}

export interface StringResultData {
  __type: 'StringResultData';
  value: string;
}

export interface ToolResult {
  toolName: string;
  success: boolean;
  result: StringResultData | Record<string, unknown>;
  error?: string | null;
}

export interface ToolPkgExecutionOutcome {
  value?: string | null;
}

export interface ToolPkgFunctionCall {
  script: string;
  functionName: string;
  params?: Record<string, unknown>;
  envOverrides?: Record<string, string>;
  executionContextKey?: string | null;
  timeoutSeconds?: number | null;
}

export interface ToolPkgMainHookCall {
  containerPackageName: string;
  functionName: string;
  event: string;
  eventName?: string | null;
  pluginId?: string | null;
  functionSource?: string | null;
  eventPayload?: unknown;
  executionContextKey?: string | null;
  runtimeKind?: string | null;
}

export interface ToolPkgIpcCall {
  packageTarget: string;
  callerContextKey?: string | null;
  targetContextKey?: string | null;
  targetRuntime?: string | null;
  channel: string;
  payload?: unknown;
}

export class ToolPkgRuntimeClient {
  constructor(options: ToolPkgRuntimeClientOptions);
  loadToolPkgFile(path: string): Promise<unknown>;
  readToolPkgTextResource(
    containerPackageName: string,
    resourcePath: string
  ): Promise<string | null>;
  runFunction(call: ToolPkgFunctionCall): Promise<ToolPkgExecutionOutcome>;
  runMainHook(call: ToolPkgMainHookCall): Promise<ToolPkgExecutionOutcome>;
  dispatchIpc(call: ToolPkgIpcCall): Promise<ToolPkgExecutionOutcome>;
  destroyContext(contextKey: string): Promise<boolean>;
  destroy(): Promise<boolean>;
  close(): void;
  on(event: 'stderr', listener: (text: string) => void): this;
  on(event: 'unmatchedResponse', listener: (response: unknown) => void): this;
}

export declare function stringToolResult(toolName: string, value: string): ToolResult;
export declare function errorToolResult(toolName: string, message: string): ToolResult;
