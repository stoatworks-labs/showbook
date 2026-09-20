//! Showbook's Rust core for the browser: the same parsers, conversion and
//! Companion export the desktop app runs, compiled to WebAssembly for
//! showbook-lite. Everything a page can do with a show *file* is here; nothing
//! that needs a socket is (the live drivers sit behind the driver crates'
//! `live` feature, which this crate turns off).
//!
//! The API is deliberately flat and JSON-shaped: a `Show` goes in and out as
//! the same JSON the desktop app writes to `show.json`, so `src/types.ts` on
//! the front end describes both. Errors come back as thrown `JsError`s with the
//! same wording the desktop app's commands produce.

use serde::Serialize;
use showbook_model::{Platform, Show};
use wasm_bindgen::prelude::*;

fn js<T: Serialize>(v: &T) -> Result<JsValue, JsError> {
    // `serde_json::Map` preserves key order (the workspace enables
    // `preserve_order`), so the serializer below sees objects in the order the
    // model wrote them. JS gets plain objects, not Maps.
    let s = serde_wasm_bindgen::Serializer::json_compatible();
    v.serialize(&s).map_err(|e| JsError::new(&e.to_string()))
}

fn show_from(json: &str) -> Result<Show, JsError> {
    serde_json::from_str(json).map_err(|e| JsError::new(&format!("not a Showbook show: {e}")))
}

fn platform_from(id: &str) -> Result<Platform, JsError> {
    serde_json::from_value(serde_json::Value::String(id.to_string()))
        .map_err(|_| JsError::new(&format!("unknown platform {id}")))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Imported {
    show: Show,
    /// What the file turned out to be: `em-store`, `em-backup`, `awc`,
    /// `aw-store`, `showbook`.
    kind: String,
    summary: showbook_model::summary::Summary,
}

/// The version of the core this page is running.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Every platform the model knows, with its label and the models the
/// capability table has for it — what the New show and Convert pickers list.
#[wasm_bindgen]
pub fn platforms() -> Result<JsValue, JsError> {
    let list: Vec<serde_json::Value> = Platform::all()
        .iter()
        .map(|p| serde_json::json!({"id": p, "label": p.label(), "models": showbook_convert::models(*p)}))
        .collect();
    js(&list)
}

/// Read a show file the way the desktop app's Import does. `name` is the file
/// name, which decides the parser for the ambiguous cases (`.json` is a Showbook
/// show or a saved LivePremier device store; `.xml` is an Event Master
/// `settings.xml`). Event Master backups (`.tar.gz`, `.zip`) and LivePremier
/// `.awc` files are recognised by content.
#[wasm_bindgen]
pub fn import_file(name: &str, bytes: &[u8]) -> Result<JsValue, JsError> {
    let lower = name.to_lowercase();
    let (show, kind) = if lower.ends_with(".awc") || showbook_aw::awc::is_awc(bytes) {
        let m = showbook_aw::awc::manifest(bytes)
            .ok_or_else(|| JsError::new("not a LivePremier .awc (no manifest in the zip comment)"))?;
        let mut show = Show::new(name.trim_end_matches(".awc"), Platform::AwLivePremier);
        show.system.firmware = m.firmware.clone();
        show.system.model = m.label.clone();
        show.meta.notes = format!(
            "Imported from {name}: a LivePremier configuration file exported {} by firmware {} with modules {}. The file is encrypted; its contents are not readable here. The desktop app can pull the show from the device to fill in the model, or push this file to a device.",
            m.timestamp.replace('_', "-"),
            m.firmware,
            m.modules.join(", ")
        );
        (show, "awc")
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || lower.ends_with(".zip") || lower.ends_with(".tar") {
        (showbook_em::import_bytes(name, bytes).map_err(|e| JsError::new(&e.to_string()))?, "em-backup")
    } else if lower.ends_with(".xml") {
        // A bare settings.xml: the store without its presets/ and cues/ folders.
        (showbook_em::import_bytes(name, bytes).map_err(|e| JsError::new(&e.to_string()))?, "em-store")
    } else if lower.ends_with(".json") {
        match serde_json::from_slice::<Show>(bytes) {
            Ok(mut s) if s.schema.starts_with("showbook/") => {
                s.id = uuid::Uuid::new_v4().to_string();
                (s, "showbook")
            }
            _ => (showbook_aw::import_store_json(name, bytes).map_err(|e| JsError::new(&e.to_string()))?, "aw-store"),
        }
    } else {
        return Err(JsError::new(&format!(
            "{name}: not a show file Showbook knows (Event Master backup .tar.gz/.zip or settings.xml, LivePremier .awc, a saved device store .json, or a Showbook .json)"
        )));
    };
    let summary = showbook_model::summary::Summary::of(&show);
    js(&Imported { show, kind: kind.to_string(), summary })
}

/// An empty show for a platform and model, as New show makes.
#[wasm_bindgen]
pub fn new_show(name: &str, platform: &str, model: &str) -> Result<JsValue, JsError> {
    let mut show = Show::new(name, platform_from(platform)?);
    show.system.model = model.to_string();
    js(&show)
}

#[wasm_bindgen]
pub fn summary(show: &str) -> Result<JsValue, JsError> {
    js(&showbook_model::summary::Summary::of(&show_from(show)?))
}

/// The reference-graph problems in a show, as the desktop app's Validate.
#[wasm_bindgen]
pub fn validate(show: &str) -> Result<JsValue, JsError> {
    js(&show_from(show)?.validate())
}

/// What changed between two versions, keyed by what the entries are.
#[wasm_bindgen]
pub fn diff(before: &str, after: &str) -> Result<JsValue, JsError> {
    let a: serde_json::Value = serde_json::from_str(before).map_err(|e| JsError::new(&e.to_string()))?;
    let b: serde_json::Value = serde_json::from_str(after).map_err(|e| JsError::new(&e.to_string()))?;
    js(&showbook_model::diff::diff(&a, &b))
}

/// The hash a version is stored under: SHA-256 of the show with `meta.modified`
/// blanked, so re-saving an unchanged show is not a new version. The same rule
/// the desktop library applies.
#[wasm_bindgen]
pub fn hash_show(show: &str) -> Result<String, JsError> {
    let mut v: serde_json::Value = serde_json::from_str(show).map_err(|e| JsError::new(&e.to_string()))?;
    if let Some(m) = v.get_mut("meta").and_then(|m| m.as_object_mut()) {
        m.insert("modified".into(), serde_json::Value::String(String::new()));
    }
    let bytes = serde_json::to_vec(&v).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(sha256(&bytes))
}

#[wasm_bindgen]
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(bytes))
}

