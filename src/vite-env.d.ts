/// <reference types="vite/client" />

/** Injected by vite.config.ts from package.json. Shown in the header and the About dialog. */
declare const __APP_VERSION__: string;
/** True when built by vite.lite.config.ts — the hosted showbook-lite. */
declare const __LITE__: boolean;

interface Window {
  STOATWORKS_ABOUT?: Record<string, string>;
}
