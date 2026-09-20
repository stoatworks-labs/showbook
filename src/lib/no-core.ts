/**
 * What `showbook-core` resolves to in the desktop build (vite.config.ts aliases
 * it here; vite.lite.config.ts aliases it to the generated wasm bindings in
 * lite/pkg). The desktop app never reaches it — the API layer only imports
 * src/lib/lite.ts under `__LITE__` — but the bundler resolves the import all
 * the same, and it must not depend on a build artefact the desktop does not
 * make.
 */
export default async function init(): Promise<never> {
  throw new Error('the Rust core runs in-process in the desktop app; the WebAssembly build is showbook-lite');
}
