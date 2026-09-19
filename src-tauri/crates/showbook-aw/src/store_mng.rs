//! The Midra 4K / Alta 4K device store → [`Show`].
//!
//! Same Web RCS, same `GET /api/stores/device`, different tree — the family
//! `openrcs-awj` calls the `mng` dialect:
//!
//! | model | store |
//! |---|---|
//! | system | `system/pp/{dev,platformLabel,label}`, `system/version/pp/updater`, `system/serial` |
//! | inputs | `inputList/items/INPUT_n` where `status/pp/isAvailable`; plug type and label on `plugList/items/<plug>` |
//! | outputs | `outputList/items/{n,MTVW}`; role from `preconfig/control/outputList/items/n/pp/{mode,useOnScreen,useOnAux}` |
//! | screens | `screenList/items/n` where `preconfig/control/screenList/items/n/pp/enable`; layers from `presetList/items/{DOWN,UP}/liveLayerList`; canvas from `canvas/size` and `canvas/grid` |
//! | auxes | `auxiliaryScreenList/items/n` where enabled in preconfig |
//! | presets | `preset/bank/slotList/items/n` (screen memories), `preset/auxBank/slotList`, `preset/masterBank/slotList` |
//! | program/preview | `transition/screenList/items/n/status/pp/transition` names the buffer on air (`UP`/`DOWN`) |
//! | multiviewer | `multiviewer/widgetList/items/n`, on output `MTVW` |
//!
//! Read from the Midra 4K simulator 3.2.29 (`PULSE`); the Alta 4K simulator
//! 1.3.7 shares the tree.

use serde_json::{Map, Value};
use showbook_model::{
    ids, Extra, Format, Frame, Input, LayerDef, LayerKind, LayerState, MasterEntry, MasterPreset, Multiviewer, MvLayout,
    Note, NoteLevel, Output, OutputMap, OutputRole, Platform, Preset, PresetTarget, Rect, Screen, ScreenKind, Show, Size,
    Source, SourceKind, Still, Transition, Widget,
};

use crate::store::{b, connector_for, f, format_of, get, items, n, plug_kind, pp, s};
use crate::Result;

pub fn model_name(dev: &str) -> String {
    match dev {
        "PULSE" => "Pulse 4K".into(),
        "QUICKVU" => "QuickVu 4K".into(),
        "EIKOS" => "Eikos 4K".into(),
        "QUICKMATRIX" => "QuickMatrix 4K".into(),
        "ZENITH100" | "ZENITH_100" | "ZEN100" => "Zenith 100".into(),
        "ZENITH200" | "ZENITH_200" | "ZEN200" => "Zenith 200".into(),
        other => other.to_string(),
    }
}

fn source_for(show: &mut Show, key: &str) -> Option<String> {
    if key.is_empty() || key == "NONE" {
        return None;
    }
    let id = ids::source(key);
    if show.sources.iter().any(|s| s.id == id) {
        return Some(id);
    }
    let (kind, ref_id, label) = if let Some(nn) = key.strip_prefix("INPUT_") {
        (SourceKind::Input, Some(ids::input(key)), format!("Input {nn}"))
    } else if let Some(nn) = key.strip_prefix("FRAME_") {
        (SourceKind::Still, Some(ids::still(key)), format!("Frame {nn}"))
    } else if let Some(sc) = key.strip_prefix("SCREEN_PRGM_") {
        (SourceKind::Screen, Some(ids::screen(sc)), format!("Screen {sc} program"))
    } else if let Some(sc) = key.strip_prefix("SCREEN_PRW_") {
        (SourceKind::Screen, Some(ids::screen(sc)), format!("Screen {sc} preview"))
    } else if key.starts_with("TIMER_") {
        (SourceKind::Unknown, None, key.replace('_', " "))
    } else {
        (SourceKind::Unknown, None, key.to_string())
    };
    let resolved = match (&kind, &ref_id) {
        (SourceKind::Input, Some(r)) => show.inputs.iter().any(|i| &i.id == r),
        (SourceKind::Still, Some(r)) => show.stills.iter().any(|i| &i.id == r),
        (SourceKind::Screen, Some(r)) => show.screens.iter().any(|i| &i.id == r),
        _ => false,
    };
    let mut extra = Extra::new();
    extra.insert("key".into(), key.into());
    show.sources.push(Source { id: id.clone(), label, kind, ref_id: if resolved { ref_id } else { None }, aoi: None, format: None, extra });
    Some(id)
}

