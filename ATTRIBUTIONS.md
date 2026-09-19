# Attributions

Showbook is built on other people's work. This file lists what that work is, who did
it, and what it is doing here.

It is generated — the master lists live in the `stoatworks-backend` repo and are
pushed out by `scripts/sync-attributions.py`. Edit it there, not here.

## Code we derived from other people's work

Someone else solved this first, and this project would not exist in its current form without their work.

### openrcs-awj — Stoatworks Labs

<https://github.com/stoatworks-labs/openrcs>  
Licence: MIT

The AWJ wire codec (0x04-terminated get/replace framing) and the T-bar letter rule — which preset letter a LivePremier screen has on program given its transition state — come in as a git dependency from the fleet's own openrcs rather than being written a second time. src-tauri/crates/showbook-aw uses them for every AWJ read and write and for naming the captured program and preview states.

## Third-party code this project uses

Libraries, SDKs and frameworks the project is built on or bundles.

### Tauri

<https://tauri.app>  
Licence: MIT or Apache-2.0  
Copyright: The Tauri Programme within The Commons Conservancy

A Cargo and npm dependency.

Puts a web front end on a native Rust core using the platform's own webview, so the binary stays small and the DSP stays in Rust.

### React

<https://react.dev>  
Licence: MIT  
Copyright: Meta Platforms, Inc. and affiliates

An npm dependency.

The UI layer for the browser tools and the Electron and Tauri front ends.

### The Rust crate ecosystem

<https://crates.io>  
Licence: predominantly MIT or Apache-2.0  
Copyright: the individual crate authors

Cargo dependencies, resolved and pinned in Cargo.lock.

Async runtimes, protocol codecs, serialisation and GUI toolkits. The exact set and versions for any build are in that repo's Cargo.lock, which is the authoritative list.

### The npm ecosystem

<https://www.npmjs.com>  
Licence: predominantly MIT  
Copyright: the individual package authors

npm dependencies, resolved and pinned in the lockfile.

Build tooling, test runners and the libraries the front ends are assembled from. The exact set and versions for any build are in that repo's lockfile, which is the authoritative list.

The full transitive dependency set for any build is pinned in this repo's lockfile,
which is the authoritative list. What is named above is the layers a reader would
want to know about, not every package that has ever been resolved.

## Work we checked ourselves against

No code was taken from these — but they were how we knew we had it right, and that is worth saying out loud.

### Barco Event Master Toolset simulators and the Event Master JSON-RPC API guide

The Event Master show file format — settings.xml, the presets/, cues/ and userkey/ folders, hwconfig.xml, the card type codes, the E3Backup.tar.gz layout — was read out of the stores the vendor's own simulators (E2 9.2, Encore3 10.0) write, and src-tauri/crates/showbook-em/src/live.rs follows the published JSON-RPC guide. No vendor code was used and no simulator file is redistributed; the fixtures under fixtures/em are trimmed captures of the simulators' own output.

### Analog Way LivePremier AWJ Protocol Programmer's Guide, LivePremier REST API Programmer's Guide, and the LivePremier, Midra 4K and Alta 4K simulators

The device store layout, the REST recall endpoints, the .awc container's manifest and the configuration import/export flow in src-tauri/crates/showbook-aw were read from the published guides and confirmed against the vendor's simulators. The .awc payload is encrypted and is deliberately kept opaque — Showbook never reads inside it. The fixtures under fixtures/aw are trimmed simulator stores.

### Bitfocus Companion and the barco-eventmaster and analogway-awj modules

The Companion page export in src-tauri/crates/showbook-companion writes the control and action shapes Companion 5.0.5 itself saves, with the action ids and option names those two modules define. No module code is copied.

## Getting this wrong

If your work is here and the description is inaccurate, the licence is wrong, or you would rather not be listed — open an issue and it will be fixed.
