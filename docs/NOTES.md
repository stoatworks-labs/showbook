# Notes

Working notes for this repo: decisions, and the traps that actually bit, written while
building it on 2026-09-19. Cross-cutting notes live in
[fleet-notes](https://github.com/stoatworks-labs/fleet-notes).

## Where the show files really are

- **Event Master keeps its show as an XML store on the frame**: `xml/settings.xml` (system,
  frames and cards, sources, destinations, output configs, multiviewers, …) plus one file
  per preset, cue, user key, custom format, EDID, external device and HDR profile in sibling
  folders. Both toolset simulators write the same tree (`wvp_sim/wvp_9876/xml`,
  `mvp_9876/xml`), and the Encore3 backup the toolset downloads (`E3Backup.tar.gz`, from
  `/api/backup`) is that tree in an `E3Backup/` folder. `showbook-em` reads a directory, a
  tar.gz, a zip or a bare `settings.xml`, and finds `settings.xml` wherever it is.
- **LivePremier's show is one JSON document** — `GET /api/stores/device` (124 MB on a Cmax,
  about a second on the simulator). The vendor's `.awc` is a zip whose comment is a JSON
  manifest (device type, firmware, timestamp, modules) and whose single entry is encrypted
  (7.9999 bits of entropy per byte). Showbook keeps the `.awc` byte-for-byte and never opens
  it; the model comes from the store.
- The `.awc` download is not a plain GET: the Web RCS server writes
  `system/configuration/backup/export/cmd/pp/{destination: EXTERNAL, path, xRequest: [modules]}`
  and waits for `status: DONE`, then serves the file it names. The **container-hosted
  simulator on lilnasx answers `ERROR_INVALID_PATH`** because the Qt half under Wine cannot
  write the node half's temp path; the local simulator works.
- Upload is `POST /api/device/hardware/config/upload` (multipart, field `FILE`); the server
  runs the extract itself and answers the status (`DONE` / `DONE_VERSION_WARNING`). Apply is
  `…/backup/import/apply/cmd/pp/xRequest = [modules]` over AWJ, and reboots.

## Event Master store, as read

- Card types are not in `settings.xml`. On Encore3 `hwconfig.xml` beside it names them
  (`<card type="4">In HDMI2.0 Card`); on every frame `OutputCfg/Config/ConnMap/CardType`
  carries the code for output slots, and the `In`/`Out` children of a slot's card say what
  each connector is (`HDMIIn`, `DPIn`, `SDIOut`, and nothing for the SDI inputs of a
  Tri-Combo, which the code table fills in). Codes seen: 0 DVI in, 1 SDI in, 2 HDMI/DP in,
  3 Tri-Combo in, 4 HDMI 2.0 in, 5 DP 1.2 in, 11/12 PDS in, 21 SDI out, 22 HDMI out, 23 DP
  out, 25 Tri-Combo out, 26 HDMI 2.0 out, 30 PDS out, 40 MVR out, 42 MVR HDMI 2.0 out, 43 PDS
  MVR, 50 VPU, 70 link. Frame types: 0 E2, 1 S3, 2 EX, 3 ImagePRO-4K, 5 E2 Gen 2, 6 PDS-4K,
  8 Encore3.
- A `ScreenDest` holds its layers **inside its first `DestOutMap`** (`LayerCollection`),
  with `BGLayer` and `DSKLayer` beside them. `LayerCfg/Source` is a copy of a `SrcMgr`
  source with its indices; `LayerState[ActiveState]/WinAdjust/OWIN` is the window on the
  canvas, `IWIN` the crop in source pixels; `PIP/Opacity` is 0–100, colours 0–1000.
- `Capacity` 1/2/4 is SL/DL/4K. `TransTime` is frames at `NativeRate`.
- `Source/SrcType` seen: 2 = a screen destination's program (`DestIndex`), 6 = a
  multiviewer output (`MvrIndex`); inputs and stills are told apart by `InputCfgIndex` /
  `StillIndex`. `InputCfgCol` was not present in either simulator store (no signals), so the
  input parser looks for `InputCfg` elements anywhere and maps the fields the output config
  uses.
- The E2 9.2 store keeps the multiviewer on the frame (`Frame/MultiViewer` with its own
  `MVOutputCfgCollection`); Encore3 has a top-level `MultiViewerCollection` and its
  multiviewers name an `OutCfgIndex`. `MVWin/Rect` is the window; `InputCfgIndex`/`DestIndex`
  say what it shows.
