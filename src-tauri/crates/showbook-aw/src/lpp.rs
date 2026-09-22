//! `livepremier-plus.json` — a LivePremier Plus configuration export.
//!
//! LivePremier Plus is the control surface; a `.awc` is the processor. The two
//! halves of a rig are useless apart — the `.awc` restores the frame, and this
//! restores the cue stacks, layer groups, patch and routers that were built
//! against it — so Showbook carries them together (see [`crate::awc::embed`]
//! and the library's bundle writer).
//!
//! ## The envelope is ours; the sections are not
//!
//! Showbook structures the envelope — who exported it, which device it was
//! written against, which section is which — and keeps every section as an
//! opaque [`Value`]. That is the same rule the `.awc` reader follows one file
//! over: **be a librarian, not a second source of truth.** LivePremier Plus
//! owns the shape of a cue, and if it grows a field, a Showbook that parsed
//! cues would quietly drop it on the next round trip. One that carries the
//! JSON through cannot.
//!
//! ## Why three sections and not one bag
//!
//! The split is lifted from LivePremier Plus's own `server/storage.js`, which
//! files these separately and says why:
//!
//! - [`installation`](LppConfig::installation) — settings and external routers.
//!   Not keyed by device: a Videohub does not move when you fail over to a
//!   backup frame, and re-pointing the app must not close an OSC port a
//!   lighting desk is sending to.
//! - [`show`](LppConfig::show) — cue stacks and layer groups. Keyed by device,
//!   because they name screens and layer slots (`S1/2`) that only mean anything
//!   on the box they were written against.
//! - [`rig`](LppConfig::rig) — the cable schedule. Keyed by device too, but kept
//!   apart from the show, because an operator importing somebody else's cue
//!   list must not import their cabling with it.
//!
//! Carrying all three in one file does not merge them. An importer applies
//! whichever sections it was asked for, and that choice only stays available
//! if the file keeps them apart.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The `format` string every export carries, and the only one [`parse`] takes.
pub const FORMAT: &str = "livepremier-plus/config";

/// The current envelope version. Bumped only for a change a reader must know
/// about; adding a section is not one, because unknown sections are carried.
pub const VERSION: u32 = 1;

/// The entry name an embedded config takes inside an `.awc`, and the file name
/// it takes inside a bundle.
pub const FILE_NAME: &str = "livepremier-plus.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LppConfig {
    /// Always [`FORMAT`]. Present so a bare `.json` on disk identifies itself.
    pub format: String,
    pub version: u32,
    /// RFC 3339, when the export was written.
    #[serde(default)]
    pub exported: String,
    #[serde(default)]
    pub app: LppApp,
    #[serde(default)]
    pub device: LppDevice,
    #[serde(default)]
    pub installation: LppInstallation,
    #[serde(default)]
    pub show: LppShow,
    #[serde(default)]
    pub rig: LppRig,
}

/// Which LivePremier Plus wrote the file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LppApp {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// The processor the device-keyed sections were written against.
///
/// `address` is what LivePremier Plus keys its files by, so it is what an
/// importer needs in order to re-key them onto a different frame. It is
/// recorded rather than stripped for exactly that reason: a restore onto a
/// box at another address has to know what it is remapping *from*.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LppDevice {
    #[serde(default)]
    pub address: String,
    /// `NLC` (LivePremier), `MNG` (Midra 4K / Alta 4K) — the manifest's spelling.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub platform: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub firmware: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub serial: String,
}

/// Not keyed by device: this installation's own settings and routers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LppInstallation {
    /// `settings.json` — console language, AWJ transport, OSC port and bind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Value>,
    /// `matrices.json` — the external routers in the rack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrices: Option<Value>,
}

/// Keyed by device: what the operator built for this show.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LppShow {
    /// `stack-<device>.json` — the cue stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<Value>,
    /// `groups-<device>.json` — layer groups.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<Value>,
    /// `names-<device>.json` — layer names. The switcher has no field for
    /// one, so this file is the only place they exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub names: Option<Value>,
}

/// Keyed by device: how the frame is cabled to the routers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct LppRig {
    /// `patch-<device>.json` — the cable schedule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Value>,
}

impl Default for LppConfig {
    fn default() -> Self {
        LppConfig {
            format: FORMAT.to_string(),
            version: VERSION,
            exported: String::new(),
            app: LppApp::default(),
            device: LppDevice::default(),
            installation: LppInstallation::default(),
            show: LppShow::default(),
            rig: LppRig::default(),
        }
    }
}

/// What a config holds, for a list row or an import dialog.
///
/// Counts rather than contents: enough to tell two exports apart and to see
/// that a section is actually populated, without Showbook claiming to
/// understand a cue.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LppSummary {
    pub device: String,
    pub app_version: String,
    pub exported: String,
    pub cues: usize,
    pub stack_name: String,
    pub groups: usize,
    pub names: usize,
    pub patch_entries: usize,
    pub matrices: usize,
    pub has_settings: bool,
}

