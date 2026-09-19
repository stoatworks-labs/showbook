//! A running frame, over JSON-RPC.
//!
//! What the API gives that a show needs: `getFrameSettings`, `listInputs`,
//! `listOutputs`, `listSources`, `listDestinations`, `listContent` (one
//! destination's layers and windows), `listPresets`, `listDestinationsForPreset`,
//! `listCues`, `listMvrPreset`. What it cannot give: the layer content saved
//! *inside* a preset (only the destinations it touches), the multiviewer
//! window geometry, and the cue steps — those come from the frame's backup
//! archive, see [`crate::import_path`].
//!
//! Writes offered here are the documented control verbs: preset recall,
//! TAKE, cue recall, and a source change on a layer. The frame's backup is
//! fetched over its HTTP interface (`/api/backup` on Encore3) where present.
//!
//! Reply shapes follow Barco's API guide; field names are taken as the guide
//! spells them and read leniently (missing fields are skipped, not fatal).

use serde_json::{json, Value};
use showbook_model::{
    ids, Cue, Extra, Format, Input, LayerDef, LayerKind, LayerState, Note, NoteLevel, Output, OutputRole, Platform, Preset,
    PresetTarget, Rect, Screen, ScreenKind, Show, Size, Source, SourceKind,
};

use crate::jsonrpc::Client;
use crate::settings::{capacity_name, parse_format_name};
use crate::Result;

pub struct Device {
    pub client: Client,
    pub host: String,
}

impl Device {
    pub fn connect(host: &str) -> Device {
        Device { client: Client::new(host), host: host.to_string() }
    }

    /// Is anyone there? `powerStatus` is the cheapest documented call.
    pub fn ping(&self) -> Result<Value> {
        self.client.call("powerStatus", json!({}))
    }

