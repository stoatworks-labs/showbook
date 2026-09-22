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

// ---------------------------------------------------------------- embedding

/// The entry names an `.awc` holds. The vendor writes exactly one, named with
/// eight hex digits; anything else in there was added by someone.
pub fn entry_names(bytes: &[u8]) -> Vec<String> {
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return vec![];
    };
    (0..z.len()).filter_map(|i| z.by_index_raw(i).ok().map(|f| f.name().to_string())).collect()
}

/// Add a file to an `.awc` without disturbing the vendor's own contents.
///
/// # This is a real thing the device accepts, and a sharp one
///
/// Measured on a LivePremier simulator (`NLC_CMAX`, 6.2.73) on 2026-09-22: an
/// `.awc` carrying an extra entry uploads and extracts `DONE`, and the device
/// reads the right module list out of it. The controls that make that mean
/// something: a flipped payload byte, a removed payload, a removed comment and
/// a non-zip all answer `ERROR_CORRUPTED_FILE`. So the importer does validate,
/// and it does tolerate this. `ShawanVar` in the manifest is not a hash over
/// the archive, the payload or the file, so an added entry does not invalidate
/// it.
///
/// **It is still one-way.** The device does not keep the extra entry: the next
/// `.awc` an operator exports from Web RCS is regenerated from device state,
/// and whatever was riding along is gone. Embedding buys a single file that
/// restores both halves; it does not make the device remember the second half.
/// That is why Showbook's normal export is a bundle, and this is the option.
///
/// Only proven on LivePremier. Midra 4K and Alta 4K have a different importer
/// and their simulators cannot export a baseline file to test against.
///
/// The payload entry is copied raw — its compressed bytes and CRC are the
/// vendor's, untouched — and the archive comment is carried across verbatim,
/// because the comment *is* the manifest the device reads. Re-embedding
/// replaces an entry of the same name rather than duplicating it.
pub fn embed(bytes: &[u8], name: &str, data: &[u8]) -> crate::Result<Vec<u8>> {
    use std::io::Write;
    let mut src = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(zerr)?;
    let comment = src.comment().to_vec();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let f = src.by_index_raw(i).map_err(zerr)?;
        if f.name() == name {
            continue; // replaced below
        }
        out.raw_copy_file(f).map_err(zerr)?;
    }
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    out.start_file(name, opts).map_err(zerr)?;
    out.write_all(data)?;
    out.set_raw_comment(comment.into_boxed_slice()).map_err(zerr)?;
    Ok(out.finish().map_err(zerr)?.into_inner())
}

