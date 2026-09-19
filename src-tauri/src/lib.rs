//! Showbook — the Tauri layer.
//!
//! Thin on purpose: every decision about a show lives in the workspace crates
//! (`showbook-model`, `-em`, `-aw`, `-library`, `-convert`, `-companion`) and
//! this file only wires them to the window. Commands that touch the network
//! or the disk for more than an instant run on the blocking pool.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use showbook_library::{Commit, Entry, Library};
use showbook_model::summary::Summary;
use showbook_model::{Platform, Show};
use tauri::{AppHandle, Manager, State};

mod settings;

use settings::Settings;

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub config_dir: PathBuf,
    pub pending_auth: Mutex<Option<showbook_library::oauth::PendingAuth>>,
}

type CmdResult<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn library(state: &AppState) -> CmdResult<Library> {
    let path = state.settings.lock().map_err(err)?.library_path.clone();
    Library::open(Path::new(&path)).map_err(err)
}

fn author(state: &AppState) -> Option<String> {
    state.settings.lock().ok().and_then(|s| if s.author.trim().is_empty() { None } else { Some(s.author.clone()) })
}

// ---------------------------------------------------------------- settings

#[tauri::command]
fn settings_get(state: State<AppState>) -> CmdResult<Settings> {
    Ok(state.settings.lock().map_err(err)?.clone())
}

