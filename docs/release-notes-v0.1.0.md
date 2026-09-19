First release — **in development**. It works, it moves, and its drivers are waiting on
real hardware: everything below was built against the vendors' own simulators and
protocol guides, and nothing has yet been run against a physical switcher. Read the
[Status](https://github.com/stoatworks-labs/showbook#status) section before trusting a
pull, a push or a preset read from a backup.

A desktop show file library for video switchers: Barco Event Master (E2, S3-4K, EX,
Encore3) and Analog Way LivePremier (Aquilon), with Midra 4K and Alta 4K read as well.

## What is in it

- **Library with version history** — plain files on disk; every save is a
  content-addressed version with a diff and one-click restore; the vendor files kept
  byte-exact beside the show.
- **Import** — Event Master backup archives, the frame's `xml/` folder or a bare
  `settings.xml`; LivePremier `.awc` files and saved device stores; Showbook JSON.
- **Devices** — pull the running show from an Aquilon (device store plus its `.awc`), a
  Midra 4K, an Alta 4K or an Event Master frame; push labels over AWJ and restore an
  `.awc` on a LivePremier; recall presets, masters and cues and TAKE on either family.
- **Inspect** — chassis with every connector and what is on it; patch tables; each screen
  drawn to scale with its outputs and layers; presets; multiviewer layouts; cues.
- **Edit** — labels, formats, patch, screens, layers, preset layer rectangles by dragging,
  master presets, cues, multiviewer windows.
- **PDF documentation** — cover, chassis and patch, every screen and preset to scale,
  multiviewers, cues, a glossary of the settings, the import notes, the version history.
- **Test patterns** — one labelled PNG per output at its raster, with arrows to its
  neighbours, and one per screen.
- **Convert** — Event Master ↔ LivePremier, with capability tables for Midra 4K, Alta 4K,
  LiveCore, PDS-4K and PixelHue; every feature that carried, was adapted or was dropped
  is named.
- **Companion** — export a page for the `barco-eventmaster` or `analogway-awj` module,
  and read a page back to check it against the show.
- **Sync** — mirror the library with a folder, or with Dropbox, Google Drive or OneDrive
  over their APIs.

## What has been checked, and what has not

Verified on the simulators: Event Master store parsing (Encore3 10.0.2 and E2 9.2,
backup archive round trip); LivePremier store, REST, `.awc` download → upload → extract,
AWJ reads and writes (simulator 6.2.73); Midra 4K and Alta 4K stores read live. The
desktop app itself pulled the LivePremier simulator and wrote the PDF.

Not yet: Event Master JSON-RPC and `/api/backup` (the Barco simulators do not serve the
API); any Event Master preset or cue file (none has ever been seen — the parser is
structural and reports what it cannot place); `.awc` *apply* (it reboots the device);
deep capture; the cloud providers against a live account; importing a page into a running
Companion.

The `.awc` is encrypted and is kept as an opaque vendor file. Restoring an Event Master
backup onto a frame is done from the Event Master Toolset; Showbook exports the archive.

Not affiliated with or endorsed by Barco or Analog Way.