impl LppConfig {
    pub fn summary(&self) -> LppSummary {
        LppSummary {
            device: self.device.address.clone(),
            app_version: self.app.version.clone(),
            exported: self.exported.clone(),
            cues: count_at(self.show.stack.as_ref(), "cues"),
            stack_name: self
                .show
                .stack
                .as_ref()
                .and_then(|s| s.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
            groups: count_at(self.show.groups.as_ref(), "groups"),
            names: count_at(self.show.names.as_ref(), "names"),
            patch_entries: len_of(self.rig.patch.as_ref()),
            matrices: len_of(self.installation.matrices.as_ref()),
            has_settings: self.installation.settings.is_some(),
        }
    }

    /// True when no section carries anything — an envelope worth refusing to
    /// write, because a file that restores nothing is worse than no file.
    pub fn is_empty(&self) -> bool {
        self.installation.settings.is_none()
            && self.installation.matrices.is_none()
            && self.show.stack.is_none()
            && self.show.groups.is_none()
            && self.show.names.is_none()
            && self.rig.patch.is_none()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap_or_default()
    }
}

/// An array's length, whether it is the value itself or the one array inside
/// a single-key wrapper (`{"matrices": [...]}`, `{"entries": [...]}` — the two
/// shapes LivePremier Plus's own loaders accept).
fn len_of(v: Option<&Value>) -> usize {
    match v {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.values().find_map(|x| x.as_array()).map(|a| a.len()).unwrap_or(0),
        _ => 0,
    }
}

fn count_at(v: Option<&Value>, key: &str) -> usize {
    match v.and_then(|v| v.get(key)) {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.len(),
        _ => 0,
    }
}

/// Read a config. Anything whose `format` is not [`FORMAT`] is refused, so a
/// stray `.json` in a bundle cannot be mistaken for one.
pub fn parse(bytes: &[u8]) -> Option<LppConfig> {
    let c: LppConfig = serde_json::from_slice(bytes).ok()?;
    (c.format == FORMAT).then_some(c)
}

pub fn is_lpp_config(bytes: &[u8]) -> bool {
    parse(bytes).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> LppConfig {
        LppConfig {
            exported: "2026-09-22T10:30:00Z".into(),
            app: LppApp { name: "LivePremier Plus".into(), version: "0.9.0".into() },
            device: LppDevice { address: "127.0.0.1:3000".into(), platform: "NLC".into(), ..Default::default() },
            installation: LppInstallation {
                settings: Some(json!({"oscPort": 8000})),
                matrices: Some(json!({"matrices": [{"kind": "videohub"}, {"kind": "lightware"}]})),
            },
            show: LppShow {
                stack: Some(json!({"version": 1, "name": "Act One", "cues": [{"n": 1}, {"n": 2}, {"n": 3}]})),
                groups: Some(json!({"groups": {"a": [], "b": []}})),
                names: Some(json!({"names": {"S1/1": "Keynote"}})),
            },
            rig: LppRig { patch: Some(json!({"entries": [{"from": "IN_1"}]})) },
            ..Default::default()
        }
    }

    #[test]
    fn round_trips_through_bytes() {
        let c = sample();
        assert_eq!(parse(&c.to_bytes()).unwrap(), c);
    }

    #[test]
    fn summarises_without_understanding_the_sections() {
        let s = sample().summary();
        assert_eq!(s.cues, 3);
        assert_eq!(s.stack_name, "Act One");
        assert_eq!(s.groups, 2);
        assert_eq!(s.names, 1);
        assert_eq!(s.patch_entries, 1);
        assert_eq!(s.matrices, 2);
        assert!(s.has_settings);
        assert_eq!(s.device, "127.0.0.1:3000");
    }

    #[test]
    fn carries_fields_it_does_not_know_about() {
        // The whole point of opaque sections: a cue LivePremier Plus grows a
        // field for must survive a trip through Showbook unchanged.
        let mut c = sample();
        c.show.stack = Some(json!({"cues": [{"n": 1, "somethingNew": {"deep": [1, 2, 3]}}]}));
        let back = parse(&c.to_bytes()).unwrap();
        assert_eq!(back.show.stack, c.show.stack);
    }

    #[test]
    fn refuses_json_that_is_not_a_config() {
        assert!(parse(br#"{"format":"something/else","version":1}"#).is_none());
        assert!(parse(br#"{"cues":[]}"#).is_none());
        assert!(parse(b"not json").is_none());
    }

    #[test]
    fn an_envelope_with_no_sections_is_empty() {
        assert!(LppConfig::default().is_empty());
        assert!(!sample().is_empty());
    }
}