fn layer_state(lv: &Value, layer_id: &str, source_id: Option<String>) -> LayerState {
    let w = f(lv, "size/pp/sizeH").unwrap_or(0.0);
    let h = f(lv, "size/pp/sizeV").unwrap_or(0.0);
    let cx = f(lv, "position/pp/posH").unwrap_or(0.0);
    let cy = f(lv, "position/pp/posV").unwrap_or(0.0);
    // Midra positions are the layer's centre.
    let rect = Rect::new(cx - w / 2.0, cy - h / 2.0, w, h);
    let opacity = f(lv, "opacity/pp/opacity").map(|o| (o / 256.0).clamp(0.0, 1.0));
    let crop = {
        let t = f(lv, "crop/pp/top").unwrap_or(0.0);
        let bo = f(lv, "crop/pp/bottom").unwrap_or(0.0);
        let l = f(lv, "crop/pp/left").unwrap_or(0.0);
        let r = f(lv, "crop/pp/right").unwrap_or(0.0);
        if t > 0.0 || bo > 0.0 || l > 0.0 || r > 0.0 { Some(Rect::new(l, t, r, bo)) } else { None }
    };
    let border = {
        let styles = get(lv, "border/edge/pp/style").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
        let size = f(lv, "border/edge/pp/sizeH").unwrap_or(0.0);
        if styles > 0 && size > 0.0 {
            let c = |k: &str| n(lv, &format!("border/edge/color/pp/{k}")).unwrap_or(0).clamp(0, 255) as u8;
            Some(showbook_model::Border { width: size as u32, color: format!("#{:02x}{:02x}{:02x}", c("red"), c("green"), c("blue")) })
        } else {
            None
        }
    };
    let mut extra = Extra::new();
    if crop.is_some() {
        extra.insert("cropIsInsets".into(), true.into());
    }
    for (k, path) in [("transition", "transition/pp"), ("opening", "transition/opening/pp"), ("closing", "transition/closing/pp"), ("flying", "flying/pp"), ("speed", "speed/pp"), ("timing", "timing/pp"), ("effects", "effects/pp"), ("mask", "mask/pp"), ("state", "status/pp")] {
        if let Some(v) = get(lv, path) {
            extra.insert(k.into(), v.clone());
        }
    }
    let on = s(lv, "status/pp/state");
    LayerState { layer_id: layer_id.to_string(), source_id: source_id.clone(), visible: source_id.is_some() && on != "OFF", rect: Some(rect), crop, opacity, border, extra }
}

