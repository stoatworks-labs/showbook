/* tslint:disable */
/* eslint-disable */

/**
 * What a model holds, from the capability table.
 */
export function capabilities(platform: string, model: string): any;

/**
 * A Bitfocus Companion page for the show; `opts` is the desktop app's
 * `ExportOptions` as JSON. Returns the `.companionconfig` document as JSON text.
 */
export function companion_export(show: string, opts: string): string;

/**
 * Read a Companion page back and check its buttons against the show.
 */
export function companion_import(bytes: Uint8Array, show: string): any;

/**
 * Convert a show for another platform: the converted show, the report, and
 * the id map. The same shape as the desktop app's `convert_show`, plus the map.
 */
export function convert(show: string, platform: string, model: string): any;

/**
 * What changed between two versions, keyed by what the entries are.
 */
export function diff(before: string, after: string): any;

/**
 * The hash a version is stored under: SHA-256 of the show with `meta.modified`
 * blanked, so re-saving an unchanged show is not a new version. The same rule
 * the desktop library applies.
 */
export function hash_show(show: string): string;

/**
 * Read a show file the way the desktop app's Import does. `name` is the file
 * name, which decides the parser for the ambiguous cases (`.json` is a Showbook
 * show or a saved LivePremier device store; `.xml` is an Event Master
 * `settings.xml`). Event Master backups (`.tar.gz`, `.zip`) and LivePremier
 * `.awc` files are recognised by content.
 */
export function import_file(name: string, bytes: Uint8Array): any;

/**
 * An empty show for a platform and model, as New show makes.
 */
export function new_show(name: string, platform: string, model: string): any;

/**
 * Every platform the model knows, with its label and the models the
 * capability table has for it — what the New show and Convert pickers list.
 */
export function platforms(): any;

export function sha256(bytes: Uint8Array): string;

export function summary(show: string): any;

/**
 * The reference-graph problems in a show, as the desktop app's Validate.
 */
export function validate(show: string): any;

/**
 * The version of the core this page is running.
 */
export function version(): string;

/**
 * A zip of files, for the test-pattern download: `names` are the entry
 * names, `datas` the bytes of each (a JS array of Uint8Array), in step.
 */
export function zip_files(names: string[], datas: Array<any>): Uint8Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly capabilities: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly companion_export: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly companion_import: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly convert: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly diff: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly hash_show: (a: number, b: number) => [number, number, number, number];
    readonly import_file: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly new_show: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly platforms: () => [number, number, number];
    readonly sha256: (a: number, b: number) => [number, number];
    readonly summary: (a: number, b: number) => [number, number, number];
    readonly validate: (a: number, b: number) => [number, number, number];
    readonly version: () => [number, number];
    readonly zip_files: (a: number, b: number, c: any) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
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
