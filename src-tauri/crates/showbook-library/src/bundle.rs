//! `.showbook` — one file holding a show and everything needed to put it back.
//!
//! ```text
//! bundle.json              { "schema": "showbook-bundle/1", … }  what is in here
//! show.json                the neutral model
//! livepremier-plus.json    the control surface's config, when there is one
//! vendor/<sha256>.awc      the vendor files, byte for byte as the device gave them
//! ```
//!
//! ## Why a bundle rather than one clever file
//!
//! A rig is two halves: the processor, which restores from the vendor's own
//! `.awc`, and the control surface, whose cue stacks and layer groups were
//! built against it. Restoring one without the other leaves an operator half
//! rigged, so they travel together.
//!
//! An `.awc` *can* carry the second half — [`showbook_aw::awc::embed`] does it
//! and a LivePremier accepts the result — but that is one-way: the device does
//! not keep the extra entry, so the next export from Web RCS silently drops it.
//! A bundle has no such asymmetry, keeps the vendor file **untouched** so it can
//! always be handed straight to Web RCS, and can hold more than one of them.
//! So the bundle is the normal export and embedding is the option, for the case
//! it is actually good at: handing somebody a single file to restore on site.
//!
//! ## The vendor bytes are the point
//!
//! Vendor files go in at their library-relative path (`vendor/<sha256>.awc`),
//! which is what `show.vendor[].file` already names — so a bundle can be
//! unpacked into a show directory as-is, and the model still finds its blobs.
//! They are stored, not recompressed-in-spirit: an `.awc` is encrypted and
//! already incompressible, and its bytes are the thing a restore depends on.

use std::io::{Cursor, Read, Write};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use showbook_model::Show;

use crate::{Error, Result};

pub const SCHEMA: &str = "showbook-bundle/1";
/// The file extension, without the dot.
pub const EXT: &str = "showbook";
pub const MANIFEST: &str = "bundle.json";
pub const SHOW: &str = "show.json";
/// The LivePremier Plus config, named the same inside a bundle as it is when
/// embedded in an `.awc`, so one name means one thing everywhere.
pub const LPP: &str = "livepremier-plus.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub schema: String,
    pub created: String,
    /// The Showbook that wrote it.
    #[serde(default)]
    pub app_version: String,
    pub show: BundleShowRef,
    /// Every file in the bundle bar the manifest itself.
    pub contents: Vec<BundleFile>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BundleShowRef {
    pub id: String,
    pub name: String,
    pub platform: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BundleFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

/// A bundle read back off disk.
#[derive(Clone, Debug)]
pub struct Bundle {
    pub manifest: BundleManifest,
    pub show: Show,
    /// The LivePremier Plus config, unparsed. The library does not read inside
    /// it; whoever cares about cues does.
    pub lpp: Option<Vec<u8>>,
    /// `(library-relative path, bytes)` — e.g. `("vendor/ab12….awc", …)`.
    pub vendor: Vec<(String, Vec<u8>)>,
}

/// Write a bundle. `vendor` is `(library-relative path, bytes)` as
/// `show.vendor[].file` names them.
pub fn write(show: &Show, vendor: &[(String, Vec<u8>)], lpp: Option<&[u8]>, app_version: &str) -> Result<Vec<u8>> {
    let show_json = serde_json::to_vec_pretty(show)?;
    let mut contents = vec![file_entry(SHOW, &show_json)];
    if let Some(l) = lpp {
        contents.push(file_entry(LPP, l));
    }
    for (path, bytes) in vendor {
        contents.push(file_entry(path, bytes));
    }
    let manifest = BundleManifest {
        schema: SCHEMA.into(),
        created: showbook_model::now(),
        app_version: app_version.to_string(),
        show: BundleShowRef {
            id: show.id.clone(),
            name: show.meta.name.clone(),
            platform: serde_json::to_value(show.platform)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            model: show.system.model.clone(),
        },
        contents,
    };

    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let deflate = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    // An .awc is encrypted and does not compress; storing it keeps a big file
    // fast to write and leaves its bytes literally present in the archive.
    let store = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    z.start_file(MANIFEST, deflate).map_err(zerr)?;
    z.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    z.start_file(SHOW, deflate).map_err(zerr)?;
    z.write_all(&show_json)?;
    if let Some(l) = lpp {
        z.start_file(LPP, deflate).map_err(zerr)?;
        z.write_all(l)?;
    }
    for (path, bytes) in vendor {
        z.start_file(path.as_str(), store).map_err(zerr)?;
        z.write_all(bytes)?;
    }
    Ok(z.finish().map_err(zerr)?.into_inner())
}

/// Read a bundle, checking every file against the hash the manifest recorded.
///
/// The check is worth its cost: a bundle is what somebody restores from when
/// the show is already in trouble, and a file that has been truncated in
/// transit should say so here rather than halfway through a restore.
pub fn read(bytes: &[u8]) -> Result<Bundle> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zerr)?;
    let manifest: BundleManifest = serde_json::from_slice(&entry(&mut z, MANIFEST)?)?;
    if !manifest.schema.starts_with("showbook-bundle/") {
        return Err(Error::Other(format!("not a Showbook bundle: schema is {:?}", manifest.schema)));
    }
    let show_json = entry(&mut z, SHOW)?;
    let show: Show = serde_json::from_slice(&show_json)?;

    let mut lpp = None;
    let mut vendor = vec![];
    for f in &manifest.contents {
        let data = entry(&mut z, &f.path)?;
        let sha = hex::encode(Sha256::digest(&data));
        if sha != f.sha256 {
            return Err(Error::Other(format!(
                "{} is damaged: the bundle says sha256 {}, the file in it is {}",
                f.path,
                &f.sha256[..8.min(f.sha256.len())],
                &sha[..8]
            )));
        }
        match f.path.as_str() {
            SHOW => {}
            LPP => lpp = Some(data),
            p if p.starts_with("vendor/") => {
                // A bundle is a file somebody sends you, and these paths get
                // joined onto a show directory. `vendor/` as a prefix is not
                // on its own a containment check — `vendor/../../…` has it too.
                if !is_safe_path(p) {
                    return Err(Error::Other(format!("the bundle names an unsafe path: {p}")));
                }
                vendor.push((p.to_string(), data))
            }
            _ => {}
        }
    }
    Ok(Bundle { manifest, show, lpp, vendor })
}