/// Read a file previously [`embed`]ded. `None` when the archive has no such
/// entry, which is the ordinary case for a vendor file.
pub fn embedded(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    let mut f = z.by_name(name).ok()?;
    let mut buf = vec![];
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

/// Remove an embedded entry, giving back an `.awc` holding only what the
/// vendor put there.
///
/// The result is equivalent to the vendor's file, not byte-identical to it:
/// the central directory is rewritten. Where the original bytes matter — the
/// library's content-addressed blobs — keep the file the device gave you
/// rather than stripping one.
pub fn strip(bytes: &[u8], name: &str) -> crate::Result<Vec<u8>> {
    let mut src = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(zerr)?;
    let comment = src.comment().to_vec();
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for i in 0..src.len() {
        let f = src.by_index_raw(i).map_err(zerr)?;
        if f.name() == name {
            continue;
        }
        out.raw_copy_file(f).map_err(zerr)?;
    }
    out.set_raw_comment(comment.into_boxed_slice()).map_err(zerr)?;
    Ok(out.finish().map_err(zerr)?.into_inner())
}

fn zerr(e: zip::result::ZipError) -> crate::Error {
    crate::Error::Zip(e.to_string())
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

    /// A stand-in for a vendor file: one entry of incompressible bytes (the
    /// real payload is encrypted) and a manifest in the archive comment.
    fn fake_awc() -> Vec<u8> {
        use std::io::Write;
        let mut payload = vec![0u8; 4096];
        for (i, b) in payload.iter_mut().enumerate() {
            *b = ((i * 2654435761) >> 13) as u8;
        }
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        z.start_file("1f392edf", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(&payload).unwrap();
        z.set_raw_comment(
            br#"{"DeviceItem":{"Dev":10,"Label":"","ShawanVar":"2cf001fe","Timestamp":"2026_09_22_10_20_39","Version":"6.2.73"},"General":{"ExportStandard":"01.00.01"},"Modules":{"ModulesList":["GENERAL","INPUT"]},"Platform":{"PlatformName":"NLC","VersionExport":"01.00.01"}}"#
                .to_vec()
                .into_boxed_slice(),
        )
        .unwrap();
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn embedding_leaves_the_vendor_file_readable() {
        let awc = fake_awc();
        let before = manifest(&awc).unwrap();
        let out = embed(&awc, crate::lpp::FILE_NAME, br#"{"format":"livepremier-plus/config"}"#).unwrap();

        // Still an .awc, and still says exactly what it said before.
        assert!(is_awc(&out), "an embedded .awc must still identify as one");
        assert_eq!(manifest(&out).unwrap(), before, "the manifest is the device's; it must survive verbatim");
        assert_eq!(entry_names(&out), vec!["1f392edf", crate::lpp::FILE_NAME]);
    }

    #[test]
    fn the_payload_is_copied_untouched() {
        use std::io::Read;
        let awc = fake_awc();
        let out = embed(&awc, crate::lpp::FILE_NAME, b"{}").unwrap();
        let mut a = zip::ZipArchive::new(std::io::Cursor::new(&awc[..])).unwrap();
        let mut b = zip::ZipArchive::new(std::io::Cursor::new(&out[..])).unwrap();
        let (mut x, mut y) = (vec![], vec![]);
        a.by_name("1f392edf").unwrap().read_to_end(&mut x).unwrap();
        b.by_name("1f392edf").unwrap().read_to_end(&mut y).unwrap();
        assert_eq!(x, y, "the encrypted payload must come through byte for byte");
    }

    #[test]
    fn reads_back_what_was_embedded() {
        let data = br#"{"format":"livepremier-plus/config","version":1}"#;
        let out = embed(&fake_awc(), crate::lpp::FILE_NAME, data).unwrap();
        assert_eq!(embedded(&out, crate::lpp::FILE_NAME).unwrap(), data.to_vec());
        assert!(crate::lpp::is_lpp_config(&embedded(&out, crate::lpp::FILE_NAME).unwrap()));
    }

    #[test]
    fn a_plain_vendor_file_has_nothing_embedded() {
        assert!(embedded(&fake_awc(), crate::lpp::FILE_NAME).is_none());
    }

    #[test]
    fn re_embedding_replaces_rather_than_duplicates() {
        let one = embed(&fake_awc(), crate::lpp::FILE_NAME, b"first").unwrap();
        let two = embed(&one, crate::lpp::FILE_NAME, b"second").unwrap();
        assert_eq!(entry_names(&two), vec!["1f392edf", crate::lpp::FILE_NAME]);
        assert_eq!(embedded(&two, crate::lpp::FILE_NAME).unwrap(), b"second".to_vec());
    }

    #[test]
    fn stripping_gives_back_a_plain_vendor_file() {
        let awc = fake_awc();
        let out = embed(&awc, crate::lpp::FILE_NAME, b"payload").unwrap();
        let back = strip(&out, crate::lpp::FILE_NAME).unwrap();
        assert_eq!(entry_names(&back), vec!["1f392edf"]);
        assert!(embedded(&back, crate::lpp::FILE_NAME).is_none());
        assert_eq!(manifest(&back).unwrap(), manifest(&awc).unwrap());
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
