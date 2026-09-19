//! Bitfocus Companion pages from a show, and back.
//!
//! The export is Companion's own `.companionconfig` JSON (`version: 12`, the
//! format Companion 5.0 writes; older Companions upgrade it on import). One
//! `button-layered` control per preset, master preset or cue, plus a TAKE
//! per screen and a TAKE ALL, on 8×4 pages. Each button carries one action
//! for the platform's Companion module:
//!
//! | platform | module | action | options |
//! |---|---|---|---|
//! | Event Master | `barco-eventmaster` | `recall_preset` | `id` (preset number), `mode` `0` preview / `1` program |
//! | Event Master | `barco-eventmaster` | `trans_all` | — |
//! | Event Master | `barco-eventmaster` | `play_cue` | `cueNumber` |
//! | LivePremier / Midra 4K / Alta 4K | `analogway-awj` | `deviceScreenMemory` | `screens` `["S1"]`, `preset` `pvw`, `memory` slot |
//! | LivePremier | `analogway-awj` | `deviceMasterMemory` | `preset` `pvw`, `memory` slot |
//! | LivePremier | `analogway-awj` | `deviceTakeScreen` | `screens` `["S1"]` or `["all"]` |
//!
//! Option ids and choice ids were read from the module bundles installed
//! with Companion 5.0.5 (barco-eventmaster 4.4.4, analogway-awj 2.5.0). The
//! control shape was taken from a control Companion 5.0.5 saved itself.
//! A generated page has been validated against those shapes but has not yet
//! been imported into a running Companion from this code.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use showbook_model::{ids, Platform, ScreenKind, Show};

pub const FILE_VERSION: u64 = 12;
pub const COLUMNS: u32 = 8;
pub const ROWS: u32 = 4;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    /// Connection label in Companion, e.g. `e3` or `aquilon`.
    pub connection_label: String,
    /// Device address written into the connection config.
    pub host: String,
    /// Recall to program instead of preview.
    pub to_program: bool,
    pub include_presets: bool,
    pub include_masters: bool,
    pub include_cues: bool,
    pub include_takes: bool,
    pub page_name: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            connection_label: "switcher".into(),
            host: "192.168.0.175".into(),
            to_program: false,
            include_presets: true,
            include_masters: true,
            include_cues: true,
            include_takes: true,
            page_name: String::new(),
        }
    }
}

fn module_for(platform: Platform) -> Option<(&'static str, &'static str)> {
    match platform {
        Platform::BarcoEm | Platform::BarcoPds4k => Some(("barco-eventmaster", "4.4.4")),
        Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k => Some(("analogway-awj", "2.5.0")),
        Platform::AwLiveCore => Some(("analogway-livecore", "2.0.0")),
        _ => None,
    }
}

fn nanoid(seed: &str) -> String {
    use sha2::Digest;
    let h = sha2::Sha256::digest(seed.as_bytes());
    // Companion ids are 21 URL-safe characters; a hash prefix serves.
    let alphabet: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
    h.iter().take(21).map(|b| alphabet[(*b as usize) % alphabet.len()] as char).collect()
}

fn v(value: Value) -> Value {
    json!({"value": value, "isExpression": false})
}

fn text_layer(text: &str) -> Value {
    json!({
        "id": "text0", "name": "", "usage": "auto", "type": "text",
        "enabled": v(json!(true)), "opacity": v(json!(100)),
        "x": v(json!(0)), "y": v(json!(0)), "width": v(json!(100)), "height": v(json!(100)), "rotation": v(json!(0)),
        "text": v(json!(text)), "color": v(json!(16777215)),
        "halign": v(json!("center")), "valign": v(json!("center")),
        "fontsize": v(json!("auto")), "fontsizeAllowShrink": v(json!(true)), "font": v(json!("companion-sans")),
        "outlineColor": v(json!(4278190080u64))
    })
}