    /// Capture the frame's configuration as a show.
    pub fn read_show(&self) -> Result<Show> {
        let mut show = Show::new("", Platform::BarcoEm);
        let mut notes = vec![];

        if let Ok(fs) = self.client.call("getFrameSettings", json!({})) {
            read_frame_settings(&fs, &mut show);
        }
        match self.client.call("listInputs", json!({})) {
            Ok(v) => read_inputs(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "inputs".into(), message: format!("listInputs: {e}") }),
        }
        match self.client.call("listOutputs", json!({})) {
            Ok(v) => read_outputs(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "outputs".into(), message: format!("listOutputs: {e}") }),
        }
        match self.client.call("listSources", json!({"type": 0})) {
            Ok(v) => read_sources(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "sources".into(), message: format!("listSources: {e}") }),
        }
        match self.client.call("listDestinations", json!({"type": 0})) {
            Ok(v) => read_destinations(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "screens".into(), message: format!("listDestinations: {e}") }),
        }
        let screen_ids: Vec<(String, u32)> = show
            .screens
            .iter()
            .filter(|s| s.kind == ScreenKind::Screen)
            .filter_map(|s| ids::tail(&s.id).parse::<u32>().ok().map(|n| (s.id.clone(), n)))
            .collect();
        for (sid, n) in screen_ids {
            match self.client.call("listContent", json!({"id": n})) {
                Ok(v) => read_content(&v, &sid, &mut show),
                Err(e) => notes.push(Note { level: NoteLevel::Info, path: sid.clone(), message: format!("listContent: {e}") }),
            }
        }
        match self.client.call("listPresets", json!({"ScreenDest": -1, "AuxDest": -1})) {
            Ok(v) => read_presets(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "presets".into(), message: format!("listPresets: {e}") }),
        }
        let preset_ids: Vec<(String, u32)> =
            show.presets.iter().filter_map(|p| ids::tail(&p.id).parse::<u32>().ok().map(|n| (p.id.clone(), n))).collect();
        for (pid, n) in preset_ids {
            if let Ok(v) = self.client.call("listDestinationsForPreset", json!({"id": n})) {
                read_preset_destinations(&v, &pid, &mut show);
            }
        }
        match self.client.call("listCues", json!({})) {
            Ok(v) => read_cues(&v, &mut show),
            Err(e) => notes.push(Note { level: NoteLevel::Info, path: "cues".into(), message: format!("listCues: {e}") }),
        }
        notes.push(Note {
            level: NoteLevel::Info,
            path: "presets".into(),
            message: "a live capture lists presets and the destinations they touch; the layer content inside each preset is only in the frame's backup archive".into(),
        });
        show.notes = notes;
        if show.meta.name.is_empty() {
            show.meta.name = if show.system.name.is_empty() { self.host.clone() } else { show.system.name.clone() };
        }
        show.meta.source = Some(showbook_model::SourceInfo {
            kind: "device".into(),
            origin: self.host.clone(),
            at: showbook_model::now(),
            firmware: Some(show.system.firmware.clone()),
        });
        Ok(show)
    }

    /// Recall a preset to preview (`program: false`) or straight to program.
    pub fn recall_preset(&self, preset_no: u32, program: bool) -> Result<()> {
        self.client.call("activatePreset", json!({"id": preset_no, "type": if program { 1 } else { 0 }}))?;
        Ok(())
    }

    /// Recall a preset by its name (the guide accepts `presetName`).
    pub fn recall_preset_named(&self, name: &str, program: bool) -> Result<()> {
        self.client.call("activatePreset", json!({"presetName": name, "type": if program { 1 } else { 0 }}))?;
        Ok(())
    }

    /// TAKE: preview to program on every screen destination, or the ones named.
    pub fn take(&self, screen_ids: &[u32], aux_ids: &[u32]) -> Result<()> {
        let mut params = json!({});
        if !screen_ids.is_empty() {
            params["screenDestination"] = json!(screen_ids.iter().map(|i| json!({"id": i})).collect::<Vec<_>>());
        }
        if !aux_ids.is_empty() {
            params["auxDestination"] = json!(aux_ids.iter().map(|i| json!({"id": i})).collect::<Vec<_>>());
        }
        self.client.call("allTrans", params)?;
        Ok(())
    }

    pub fn recall_cue(&self, cue_no: u32) -> Result<()> {
        self.client.call("activateCue", json!({"id": cue_no, "type": 0}))?;
        Ok(())
    }

    /// Save the current program/preview of the given destinations as a preset.
    pub fn save_preset(&self, preset_no: Option<u32>, name: &str, screen_ids: &[u32], aux_ids: &[u32]) -> Result<Value> {
        let mut params = json!({"presetName": name});
        if let Some(n) = preset_no {
            params["id"] = json!(n);
        }
        if !screen_ids.is_empty() {
            params["ScreenDestination"] = json!(screen_ids.iter().map(|i| json!({"id": i})).collect::<Vec<_>>());
        }
        if !aux_ids.is_empty() {
            params["AuxDestination"] = json!(aux_ids.iter().map(|i| json!({"id": i})).collect::<Vec<_>>());
        }
        self.client.call("savePreset", params)
    }

    /// Put a source on a layer of a screen destination's preview.
    pub fn change_layer_source(&self, screen_no: u32, layer_no: u32, source_no: u32) -> Result<()> {
        self.client.call(
            "changeContent",
            json!({"id": screen_no, "Layers": [{"id": layer_no, "LastSrcIdx": source_no, "PvwMode": 1}]}),
        )?;
        Ok(())
    }

    /// Fetch the frame's backup archive over HTTP (Encore3: `/api/backup`).
    /// Returns the archive bytes and the name the frame gave it.
    pub fn download_backup(&self) -> Result<(String, Vec<u8>)> {
        let base = self.host.split(':').next().unwrap_or(&self.host).to_string();
        let candidates = [format!("http://{base}/api/backup"), format!("http://{base}:9999/api/backup")];
        let mut last = String::new();
        for url in candidates {
            let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(600)).build();
            match agent.get(&url).call() {
                Ok(resp) => {
                    let name = resp
                        .header("Content-Disposition")
                        .and_then(|cd| cd.split("filename=").nth(1))
                        .map(|f| f.trim_matches('"').trim().to_string())
                        .unwrap_or_else(|| "E3Backup.tar.gz".into());
                    let mut buf = vec![];
                    resp.into_reader().read_to_end(&mut buf)?;
                    if buf.is_empty() {
                        last = format!("{url}: empty reply");
                        continue;
                    }
                    return Ok((name, buf));
                }
                Err(e) => last = format!("{url}: {e}"),
            }
        }
        Err(crate::Error::Device(format!("no backup endpoint answered ({last})")))
    }
}

