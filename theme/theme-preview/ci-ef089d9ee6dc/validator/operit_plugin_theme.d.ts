/* tslint:disable */
/* eslint-disable */

export function exportToolPkgFromTheme(metadata_json: Uint8Array, theme_json: Uint8Array, preview_png: Uint8Array, directory_json: Uint8Array, payload: Uint8Array): Uint8Array;

export function normalizeThemeDraft(bytes: Uint8Array): string;

/**
 * Checks the complete merged ToolPkg before Studio exposes its download.
 */
export function validateAuthoredToolPkg(archive_bytes: Uint8Array, manifest_path: string): string;

export function validateThemePreviewDraft(theme_json: Uint8Array, directory_json: Uint8Array, payload: Uint8Array): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly exportToolPkgFromTheme: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number) => [number, number, number, number];
    readonly normalizeThemeDraft: (a: number, b: number) => [number, number];
    readonly validateAuthoredToolPkg: (a: number, b: number, c: number, d: number) => [number, number];
    readonly validateThemePreviewDraft: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
