import type {CatalogItem} from '../src/layout/layout-model.mts';
import type {ComponentSource} from '../src/source/component-source.mts';
import type {LayoutDocument, LayoutNode, LayoutPage, ProjectRoute} from '../src/layout/project-model.mts';
import type {EventBinding, Route} from '../src/layout/routes.mts';

export type {CatalogItem} from '../src/layout/layout-model.mts';
export type {ComponentSource} from '../src/source/component-source.mts';
export type {LayoutDocument, LayoutNode, LayoutPage, ProjectRoute} from '../src/layout/project-model.mts';
export type {EventBinding, Route} from '../src/layout/routes.mts';

export type PropertyInput = HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement;

export type NodePropertyKey =
  | 'x'
  | 'y'
  | 'w'
  | 'h'
  | 'radius'
  | 'value'
  | 'color'
  | 'text'
  | 'action'
  | 'longAction'
  | 'parent'
  | 'binding'
  | 'fontSize';

export type PageLike = LayoutDocument | LayoutPage;

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

export interface EditorBridge {
  isDirty(): boolean;
  isEditing(): boolean;
  isBusy(): boolean;
}

export interface EditorSnapshot {
  document: LayoutDocument;
  revision: string;
  dirty: boolean;
  pageId: string;
  componentId: string | null;
}

export type CommitDocument = (document: LayoutDocument, selection?: string | null) => boolean;
export type Notify = (message: string) => void;
export type SelectPage = (id: string, run?: boolean) => void;
export type Log = (message: string) => void;

export interface RuntimeModule {
  HEAPU8: Uint8Array<ArrayBufferLike>;
  _operit_lvgl_pump(milliseconds: number): void;
  _operit_lvgl_layout_clear(color: number): void;
  _operit_lvgl_layout_geometry(index: number, x: number, y: number, w: number, h: number): void;
  _operit_lvgl_navigate_home(): void;
  _operit_lvgl_navigate_apps(): void;
  _operit_lvgl_round_icons(): boolean;
  _operit_lvgl_set_connection(connected: boolean, configured: boolean): void;
  _operit_lvgl_set_theme(index: number, circle: boolean): void;
  _operit_lvgl_theme_index(): number;
  _operit_lvgl_debug_tree?(): number;
  _operit_lvgl_debug_snapshot?(): number;
  _operit_lvgl_debug_tap?(id: number): boolean;
  _operit_lvgl_debug_swipe?(direction: number): boolean;
  _simulator_frame(): number;
  _simulator_generation(): number;
  _simulator_heap_used(): number;
  _simulator_init(): boolean;
  _simulator_touch(x: number, y: number, pressed: number): void;
  _operit_lvgl_layout_bind?: () => void;
  _operit_lvgl_layout_page_meta?: () => void;
  _operit_lvgl_layout_style?: () => void;
  ccall(
    identifier: string,
    returnType: 'number',
    argumentTypes: string[],
    values: Array<string | number>,
  ): number;
  ccall(
    identifier: string,
    returnType: 'string',
    argumentTypes: string[],
    values: Array<string | number>,
  ): string;
  ccall(
    identifier: string,
    returnType: null,
    argumentTypes: string[],
    values: Array<string | number>,
  ): null;
  onAction?: (value: string) => void;
}

export type EditorRuntime = RuntimeModule;
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
  lvgl?: string;
  source?: string;
}

export interface BuildStatus {
  running: boolean;
  error: string | null;
  manifest: BuildManifest | null;
  stale: boolean;
  output: string;
}

export interface EditorSetupOptions {
  snapshot: () => EditorSnapshot;
  commit: CommitDocument;
  notify: Notify;
}

export interface ComponentSetupOptions extends EditorSetupOptions {}

export interface PageSetupOptions {
  snapshot: () => EditorSnapshot;
  commit: CommitDocument;
  selectPage: SelectPage;
}

export interface DeploySetupOptions {
  snapshot: () => EditorSnapshot;
  notify: Notify;
}

export interface ComponentContextPayload {
  kind: 'operit.hardware.component';
  version: number;
  source: string;
  revision: string;
  dirty: boolean;
  componentId: string;
  component: LayoutNode;
  document: LayoutDocument;
  routes: Array<Route | ProjectRoute>;
  eventBindings: EventBinding[];
  requirement: string;
  codeReference: ComponentSource['location'] | {
    status: string;
    location: null;
    draftPointer?: string | null;
  };
  instructions: string;
}

export interface AiProposal {
  summary: string;
  operations: unknown[];
  codeEdits: unknown[];
  notice?: string;
}

export interface AiContext {
  document: LayoutDocument;
  revision: string;
  dirty: boolean;
  pageId: string;
  componentId: string | null;
  source: ComponentSource | null;
  component: LayoutNode | undefined;
  pointer: string | null;
  sourceFile: string;
  instruction: string;
}

export interface CatalogState {
  catalog: CatalogItem[];
  routes: Route[];
}

export interface LayoutResponse {
  document: LayoutDocument;
  revision: string;
}

export interface DeviceCapabilities {
  protocol: number;
  board: string;
  maxPackageBytes: number;
  revision: unknown;
  accepted?: boolean;
  bytes?: number;
  previousRevision?: unknown;
}

export interface SerialPortInfo {
  port: string;
  name: string;
}

export interface SerialPortsResponse {
  ports: SerialPortInfo[];
}

export interface FlashState {
  running: boolean;
  output: string;
  error: string | null;
}

/** Returns a required DOM element with a precise element type. */
export function query<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error('Missing DOM element: ' + selector);
  return element;
}

/** Returns a required input-like element with a precise element type. */
export function queryInput(selector: string): PropertyInput {
  return query<PropertyInput>(selector);
}

/** Converts an unknown thrown value into a displayable message. */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

declare global {
  interface Window {
    operitEditor?: EditorBridge;
    operitHost?: HostBridge;
  }
}