fn arr<'a>(v: &'a Value, keys: &[&str]) -> Vec<&'a Value> {
    if let Some(a) = v.as_array() {
        return a.iter().collect();
    }
    for k in keys {
        if let Some(a) = v.get(k).and_then(Value::as_array) {
            return a.iter().collect();
        }
    }
    vec![]
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).map(str::to_string).unwrap_or_default()
}
fn n(v: &Value, k: &str) -> Option<i64> {
    v.get(k).and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|f| f as i64)))
}
fn idx(v: &Value, k: &str) -> Option<u32> {
    n(v, k).filter(|x| *x >= 0).map(|x| x as u32)
}
fn extra_of(v: &Value) -> Extra {
    v.as_object().map(|o| o.iter().filter(|(_, x)| !x.is_object() && !x.is_array()).map(|(k, x)| (k.clone(), x.clone())).collect()).unwrap_or_default()
}

fn read_frame_settings(v: &Value, show: &mut Show) {
    let sys = v.get("System").unwrap_or(v);
    show.system.name = s(sys, "Name");
    show.system.firmware = s(sys, "Version");
    show.system.native_rate = sys.get("NativeRate").and_then(Value::as_f64);
    show.meta.name = show.system.name.clone();
    if let Some(fc) = v.get("FrameCollection").or_else(|| sys.get("FrameCollection")) {
        for (i, f) in arr(fc, &["Frame"]).iter().enumerate() {
            let ftype = n(f, "FrameType").unwrap_or(-1);
            show.system.frames.push(showbook_model::Frame {
                id: ids::frame(i + 1),
                label: { let x = s(f, "Name"); if x.is_empty() { format!("Frame {}", i + 1) } else { x } },
                model: crate::cards::frame_model(ftype).to_string(),
                address: f.get("Enet").and_then(|e| e.get("IP")).and_then(Value::as_str).map(str::to_string),
                slots: vec![],
            });
            if show.system.model.is_empty() {
                show.system.model = crate::cards::frame_model(ftype).to_string();
            }
        }
    }
    show.system.extra = extra_of(sys);
}

fn read_inputs(v: &Value, show: &mut Show) {
    for i in arr(v, &["Inputs", "InputCfg"]) {
        let Some(id) = idx(i, "id") else { continue };
        show.inputs.push(Input {
            id: ids::input(id),
            label: { let x = s(i, "Name"); if x.is_empty() { format!("Input {}", id + 1) } else { x } },
            connector_ids: vec![],
            format: i.get("FormatName").and_then(Value::as_str).and_then(parse_format_name).or_else(|| {
                let w = idx(i, "HSize")?;
                let h = idx(i, "VSize")?;
                Some(Format { width: w, height: h, rate: i.get("Rate").and_then(Value::as_f64).unwrap_or(0.0), interlaced: false, name: None })
            }),
            enabled: true,
            capacity: n(i, "Capacity").map(capacity_name),
            hdcp: n(i, "HdcpMode").map(|x| x != 0),
            extra: extra_of(i),
        });
    }
}

fn read_outputs(v: &Value, show: &mut Show) {
    for o in arr(v, &["Outputs", "OutputCfg"]) {
        let Some(id) = idx(o, "id") else { continue };
        show.outputs.push(Output {
            id: ids::output(id),
            label: { let x = s(o, "Name"); if x.is_empty() { format!("Output {}", id + 1) } else { x } },
            connector_ids: vec![],
            format: o.get("FormatName").and_then(Value::as_str).and_then(parse_format_name).or_else(|| {
                let w = idx(o, "HSize")?;
                let h = idx(o, "VSize")?;
                Some(Format { width: w, height: h, rate: 0.0, interlaced: false, name: None })
            }),
            role: OutputRole::Unassigned,
            test_pattern: None,
            extra: extra_of(o),
        });
    }
}