#[tauri::command]
fn settings_set(state: State<AppState>, settings: Settings) -> CmdResult<Settings> {
    settings.save(&state.config_dir).map_err(err)?;
    *state.settings.lock().map_err(err)? = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn app_info(state: State<AppState>) -> CmdResult<Value> {
    Ok(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "configDir": state.config_dir.display().to_string(),
        "platforms": Platform::all().iter().map(|p| serde_json::json!({"id": p, "label": p.label(), "models": showbook_convert::models(*p)})).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------- library

#[tauri::command]
fn library_list(state: State<AppState>) -> CmdResult<Vec<Entry>> {
    library(&state)?.list().map_err(err)
}

#[tauri::command]
fn show_load(state: State<AppState>, id: String) -> CmdResult<Show> {
    library(&state)?.load(&id).map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveResult {
    show: Show,
    commit: Option<Commit>,
}

#[tauri::command]
fn show_save(state: State<AppState>, mut show: Show, message: String) -> CmdResult<SaveResult> {
    let lib = library(&state)?;
    let commit = lib.save(&mut show, &message, author(&state).as_deref()).map_err(err)?;
    Ok(SaveResult { show, commit })
}

#[tauri::command]
fn show_new(state: State<AppState>, name: String, platform: Platform, model: String) -> CmdResult<Show> {
    let lib = library(&state)?;
    let mut show = Show::new(&name, platform);
    show.system.model = model;
    lib.save(&mut show, "Created", author(&state).as_deref()).map_err(err)?;
    Ok(show)
}

#[tauri::command]
fn show_history(state: State<AppState>, id: String) -> CmdResult<Vec<Commit>> {
    library(&state)?.history(&id).map_err(err)
}

#[tauri::command]
fn show_snapshot(state: State<AppState>, id: String, commit: String) -> CmdResult<Show> {
    library(&state)?.snapshot(&id, &commit).map_err(err)
}

#[tauri::command]
fn show_diff(state: State<AppState>, id: String, from: String, to: String) -> CmdResult<Vec<showbook_model::diff::Change>> {
    library(&state)?.diff(&id, &from, &to).map_err(err)
}

#[tauri::command]
fn show_restore(state: State<AppState>, id: String, commit: String) -> CmdResult<Commit> {
    library(&state)?.restore(&id, &commit, author(&state).as_deref()).map_err(err)
}

#[tauri::command]
fn show_delete(state: State<AppState>, id: String) -> CmdResult<()> {
    library(&state)?.delete(&id).map_err(err)
}

#[tauri::command]
fn show_duplicate(state: State<AppState>, id: String, name: String) -> CmdResult<Show> {
    library(&state)?.duplicate(&id, &name, author(&state).as_deref()).map_err(err)
}

#[tauri::command]
fn show_validate(show: Show) -> CmdResult<Vec<String>> {
    Ok(show.validate())
}

// ---------------------------------------------------------------- import / export of files

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportResult {
    show: Show,
    summary: Summary,
    /// What the file turned out to be.
    kind: String,
}

fn import_any(lib: &Library, path: &Path, author: Option<&str>) -> CmdResult<ImportResult> {
    let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let lower = name.to_lowercase();
    let bytes = if path.is_dir() { vec![] } else { std::fs::read(path).map_err(err)? };
    let (mut show, kind) = if path.is_dir() || lower.ends_with("settings.xml") {
        (showbook_em::import_path(path).map_err(err)?, "em-store".to_string())
    } else if lower.ends_with(".awc") || showbook_aw::awc::is_awc(&bytes) {
        let m = showbook_aw::awc::manifest(&bytes).ok_or("not a LivePremier .awc (no manifest in the zip comment)")?;
        let mut show = Show::new(name.trim_end_matches(".awc"), Platform::AwLivePremier);
        show.system.firmware = m.firmware.clone();
        show.system.model = m.label.clone();
        show.meta.notes = format!(
            "Imported from {name}: a LivePremier configuration file exported {} by firmware {} with modules {}. The file is encrypted; its contents are not readable here. Pull the show from the device to fill in the model, or push this file to a device with Devices → Push.",
            m.timestamp.replace('_', "-"),
            m.firmware,
            m.modules.join(", ")
        );
        (show, "awc".to_string())
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") || lower.ends_with(".zip") || lower.ends_with(".tar") {
        (showbook_em::import_bytes(&name, &bytes).map_err(err)?, "em-backup".to_string())
    } else if lower.ends_with(".json") {
        // A Showbook show, or a saved LivePremier device store.
        match serde_json::from_slice::<Show>(&bytes) {
            Ok(mut s) if s.schema.starts_with("showbook/") => {
                s.id = uuid::Uuid::new_v4().to_string();
                (s, "showbook".to_string())
            }
            _ => (showbook_aw::import_store_json(&name, &bytes).map_err(err)?, "aw-store".to_string()),
        }
    } else {
        return Err(format!("{name}: not a show file Showbook knows (Event Master backup .tar.gz/.zip, settings.xml or its folder, LivePremier .awc, a saved device store .json, or a Showbook .json)"));
    };
    if kind == "awc" || kind == "em-backup" {
        let platform = show.platform;
        let vkind = if kind == "awc" { "awc" } else { "em-backup" };
        lib.save(&mut show, "Imported", author).map_err(err)?;
        lib.add_vendor(&mut show, platform, vkind, &name, &bytes, "imported file").map_err(err)?;
    }
    lib.save(&mut show, &format!("Imported {name}"), author).map_err(err)?;
    let summary = Summary::of(&show);
    Ok(ImportResult { show, summary, kind })
}

/// The import the desktop app does, for scripts and the `seed` example.
pub fn import_for_cli(lib: &Library, path: &Path) -> CmdResult<Show> {
    import_any(lib, path, None).map(|r| r.show)
}

#[tauri::command]
async fn import_path(app: AppHandle, path: String) -> CmdResult<ImportResult> {
    let state = app.state::<AppState>();
    let lib = library(&state)?;
    let a = author(&state);
    tauri::async_runtime::spawn_blocking(move || import_any(&lib, Path::new(&path), a.as_deref())).await.map_err(err)?
}

#[tauri::command]
fn export_show_json(state: State<AppState>, id: String, path: String) -> CmdResult<()> {
    let show = library(&state)?.load(&id).map_err(err)?;
    std::fs::write(&path, serde_json::to_string_pretty(&show).map_err(err)?).map_err(err)
}

#[tauri::command]
fn vendor_export(state: State<AppState>, id: String, sha256: String, path: String) -> CmdResult<()> {
    let lib = library(&state)?;
    let show = lib.load(&id).map_err(err)?;
    let blob = show.vendor.iter().find(|b| b.sha256 == sha256).ok_or("no such vendor file")?;
    let bytes = lib.vendor_bytes(&id, blob).map_err(err)?;
    std::fs::write(&path, bytes).map_err(err)
}

/// Write bytes the webview produced (a PDF, a PNG) to a path it chose.
#[tauri::command]
fn write_file(path: String, base64: String) -> CmdResult<()> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64.as_bytes()).map_err(err)?;
    std::fs::write(&path, bytes).map_err(err)
}

#[tauri::command]
fn write_text(path: String, text: String) -> CmdResult<()> {
    std::fs::write(&path, text).map_err(err)
}

#[tauri::command]
fn read_text(path: String) -> CmdResult<String> {
    std::fs::read_to_string(&path).map_err(err)
}

// ---------------------------------------------------------------- devices

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeviceRef {
    platform: Platform,
    host: String,
    #[serde(default)]
    awj_host: Option<String>,
}

#[tauri::command]
async fn device_probe(dev: DeviceRef) -> CmdResult<Value> {
    tauri::async_runtime::spawn_blocking(move || match dev.platform {
        Platform::BarcoEm | Platform::BarcoPds4k => {
            let d = showbook_em::live::Device::connect(&dev.host);
            let power = d.ping().map_err(err)?;
            let fs = d.client.call("getFrameSettings", serde_json::json!({})).ok();
            Ok(serde_json::json!({"ok": true, "platform": dev.platform, "power": power, "frame": fs}))
        }
        Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => {
            let d = showbook_aw::Device::connect(&dev.host, dev.awj_host.as_deref());
            let sys = d.ping().map_err(err)?;
            Ok(serde_json::json!({"ok": true, "platform": dev.platform, "system": sys}))
        }
        p => Err(format!("no live driver for {} yet", p.label())),
    })
    .await
    .map_err(err)?
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PullOptions {
    /// Also fetch the vendor file (E3 backup / .awc) and keep it with the show.
    #[serde(default = "yes")]
    vendor_file: bool,
    /// LivePremier: recall each memory into preview to read its layers (writes to preview buffers).
    #[serde(default)]
    deep_capture: bool,
    /// Existing show to update instead of creating a new one.
    #[serde(default)]
    into_show: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

fn yes() -> bool {
    true
}

#[tauri::command]
async fn device_pull(app: AppHandle, dev: DeviceRef, opts: PullOptions) -> CmdResult<ImportResult> {
    let state = app.state::<AppState>();
    let lib = library(&state)?;
    let a = author(&state);
    tauri::async_runtime::spawn_blocking(move || {
        type Pulled = (Show, &'static str, Option<(String, Vec<u8>, &'static str)>);
        let (mut show, kind, vendor): Pulled = match dev.platform {
            Platform::BarcoEm | Platform::BarcoPds4k => {
                let d = showbook_em::live::Device::connect(&dev.host);
                let mut show = d.read_show().map_err(err)?;
                let mut vendor = None;
                if opts.vendor_file {
                    match d.download_backup() {
                        Ok((name, bytes)) => {
                            // The backup is the richer source: presets and cues with content.
                            if let Ok(full) = showbook_em::import_bytes(&name, &bytes) {
                                let src = show.meta.source.clone();
                                show = full;
                                show.meta.source = src;
                            }
                            vendor = Some((name, bytes, "em-backup"));
                        }
                        Err(e) => show.notes.push(showbook_model::Note { level: showbook_model::NoteLevel::Info, path: "vendor".into(), message: format!("backup archive not fetched: {e}") }),
                    }
                }
                (show, "device", vendor)
            }
            Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => {
                let d = showbook_aw::Device::connect(&dev.host, dev.awj_host.as_deref());
                let mut show = d.read_show().map_err(err)?;
                if opts.deep_capture {
                    let screens: Vec<String> = show.screens.iter().map(|s| s.id.clone()).collect();
                    match d.deep_capture(&mut show, &screens) {
                        Ok(n) => show.notes.push(showbook_model::Note { level: showbook_model::NoteLevel::Info, path: "presets".into(), message: format!("deep capture read the layers of {n} memories by recalling each into preview") }),
                        Err(e) => show.notes.push(showbook_model::Note { level: showbook_model::NoteLevel::Info, path: "presets".into(), message: format!("deep capture failed: {e}") }),
                    }
                }
                let mut vendor = None;
                if opts.vendor_file {
                    match d.download_config(showbook_aw::awc::SHOW_MODULES) {
                        Ok((name, bytes)) => vendor = Some((name, bytes, "awc")),
                        Err(e) => show.notes.push(showbook_model::Note { level: showbook_model::NoteLevel::Info, path: "vendor".into(), message: format!(".awc not fetched: {e}") }),
                    }
                }
                (show, "device", vendor)
            }
            p => return Err(format!("no live driver for {} yet", p.label())),
        };
        if let Some(existing) = &opts.into_show {
            if let Ok(prev) = lib.load(existing) {
                show.id = prev.id;
                show.meta.name = prev.meta.name;
                show.meta.created = prev.meta.created;
                show.meta.notes = prev.meta.notes;
                show.meta.tags = prev.meta.tags;
                show.vendor = prev.vendor;
            }
        }
        if let Some(n) = &opts.name {
            if !n.trim().is_empty() {
                show.meta.name = n.trim().to_string();
            }
        }
        lib.save(&mut show, &format!("Pulled from {}", dev.host), a.as_deref()).map_err(err)?;
        if let Some((name, bytes, vk)) = vendor {
            let platform = show.platform;
            lib.add_vendor(&mut show, platform, vk, &name, &bytes, &format!("pulled from {}", dev.host)).map_err(err)?;
            lib.save(&mut show, &format!("Vendor file from {}", dev.host), a.as_deref()).map_err(err)?;
        }
        let summary = Summary::of(&show);
        Ok(ImportResult { show, summary, kind: kind.into() })
    })
    .await
    .map_err(err)?
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PushOptions {
    /// Restore the show's vendor file on the device (exact restore; reboots a LivePremier).
    #[serde(default)]
    vendor_file: bool,
    /// LivePremier modules to apply from the vendor file.
    #[serde(default)]
    modules: Vec<String>,
    /// Write labels (screens, inputs, outputs, memories) from the model.
    #[serde(default)]
    labels: bool,
}

#[tauri::command]
async fn device_push(app: AppHandle, dev: DeviceRef, id: String, opts: PushOptions) -> CmdResult<Value> {
    let state = app.state::<AppState>();
    let lib = library(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let show = lib.load(&id).map_err(err)?;
        let mut log: Vec<String> = vec![];
        match dev.platform {
            Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => {
                let d = showbook_aw::Device::connect(&dev.host, dev.awj_host.as_deref());
                if opts.labels {
                    for sc in &show.screens {
                        let key = showbook_model::ids::tail(&sc.id);
                        match d.set_screen_label(key, &sc.label) {
                            Ok(()) => log.push(format!("label {key} = {:?}", sc.label)),
                            Err(e) => log.push(format!("label {key}: {e}")),
                        }
                    }
                    for i in &show.inputs {
                        let key = showbook_model::ids::tail(&i.id);
                        if let Err(e) = d.set_input_label(key, &i.label) {
                            log.push(format!("input {key}: {e}"));
                        }
                    }
                    for o in &show.outputs {
                        let key = showbook_model::ids::tail(&o.id);
                        if key.starts_with("MV_") {
                            continue;
                        }
                        if let Err(e) = d.set_output_label(key, &o.label) {
                            log.push(format!("output {key}: {e}"));
                        }
                    }
                    for p in &show.presets {
                        if let Some(n) = p.number {
                            if let Err(e) = d.set_memory_label(n, &p.label) {
                                log.push(format!("memory {n}: {e}"));
                            }
                        }
                    }
                    log.push("labels written".into());
                }
                if opts.vendor_file {
                    let blob = show.vendor.iter().rfind(|b| b.kind == "awc").ok_or("the show has no .awc to push")?;
                    let bytes = lib.vendor_bytes(&id, blob).map_err(err)?;
                    let fname = blob.note.split(" — ").next().unwrap_or("config.awc").to_string();
                    let status = d.upload_config(&fname, &bytes).map_err(err)?;
                    log.push(format!("uploaded {fname}: extract {status}"));
                    let modules: Vec<String> = if opts.modules.is_empty() { d.extracted_modules().unwrap_or_default() } else { opts.modules.clone() };
                    let mods: Vec<&str> = modules.iter().map(String::as_str).collect();
                    let st = d.apply_config(&mods, "MERGE_AND_REPLACE").map_err(err)?;
                    log.push(format!("apply {:?}: {st}", modules));
                }
                Ok(serde_json::json!({"ok": true, "log": log}))
            }
            Platform::BarcoEm | Platform::BarcoPds4k => {
                if opts.vendor_file {
                    return Err("restoring an Event Master backup onto a frame is done from the Event Master Toolset (Configuration → Restore); Showbook can export the archive for it (Show → Vendor files)".into());
                }
                Err("the Event Master API has no verb for writing configuration; use recall/take, or restore a backup with the toolset".into())
            }
            p => Err(format!("no live driver for {} yet", p.label())),
        }
    })
    .await
    .map_err(err)?
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RecallRequest {
    /// `preset`, `master` or `cue`.
    kind: String,
    /// Preset/master number as the show records it, or the frame's id for a cue.
    number: u32,
    /// Screen key for LivePremier screen memories (`S1`, `A2`); ignored elsewhere.
    #[serde(default)]
    screen: Option<String>,
    #[serde(default)]
    program: bool,
}

#[tauri::command]
async fn device_recall(dev: DeviceRef, req: RecallRequest) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || match dev.platform {
        Platform::BarcoEm | Platform::BarcoPds4k => {
            let d = showbook_em::live::Device::connect(&dev.host);
            match req.kind.as_str() {
                "cue" => d.recall_cue(req.number).map_err(err),
                _ => d.recall_preset(req.number, req.program).map_err(err),
            }
        }
        Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => {
            let d = showbook_aw::Device::connect(&dev.host, dev.awj_host.as_deref());
            match req.kind.as_str() {
                "master" => d.recall_master_memory(req.number, req.program).map_err(err),
                _ => {
                    let key = req.screen.clone().unwrap_or_else(|| "S1".into());
                    let n: u32 = key.trim_start_matches(['S', 'A']).parse().map_err(|_| format!("bad screen key {key}"))?;
                    if key.starts_with('A') {
                        d.recall_aux_memory(n, req.number, req.program).map_err(err)
                    } else {
                        d.recall_screen_memory(n, req.number, req.program).map_err(err)
                    }
                }
            }
        }
        p => Err(format!("no live driver for {} yet", p.label())),
    })
    .await
    .map_err(err)?
}

#[tauri::command]
async fn device_take(dev: DeviceRef, screens: Vec<String>) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || match dev.platform {
        Platform::BarcoEm | Platform::BarcoPds4k => {
            let d = showbook_em::live::Device::connect(&dev.host);
            let ids: Vec<u32> = screens.iter().filter_map(|s| showbook_model::ids::tail(s).parse().ok()).collect();
            d.take(&ids, &[]).map_err(err)
        }
        Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => {
            let d = showbook_aw::Device::connect(&dev.host, dev.awj_host.as_deref());
            let mut s = vec![];
            let mut a = vec![];
            for key in &screens {
                let k = showbook_model::ids::tail(key);
                if let Ok(n) = k.trim_start_matches(['S', 'A']).parse::<u32>() {
                    if k.starts_with('A') { a.push(n) } else { s.push(n) }
                }
            }
            d.take(&s, &a).map_err(err)
        }
        p => Err(format!("no live driver for {} yet", p.label())),
    })
    .await
    .map_err(err)?
}