/// What a model holds, from the capability table.
#[wasm_bindgen]
pub fn capabilities(platform: &str, model: &str) -> Result<JsValue, JsError> {
    js(&showbook_convert::capabilities(platform_from(platform)?, model))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Converted {
    show: Show,
    notes: Vec<showbook_model::Note>,
    /// Old id → new id, for anyone holding references.
    id_map: std::collections::BTreeMap<String, String>,
}

/// Convert a show for another platform: the converted show, the report, and
/// the id map. The same shape as the desktop app's `convert_show`, plus the map.
#[wasm_bindgen]
pub fn convert(show: &str, platform: &str, model: &str) -> Result<JsValue, JsError> {
    let c = showbook_convert::convert(&show_from(show)?, platform_from(platform)?, model);
    js(&Converted { show: c.show, notes: c.notes, id_map: c.id_map })
}

/// A Bitfocus Companion page for the show; `opts` is the desktop app's
/// `ExportOptions` as JSON. Returns the `.companionconfig` document as JSON text.
#[wasm_bindgen]
pub fn companion_export(show: &str, opts: &str) -> Result<String, JsError> {
    let opts: showbook_companion::ExportOptions =
        serde_json::from_str(opts).map_err(|e| JsError::new(&format!("bad Companion options: {e}")))?;
    let doc = showbook_companion::export(&show_from(show)?, &opts).map_err(|e| JsError::new(&e))?;
    serde_json::to_string_pretty(&doc).map_err(|e| JsError::new(&e.to_string()))
}

/// Read a Companion page back and check its buttons against the show.
#[wasm_bindgen]
pub fn companion_import(bytes: &[u8], show: &str) -> Result<JsValue, JsError> {
    js(&showbook_companion::import(bytes, &show_from(show)?).map_err(|e| JsError::new(&e))?)
}

/// A zip of files, for the test-pattern download: `names` are the entry
/// names, `datas` the bytes of each (a JS array of Uint8Array), in step.
#[wasm_bindgen]
pub fn zip_files(names: Vec<String>, datas: js_sys::Array) -> Result<Vec<u8>, JsError> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (i, name) in names.iter().enumerate() {
            let data = js_sys::Uint8Array::new(&datas.get(i as u32)).to_vec();
            z.start_file(name, opts).map_err(|e| JsError::new(&e.to_string()))?;
            z.write_all(&data).map_err(|e| JsError::new(&e.to_string()))?;
        }
        z.finish().map_err(|e| JsError::new(&e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_the_fixtures_by_content_and_name() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
        let em = std::fs::read(root.join("em/e2-sim-9.2-settings.xml")).unwrap();
        let aw = std::fs::read(root.join("aw/livepremier-sim-6.2.73-store.json")).unwrap();
        // The wasm-bindgen return type is only convertible on wasm; test the
        // pieces beneath it instead.
        assert!(showbook_em::import_bytes("settings.xml", &em).is_ok());
        assert!(showbook_aw::import_store_json("store.json", &aw).is_ok());
        assert_eq!(sha256(b"showbook").len(), 64);
    }
}
