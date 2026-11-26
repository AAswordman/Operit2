export interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH';
  body?: unknown;
  signal?: AbortSignal;
}

export interface HostRequest {
  path: string;
  method: RequestOptions['method'];
  body: unknown;
  signal?: AbortSignal;
}

export interface HostBridge {
  request<T>(request: HostRequest): Promise<T>;
}

export interface RuntimeModule {
  HEAPU8: Uint8Array<ArrayBufferLike>;
  addFunction(callback: (...args: number[]) => void, signature: string): number;
  removeFunction(pointer: number): void;
  _operit_ui_pump(milliseconds: number): void;
  _operit_ui_navigate_home(): void;
  _operit_ui_navigate_apps(): void;
  _operit_ui_round_icons(): boolean;
  _operit_ui_set_connection(connected: boolean, configured: boolean): void;
  _operit_ui_set_paired(paired: boolean): void;
  _operit_ui_set_theme(index: number, circle: boolean): void;
  _operit_ui_set_emoji_style(style: number): boolean;
  _operit_ui_emoji_style(): number;
  _operit_ui_theme_index(): number;
  _operit_ui_debug_tree?(): number;
  _operit_ui_debug_snapshot?(): number;
  _operit_ui_debug_tap?(id: number): boolean;
  _operit_ui_debug_swipe?(direction: number): boolean;
  _simulator_frame(): number;
  _simulator_generation(): number;
  _simulator_heap_used(): number;
  _simulator_heap_total?(): number;
  _simulator_heap_free?(): number;
  _simulator_heap_largest?(): number;
  _simulator_heap_peak?(): number;
  _simulator_stack_size?(): number;
  _simulator_stack_free?(): number;
  _simulator_init(): boolean;
  _simulator_touch(x: number, y: number, pressed: number): void;
  ccall(
    identifier: string,
    returnType: 'number',
    argumentTypes: string[],
    values: Array<string | number | Uint8Array>,
  ): number;
  ccall(
    identifier: string,
    returnType: 'string',
    argumentTypes: string[],
    values: Array<string | number | Uint8Array>,
  ): string;
  ccall(
    identifier: string,
    returnType: null,
    argumentTypes: string[],
    values: Array<string | number | Uint8Array>,
  ): null;
  onAction?: (value: string) => void;
}

export type RuntimeFactory = () => Promise<RuntimeModule>;

export interface BoardTheme {
  name: string;
  bg: string;
  surface: string;
  accent: string;
  muted: string;
}

export interface BoardInfo {
  model: string;
  width: number;
  height: number;
  controller: string;
  themes: BoardTheme[];
  mode: string;
}

export interface BuildManifest {
  runtimeHash?: string;
  sourceHash?: string;
  builtAt?: string;
  firmwareBuilt?: boolean;
  firmwareElf?: string;
  firmwareSha256?: string;
  renderer?: 'mini';
  source?: string;
}

export interface BuildStatus {
  running: boolean;
  error: string | null;
  manifest: BuildManifest | null;
  stale: boolean;
  output: string;
}

/** Returns a required DOM element with a precise element type. */
export function query<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error('Missing DOM element: ' + selector);
  return element;
}

/** Converts an unknown thrown value into a displayable message. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

declare global {
  interface Window {
    operitHost?: HostBridge;
  }
}