fn button(text: &str, bg: u32, connection_id: &str, definition: &str, options: Map<String, Value>, seed: &str) -> Value {
    let opts: Map<String, Value> = options.into_iter().map(|(k, val)| (k, v(val))).collect();
    json!({
        "type": "button-layered",
        "style": {"layers": [
            {"id": "canvas", "name": "Canvas", "usage": "auto", "type": "canvas", "decoration": v(json!("default")), "showStatusIcons": v(json!("default"))},
            {"id": "box0", "name": "Background", "usage": "auto", "type": "box", "enabled": v(json!(true)), "opacity": v(json!(100)),
             "x": v(json!(0)), "y": v(json!(0)), "width": v(json!(100)), "height": v(json!(100)), "rotation": v(json!(0)),
             "color": v(json!(bg)), "borderWidth": v(json!(0)), "borderColor": v(json!(0)), "borderPosition": v(json!("inside"))},
            text_layer(text)
        ]},
        "options": {"stepProgression": "auto", "stepExpression": "", "rotaryActions": false, "canModifyStyleInApis": false, "notes": ""},
        "feedbacks": [],
        "steps": {"0": {"action_sets": {"down": [
            {"id": nanoid(seed), "definitionId": definition, "connectionId": connection_id, "options": opts, "upgradeIndex": -1, "type": "action"}
        ], "up": []}, "options": {"runWhileHeld": []}}},
        "localVariables": []
    })
}

const BG_TAKE: u32 = 0x8b0000;
const BG_PRESET: u32 = 0x1f3a5f;
const BG_MASTER: u32 = 0x4b2a6f;
const BG_CUE: u32 = 0x1e5f2a;

struct Plan {
    text: String,
    bg: u32,
    definition: &'static str,
    options: Map<String, Value>,
    seed: String,
}

fn plan(show: &Show, opts: &ExportOptions) -> Vec<Plan> {
    let is_em = matches!(show.platform, Platform::BarcoEm | Platform::BarcoPds4k);
    let mode = if opts.to_program { "1" } else { "0" };
    let preset_side = if opts.to_program { "pgm" } else { "pvw" };
    let mut out = vec![];
    if opts.include_takes {
        if is_em {
            out.push(Plan { text: "TAKE\nALL".into(), bg: BG_TAKE, definition: "trans_all", options: Map::new(), seed: "take-all".into() });
        } else {
            let mut o = Map::new();
            o.insert("screens".into(), json!(["all"]));
            out.push(Plan { text: "TAKE\nALL".into(), bg: BG_TAKE, definition: "deviceTakeScreen", options: o, seed: "take-all".into() });
            for sc in show.screens.iter().filter(|s| s.kind == ScreenKind::Screen) {
                let key = ids::tail(&sc.id).to_string();
                let mut o = Map::new();
                o.insert("screens".into(), json!([key]));
                out.push(Plan { text: format!("TAKE\n{}", sc.label), bg: BG_TAKE, definition: "deviceTakeScreen", options: o, seed: format!("take-{}", sc.id) });
            }
        }
    }
    if opts.include_presets {
        for p in &show.presets {
            let number = p.number.unwrap_or(0);
            if is_em {
                let mut o = Map::new();
                // The module's preset dropdown is keyed by the frame's preset id (0-based).
                o.insert("id".into(), json!(ids::tail(&p.id)));
                o.insert("mode".into(), json!(mode));
                out.push(Plan { text: format!("{}\n{}", number, p.label), bg: BG_PRESET, definition: "recall_preset", options: o, seed: format!("preset-{}", p.id) });
            } else {
                let screens: Vec<String> = p.targets.iter().map(|t| ids::tail(&t.screen_id).to_string()).collect();
                let screens = if screens.is_empty() { vec!["sel".to_string()] } else { screens };
                let mut o = Map::new();
                o.insert("screens".into(), json!(screens));
                o.insert("preset".into(), json!(preset_side));
                o.insert("memory".into(), json!(number.to_string()));
                o.insert("selectScreens".into(), json!(true));
                out.push(Plan { text: format!("SM{}\n{}", number, p.label), bg: BG_PRESET, definition: "deviceScreenMemory", options: o, seed: format!("preset-{}", p.id) });
            }
        }
    }
    if opts.include_masters && !is_em {
        for m in &show.master_presets {
            let number = m.number.unwrap_or(0);
            let mut o = Map::new();
            o.insert("preset".into(), json!(preset_side));
            o.insert("memory".into(), json!(number.to_string()));
            o.insert("selectScreens".into(), json!(true));
            out.push(Plan { text: format!("MM{}\n{}", number, m.label), bg: BG_MASTER, definition: "deviceMasterMemory", options: o, seed: format!("master-{}", m.id) });
        }
    }
    if opts.include_cues && is_em {
        for c in &show.cues {
            let mut o = Map::new();
            o.insert("cueNumber".into(), json!(ids::tail(&c.id)));
            out.push(Plan { text: format!("CUE\n{}", c.label), bg: BG_CUE, definition: "play_cue", options: o, seed: format!("cue-{}", c.id) });
        }
    }
    out
}

