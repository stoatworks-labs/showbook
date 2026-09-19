# Attributions

Showbook is built on:

- [Tauri](https://tauri.app) (MIT / Apache-2.0) — the desktop shell.
- [React](https://react.dev) (MIT), [zustand](https://github.com/pmndrs/zustand) (MIT),
  [Vite](https://vite.dev) (MIT), [vitest](https://vitest.dev) (MIT), [oxlint](https://oxc.rs) (MIT).
- [pdf-lib](https://pdf-lib.js.org) (MIT) — the PDF.
- Rust crates: serde, serde_json, roxmltree, flate2, tar, zip, ureq, sha2, hex, uuid, chrono,
  base64, rand, url, thiserror, tokio (MIT / Apache-2.0).
- [`openrcs-awj`](https://github.com/stoatworks-labs/openrcs) (MIT) — the AWJ wire codec and
  the T-bar letter rule, from the fleet's own openrcs.

Protocol and format knowledge came from:

- Barco, *Event Master JSON-RPC API* guide, and the toolset's simulator stores
  (`wvp_sim`, `mvp`) — the XML store layout and card type codes.
- Analog Way, *LivePremier AWJ Protocol Programmer's Guide* v6.2 and *LivePremier REST API
  Programmer's Guide* v4.1, and the LivePremier / Midra 4K / Alta 4K simulators — the device
  store, the `.awc` manifest, the configuration import/export flow.
- Bitfocus Companion 5.0.5 and the `barco-eventmaster` 4.4.4 and `analogway-awj` 2.5.0
  modules — the page export shape and the action ids.
- The capability figures follow the vendors' spec sheets as cited in the fleet's
  *Will my show fit?*; where a figure is a family convention it is marked in the code.

Barco, Event Master, Encore, Analog Way, LivePremier, Aquilon, Midra, Alta and Bitfocus
Companion are trademarks of their owners. Showbook is not affiliated with any of them.
