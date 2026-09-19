//! Getting at the XML store wherever it is: a directory (the simulator's
//! `xml/`, an unpacked backup), an `E3Backup.tar.gz`, a zip of the same, or
//! `settings.xml` on its own.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// The store's files, keyed by path relative to the directory holding
/// `settings.xml` (`settings.xml`, `presets/1.xml`, `hwconfig.xml`, …).
#[derive(Debug, Default)]
pub struct Store {
    pub files: BTreeMap<String, Vec<u8>>,
    /// Where the bytes came from, for the show's provenance note.
    pub origin: String,
    /// The original archive bytes, when the store was read from one, so the
    /// library can keep the vendor file beside the model.
    pub archive: Option<(String, Vec<u8>)>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        let name = path.file_name().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        if path.is_dir() {
            return Store::from_dir(path);
        }
        if name == "settings.xml" {
            let dir = path.parent().unwrap_or(Path::new("."));
            return Store::from_dir(dir);
        }
        let bytes = std::fs::read(path)?;
        Store::from_bytes(&name, &bytes)
    }

    pub fn from_bytes(name: &str, bytes: &[u8]) -> Result<Store> {
        let lower = name.to_lowercase();
        let mut store = if lower.ends_with(".zip") || bytes.starts_with(b"PK\x03\x04") {
            Store::from_zip(bytes)?
        } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || bytes.starts_with(&[0x1f, 0x8b]) {
            Store::from_targz(bytes)?
        } else if lower.ends_with(".tar") {
            Store::from_tar(bytes)?
        } else if lower.ends_with(".xml") {
            let mut s = Store::default();
            s.files.insert("settings.xml".into(), bytes.to_vec());
            s
        } else {
            return Err(Error::NotAStore(format!("{name}: not a directory, .tar.gz, .zip or settings.xml")));
        };
        store.origin = name.to_string();
        if !lower.ends_with(".xml") {
            store.archive = Some((name.to_string(), bytes.to_vec()));
        }
        Ok(store)
    }

    fn from_dir(dir: &Path) -> Result<Store> {
        // Accept the directory holding settings.xml, or one level up (a
        // backup folder holding `xml/`, or `E3Backup/xml/`).
        let root = find_settings_dir(dir)
            .ok_or_else(|| Error::NotAStore(format!("no settings.xml under {}", dir.display())))?;
        let mut store = Store { origin: dir.display().to_string(), ..Default::default() };
        collect_dir(&root, &root, &mut store.files)?;
        Ok(store)
    }

    fn from_targz(bytes: &[u8]) -> Result<Store> {
        let gz = flate2::read::GzDecoder::new(bytes);
        let mut ar = tar::Archive::new(gz);
        let mut raw: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for entry in ar.entries().map_err(|e| Error::Archive(e.to_string()))? {
            let mut entry = entry.map_err(|e| Error::Archive(e.to_string()))?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().map_err(|e| Error::Archive(e.to_string()))?.to_string_lossy().into_owned();
            let mut buf = vec![];
            entry.read_to_end(&mut buf)?;
            raw.insert(path, buf);
        }
        rebase(raw)
    }

    fn from_tar(bytes: &[u8]) -> Result<Store> {
        let mut ar = tar::Archive::new(bytes);
        let mut raw: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for entry in ar.entries().map_err(|e| Error::Archive(e.to_string()))? {
            let mut entry = entry.map_err(|e| Error::Archive(e.to_string()))?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().map_err(|e| Error::Archive(e.to_string()))?.to_string_lossy().into_owned();
            let mut buf = vec![];
            entry.read_to_end(&mut buf)?;
            raw.insert(path, buf);
        }
        rebase(raw)
    }

    fn from_zip(bytes: &[u8]) -> Result<Store> {
        let cursor = std::io::Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(cursor).map_err(|e| Error::Archive(e.to_string()))?;
        let mut raw: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| Error::Archive(e.to_string()))?;
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string();
            let mut buf = vec![];
            f.read_to_end(&mut buf)?;
            raw.insert(name, buf);
        }
        rebase(raw)
    }

    pub fn get(&self, rel: &str) -> Option<&[u8]> {
        self.files.get(rel).map(|v| v.as_slice())
    }

    pub fn text(&self, rel: &str) -> Option<String> {
        self.get(rel).map(|b| String::from_utf8_lossy(b).into_owned())
    }

    /// Files under a subdirectory of the store, sorted by name.
    pub fn under(&self, dir: &str) -> Vec<(&str, &[u8])> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let mut v: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix) && k.to_lowercase().ends_with(".xml"))
            .map(|(k, v)| (k.as_str(), v.as_slice()))
            .collect();
        v.sort_by_key(|(k, _)| natural_key(k));
        v
    }
}

