# CLAUDE.md — Showbook

Command reference. For the model, the invariants and the traps, read
[AGENTS.md](AGENTS.md) first.

## Commands

```bash
npm install
npm run app          # tauri dev: vite on :5177 + cargo, opens the window
npm run app:build    # release bundle for this platform (src-tauri/target/release/bundle)
npm test             # vitest — the PDF builder and the TS helpers
npm run lint         # oxlint
npm run typecheck    # tsc -b
cd src-tauri && cargo test --workspace     # every crate, on the fixtures
cd src-tauri && cargo run -p showbook-em --example dump -- ../fixtures/em/e3-sim-10.0.2
cd src-tauri && cargo run -p showbook-aw --example capture -- 127.0.0.1:3000
```

`npm run dev` alone serves the browser demo (two simulator captures in memory).

```bash
npm run lite:wasm    # the Rust core for the browser → lite/pkg (needs `rustup target add wasm32-unknown-unknown` and `cargo install wasm-bindgen-cli` at the version Cargo.lock pins)
npm run lite:dev     # showbook-lite on :5179
npm run lite:build   # → dist-lite, what .github/workflows/deploy.yml puts on showbook-lite.stoatworks-labs.com
```

## Release

`.github/workflows/desktop.yml` builds on a `v*` tag: macOS universal (unsigned — the
fleet's autosign agent notarises after publishing), Linux deb/rpm, Windows NSIS. Bump the
version in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` together.

## Verifying against a device

The Analog Way simulators run locally (LivePremier `:3000`, Midra 4K `:3010`, Alta 4K
`:3021`; see docs/NOTES.md). The Barco simulators have no JSON-RPC, so the Event Master live
driver can only be checked against a frame.
