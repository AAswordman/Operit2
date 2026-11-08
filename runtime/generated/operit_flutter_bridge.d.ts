/* tslint:disable */
/* eslint-disable */

export class OperitFlutterBridgeWasm {
    free(): void;
    [Symbol.dispose](): void;
    call(request: Uint8Array): Promise<Uint8Array>;
    closeWatchStream(subscriptionId: string): Uint8Array;
    /**
     * Delivers owner events through the same application ingress as native hosts.
     */
    emitRuntimeEvent(eventJson: string): Promise<string>;
    constructor();
    /**
     * Closes one wasm Link push stream.
     */
    pushClose(pushId: string): Promise<Uint8Array>;
    /**
     * Dispatches one wasm Link push item.
     */
    pushItem(item: Uint8Array): Promise<Uint8Array>;
    /**
     * Opens one wasm Link push stream.
     */
    pushOpen(request: Uint8Array): Promise<Uint8Array>;
    /**
     * Reads one wasm Link watch snapshot.
     */
    watchSnapshot(request: Uint8Array): Promise<Uint8Array>;
    /**
     * Opens one wasm Link watch stream.
     */
    watchStream(request: Uint8Array, onEvent: Function): Promise<Uint8Array>;
}

/**
 * Exposes the independent browser storage engine without creating a Core tree.
 */
export function operitDataRescue(request: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_operitflutterbridgewasm_free: (a: number, b: number) => void;
    readonly operitDataRescue: (a: number, b: number) => [number, number];
    readonly operit_flutter_bridge_create: () => number;
    readonly operit_flutter_bridge_create_error: () => number;
    readonly operit_flutter_bridge_create_with_storage_roots: (a: number, b: number) => number;
    readonly operit_flutter_bridge_destroy: (a: number) => void;
    readonly operit_flutter_bridge_free_string: (a: number) => void;
    readonly operitflutterbridgewasm_call: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_closeWatchStream: (a: number, b: number, c: number) => [number, number];
    readonly operitflutterbridgewasm_emitRuntimeEvent: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_new: () => [number, number, number];
    readonly operitflutterbridgewasm_pushClose: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_pushItem: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_pushOpen: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_watchSnapshot: (a: number, b: number, c: number) => any;
    readonly operitflutterbridgewasm_watchStream: (a: number, b: number, c: number, d: any) => any;
    readonly ring_core_0_17_14__bn_mul_mont: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h28ac359012e8a723: (a: number, b: number, c: number, d: number, e: any) => [number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h23695230caa1583e: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h22ac2df73405c9b5: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2884b473af3fabfe: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2884b473af3fabfe_2: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2884b473af3fabfe_3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2884b473af3fabfe_4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h153a1c0d73961aa4: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
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
