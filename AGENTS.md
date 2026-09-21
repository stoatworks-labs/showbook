# AGENTS.md — bringing an LLM up to speed on Showbook

Orientation for an AI assistant (or a new human) picking this project up cold. `CLAUDE.md`
holds the short command reference; this file explains the model and the traps.

## 1. What this is

A Tauri v2 desktop app (React front end, Rust core) that keeps a library of video switcher
show files with version history, and does the things a show file should let you do:
inspect, edit, document (PDF or web page), test-pattern, convert between platforms, build Companion
pages, pull from and push to live hardware. Two platforms are real today — Barco Event
Master and Analog Way LivePremier (with the Midra 4K / Alta 4K store parser beside it) — and
everything else is a capability descriptor waiting for a driver.

It is a desktop app and not a browser tool for one reason: the switchers speak raw TCP
(AWJ on 10606) and plain-HTTP JSON-RPC (9999) that a hosted page cannot reach.

## 2. The one idea

**Everything above the drivers sees only `showbook_model::Show`.** A driver reads a vendor
file or a live device into that model and writes it back out; the inspector, the documentation, the
conversion and the history never look at vendor data. What a driver cannot express in the
shared fields goes into an entity's `extra` bag under the vendor's own name, so nothing is
lost on a round trip but nothing vendor-specific leaks up. Conversion never reads `extra`.

IDs are stable strings with a kind prefix (`in:`, `src:`, `out:`, `scr:`, `aux:`, `layer:`,
`pre:`, `master:`, `cue:`, `mv:`, `mvl:`, `still:`, `frame:`, `conn:`), chosen by the driver so
that re-importing the same file yields the same IDs and history diffs stay readable. Layer
IDs are scoped to their screen. `Show::validate` checks every reference; a driver's fixture
test asserts it is empty.

## 3. Layout

```
src/types.ts                 the JSON shapes shared with Rust — read this first
src/lib/ipc.ts               every invoke in one place; lite.ts is the hosted build's answer to each, mock.ts the browser demo
src/store.ts                 zustand: settings, library entries, the open show, dirty flag
src/components/ShowView.tsx  the tabs; each tab is one file
src/lib/pdf.ts               entry: builds the document, renders it as PDF or HTML
src/lib/document.ts          every section of the documentation, as blocks and diagrams
src/lib/doc/ir.ts            the block/diagram model both renderers draw
src/lib/doc/theme.ts         the five themes, the papers, colour helpers
src/lib/doc/diagrams.ts      shared diagram helpers (SVG paths, the column flow)
src/lib/doc/render-pdf.ts    pdf-lib renderer; render-html.ts writes one self-contained page
src/lib/glossary.ts          the glossary chapter
src/lib/patterns.ts          test patterns on a canvas
src-tauri/src/lib.rs         Tauri commands: library, import, devices, convert, companion, sync
src-tauri/crates/showbook-model     Show, diff, summary, ids
src-tauri/crates/showbook-em        cards.rs (type codes), settings.rs (XML → Show), presets.rs, archive.rs, jsonrpc.rs, live.rs
src-tauri/crates/showbook-aw        store.rs (LivePremier JSON → Show), store_mng.rs (Midra/Alta), device.rs, awj.rs, awc.rs
src-tauri/crates/showbook-library   Library (history, vendor blobs), sync.rs (four providers), oauth.rs (PKCE)
src-tauri/crates/showbook-convert   capabilities.rs (per model), lib.rs (the conversion + report)
src-tauri/crates/showbook-companion export/import of .companionconfig pages
src-tauri/crates/showbook-wasm      the core for the browser: the parsers, conversion, Companion, zip, hashing — as wasm-bindgen exports
lite/                        showbook-lite: vite.lite.config.ts builds it, lite/pkg is the generated wasm (not committed), lite/public the footer and headers
fixtures/em, fixtures/aw     simulator captures the tests run on
```

## 4. Invariants

- **Rust decides, TypeScript displays.** The model, every parse, the conversion and its
  report, the diff, the Companion page — all Rust. The UI edits the model and draws it. The
  two exceptions, deliberately: the documentation and the test patterns are rendered in the webview
  (pdf-lib and canvas) and written to disk through one command.
- **Every save is a commit.** `Library::save` hashes the canonical JSON (with
  `meta.modified` blanked), keeps a gzip'd snapshot per distinct hash, appends to
  `history/index.json`. Saving the same thing twice makes no commit.
- **Vendor files are opaque and content-addressed.** An `.awc` or an `E3Backup.tar.gz` is
  stored under `vendor/<sha256><ext>` and listed in `show.vendor`. Showbook does not read
  inside an `.awc` (it is encrypted) and does not try.