/// Build a Companion export. One page → `type: "page"`; more → `type: "full"`
/// with numbered pages (the import wizard lets the user place them).
pub fn export(show: &Show, opts: &ExportOptions) -> Result<Value, String> {
    let (module, module_version) = module_for(show.platform).ok_or_else(|| format!("no Companion module mapping for {}", show.platform.label()))?;
    let connection_id = nanoid(&format!("conn-{}-{}", module, opts.connection_label));
    let plans = plan(show, opts);
    let per_page = (COLUMNS * ROWS) as usize;
    let page_count = plans.len().div_ceil(per_page).max(1);
    let mut pages: Map<String, Value> = Map::new();
    for page in 0..page_count {
        let mut controls: Map<String, Value> = Map::new();
        for (i, p) in plans.iter().skip(page * per_page).take(per_page).enumerate() {
            let row = (i as u32) / COLUMNS;
            let col = (i as u32) % COLUMNS;
            let entry = controls.entry(row.to_string()).or_insert_with(|| json!({}));
            entry[col.to_string()] = button(&p.text, p.bg, &connection_id, p.definition, p.options.clone(), &format!("{}-{}", show.id, p.seed));
        }
        let name = if opts.page_name.is_empty() { show.meta.name.clone() } else { opts.page_name.clone() };
        let name = if page_count > 1 { format!("{name} {}", page + 1) } else { name };
        pages.insert(
            (page + 1).to_string(),
            json!({
                "id": nanoid(&format!("page-{}-{}", show.id, page)),
                "name": name,
                "controls": controls,
                "gridSize": {"minColumn": 0, "maxColumn": COLUMNS - 1, "minRow": 0, "maxRow": ROWS - 1}
            }),
        );
    }
    let config = match module {
        "barco-eventmaster" => json!({"host": opts.host, "port": 9999}),
        "analogway-awj" => json!({"deviceaddr": opts.host, "sync": true, "color_dark": 0, "color_bright": 16777215, "color_highlight": 0}),
        _ => json!({"host": opts.host}),
    };
    let instances = json!({
        connection_id: {
            "label": opts.connection_label,
            "moduleId": module,
            "moduleVersionId": module_version,
            "updatePolicy": "stable",
            "sortOrder": 0,
            "isFirstInit": false,
            "lastUpgradeIndex": -1,
            "enabled": true,
            "config": config,
            "secrets": {}
        }
    });
    let build = format!("showbook {}", env!("CARGO_PKG_VERSION"));
    if page_count == 1 {
        let page = pages.remove("1").unwrap();
        Ok(json!({"version": FILE_VERSION, "type": "page", "companionBuild": build, "page": page, "instances": instances, "connectionCollections": [], "oldPageNumber": 1}))
    } else {
        Ok(json!({"version": FILE_VERSION, "type": "full", "companionBuild": build, "pages": pages, "instances": instances, "connectionCollections": []}))
    }
}

// ---------------------------------------------------------------- import

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedButton {
    pub page: u32,
    pub row: u32,
    pub column: u32,
    pub text: String,
    pub module: String,
    pub definition: String,
    pub options: Map<String, Value>,
    /// The show entity this button recalls, when it could be matched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub file_version: u64,
    pub kind: String,
    pub pages: Vec<String>,
    pub connections: Vec<String>,
    pub buttons: Vec<ImportedButton>,
    pub unmatched: usize,
}