// ---------------------------------------------------------------- conversion

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConvertResult {
    show: Show,
    notes: Vec<showbook_model::Note>,
    saved: bool,
}

#[tauri::command]
fn convert_show(state: State<AppState>, id: String, target: Platform, model: String, save: bool) -> CmdResult<ConvertResult> {
    let lib = library(&state)?;
    let show = lib.load(&id).map_err(err)?;
    let c = showbook_convert::convert(&show, target, &model);
    let mut out = c.show;
    if save {
        lib.save(&mut out, &format!("Converted from {} ({})", show.meta.name, show.system.model), author(&state).as_deref()).map_err(err)?;
    }
    Ok(ConvertResult { show: out, notes: c.notes, saved: save })
}

#[tauri::command]
fn capabilities(platform: Platform, model: String) -> CmdResult<showbook_convert::Capabilities> {
    Ok(showbook_convert::capabilities(platform, &model))
}

// ---------------------------------------------------------------- companion

#[tauri::command]
fn companion_export(state: State<AppState>, id: String, opts: showbook_companion::ExportOptions, path: String) -> CmdResult<Value> {
    let show = library(&state)?.load(&id).map_err(err)?;
    let doc = showbook_companion::export(&show, &opts)?;
    std::fs::write(&path, serde_json::to_string_pretty(&doc).map_err(err)?).map_err(err)?;
    Ok(serde_json::json!({"path": path, "type": doc["type"], "pages": doc.get("pages").map(|p| p.as_object().map(|o| o.len()).unwrap_or(1)).unwrap_or(1)}))
}