/// Sort `presets/10.xml` after `presets/9.xml`.
fn natural_key(s: &str) -> (Vec<(u64, String)>, String) {
    let mut key = vec![];
    let mut num = String::new();
    let mut txt = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !txt.is_empty() {
                key.push((u64::MAX, std::mem::take(&mut txt)));
            }
            num.push(ch);
        } else {
            if !num.is_empty() {
                key.push((num.parse().unwrap_or(0), String::new()));
                num.clear();
            }
            txt.push(ch);
        }
    }
    if !num.is_empty() {
        key.push((num.parse().unwrap_or(0), String::new()));
    }
    (key, s.to_string())
}

/// Find the entry named settings.xml and make its directory the root.
fn rebase(raw: BTreeMap<String, Vec<u8>>) -> Result<Store> {
    let settings = raw
        .keys()
        .filter(|k| k.rsplit('/').next().map(|n| n.eq_ignore_ascii_case("settings.xml")).unwrap_or(false))
        .min_by_key(|k| k.matches('/').count())
        .cloned()
        .ok_or_else(|| Error::NotAStore("archive holds no settings.xml".into()))?;
    let base = match settings.rfind('/') {
        Some(i) => settings[..=i].to_string(),
        None => String::new(),
    };
    let mut files = BTreeMap::new();
    for (k, v) in raw {
        if let Some(rel) = k.strip_prefix(&base) {
            if !rel.is_empty() {
                files.insert(rel.to_string(), v);
            }
        }
    }
    Ok(Store { files, ..Default::default() })
}

fn find_settings_dir(dir: &Path) -> Option<PathBuf> {
    if dir.join("settings.xml").is_file() {
        return Some(dir.to_path_buf());
    }
    for sub in ["xml", "E3Backup/xml", "E3Backup"] {
        let p = dir.join(sub);
        if p.join("settings.xml").is_file() {
            return Some(p);
        }
    }
    // One level of subdirectories, for a backup folder with an arbitrary name.
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.join("settings.xml").is_file() {
                    return Some(p);
                }
                if p.join("xml").join("settings.xml").is_file() {
                    return Some(p.join("xml"));
                }
            }
        }
    }
    None
}

fn collect_dir(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_dir(root, &p, out)?;
        } else if p.extension().map(|x| x.eq_ignore_ascii_case("xml")).unwrap_or(false) {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.insert(rel, std::fs::read(&p)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebases_to_the_settings_directory() {
        let mut raw = BTreeMap::new();
        raw.insert("E3Backup/xml/settings.xml".to_string(), b"<System/>".to_vec());
        raw.insert("E3Backup/xml/presets/1.xml".to_string(), b"<Preset/>".to_vec());
        raw.insert("E3Backup/readme.txt".to_string(), b"x".to_vec());
        let s = rebase(raw).unwrap();
        assert!(s.get("settings.xml").is_some());
        assert_eq!(s.under("presets").len(), 1);
        assert!(s.get("readme.txt").is_none());
    }

    #[test]
    fn natural_sort() {
        let mut v = vec!["presets/10.xml", "presets/9.xml", "presets/1.xml"];
        v.sort_by_key(|k| natural_key(k));
        assert_eq!(v, vec!["presets/1.xml", "presets/9.xml", "presets/10.xml"]);
    }
}