/// Read a `.companionconfig` (JSON, YAML-as-JSON, or gzip of either) and
/// match its recall buttons against the show.
pub fn import(bytes: &[u8], show: &Show) -> Result<ImportReport, String> {
    let text = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut dec = flate2::read::GzDecoder::new(bytes);
        let mut s = String::new();
        std::io::Read::read_to_string(&mut dec, &mut s).map_err(|e| e.to_string())?;
        s
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("not JSON ({e}); Companion also writes YAML, which Showbook does not read yet"))?;
    let kind = doc.get("type").and_then(Value::as_str).unwrap_or("").to_string();
    let mut report = ImportReport { file_version: doc.get("version").and_then(Value::as_u64).unwrap_or(0), kind: kind.clone(), ..Default::default() };
    let instances: Map<String, Value> = doc.get("instances").and_then(Value::as_object).cloned().unwrap_or_default();
    let module_of = |conn: &str| instances.get(conn).and_then(|i| i.get("moduleId")).and_then(Value::as_str).unwrap_or("").to_string();
    report.connections = instances.values().filter_map(|i| i.get("label").and_then(Value::as_str).map(str::to_string)).collect();
    let mut pages: Vec<(u32, Value)> = vec![];
    match kind.as_str() {
        "page" => pages.push((doc.get("oldPageNumber").and_then(Value::as_u64).unwrap_or(1) as u32, doc.get("page").cloned().unwrap_or(Value::Null))),
        "full" => {
            if let Some(ps) = doc.get("pages").and_then(Value::as_object) {
                let mut keys: Vec<(u32, &String)> = ps.keys().map(|k| (k.parse().unwrap_or(0), k)).collect();
                keys.sort();
                for (n, k) in keys {
                    pages.push((n, ps[k].clone()));
                }
            }
        }
        other => return Err(format!("unsupported export type {other:?}")),
    }
    for (pn, page) in pages {
        report.pages.push(page.get("name").and_then(Value::as_str).unwrap_or("").to_string());
        let Some(rows) = page.get("controls").and_then(Value::as_object) else { continue };
        for (r, cols) in rows {
            let Some(cols) = cols.as_object() else { continue };
            for (c, control) in cols {
                let text = button_text(control);
                let actions = control
                    .pointer("/steps/0/action_sets/down")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for a in actions {
                    let definition = a.get("definitionId").and_then(Value::as_str).unwrap_or("").to_string();
                    let conn = a.get("connectionId").and_then(Value::as_str).unwrap_or("");
                    let module = module_of(conn);
                    let options: Map<String, Value> = a
                        .get("options")
                        .and_then(Value::as_object)
                        .map(|o| o.iter().map(|(k, x)| (k.clone(), x.get("value").cloned().unwrap_or(x.clone()))).collect())
                        .unwrap_or_default();
                    let (matched, problem) = match_action(show, &module, &definition, &options);
                    if problem.is_some() {
                        report.unmatched += 1;
                    }
                    report.buttons.push(ImportedButton {
                        page: pn,
                        row: r.parse().unwrap_or(0),
                        column: c.parse().unwrap_or(0),
                        text: text.clone(),
                        module,
                        definition,
                        options,
                        matched,
                        problem,
                    });
                }
            }
        }
    }
    Ok(report)
}

fn button_text(control: &Value) -> String {
    if let Some(layers) = control.pointer("/style/layers").and_then(Value::as_array) {
        for l in layers {
            if l.get("type").and_then(Value::as_str) == Some("text") {
                if let Some(t) = l.pointer("/text/value").and_then(Value::as_str) {
                    return t.to_string();
                }
            }
        }
    }
    control.pointer("/style/text").and_then(Value::as_str).unwrap_or("").to_string()
}