/// Cheap enough to run on any zip before deciding what it is.
pub fn is_bundle(bytes: &[u8]) -> bool {
    let Ok(mut z) = zip::ZipArchive::new(Cursor::new(bytes)) else {
        return false;
    };
    let Ok(raw) = entry(&mut z, MANIFEST) else {
        return false;
    };
    serde_json::from_slice::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("schema").and_then(|s| s.as_str().map(str::to_string)))
        .is_some_and(|s| s.starts_with("showbook-bundle/"))
}

fn entry(z: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
    let mut f = z.by_name(name).map_err(|_| Error::Other(format!("the bundle has no {name}")))?;
    let mut buf = vec![];
    // The zip's own CRC catches damage before the manifest's sha256 gets the
    // chance, and it means exactly the same thing, so it says the same thing.
    f.read_to_end(&mut buf).map_err(|e| Error::Other(format!("{name} is damaged: {e}")))?;
    Ok(buf)
}

/// A relative path that stays inside the directory it is joined to: no root,
/// no drive letter, no `..`, and nothing Windows would read as a stream.
fn is_safe_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.starts_with('\\')
        && !p.contains(':')
        && !p.split(['/', '\\']).any(|seg| seg == ".." || seg.is_empty())
}

fn file_entry(path: &str, bytes: &[u8]) -> BundleFile {
    BundleFile { path: path.to_string(), size: bytes.len() as u64, sha256: hex::encode(Sha256::digest(bytes)) }
}

fn zerr(e: zip::result::ZipError) -> Error {
    Error::Other(format!("zip: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use showbook_model::Platform;

    fn show() -> Show {
        let mut s = Show::new("Bundle test", Platform::AwLivePremier);
        s.system.model = "Aquilon C max".into();
        s
    }

    fn vendor() -> Vec<(String, Vec<u8>)> {
        vec![("vendor/abc123.awc".to_string(), b"PK\x03\x04 pretend this is encrypted".to_vec())]
    }

    #[test]
    fn round_trips() {
        let (s, v) = (show(), vendor());
        let lpp = br#"{"format":"livepremier-plus/config","version":1}"#;
        let bytes = write(&s, &v, Some(lpp), "0.1.1").unwrap();

        assert!(is_bundle(&bytes));
        let b = read(&bytes).unwrap();
        assert_eq!(b.show.meta.name, "Bundle test");
        assert_eq!(b.lpp.unwrap(), lpp.to_vec());
        assert_eq!(b.vendor, v, "vendor bytes must come back exactly");
        assert_eq!(b.manifest.show.platform, "aw-live-premier");
    }

    #[test]
    fn works_without_a_livepremier_plus_config() {
        let bytes = write(&show(), &vendor(), None, "0.1.1").unwrap();
        let b = read(&bytes).unwrap();
        assert!(b.lpp.is_none());
        assert_eq!(b.vendor.len(), 1);
    }

    #[test]
    fn vendor_paths_match_what_the_model_names() {
        // A bundle unpacks into a show directory as-is, so the paths in it
        // have to be the ones show.vendor[].file already carries.
        let bytes = write(&show(), &vendor(), None, "0.1.1").unwrap();
        let b = read(&bytes).unwrap();
        assert_eq!(b.vendor[0].0, "vendor/abc123.awc");
    }

    #[test]
    fn a_damaged_file_is_reported_not_returned() {
        let bytes = write(&show(), &vendor(), None, "0.1.1").unwrap();
        // Corrupt the stored (uncompressed) vendor bytes in place.
        let needle = b"pretend this is encrypted";
        let at = bytes.windows(needle.len()).position(|w| w == needle).expect("stored plainly");
        let mut damaged = bytes.clone();
        damaged[at] = b'X';
        let e = read(&damaged).unwrap_err().to_string();
        assert!(e.contains("is damaged"), "{e}");
    }

    #[test]
    fn refuses_a_bundle_that_points_outside_the_show() {
        let escape = vec![("vendor/../../../../etc/passwd".to_string(), b"nope".to_vec())];
        let bytes = write(&show(), &escape, None, "0.1.1").unwrap();
        let e = read(&bytes).unwrap_err().to_string();
        assert!(e.contains("unsafe path"), "{e}");
    }

    #[test]
    fn judges_paths_on_containment_not_prefix() {
        assert!(is_safe_path("vendor/abc.awc"));
        assert!(!is_safe_path("vendor/../../evil"));
        assert!(!is_safe_path("/etc/passwd"));
        assert!(!is_safe_path("C:\\windows\\system32"));
        assert!(!is_safe_path("vendor\\..\\..\\evil"));
        assert!(!is_safe_path(""));
    }

    #[test]
    fn refuses_a_zip_that_is_not_a_bundle() {
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        z.start_file("settings.xml", zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(b"<xml/>").unwrap();
        let other = z.finish().unwrap().into_inner();
        assert!(!is_bundle(&other));
        assert!(read(&other).is_err());
    }
}
