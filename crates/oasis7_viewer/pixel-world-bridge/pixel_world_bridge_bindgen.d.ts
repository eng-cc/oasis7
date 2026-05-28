/* tslint:disable */
/* eslint-disable */

export class PixelWorldBridge {
    free(): void;
    [Symbol.dispose](): void;
    click(x: number, y: number): any;
    mount(canvas: HTMLCanvasElement, initial_render_state: any): any;
    constructor(on_event: Function, on_fatal: Function);
    pointer_down(x: number, y: number, pointer_id: number): any;
    pointer_move(x: number, y: number, is_leave: boolean, pointer_id: number): any;
    pointer_up(pointer_id: number): any;
    tick(_animation_ms: number): any;
    unmount(): any;
    update(next_render_state: any): any;
    wheel(delta_y: number): any;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_pixelworldbridge_free: (a: number, b: number) => void;
    readonly pixelworldbridge_click: (a: number, b: number, c: number) => any;
    readonly pixelworldbridge_mount: (a: number, b: any, c: any) => any;
    readonly pixelworldbridge_new: (a: any, b: any) => number;
    readonly pixelworldbridge_pointer_down: (a: number, b: number, c: number, d: number) => any;
    readonly pixelworldbridge_pointer_move: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly pixelworldbridge_pointer_up: (a: number, b: number) => any;
    readonly pixelworldbridge_tick: (a: number, b: number) => any;
    readonly pixelworldbridge_unmount: (a: number) => any;
    readonly pixelworldbridge_update: (a: number, b: any) => any;
    readonly pixelworldbridge_wheel: (a: number, b: number) => any;
    readonly wasm_bindgen__convert__closures_____invoke__h656285ab01c66ca0: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h5b8e399d53c6236b: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_4: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_5: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_6: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_7: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_8: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h1089e505c78a14e4_9: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h2c6d6c1dfa024e53: (a: number, b: number, c: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h953c336461e674c0: (a: number, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__ha544621b2880cc31: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
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