fn match_action(show: &Show, module: &str, definition: &str, options: &Map<String, Value>) -> (Option<String>, Option<String>) {
    let s = |k: &str| options.get(k).map(|x| match x {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    });
    match (module, definition) {
        ("barco-eventmaster", "recall_preset") => {
            let id = s("id").unwrap_or_default();
            let want = ids::preset(&id);
            match show.presets.iter().find(|p| p.id == want || p.label == id) {
                Some(p) => (Some(p.id.clone()), None),
                None => (None, Some(format!("preset {id} is not in the show"))),
            }
        }
        ("barco-eventmaster", "play_cue") => {
            let id = s("cueNumber").unwrap_or_default();
            match show.cues.iter().find(|c| c.id == ids::cue(&id)) {
                Some(c) => (Some(c.id.clone()), None),
                None => (None, Some(format!("cue {id} is not in the show"))),
            }
        }
        ("analogway-awj", "deviceScreenMemory") | ("analogway-awj", "deviceAuxMemory") => {
            let slot = s("memory").unwrap_or_default();
            match show.presets.iter().find(|p| p.number.map(|n| n.to_string()) == Some(slot.clone())) {
                Some(p) => (Some(p.id.clone()), None),
                None => (None, Some(format!("memory {slot} is not in the show"))),
            }
        }
        ("analogway-awj", "deviceMasterMemory") => {
            let slot = s("memory").unwrap_or_default();
            match show.master_presets.iter().find(|p| p.number.map(|n| n.to_string()) == Some(slot.clone())) {
                Some(p) => (Some(p.id.clone()), None),
                None => (None, Some(format!("master memory {slot} is not in the show"))),
            }
        }
        (_, "trans_all") | (_, "deviceTakeScreen") | (_, "cut_all") | (_, "deviceCutScreen") => (None, None),
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use showbook_model::*;

    fn show(platform: Platform) -> Show {
        let mut s = Show::new("Gala", platform);
        let sid = if platform == Platform::BarcoEm { ids::screen(0) } else { ids::screen("S1") };
        s.screens.push(Screen { id: sid.clone(), label: "Main".into(), kind: ScreenKind::Screen, size: Size { w: 1920, h: 1080 }, outputs: vec![], layers: vec![], transition: None, extra: Extra::new() });
        for n in 1..=3u32 {
            let id = if platform == Platform::BarcoEm { ids::preset(n - 1) } else { ids::preset(n) };
            s.presets.push(Preset { id, number: Some(n), label: format!("Look {n}"), notes: String::new(), targets: vec![PresetTarget { screen_id: sid.clone(), background: None, layers: vec![], transition: None }], extra: Extra::new() });
        }
        s.cues.push(Cue { id: ids::cue(0), number: Some(1), label: "Open".into(), steps: vec![], extra: Extra::new() });
        s.master_presets.push(MasterPreset { id: ids::master(1), number: Some(1), label: "All".into(), entries: vec![], extra: Extra::new() });
        s
    }

    #[test]
    fn em_page_round_trips() {
        let s = show(Platform::BarcoEm);
        let doc = export(&s, &ExportOptions::default()).unwrap();
        assert_eq!(doc["type"], "page");
        assert_eq!(doc["version"], 12);
        // TAKE ALL + 3 presets + 1 cue = 5 buttons on row 0.
        let row0 = doc["page"]["controls"]["0"].as_object().unwrap();
        assert_eq!(row0.len(), 5);
        assert_eq!(row0["1"]["steps"]["0"]["action_sets"]["down"][0]["definitionId"], "recall_preset");
        assert_eq!(row0["1"]["steps"]["0"]["action_sets"]["down"][0]["options"]["id"]["value"], "0");
        let bytes = serde_json::to_vec(&doc).unwrap();
        let rep = import(&bytes, &s).unwrap();
        assert_eq!(rep.kind, "page");
        assert_eq!(rep.buttons.len(), 5);
        assert_eq!(rep.unmatched, 0);
        assert_eq!(rep.buttons[1].matched.as_deref(), Some("pre:0"));
        assert_eq!(rep.buttons[4].matched.as_deref(), Some("cue:0"));
    }

    #[test]
    fn aw_page_uses_awj_actions() {
        let s = show(Platform::AwLivePremier);
        let doc = export(&s, &ExportOptions { to_program: true, ..Default::default() }).unwrap();
        let row0 = doc["page"]["controls"]["0"].as_object().unwrap();
        // TAKE ALL, TAKE Main, 3 memories, 1 master = 6.
        assert_eq!(row0.len(), 6);
        let mem = &row0["2"]["steps"]["0"]["action_sets"]["down"][0];
        assert_eq!(mem["definitionId"], "deviceScreenMemory");
        assert_eq!(mem["options"]["screens"]["value"], json!(["S1"]));
        assert_eq!(mem["options"]["preset"]["value"], "pgm");
        assert_eq!(mem["options"]["memory"]["value"], "1");
        let master = &row0["5"]["steps"]["0"]["action_sets"]["down"][0];
        assert_eq!(master["definitionId"], "deviceMasterMemory");
        // A mismatched show reports the problem.
        let other = show(Platform::AwLivePremier);
        let mut other = other;
        other.presets.truncate(1);
        let rep = import(&serde_json::to_vec(&doc).unwrap(), &other).unwrap();
        assert_eq!(rep.unmatched, 2);
    }

    #[test]
    fn many_buttons_make_a_full_export() {
        let mut s = show(Platform::BarcoEm);
        for n in 4..=40u32 {
            s.presets.push(Preset { id: ids::preset(n - 1), number: Some(n), label: format!("Look {n}"), notes: String::new(), targets: vec![], extra: Extra::new() });
        }
        let doc = export(&s, &ExportOptions::default()).unwrap();
        assert_eq!(doc["type"], "full");
        assert_eq!(doc["pages"].as_object().unwrap().len(), 2);
    }
}