fn read_sources(v: &Value, show: &mut Show) {
    for src in arr(v, &["Sources"]) {
        let Some(id) = idx(src, "id") else { continue };
        let kind_code = n(src, "SrcType").unwrap_or(-1);
        let (kind, ref_id) = if let Some(i) = idx(src, "InputCfgIndex") {
            (SourceKind::Input, Some(ids::input(i)))
        } else if let Some(i) = idx(src, "StillIndex") {
            (SourceKind::Still, Some(ids::still(i)))
        } else if let Some(i) = idx(src, "DestIndex") {
            if kind_code == 3 { (SourceKind::Aux, Some(ids::aux(i))) } else { (SourceKind::Screen, Some(ids::screen(i))) }
        } else {
            (match kind_code { 0 => SourceKind::Input, 1 => SourceKind::Background, 2 => SourceKind::Screen, 3 => SourceKind::Aux, 4 => SourceKind::Still, _ => SourceKind::Unknown }, None)
        };
        let format = match (idx(src, "HSize"), idx(src, "VSize")) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some(Format { width: w, height: h, rate: 0.0, interlaced: false, name: None }),
            _ => None,
        };
        show.sources.push(Source {
            id: ids::source(id),
            label: { let x = s(src, "Name"); if x.is_empty() { format!("Source {}", id + 1) } else { x } },
            kind,
            ref_id,
            aoi: None,
            format,
            extra: extra_of(src),
        });
    }
    // Sources that point at inputs the frame did not list: keep the reference honest.
    let known: Vec<String> = show.inputs.iter().map(|i| i.id.clone()).collect();
    for src in &mut show.sources {
        if src.kind == SourceKind::Input {
            if let Some(r) = &src.ref_id {
                if !known.contains(r) {
                    show.inputs.push(Input {
                        id: r.clone(),
                        label: src.label.clone(),
                        connector_ids: vec![],
                        format: src.format.clone(),
                        enabled: true,
                        capacity: None,
                        hdcp: None,
                        extra: Extra::new(),
                    });
                }
            }
        }
    }
}

fn read_destinations(v: &Value, show: &mut Show) {
    for d in arr(v.get("ScreenDestination").unwrap_or(&Value::Null), &[]) {
        let Some(id) = idx(d, "id") else { continue };
        let size = Size { w: idx(d, "HSize").unwrap_or(0), h: idx(d, "VSize").unwrap_or(0) };
        let mut layers = vec![LayerDef { id: ids::layer("bg"), label: "Background".into(), kind: LayerKind::Background, z: 0, capacity: None, extra: Extra::new() }];
        for (k, l) in arr(d.get("Layers").unwrap_or(&Value::Null), &[]).iter().enumerate() {
            let ln = idx(l, "id").unwrap_or(k as u32) + 1;
            layers.push(LayerDef {
                id: ids::layer(ln),
                label: { let x = s(l, "Name"); if x.is_empty() { format!("Layer {ln}") } else { x } },
                kind: LayerKind::Mixer,
                z: ln,
                capacity: n(l, "Capacity").map(capacity_name),
                extra: extra_of(l),
            });
        }
        let outputs = arr(d.get("DestOutMapCol").unwrap_or(&Value::Null), &["DestOutMap"])
            .iter()
            .filter_map(|m| {
                let oi = idx(m, "OutCfgIndex")?;
                Some(showbook_model::OutputMap {
                    output_id: ids::output(oi),
                    rect: Rect::new(n(m, "HPos").unwrap_or(0) as f64, n(m, "VPos").unwrap_or(0) as f64, idx(m, "HSize").unwrap_or(size.w) as f64, idx(m, "VSize").unwrap_or(size.h) as f64),
                })
            })
            .collect::<Vec<_>>();
        for om in &outputs {
            if let Some(o) = show.outputs.iter_mut().find(|o| o.id == om.output_id) {
                o.role = OutputRole::Screen;
            }
        }
        show.screens.push(Screen {
            id: ids::screen(id),
            label: { let x = s(d, "Name"); if x.is_empty() { format!("Screen {}", id + 1) } else { x } },
            kind: ScreenKind::Screen,
            size,
            outputs,
            layers,
            transition: None,
            extra: extra_of(d),
        });
    }
    for d in arr(v.get("AuxDestination").unwrap_or(&Value::Null), &[]) {
        let Some(id) = idx(d, "id") else { continue };
        show.screens.push(Screen {
            id: ids::aux(id),
            label: { let x = s(d, "Name"); if x.is_empty() { format!("Aux {}", id + 1) } else { x } },
            kind: ScreenKind::Aux,
            size: Size { w: idx(d, "HSize").unwrap_or(0), h: idx(d, "VSize").unwrap_or(0) },
            outputs: vec![],
            layers: vec![LayerDef { id: ids::layer("aux"), label: "Aux".into(), kind: LayerKind::Mixer, z: 0, capacity: None, extra: Extra::new() }],
            transition: None,
            extra: extra_of(d),
        });
    }
}

