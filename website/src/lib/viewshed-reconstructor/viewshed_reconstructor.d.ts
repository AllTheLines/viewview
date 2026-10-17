/* tslint:disable */
/* eslint-disable */

/**
 * WASM only supports simple types, so this is a simple representation of a polygon.
 */
export class PlainPolygon {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Getter for `exterior`.
     */
    readonly exterior: Float64Array;
    /**
     * Getter for `hole_indices`.
     */
    readonly hole_indices: Uint32Array;
    /**
     * Getter for `interiors`.
     */
    readonly interiors: Float64Array;
}

/**
 * Reconstruct a viewshed from raw polar segments.
 *
 * # Panics
 *   When reconstructing the viewshed fails.
 */
export function reconstruct(js_data: Array<any>, dem_scale: number): Array<any>;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_plainpolygon_free: (a: number, b: number) => void;
    readonly plainpolygon_exterior: (a: number) => [number, number];
    readonly plainpolygon_hole_indices: (a: number) => [number, number];
    readonly plainpolygon_interiors: (a: number) => [number, number];
    readonly reconstruct: (a: any, b: number) => any;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
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