#[tauri::command]
fn companion_import(state: State<AppState>, id: String, path: String) -> CmdResult<showbook_companion::ImportReport> {
    let show = library(&state)?.load(&id).map_err(err)?;
    let bytes = std::fs::read(&path).map_err(err)?;
    showbook_companion::import(&bytes, &show)
}

// ---------------------------------------------------------------- sync

#[tauri::command]
async fn sync_run(app: AppHandle, direction: String) -> CmdResult<showbook_library::sync::SyncReport> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map_err(err)?.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use showbook_library::sync::*;
        let dir = match direction.as_str() {
            "push" => Direction::Push,
            "pull" => Direction::Pull,
            _ => Direction::Both,
        };
        let s = &settings.sync;
        let mut provider: Box<dyn SyncProvider> = match s.provider.as_str() {
            "folder" => Box::new(FolderProvider { root: PathBuf::from(&s.root) }),
            "dropbox" => Box::new(DropboxProvider { token: s.tokens.as_ref().map(|t| t.access_token.clone()).ok_or("not connected to Dropbox")?, root: if s.root.is_empty() { "/Showbook".into() } else { s.root.clone() } }),
            "google" => Box::new(GoogleDriveProvider::new(s.tokens.as_ref().map(|t| t.access_token.clone()).ok_or("not connected to Google Drive")?, if s.root.is_empty() { "Showbook".into() } else { s.root.clone() })),
            "onedrive" => Box::new(OneDriveProvider { token: s.tokens.as_ref().map(|t| t.access_token.clone()).ok_or("not connected to OneDrive")?, root: if s.root.is_empty() { "Showbook".into() } else { s.root.clone() } }),
            other => return Err(format!("unknown sync provider {other:?}")),
        };
        sync(Path::new(&settings.library_path), provider.as_mut(), dir).map_err(err)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