- **No preset or cue file was available.** The parser is structural (every `Layer` under a
  `…Dest` ancestor becomes a layer state) and reports what it could not place.

## LivePremier store, as read

- Inputs: `inputList/items/IN_n` with `status/pp/isAvailable`; the active plug's
  `status/pp/type` is `HDMI` / `SDI` / `DISPLAY_PORT`; the signal is under
  `plugList/items/<plug>/status/signal/pp` (`formatWidth/Height`, `fieldFrequency` in mHz,
  `scanType`). `mapping/pp/physical` is a card-relative index (IN_5 → `IN_9`), not the panel
  print; connectors are named after the input instead.
- Outputs: `status/pp/{sizeH,sizeV,rate(mHz),isFormatInterlaced,format}`; their role and
  screen come from `preconfig/resources/current/outputList/items/n/status/pp/{mode,usedInScreenAux}`.
- Screens: `screenAuxGroupList/…/status/pp/isUsed` says which exist; size in
  `status/size/pp`; layers are `layerList` items whose `status/pp/capability != OFF`; the
  three lettered presets under `presetList/items/{A,B,C}` hold the layers
  (`source/pp/inputNum`, anchor-relative `position`, `opacity` 0–256, `cropping.classic`
  as edge insets, `border.edge`). `control/canvas/pp/mode` is `GRID` or `FREE`; positions
  come from `canvas/grid/control/outputList` (column/row) or `canvas/free/control/outputList` (left/top).
- The bank (`presetBank/bankList`) has labels, `isValid`, filters and `transitionDuration`
  but **not the layer values**. `masterPresetBank/bankList/items/n/status/screenList/items/Sx/pp/presetBankSlot`
  gives the member memories.
- Multiviewers are `monitoringList/items/n` (on the MOC card) with `layout/widgetList`.
- `deviceList` lists four slots even on a lone device; the empty link slots have no
  plugged cards and are skipped.

## Midra 4K / Alta 4K store

Same Web RCS, different tree: `system/pp/{dev,platformLabel}` (no `deviceList`),
`inputList/items/INPUT_n`, `outputList/items/{n,MTVW}`, `screenList/items/n` with
`presetList/items/{DOWN,UP}/liveLayerList` (positions are the centre, `size` separate),
`auxiliaryScreenList`, `transition/screenList/…/status/pp/transition` naming the buffer on
air, `preset/{bank,auxBank,masterBank}/slotList`, a single `multiviewer` with `widgetList`
and `bankList`, `stillLibrary/bankList` for frames. Which outputs and layers a screen has
is in `preconfig/control/{outputList,resourcesList,screenList}` (`useOnScreen`, `mode`
`SPLIT` = two layers). Read from Pulse 4K 3.2.29 and Zenith 200 1.3.7 simulators.

## Simulators on this machine

- LivePremier: `~/Library/Application Support/ANALOG WAY/LivePremier_Simulator/<session>/`,
  run `AW_APP_SIMULATOR settings_0.ini` from the session directory (the launcher app alone
  starts nothing). Web RCS `:3000`, AWJ `:10606`.
- Midra 4K on `:3010` / `:10610`, Alta 4K on `:3021` / `:13021`, started the same way from
  their own simulator apps.
- Event Master: `/Applications/EventMaster-10.0.3.3021/mvp.app/Contents/MacOS/mvp --sim 1 --rwdir <dir>/ --rodir <dir>/`
  writes its store under `<dir>/mvp_9876/xml/`; the 9.2 `wvp_sim/wvp` script does the same
  under `wvp_9876/xml/`. Neither serves JSON-RPC; the toolset GUI talks XML on 9876.

## Driving the desktop app from a script

System Events sees the webview's buttons by name:
`tell application "System Events" to tell process "showbook" to click (first button of
entire contents of window 1 whose name starts with "…")`. Native save panels take
⌘⇧G, a path, Return, Return. Capture the window with `screencapture -l <CGWindowID>` (a
small Swift lister finds the id; it changes on every relaunch). The PDF in this repo's
first verification was produced that way from the running app.

## Things deliberately not done

- Reading inside an `.awc`. The key would have to come out of the vendor's binary; the
  file is opaque here and goes back to the device through the documented upload/apply path.
- Reverse-engineering the Event Master toolset's XML protocol on 9876. It would give a
  write path for configuration; the documented JSON-RPC does not, and the backup/restore
  round trip through the toolset covers full restores.
