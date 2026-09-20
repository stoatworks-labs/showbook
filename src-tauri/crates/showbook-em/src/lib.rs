//! Barco Event Master driver.
//!
//! Event Master frames — E2 (Gen 1 and 2), S3-4K, EX, PDS-4K and Encore3 (E3)
//! — keep their whole configuration as an XML store on the frame:
//!
//! ```text
//! xml/settings.xml          system, frames and cards, sources, destinations,
//!                           output configs, multiviewers, operators, timers…
//! xml/presets/<n>.xml       one file per preset
//! xml/cues/<n>.xml          one file per cue
//! xml/userkey/, stills/, customformats/, edidfiles/, externaldevices/, hdrfiles/
//! xml/hwconfig.xml          (E3) card names per slot
//! ```
//!
//! That store is what the toolset's backup carries: an Encore3 backup is
//! `E3Backup.tar.gz` holding an `E3Backup/` folder with the tree above, and it
//! is served by the frame at `/api/backup`. The simulator that ships with the
//! toolset writes the identical tree under `wvp_9876/xml/` (E2/S3/EX) or
//! `mvp_9876/xml/` (E3), which is how this driver was developed without a
//! frame in the room.
//!
//! [`import_path`] reads any of those shapes — a directory, a `.tar.gz`, a
//! `.zip`, or `settings.xml` itself — into a [`showbook_model::Show`].
//! [`live`] talks to a running frame over the documented JSON-RPC API on
//! port 9999.
//!
//! What is verified and what is not: the `settings.xml` mapping was built from
//! two simulator stores (E2 firmware 9.2 and Encore3 10.0.2, the latter with six
//! screens, six outputs and two multiviewers). **No preset or cue file has been
//! seen** — the simulators never had one saved — so [`presets`] parses the
//! structures the settings file already uses (a `Layer` with its `LayerCfg`,
//! `Source` and `WinAdjust`) wherever they appear under a preset, and reports
//! what it could not place in `Show::notes`. Treat a preset import as
//! provisional until a real backup has been through it.

pub mod archive;
pub mod cards;
#[cfg(feature = "live")]
pub mod jsonrpc;
#[cfg(feature = "live")]
pub mod live;
pub mod presets;
pub mod settings;
pub mod xml;

use std::path::Path;

use showbook_model::Show;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not an Event Master store: {0}")]
    NotAStore(String),
    #[error("xml: {0}")]
    Xml(String),
    #[error("archive: {0}")]
    Archive(String),
    #[error("device: {0}")]
    Device(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Read an Event Master store from a directory, a backup archive
/// (`.tar.gz` / `.zip`) or a bare `settings.xml`.
pub fn import_path(path: &Path) -> Result<Show> {
    let store = archive::Store::open(path)?;
    let mut show = settings::parse_store(&store)?;
    show.meta.source = Some(showbook_model::SourceInfo {
        kind: "file".into(),
        origin: path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
        at: showbook_model::now(),
        firmware: Some(show.system.firmware.clone()),
    });
    Ok(show)
}

/// Read an Event Master store already in memory (an archive's bytes).
pub fn import_bytes(name: &str, bytes: &[u8]) -> Result<Show> {
    let store = archive::Store::from_bytes(name, bytes)?;
    let mut show = settings::parse_store(&store)?;
    show.meta.source = Some(showbook_model::SourceInfo {
        kind: "file".into(),
        origin: name.to_string(),
        at: showbook_model::now(),
        firmware: Some(show.system.firmware.clone()),
    });
    Ok(show)
}

#[cfg(test)]
mod tests {
    use super::*;
    use showbook_model::{OutputRole, ScreenKind, SourceKind};

    fn fixture(rel: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/em").join(rel)
    }

    #[test]
    fn e3_simulator_store() {
        let show = import_path(&fixture("e3-sim-10.0.2")).unwrap();
        assert_eq!(show.system.model, "Encore3");
        assert_eq!(show.system.firmware, "10.0.2");
        assert_eq!(show.system.native_rate, Some(59.94));
        // One frame, seven slots, card names from hwconfig.xml.
        assert_eq!(show.system.frames.len(), 1);
        let f = &show.system.frames[0];
        assert_eq!(f.slots.len(), 7);
        assert_eq!(f.slots[1].card, "4x HDMI 2.0 input card");
        assert_eq!(f.slots[4].card, "Tri-Combo input card");
        assert_eq!(f.slots[4].connectors.len(), 6);
        assert_eq!(f.slots[5].card, "Tri-Combo output card");
        // Six outputs on the two output cards, each on a connector that exists.
        assert_eq!(show.outputs.len(), 6);
        for o in &show.outputs {
            assert_eq!(o.connector_ids.len(), 1, "{}", o.label);
            assert!(show.connector(&o.connector_ids[0]).is_some(), "{} → {}", o.label, o.connector_ids[0]);
        }
        // Six screens, 1920x1080 each, one output each, 8 layers + bg + dsk.
        assert_eq!(show.screens.len(), 6);
        let s = &show.screens[0];
        assert_eq!(s.kind, ScreenKind::Screen);
        assert_eq!((s.size.w, s.size.h), (1920, 1080));
        assert_eq!(s.outputs.len(), 1);
        assert_eq!(s.layers.len(), 10);
        assert_eq!(show.outputs.iter().filter(|o| o.role == OutputRole::Screen).count(), 6);
        // Eight sources: the six screen program returns and two multiviewers' worth.
        assert_eq!(show.sources.len(), 8);
        assert!(show.sources.iter().filter(|s| s.kind == SourceKind::Screen).count() >= 6);
        // Two multiviewers with layouts and windows.
        assert_eq!(show.multiviewers.len(), 2);
        assert!(!show.multiviewers[0].layouts.is_empty());
        assert!(show.multiviewers[0].layouts[0].widgets.len() > 3);
        assert!(show.validate().is_empty(), "{:?}", show.validate());
    }

    #[test]
    fn e2_simulator_store_bare_settings() {
        let show = import_path(&fixture("e2-sim-9.2-settings.xml")).unwrap();
        assert_eq!(show.system.model, "E2 Gen 2");
        assert_eq!(show.system.firmware, "9.2.68800");
        // The E2 store keeps the multiviewer on the frame with its own outputs.
        assert_eq!(show.outputs.iter().filter(|o| o.role == OutputRole::Multiviewer).count(), 4);
        assert_eq!(show.multiviewers.len(), 1);
        assert!(show.validate().is_empty(), "{:?}", show.validate());
    }

    #[test]
    fn targz_round_trip() {
        // Pack the E3 fixture the way an Encore3 backup is packed and read it back.
        let dir = fixture("e3-sim-10.0.2/xml");
        let mut buf = vec![];
        {
            let enc = flate2::write::GzEncoder::new(&mut buf, flate2::Compression::fast());
            let mut tar = tar::Builder::new(enc);
            for name in ["settings.xml", "hwconfig.xml"] {
                let data = std::fs::read(dir.join(name)).unwrap();
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(0o644);
                h.set_cksum();
                tar.append_data(&mut h, format!("E3Backup/xml/{name}"), data.as_slice()).unwrap();
            }
            tar.into_inner().unwrap().finish().unwrap();
        }
        let show = import_bytes("E3Backup.tar.gz", &buf).unwrap();
        assert_eq!(show.screens.len(), 6);
        assert_eq!(show.system.frames[0].slots[1].card, "4x HDMI 2.0 input card");
    }
}