pub fn parse(d: &Value) -> Result<Show> {
    let dev = s(d, "system/pp/dev");
    let platform_label = s(d, "system/pp/platformLabel");
    let platform = if platform_label.to_lowercase().contains("alta") { Platform::AwAlta4k } else { Platform::AwMidra4k };
    let mut show = Show::new("", platform);
    show.system.model = model_name(&dev);
    show.system.firmware = s(d, "system/version/pp/updater");
    show.system.name = s(d, "system/pp/label");
    show.meta.name = if show.system.name.is_empty() { show.system.model.clone() } else { show.system.name.clone() };
    let mut sys_extra = Extra::new();
    sys_extra.insert("dev".into(), dev.clone().into());
    sys_extra.insert("platformLabel".into(), platform_label.into());
    sys_extra.insert("serial".into(), s(d, "system/serial/pp/serialNumber").into());
    sys_extra.insert("preconfigTemplate".into(), s(d, "preconfig/control/template/pp/select").into());
    show.system.extra = sys_extra;
    show.system.frames.push(Frame { id: ids::frame(1), label: show.system.model.clone(), model: show.system.model.clone(), address: None, slots: vec![] });
    let mut notes = vec![];

    // ---- inputs -------------------------------------------------------
    for (ik, iv) in items(d, "inputList") {
        if !b(iv, "status/pp/isAvailable") {
            continue;
        }
        let plug_key = { let p = s(iv, "control/pp/plug"); if p.is_empty() { "1".into() } else { p } };
        let plug = get(iv, &format!("plugList/items/{plug_key}")).cloned().unwrap_or(Value::Null);
        let ptype = s(&plug, "status/pp/type");
        let num = ik.trim_start_matches("INPUT_");
        let conn_id = connector_for(&mut show, "1", "INPUTS", &format!("IN_{num}"), plug_kind(&ptype), showbook_model::Direction::In, &ptype);
        let label = s(&plug, "control/pp/label");
        let mut extra = pp(iv, "status/pp");
        extra.insert("plugType".into(), ptype.into());
        let format = get(&plug, "status/signal/pp").and_then(|sig| {
            let w = n(sig, "formatWidth")? as u32;
            let h = n(sig, "formatHeight")? as u32;
            if w == 0 || h == 0 { return None; }
            let interlaced = s(sig, "scanType").starts_with("INTERL");
            let rate = f(sig, "fieldFrequency").map(|r| if r > 1000.0 { r / 1000.0 } else { r }).map(|r| if interlaced { r / 2.0 } else { r }).unwrap_or(0.0);
            Some(Format { width: w, height: h, rate, interlaced, name: get(sig, "formatName").and_then(Value::as_str).map(str::to_string) })
        });
        show.inputs.push(Input {
            id: ids::input(&ik),
            label: if label.is_empty() { format!("Input {num}") } else { label },
            connector_ids: vec![conn_id],
            format,
            enabled: true,
            capacity: None,
            hdcp: None,
            extra,
        });
    }

    // ---- outputs ------------------------------------------------------
    let pre_out: Map<String, Value> = items(d, "preconfig/control/outputList").into_iter().map(|(k, v)| (k, v.clone())).collect();
    for (ok, ov) in items(d, "outputList") {
        if !b(ov, "status/pp/isAvailable") {
            continue;
        }
        let pre = pre_out.get(&ok).cloned().unwrap_or(Value::Null);
        let mode = s(&pre, "pp/mode");
        let plug = items(ov, "plugList").first().map(|(_, v)| (*v).clone()).unwrap_or(Value::Null);
        let ptype = s(&plug, "status/pp/type");
        let is_mv = ok == "MTVW" || mode == "MULTIVIEWER";
        let conn_id = connector_for(&mut show, "1", "OUTPUTS", &format!("OUT_{ok}"), plug_kind(&ptype), showbook_model::Direction::Out, &ptype);
        let role = if is_mv {
            OutputRole::Multiviewer
        } else {
            match mode.as_str() {
                "SCREEN_FORMAT" | "SCREEN" => OutputRole::Screen,
                "AUX" | "AUX_FORMAT" | "AUXILIARY" => OutputRole::Aux,
                _ => OutputRole::Unassigned,
            }
        };
        let label = s(ov, "control/pp/label");
        let mut extra = pp(&pre, "pp");
        extra.insert("plugType".into(), ptype.into());
        extra.insert("formatMode".into(), s(ov, "status/pp/mode").into());
        let pattern = if !b(ov, "pattern/control/pp/inhibit") { Some(s(ov, "pattern/control/pp/type")).filter(|t| !t.is_empty() && t != "NO_PATTERN") } else { None };
        show.outputs.push(Output {
            id: ids::output(&ok),
            label: if label.is_empty() { if is_mv { "Multiviewer output".into() } else { format!("OUT {ok}") } } else { label },
            connector_ids: vec![conn_id],
            format: get(ov, "status/pp").and_then(format_of),
            role,
            test_pattern: pattern,
            extra,
        });
    }
    show.system.native_rate = show.outputs.iter().find_map(|o| o.format.as_ref().map(|f| f.rate)).filter(|r| *r > 0.0);

    // ---- stills (frames) ------------------------------------------------
    for (fk, fv) in items(d, "stillLibrary/bankList") {
        let label = s(fv, "control/pp/label");
        if !b(fv, "status/pp/isValid") && label.is_empty() {
            continue;
        }
        show.stills.push(Still { id: ids::still(format!("FRAME_{fk}")), label: if label.is_empty() { format!("Frame {fk}") } else { label }, size: None, file: None });
    }

    // ---- screens --------------------------------------------------------
    let enabled_screen = |k: &str| b(d, &format!("preconfig/control/screenList/items/{k}/pp/enable"));
    let layer_count = |k: &str| -> usize {
        items(d, "preconfig/control/resourcesList")
            .iter()
            .filter(|(_, r)| s(r, "pp/useOnScreen") == k && s(r, "pp/mode") != "DISABLE")
            .map(|(_, r)| if s(r, "pp/mode") == "SPLIT" { 2 } else { 1 })
            .sum()
    };
    for (sk, sv) in items(d, "screenList") {
        if !enabled_screen(&sk) {
            continue;
        }
        let label = s(sv, "control/pp/label");
        let size = Size { w: n(sv, "canvas/size/pp/sizeH").unwrap_or(0) as u32, h: n(sv, "canvas/size/pp/sizeV").unwrap_or(0) as u32 };
        let count = layer_count(&sk);
        let mut layers = vec![LayerDef { id: ids::layer("BACKGROUND"), label: "Background".into(), kind: LayerKind::Background, z: 0, capacity: None, extra: Extra::new() }];
        let live: Vec<(String, &Value)> = items(sv, "liveLayerList");
        let take = if count > 0 { count.min(live.len()) } else { live.len() };
        for (lk, _) in live.iter().take(take) {
            layers.push(LayerDef { id: ids::layer(lk), label: format!("Layer {lk}"), kind: LayerKind::Mixer, z: lk.parse().unwrap_or(0), capacity: None, extra: Extra::new() });
        }
        if b(d, &format!("preconfig/status/screenList/items/{sk}/pp/topLayerValidity")) {
            layers.push(LayerDef { id: ids::layer("TOP"), label: "Top layer".into(), kind: LayerKind::Key, z: 100, capacity: None, extra: Extra::new() });
        }
        let mut outputs = vec![];
        for (ok, pre) in pre_out.iter() {
            if s(pre, "pp/useOnScreen") != sk || s(pre, "pp/mode") == "DISABLE" || s(pre, "pp/mode") == "MULTIVIEWER" {
                continue;
            }
            let out_id = ids::output(ok);
            let Some(o) = show.outputs.iter().find(|o| o.id == out_id) else { continue };
            let fmt = o.format.clone().unwrap_or_default();
            let col = n(sv, &format!("canvas/grid/outputList/items/{ok}/status/pp/column")).unwrap_or(1).max(1) - 1;
            let row = n(sv, &format!("canvas/grid/outputList/items/{ok}/status/pp/row")).unwrap_or(1).max(1) - 1;
            let left = n(d, &format!("outputList/items/{ok}/canvas/status/pp/left")).unwrap_or(0);
            let top = n(d, &format!("outputList/items/{ok}/canvas/status/pp/top")).unwrap_or(0);
            let (x, y) = if left != 0 || top != 0 { (left as f64, top as f64) } else { ((col * fmt.width as i64) as f64, (row * fmt.height as i64) as f64) };
            outputs.push(OutputMap { output_id: out_id, rect: Rect::new(x, y, fmt.width as f64, fmt.height as f64) });
        }
        outputs.sort_by(|a, b| a.output_id.cmp(&b.output_id));
        let tr = get(d, &format!("transition/screenList/items/{sk}")).cloned().unwrap_or(Value::Null);
        let mut extra = pp(sv, "status/pp");
        extra.insert("canvasMode".into(), s(sv, "control/pp/mode").into());
        extra.insert("transitionState".into(), s(&tr, "status/pp/transition").into());
        show.screens.push(Screen {
            id: ids::screen(&sk),
            label: if label.is_empty() { format!("Screen {sk}") } else { label },
            kind: ScreenKind::Screen,
            size,
            outputs,
            layers,
            transition: f(&tr, "control/pp/takeTime").map(|t| Transition { duration_ms: Some((t * 100.0) as u32), kind: Some("mix".into()) }),
            extra,
        });
    }
    for (ak, av) in items(d, "auxiliaryScreenList") {
        if !b(d, &format!("preconfig/control/auxiliaryScreenList/items/{ak}/pp/enable")) {
            continue;
        }
        let label = s(av, "control/pp/label");
        let mut outputs = vec![];
        for (ok, pre) in pre_out.iter() {
            if s(pre, "pp/useOnAux") != ak || !s(pre, "pp/mode").starts_with("AUX") {
                continue;
            }
            let out_id = ids::output(ok);
            if let Some(o) = show.outputs.iter().find(|o| o.id == out_id) {
                let fmt = o.format.clone().unwrap_or_default();
                outputs.push(OutputMap { output_id: out_id, rect: Rect::new(0.0, 0.0, fmt.width as f64, fmt.height as f64) });
            }
        }
        let size = outputs.first().map(|o| Size { w: o.rect.w as u32, h: o.rect.h as u32 }).unwrap_or_default();
        let tr = get(d, &format!("transition/auxiliaryScreenList/items/{ak}")).cloned().unwrap_or(Value::Null);
        let mut extra = Extra::new();
        extra.insert("transitionState".into(), s(&tr, "status/pp/transition").into());
        show.screens.push(Screen {
            id: ids::aux(&ak),
            label: if label.is_empty() { format!("Aux {ak}") } else { label },
            kind: ScreenKind::Aux,
            size,
            outputs,
            layers: vec![LayerDef { id: ids::layer("BACKGROUND"), label: "Source".into(), kind: LayerKind::Background, z: 0, capacity: None, extra: Extra::new() }],
            transition: f(&tr, "control/pp/takeTime").map(|t| Transition { duration_ms: Some((t * 100.0) as u32), kind: Some("mix".into()) }),
            extra,
        });
    }

    // Program / preview buffers.
    let keys: Vec<(String, ScreenKind)> = show.screens.iter().map(|sc| (ids::tail(&sc.id).to_string(), sc.kind)).collect();
    for (key, kind) in keys {
        let (list, tlist) = if kind == ScreenKind::Screen { ("screenList", "transition/screenList") } else { ("auxiliaryScreenList", "transition/auxiliaryScreenList") };
        let sv = get(d, &format!("{list}/items/{key}")).cloned().unwrap_or(Value::Null);
        let state = s(d, &format!("{tlist}/items/{key}/status/pp/transition"));
        let transition = openrcs_awj::Transition::parse(&state).unwrap_or(openrcs_awj::Transition::AtDown);
        let screen_id = if kind == ScreenKind::Screen { ids::screen(&key) } else { ids::aux(&key) };
        let layer_ids: Vec<String> = show.screens.iter().find(|x| x.id == screen_id).map(|x| x.layers.iter().map(|l| l.id.clone()).collect()).unwrap_or_default();
        for (which, buf) in [("programState", openrcs_awj::Buffer::program(transition)), ("previewState", openrcs_awj::Buffer::preview(transition))] {
            let preset = get(&sv, &format!("presetList/items/{}", buf.key())).cloned().unwrap_or(Value::Null);
            if preset.is_null() {
                continue;
            }
            let mut target = PresetTarget { screen_id: screen_id.clone(), background: None, layers: vec![], transition: None };
            if kind == ScreenKind::Screen {
                let set = s(&preset, "background/source/pp/set");
                if !set.is_empty() && set != "NONE" {
                    // A background set names per-output content; record the set number.
                    target.background = source_for(&mut show, &format!("BACKGROUND_SET_{set}"));
                }
                for (lk, lv) in items(&preset, "liveLayerList") {
                    let lid = ids::layer(&lk);
                    if !layer_ids.contains(&lid) {
                        continue;
                    }
                    let src = source_for(&mut show, &s(lv, "source/pp/input"));
                    target.layers.push(layer_state(lv, &lid, src));
                }
                if layer_ids.contains(&ids::layer("TOP")) {
                    let frame = s(&preset, "top/source/pp/frame");
                    let src = source_for(&mut show, &if frame.is_empty() || frame == "NONE" { String::new() } else { format!("FRAME_{frame}") });
                    let top = get(&preset, "top").cloned().unwrap_or(Value::Null);
                    let mut st = layer_state(&top, &ids::layer("TOP"), src.clone());
                    st.rect = None;
                    st.visible = src.is_some();
                    target.layers.push(st);
                }
            } else {
                target.background = source_for(&mut show, &s(&preset, "background/source/pp/content"));
            }
            if let Some(sc) = show.screens.iter_mut().find(|x| x.id == screen_id) {
                sc.extra.insert(which.into(), serde_json::to_value(&target).unwrap_or_default());
                sc.extra.insert(if which == "programState" { "programBuffer" } else { "previewBuffer" }.into(), buf.key().into());
            }
        }
    }

    // ---- memories -------------------------------------------------------
    for (pk, pv) in items(d, "preset/bank/slotList") {
        if !b(pv, "status/pp/isValid") {
            continue;
        }
        let label = s(pv, "control/pp/label");
        let slot: u32 = pk.parse().unwrap_or(0);
        let mut extra = pp(pv, "status/pp");
        extra.remove("isValid");
        show.presets.push(Preset { id: ids::preset(slot), number: Some(slot), label: if label.is_empty() { format!("Memory {slot}") } else { label }, notes: String::new(), targets: vec![], extra });
    }
    for (pk, pv) in items(d, "preset/auxBank/slotList") {
        if !b(pv, "status/pp/isValid") {
            continue;
        }
        let label = s(pv, "control/pp/label");
        let slot: u32 = pk.parse().unwrap_or(0);
        let mut extra = pp(pv, "status/pp");
        extra.insert("bank".into(), "aux".into());
        show.presets.push(Preset { id: ids::preset(format!("aux{slot}")), number: Some(slot), label: if label.is_empty() { format!("Aux memory {slot}") } else { label }, notes: String::new(), targets: vec![], extra });
    }
    if !show.presets.is_empty() {
        notes.push(Note { level: NoteLevel::Info, path: "presets".into(), message: "memory labels and filters come from the bank; the layer values inside a memory are only on the device".into() });
    }
    for (mk, mv) in items(d, "preset/masterBank/slotList") {
        if !b(mv, "status/pp/isValid") {
            continue;
        }
        let label = s(mv, "control/pp/label");
        let slot: u32 = mk.parse().unwrap_or(0);
        let filters: Vec<String> = get(mv, "status/pp/screenFilter").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        let mut entries = vec![];
        for (sk, sv) in items(mv, "status/screenList") {
            if !filters.contains(&sk) {
                continue;
            }
            let bank_slot = n(sv, "pp/bankSlot").unwrap_or(0) as u32;
            let (sid, pid) = (ids::screen(&sk), ids::preset(bank_slot));
            if show.screens.iter().any(|x| x.id == sid) && show.presets.iter().any(|p| p.id == pid) {
                entries.push(MasterEntry { screen_id: sid, preset_id: pid });
            }
        }
        let aux_filters: Vec<String> = get(mv, "status/pp/auxFilter").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        for (ak, av) in items(mv, "status/auxiliaryScreenList") {
            if !aux_filters.contains(&ak) {
                continue;
            }
            let bank_slot = n(av, "pp/bankSlot").unwrap_or(0) as u32;
            let (sid, pid) = (ids::aux(&ak), ids::preset(format!("aux{bank_slot}")));
            if show.screens.iter().any(|x| x.id == sid) && show.presets.iter().any(|p| p.id == pid) {
                entries.push(MasterEntry { screen_id: sid, preset_id: pid });
            }
        }
        let mut extra = Extra::new();
        extra.insert("screenFilter".into(), filters.into());
        show.master_presets.push(MasterPreset { id: ids::master(slot), number: Some(slot), label: if label.is_empty() { format!("Master {slot}") } else { label }, entries, extra });
    }

    // ---- multiviewer ------------------------------------------------------
    if get(d, "multiviewer/widgetList").is_some() {
        let out_id = ids::output("MTVW");
        let size = show.outputs.iter().find(|o| o.id == out_id).and_then(|o| o.format.as_ref()).map(|f| Size { w: f.width, h: f.height }).unwrap_or(Size { w: 1920, h: 1080 });
        let mut widgets = vec![];
        for (wk, wv) in items(d, "multiviewer/widgetList") {
            if !b(wv, "control/pp/enable") {
                continue;
            }
            let rect = Rect::new(f(wv, "control/pp/posH").unwrap_or(0.0), f(wv, "control/pp/posV").unwrap_or(0.0), f(wv, "control/pp/sizeH").unwrap_or(0.0), f(wv, "control/pp/sizeV").unwrap_or(0.0));
            if rect.w <= 0.0 || rect.h <= 0.0 {
                continue;
            }
            let src = source_for(&mut show, &s(wv, "control/pp/source"));
            widgets.push(Widget { id: format!("w:{wk}"), rect, source_id: src, label: None, show_label: s(wv, "control/pp/displayOsd") != "NONE", tally: false, extra: Extra::new() });
        }
        let mut layouts = vec![MvLayout { id: ids::layout(1, 1), label: "Current layout".into(), size, widgets }];
        for (bk, bv) in items(d, "multiviewer/bankList") {
            if !b(bv, "status/pp/isValid") {
                continue;
            }
            let label = s(bv, "control/pp/label");
            let mut ws = vec![];
            for (wk, wv) in items(bv, "status/widgetList") {
                if !b(wv, "pp/isEnabled") {
                    continue;
                }
                let rect = Rect::new(f(wv, "pp/posH").unwrap_or(0.0), f(wv, "pp/posV").unwrap_or(0.0), f(wv, "pp/sizeH").unwrap_or(0.0), f(wv, "pp/sizeV").unwrap_or(0.0));
                let src = source_for(&mut show, &s(wv, "pp/source"));
                ws.push(Widget { id: format!("w:{wk}"), rect, source_id: src, label: None, show_label: s(wv, "pp/displayOsd") != "NONE", tally: false, extra: Extra::new() });
            }
            layouts.push(MvLayout { id: ids::layout(1, format!("m{bk}")), label: if label.is_empty() { format!("Memory {bk}") } else { label }, size, widgets: ws });
        }
        let has_out = show.outputs.iter().any(|o| o.id == out_id);
        show.multiviewers.push(Multiviewer { id: ids::multiviewer(1), label: "Multiviewer".into(), output_ids: if has_out { vec![out_id] } else { vec![] }, layouts, active_layout: Some(ids::layout(1, 1)), extra: Extra::new() });
    }

    show.notes = notes;
    Ok(show)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midra_simulator_store() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/aw/midra4k-sim-3.2.29-store.json");
        let v: Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        let show = crate::store::parse(&v).unwrap();
        assert_eq!(show.platform, Platform::AwMidra4k);
        assert_eq!(show.system.model, "Pulse 4K");
        assert_eq!(show.system.firmware, "3.2.29");
        assert_eq!(show.inputs.len(), 10);
        assert_eq!(show.screens.iter().filter(|s| s.kind == ScreenKind::Screen).count(), 2);
        let s1 = show.screen("scr:1").unwrap();
        assert_eq!(s1.outputs.len(), 1);
        assert_eq!(s1.outputs[0].output_id, "out:1");
        // One SPLIT resource on screen 1 → 2 mixing layers; plus background and top.
        assert_eq!(s1.layers.iter().filter(|l| l.kind == LayerKind::Mixer).count(), 2);
        assert!(s1.layers.iter().any(|l| l.id == "layer:TOP"));
        let pgm: PresetTarget = serde_json::from_value(s1.extra["programState"].clone()).unwrap();
        assert_eq!(pgm.layers[0].source_id.as_deref(), Some("src:INPUT_3"));
        assert_eq!(s1.extra["programBuffer"], "UP");
        // Three screen memories and one aux memory are valid in the fixture.
        assert_eq!(show.presets.len(), 4);
        assert_eq!(show.presets[0].label, "Fixture one");
        assert_eq!(show.master_presets.len(), 1);
        // Master 1 names screens 1-4 and auxes 1-4, of which screens 1, 2 exist (no aux is enabled).
        assert_eq!(show.master_presets[0].entries.len(), 2);
        assert_eq!(show.multiviewers.len(), 1);
        assert!(show.outputs.iter().any(|o| o.role == OutputRole::Multiviewer));
        assert!(show.validate().is_empty(), "{:?}", show.validate());
    }
}