fn oauth_begin(state: State<AppState>, provider: String, client_id: String, client_secret: Option<String>) -> CmdResult<String> {
    let config = showbook_library::sync::oauth_config(&provider, &client_id, client_secret.as_deref()).ok_or("unknown provider")?;
    let pending = showbook_library::oauth::begin(config).map_err(err)?;
    let url = pending.url.clone();
    *state.pending_auth.lock().map_err(err)? = Some(pending);
    Ok(url)
}

#[tauri::command]
async fn oauth_finish(app: AppHandle) -> CmdResult<showbook_library::oauth::Tokens> {
    let state = app.state::<AppState>();
    let pending = state.pending_auth.lock().map_err(err)?.take().ok_or("no sign-in in progress")?;
    let tokens = tauri::async_runtime::spawn_blocking(move || pending.wait(std::time::Duration::from_secs(300)).map_err(err)).await.map_err(err)??;
    let mut settings = state.settings.lock().map_err(err)?;
    settings.sync.tokens = Some(tokens.clone());
    settings.save(&state.config_dir).map_err(err)?;
    Ok(tokens)
}

#[tauri::command]
fn oauth_refresh(state: State<AppState>) -> CmdResult<showbook_library::oauth::Tokens> {
    let mut settings = state.settings.lock().map_err(err)?;
    let s = settings.sync.clone();
    let config = showbook_library::sync::oauth_config(&s.provider, &s.client_id, s.client_secret.as_deref()).ok_or("unknown provider")?;
    let rt = s.tokens.and_then(|t| t.refresh_token).ok_or("no refresh token")?;
    let tokens = showbook_library::oauth::refresh(&config, &rt).map_err(err)?;
    settings.sync.tokens = Some(tokens.clone());
    settings.save(&state.config_dir).map_err(err)?;
    Ok(tokens)
}