fn read_content(v: &Value, screen_id: &str, show: &mut Show) {
    let Some(screen) = show.screens.iter_mut().find(|s| &s.id == screen_id) else { return };
    let mut states = vec![];
    let mut background = None;
    for bg in arr(v.get("BGLyr").unwrap_or(&Value::Null), &[]) {
        if let Some(i) = idx(bg, "LastBGSourceIndex") {
            background = Some(ids::source(i));
        }
    }
    for (k, l) in arr(v.get("Layers").unwrap_or(&Value::Null), &[]).iter().enumerate() {
        let ln = idx(l, "id").unwrap_or(k as u32) + 1;
        if !screen.layers.iter().any(|d| d.id == ids::layer(ln)) {
            screen.layers.push(LayerDef {
                id: ids::layer(ln),
                label: { let x = s(l, "Name"); if x.is_empty() { format!("Layer {ln}") } else { x } },
                kind: LayerKind::Mixer,
                z: idx(l, "PgmZOrder").unwrap_or(ln),
                capacity: n(l, "Capacity").map(capacity_name),
                extra: Extra::new(),
            });
        }
        let win = l.get("Window").or_else(|| l.get("OWIN"));
        let rect = win.map(|w| Rect::new(n(w, "HPos").unwrap_or(0) as f64, n(w, "VPos").unwrap_or(0) as f64, n(w, "HSize").unwrap_or(0) as f64, n(w, "VSize").unwrap_or(0) as f64));
        states.push(LayerState {
            layer_id: ids::layer(ln),
            source_id: idx(l, "LastSrcIdx").map(ids::source),
            visible: n(l, "PgmMode").unwrap_or(0) != 0,
            rect,
            crop: None,
            opacity: None,
            border: None,
            extra: extra_of(l),
        });
    }
    screen.extra.insert(
        "programState".into(),
        serde_json::to_value(PresetTarget { screen_id: screen_id.to_string(), background, layers: states, transition: None }).unwrap_or_default(),
    );
}

fn read_presets(v: &Value, show: &mut Show) {
    for p in arr(v, &["Presets"]) {
        let Some(id) = idx(p, "id") else { continue };
        show.presets.push(Preset {
            id: ids::preset(id),
            number: idx(p, "presetSno").or(Some(id + 1)),
            label: { let x = s(p, "Name"); if x.is_empty() { format!("Preset {}", id + 1) } else { x } },
            notes: String::new(),
            targets: vec![],
            extra: extra_of(p),
        });
    }
}

fn read_preset_destinations(v: &Value, preset_id: &str, show: &mut Show) {
    let Some(p) = show.presets.iter_mut().find(|p| p.id == preset_id) else { return };
    for d in arr(v.get("ScreenDest").unwrap_or(&Value::Null), &[]) {
        if let Some(i) = idx(d, "id") {
            p.targets.push(PresetTarget { screen_id: ids::screen(i), background: None, layers: vec![], transition: None });
        }
    }
    for d in arr(v.get("AuxDest").unwrap_or(&Value::Null), &[]) {
        if let Some(i) = idx(d, "id") {
            p.targets.push(PresetTarget { screen_id: ids::aux(i), background: None, layers: vec![], transition: None });
        }
    }
}

fn read_cues(v: &Value, show: &mut Show) {
    for c in arr(v, &["Cues"]) {
        let Some(id) = idx(c, "id") else { continue };
        show.cues.push(Cue {
            id: ids::cue(id),
            number: Some(id + 1),
            label: { let x = s(c, "Name"); if x.is_empty() { format!("Cue {}", id + 1) } else { x } },
            steps: vec![],
            extra: extra_of(c),
        });
    }
}
