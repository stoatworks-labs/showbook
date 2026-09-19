//! Analog Way LivePremier driver (Aquilon C / RS).
//!
//! A LivePremier keeps its whole state as one JSON document — the Web RCS
//! seeds itself from `GET /api/stores/device` and then follows changes over a
//! WebSocket; the AWJ control protocol on TCP 10606 addresses the same tree
//! with `DeviceObject/…/@items/…/@props/…` spellings. That document is what
//! [`store::parse`] reads into a [`showbook_model::Show`]: inputs and their
//! plugs, outputs and their formats, screens with their canvas, layers and
//! the live program state, auxes, the preset (memory) and master-preset
//! banks, multiviewers and their widgets, stills, and the mixer allocation.
//!
//! The vendor's own show file, an `.awc`, is an encrypted zip whose comment
//! names the device, firmware and modules inside. Showbook does not read
//! inside it: an `.awc` is kept as a vendor blob next to the model, downloaded
//! from a device with [`Device::download_config`] and put back on one with
//! [`Device::upload_config`] + [`Device::apply_config`] — the exact restore
//! path the Web RCS itself uses.
//!
//! Memories: the bank lists what each slot is (label, valid, which screens,
//! canvas size, transition time) but not the layer values inside it; those
//! only exist on the device and are read by recalling a memory into a preview
//! and reading the layers back, which [`Device::deep_capture`] does — it is
//! explicitly a write to the device's preview buffers and is never run
//! implicitly.
//!
//! Verified against the LivePremier simulator 6.2.73 (`NLC_CMAX`). The REST
//! API calls follow the *LivePremier REST API Programmer's Guide v4.1*; AWJ
//! paths follow the v6.2 AWJ guide and the fleet's `openrcs-awj` crate.

pub mod awc;
pub mod awj;
pub mod device;
pub mod store;
pub mod store_mng;

pub use device::Device;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("not a LivePremier store: {0}")]
    NotAStore(String),
    #[error("device: {0}")]
    Device(String),
    #[error("awj: {0}")]
    Awj(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Read a saved device store (`GET /api/stores/device` as a file) into a show.
pub fn import_store_json(name: &str, bytes: &[u8]) -> Result<showbook_model::Show> {
    let v: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| Error::NotAStore(e.to_string()))?;
    let mut show = store::parse(&v)?;
    show.meta.source = Some(showbook_model::SourceInfo {
        kind: "file".into(),
        origin: name.to_string(),
        at: showbook_model::now(),
        firmware: Some(show.system.firmware.clone()),
    });
    Ok(show)
}
