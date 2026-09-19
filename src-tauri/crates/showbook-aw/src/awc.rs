//! `.awc` — the LivePremier configuration file.
//!
//! A zip (`PK\x03\x04`) with one payload entry and an archive comment holding
//! a JSON manifest:
//!
//! ```json
//! { "DeviceItem": { "Dev": 10, "Label": "", "Timestamp": "2026_09_19_16_59_11",
//!                   "Version": "6.2.73", "ShawanVar": "<sha1>", "VerVar": 0 },
//!   "General":  { "ExportStandard": "01.00.01" },
//!   "Modules":  { "ModulesList": ["GENERAL", "INPUT", "OUTPUT", "PRESET_BANK", …] },
//!   "Platform": { "PlatformName": "NLC", "VersionExport": "01.00.01" } }
//! ```
//!
//! The payload is encrypted (measured at 7.9999 bits of entropy per byte on a
//! simulator export), so this module reads the manifest and nothing else.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AwcManifest {
    pub device_type: Option<i64>,
    pub label: String,
    pub timestamp: String,
    pub firmware: String,
    pub platform: String,
    pub modules: Vec<String>,
    pub export_standard: String,
}

/// The module keys a device knows, in the order the Web RCS lists them.
pub const MODULES: &[&str] = &[
    "GENERAL", "FRONTPANEL", "COMM_INTERFACE", "INPUT", "INPUT_EDID", "OUTPUT", "MULTIVIEWER", "STILL", "STILL_BANK",
    "PRESET_BANK", "CUSTOM_FORMAT_BANK", "MTVW_BANK", "EDID_BANK", "WEBAPP_SETTINGS", "AUDIO", "GPO", "LUT_BANK",
    "HDR_INFO_BANK", "LOGS",
];

/// The modules a *show* is: everything but the still library (large), LUTs
/// and logs. `STILL` (frame parameters) stays in.
pub const SHOW_MODULES: &[&str] = &[
    "GENERAL", "FRONTPANEL", "COMM_INTERFACE", "INPUT", "INPUT_EDID", "OUTPUT", "MULTIVIEWER", "STILL", "PRESET_BANK",
    "CUSTOM_FORMAT_BANK", "MTVW_BANK", "EDID_BANK", "WEBAPP_SETTINGS", "AUDIO", "GPO", "HDR_INFO_BANK",
];

/// Read the manifest out of an `.awc`'s zip comment.
pub fn manifest(bytes: &[u8]) -> Option<AwcManifest> {
    let comment = zip_comment(bytes)?;
    let v: serde_json::Value = serde_json::from_slice(comment).ok()?;
    let dev = v.get("DeviceItem")?;
    Some(AwcManifest {
        device_type: dev.get("Dev").and_then(|d| d.as_i64()),
        label: dev.get("Label").and_then(|d| d.as_str()).unwrap_or("").to_string(),
        timestamp: dev.get("Timestamp").and_then(|d| d.as_str()).unwrap_or("").to_string(),
        firmware: dev.get("Version").and_then(|d| d.as_str()).unwrap_or("").to_string(),
        platform: v.pointer("/Platform/PlatformName").and_then(|d| d.as_str()).unwrap_or("").to_string(),
        modules: v
            .pointer("/Modules/ModulesList")
            .and_then(|m| m.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
        export_standard: v.pointer("/General/ExportStandard").and_then(|d| d.as_str()).unwrap_or("").to_string(),
    })
}

/// The archive comment of a zip: the end-of-central-directory record is the
/// last 22 bytes plus the comment, whose length sits at offset 20.
fn zip_comment(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 22 {
        return None;
    }
    let max_back = bytes.len().min(22 + 65535);
    let start = bytes.len() - max_back;
    let eocd = (start..=bytes.len() - 22).rev().find(|&i| &bytes[i..i + 4] == b"PK\x05\x06")?;
    let len = u16::from_le_bytes([bytes[eocd + 20], bytes[eocd + 21]]) as usize;
    let cstart = eocd + 22;
    if cstart + len > bytes.len() {
        return None;
    }
    Some(&bytes[cstart..cstart + len])
}

pub fn is_awc(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") && manifest(bytes).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_zip_with_comment(comment: &str) -> Vec<u8> {
        // An empty zip: just an end-of-central-directory record + comment.
        let mut v = b"PK\x05\x06".to_vec();
        v.extend_from_slice(&[0u8; 16]);
        v.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        v.extend_from_slice(comment.as_bytes());
        v
    }

    #[test]
    fn reads_the_manifest() {
        let z = tiny_zip_with_comment(
            r#"{"DeviceItem":{"Dev":10,"Label":"","Timestamp":"2026_09_19_16_59_11","Version":"6.2.73"},"General":{"ExportStandard":"01.00.01"},"Modules":{"ModulesList":["GENERAL","INPUT"]},"Platform":{"PlatformName":"NLC","VersionExport":"01.00.01"}}"#,
        );
        let m = manifest(&z).unwrap();
        assert_eq!(m.firmware, "6.2.73");
        assert_eq!(m.platform, "NLC");
        assert_eq!(m.modules, vec!["GENERAL", "INPUT"]);
        assert_eq!(m.device_type, Some(10));
    }
}