- **Conversion reports; it does not claim.** Every dropped feature is a `Note` with
  `level: dropped`, every nearest-neighbour substitution `adapted`. The converted show's
  `notes` carry the report so it survives being saved.
- **Nothing writes to a device implicitly.** Pull is read-only except "deep capture",
  which recalls memories into preview and says so in the UI. Push is a separate button with
  a confirm. The Devices tab is the only place that writes.
- **Sync never deletes.** The mirror copies newer files across and nothing else.
- **The parsers stay wasm-clean.** `showbook-em` and `showbook-aw` compile for
  `wasm32-unknown-unknown` with `default-features = false`: everything that opens a socket
  (`jsonrpc`, `live`, `awj`, `device`, the `ureq` dependency) sits behind the `live`
  feature, and nothing on the parse path touches `std::fs` or the network. showbook-lite is
  the same front end with `src/lib/lite.ts` answering the commands from `showbook-wasm`
  and an IndexedDB library that keeps the same content-addressed history; what a browser
  cannot do (devices, sync) is hidden, not stubbed, and the page points at the full app.

## 5. Traps

- **The Event Master simulators do not serve JSON-RPC.** `wvp_sim` (E2/S3/EX) and `mvp`
  (Encore3) speak the toolset's own XML protocol on 9876 and an ASCII console on 9878;
  nothing answers on 9999. `live.rs` is verified against the API guide only.
- **Event Master `settings.xml` does not name card types.** Slot cards are identified by
  their `In`/`Out` children (`HDMIIn`, `DPIn`, `SDIOut`, …) and by `ConnMap/CardType` codes,
  and on Encore3 by `hwconfig.xml` beside it. `cards.rs` holds the code table; unknown codes
  become "Card type N", never a guess.
- **Layers live under `DestOutMap`, not `ScreenDest`.** In the store a screen's
  `LayerCollection` sits inside its first output map (`FollowDestOutMapIndex` ties the rest).
  `BGLayer` and `DSKLayer` become `layer:bg` and `layer:dsk`.
- **`TransTime` is frames.** Converted to ms at the system's `NativeRate`; the raw value is
  kept in `extra`.
- **LivePremier positions are anchor-relative.** `position.pp.posH/posV` is the anchor point
  (`MIDDLE_CENTER` by default); the model wants a top-left rect. `anchor_factors` does the
  arithmetic; Midra 4K positions are always the centre.
- **Which letter is on air comes from the T-bar.** `screenAuxGroupList/…/status/pp/transition`
  plus `presetUp/presetDown`, through `openrcs_awj::Letters` — the DOWN/UP suffix rule, not
  a test for `AT_UP`. Midra/Alta name their buffers `UP`/`DOWN` directly.
- **AWJ paths are the store paths respelled**: `device` → `DeviceObject`, `xxxList/items/K`
  → `$xxx/@items/K`, `pp` → `@props`. `awj::awj_path` does it; never hand-write both.
- **Memory content is not in the store.** The bank lists labels and filters; the layers a
  memory holds are only on the device. `deep_capture` reads them by recalling into preview.
- **The `.awc` download is two steps on the device.** The Web RCS server sets
  `system/configuration/backup/export/cmd` and waits for `DONE`; a plain GET on the download
  URL returns `ERROR_INVALID_PATH` when the device cannot write its temp path (the
  container-hosted simulator does that; the local one works).
- **Companion controls are `button-layered`** in 5.0 (layers: canvas, box, text). Option
  values are wrapped `{value, isExpression}`. The awj module keys screens as `S1`/`A1`,
  presets as `pvw`/`pgm`, memories by slot string; the eventmaster module's `recall_preset`
  takes the frame's 0-based preset id in `id` and `mode` `"0"`/`"1"`.
- **`tauri dev` needs port 5177 free** (`strictPort`); the browser preview server and the app
  cannot run at once.

## 6. Verifying

- `cd src-tauri && cargo test --workspace` — 29 tests over the fixtures.
- `cargo run -p showbook-em --example dump -- <store>` prints the model for any Event Master
  store; `cargo run -p showbook-aw --example capture -- <host[:port]>` captures a live
  LivePremier/Midra/Alta; `--example awc` downloads an `.awc`; `--example awc_upload` uploads
  one and reports the extract status; `cargo run --example seed -- <library> <files…>`
  imports into a library from the terminal.
- The vendors' simulators on this Mac: LivePremier on `127.0.0.1:3000`, Midra 4K `:3010`,
  Alta 4K `:3021` (see `docs/NOTES.md` for how they are started).
- The desktop app can be driven from a script through the accessibility tree: System
  Events sees the webview's buttons by name (`docs/NOTES.md` has the recipe).
