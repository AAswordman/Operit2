/* tslint:disable */
/* eslint-disable */
/**
 * The `ReadableStreamType` enum.
 *
 * *This API requires the following crate features to be activated: `ReadableStreamType`*
 */

export type ReadableStreamType = "bytes";

export class IntoUnderlyingByteSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableByteStreamController): Promise<any>;
    start(controller: ReadableByteStreamController): void;
    readonly autoAllocateChunkSize: number;
    readonly type: ReadableStreamType;
}

export class IntoUnderlyingSink {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    abort(reason: any): Promise<any>;
    close(): Promise<any>;
    write(chunk: any): Promise<any>;
}

export class IntoUnderlyingSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableStreamDefaultController): Promise<any>;
}

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
    readonly operit_flutter_bridge_close_watch_stream: (a: number, b: number, c: number) => void;
    readonly operit_flutter_bridge_create: () => number;
    readonly operit_flutter_bridge_create_error: () => number;
    readonly operit_flutter_bridge_create_with_storage_roots: (a: number, b: number) => number;
    readonly operit_flutter_bridge_destroy: (a: number) => void;
    readonly operit_flutter_bridge_free_bytes: (a: number) => void;
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
    readonly __wbg_intounderlyingsource_free: (a: number, b: number) => void;
    readonly intounderlyingsource_cancel: (a: number) => void;
    readonly intounderlyingsource_pull: (a: number, b: any) => any;
    readonly __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
    readonly __wbg_intounderlyingsink_free: (a: number, b: number) => void;
    readonly intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
    readonly intounderlyingbytesource_cancel: (a: number) => void;
    readonly intounderlyingbytesource_pull: (a: number, b: any) => any;
    readonly intounderlyingbytesource_start: (a: number, b: any) => void;
    readonly intounderlyingbytesource_type: (a: number) => number;
    readonly intounderlyingsink_abort: (a: number, b: any) => any;
    readonly intounderlyingsink_close: (a: number) => any;
    readonly intounderlyingsink_write: (a: number, b: any) => any;
    readonly ring_core_0_17_14__bn_mul_mont: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h53406515088c9d43: (a: number, b: number, c: number, d: number, e: any) => [number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h23695230caa1583e: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h22ac2df73405c9b5: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__he27900793f25c31c: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h0b22a1486eb59ede: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h0b22a1486eb59ede_3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h0b22a1486eb59ede_4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h0b22a1486eb59ede_5: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h9d21f180502235d6: (a: number, b: number) => number;
    readonly wasm_bindgen__convert__closures_____invoke__haf93522e2dbec25a: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hcda227c5a0afa830: (a: number, b: number) => void;
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