// ---------------------------------------------------------------- run

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config_dir = app.path().app_config_dir().unwrap_or_else(|_| PathBuf::from("."));
            std::fs::create_dir_all(&config_dir).ok();
            let default_library = app.path().document_dir().map(|d| d.join("Showbook")).unwrap_or_else(|_| config_dir.join("library"));
            let settings = Settings::load(&config_dir, &default_library);
            app.manage(AppState { settings: Mutex::new(settings), config_dir, pending_auth: Mutex::new(None) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings_get,
            settings_set,
            app_info,
            library_list,
            show_load,
            show_save,
            show_new,
            show_history,
            show_snapshot,
            show_diff,
            show_restore,
            show_delete,
            show_duplicate,
            show_validate,
            import_path,
            export_show_json,
            vendor_export,
            write_file,
            write_text,
            read_text,
            device_probe,
            device_pull,
            device_push,
            device_recall,
            device_take,
            convert_show,
            capabilities,
            companion_export,
            companion_import,
            sync_run,
            oauth_begin,
            oauth_finish,
            oauth_refresh,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Showbook");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    fn temp_lib() -> (PathBuf, Library) {
        let root = std::env::temp_dir().join(format!("showbook-app-{}", uuid::Uuid::new_v4()));
        let lib = Library::open(&root).unwrap();
        (root, lib)
    }

    #[test]
    fn imports_every_shape_the_library_accepts() {
        let (root, lib) = temp_lib();
        let em = import_any(&lib, &fixtures().join("em/e3-sim-10.0.2"), Some("test")).unwrap();
        assert_eq!(em.kind, "em-store");
        assert_eq!(em.summary.screens, 6);
        let em2 = import_any(&lib, &fixtures().join("em/e2-sim-9.2-settings.xml"), None).unwrap();
        assert_eq!(em2.kind, "em-store");
        let aw = import_any(&lib, &fixtures().join("aw/livepremier-sim-6.2.73-store.json"), None).unwrap();
        assert_eq!(aw.kind, "aw-store");
        assert_eq!(aw.summary.inputs, 8);
        // A Showbook JSON round-trips as a new show.
        let out = root.join("copy.json");
        std::fs::write(&out, serde_json::to_string(&aw.show).unwrap()).unwrap();
        let again = import_any(&lib, &out, None).unwrap();
        assert_eq!(again.kind, "showbook");
        assert_ne!(again.show.id, aw.show.id);
        assert_eq!(lib.list().unwrap().len(), 4);
        // An .awc is kept as a vendor blob on a show that says what it is.
        let mut awc = b"PK\x03\x04".to_vec();
        awc.extend_from_slice(&[0u8; 20]);
        let comment = br#"{"DeviceItem":{"Dev":10,"Label":"","Timestamp":"2026_09_19_16_59_11","Version":"6.2.73"},"General":{"ExportStandard":"01.00.01"},"Modules":{"ModulesList":["GENERAL"]},"Platform":{"PlatformName":"NLC","VersionExport":"01.00.01"}}"#;
        awc.extend_from_slice(b"PK\x05\x06");
        awc.extend_from_slice(&[0u8; 16]);
        awc.extend_from_slice(&(comment.len() as u16).to_le_bytes());
        awc.extend_from_slice(comment);
        let f = root.join("AQL_CONFIG.awc");
        std::fs::write(&f, &awc).unwrap();
        let r = import_any(&lib, &f, None).unwrap();
        assert_eq!(r.kind, "awc");
        assert_eq!(r.show.vendor.len(), 1);
        assert_eq!(r.show.system.firmware, "6.2.73");
        std::fs::remove_dir_all(root).unwrap();
    }
}
