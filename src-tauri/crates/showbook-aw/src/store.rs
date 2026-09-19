//! The device store (`GET /api/stores/device`) → [`Show`].
//!
//! Paths below are store paths (`device/…`); the AWJ spelling of any of them
//! is [`crate::awj::awj_path`].
//!
//! | model | store |
//! |---|---|
//! | system | `system/deviceList/items/1/{pp,version,serial,hardware}` |
//! | inputs | `inputList/items/IN_n` where `status/pp/isAvailable` |
//! | outputs | `outputList/items/n` where `status/pp/isAvailable`; role from `preconfig/resources/current/outputList/items/n/status/pp/{mode,usedInScreenAux}` |
//! | screens | `screenList/items/Sn` where `screenAuxGroupList/items/Sn/status/pp/isUsed`; layers from `layerList`, canvas from `control/canvas`, program from `presetList/items/<letter>` |
//! | auxes | `auxiliaryList/items/An` where `status/pp/mode != DISABLED` |
//! | presets | `presetBank/bankList/items/n` where `status/pp/isValid` — metadata only |
//! | master presets | `masterPresetBank/bankList/items/n` where `status/pp/isValid` |
//! | multiviewers | `monitoringList/items/n` where `status/pp/isAvailable`, widgets from `layout/widgetList` |
//! | stills | `stillList/items/n` with a label or a mode other than `NONE` |
//! | mixer allocation | `preconfig/resources/current/status/mapping/deviceList/items/d/vpuMixerList` → `system.extra.mixers` |

use serde_json::{Map, Value};
use showbook_model::{
    ids, Connector, ConnectorKind, Direction, Extra, Format, Frame, Input, LayerDef, LayerKind, LayerState, MasterEntry,
    MasterPreset, Multiviewer, MvLayout, Note, NoteLevel, Output, OutputMap, OutputRole, Platform, Preset, PresetTarget,
    Rect, Screen, ScreenKind, Show, Size, Slot, Source, SourceKind, Still, Transition, Widget,
};

use crate::{Error, Result};

pub(crate) fn get<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = v;
    for seg in path.split('/') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}
pub(crate) fn s(v: &Value, path: &str) -> String {
    get(v, path).and_then(Value::as_str).unwrap_or("").to_string()
}
pub(crate) fn b(v: &Value, path: &str) -> bool {
    get(v, path).and_then(Value::as_bool).unwrap_or(false)
}
pub(crate) fn f(v: &Value, path: &str) -> Option<f64> {
    get(v, path).and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
}
pub(crate) fn n(v: &Value, path: &str) -> Option<i64> {
    get(v, path).and_then(|x| x.as_i64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
}
/// `xList/items` in key order (`itemKeys` when present).
pub(crate) fn items<'a>(v: &'a Value, path: &str) -> Vec<(String, &'a Value)> {
    let Some(list) = get(v, path) else { return vec![] };
    let Some(map) = list.get("items").and_then(Value::as_object) else { return vec![] };
    if let Some(keys) = list.get("itemKeys").and_then(Value::as_array) {
        keys.iter().filter_map(|k| k.as_str()).filter_map(|k| map.get(k).map(|v| (k.to_string(), v))).collect()
    } else {
        let mut v: Vec<(String, &Value)> = map.iter().map(|(k, v)| (k.clone(), v)).collect();
        v.sort_by_key(|(k, _)| natural(k));
        v
    }
}
fn natural(k: &str) -> (String, u64) {
    let digits: String = k.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    (k[..k.len() - digits.len()].to_string(), digits.parse().unwrap_or(0))
}
pub(crate) fn pp(v: &Value, path: &str) -> Extra {
    get(v, path).and_then(Value::as_object).map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default()
}

pub fn model_name(dev: &str) -> String {
    match dev {
        "NLC_C" => "Aquilon C".into(),
        "NLC_CMAX" => "Aquilon C max".into(),
        "NLC_CPLUS" => "Aquilon C+".into(),
        "NLC_CMINI" => "Aquilon C mini".into(),
        "NLC_RS1" => "Aquilon RS1".into(),
        "NLC_RS2" => "Aquilon RS2".into(),
        "NLC_RS3" => "Aquilon RS3".into(),
        "NLC_RS4" => "Aquilon RS4".into(),
        "NLC_RS6" => "Aquilon RS6".into(),
        "NLC_RSALPHA" => "Aquilon RS alpha".into(),
        "NLC_DBG" => "LivePremier (debug chassis)".into(),
        other if other.starts_with("NLC_") => format!("LivePremier {}", &other[4..]),
        other => other.to_string(),
    }
}

pub(crate) fn plug_kind(t: &str) -> ConnectorKind {
    match t {
        "HDMI" => ConnectorKind::Hdmi,
        "SDI" | "SDI_12G" | "SDI_3G" => ConnectorKind::Sdi,
        "DP" | "DISPLAYPORT" | "DISPLAY_PORT" | "DP_1_2" | "DP_1_4" => ConnectorKind::DisplayPort,
        "DVI" => ConnectorKind::Dvi,
        "SFP" | "OPTICAL" | "FIBER" | "FIBRE" => ConnectorKind::Fibre,
        "IP" | "ST2110" | "NDI" | "SDVOE" => ConnectorKind::Ip,
        _ => ConnectorKind::Other,
    }
}

/// `HDTV_1080P` + 60000 mHz → a format.
pub(crate) fn format_of(status: &Value) -> Option<Format> {
    let w = n(status, "sizeH")? as u32;
    let h = n(status, "sizeV")? as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let rate = f(status, "rate").map(|r| if r > 1000.0 { r / 1000.0 } else { r }).unwrap_or(0.0);
    Some(Format {
        width: w,
        height: h,
        rate,
        interlaced: b(status, "isFormatInterlaced"),
        name: get(status, "format").and_then(Value::as_str).map(str::to_string),
    })
}

fn signal_format(sig: &Value) -> Option<Format> {
    let w = n(sig, "formatWidth")? as u32;
    let h = n(sig, "formatHeight")? as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let interlaced = s(sig, "scanType").starts_with("INTERL");
    // fieldFrequency is in mHz; for an interlaced signal it counts fields.
    let rate = ["fieldFrequency", "rate", "refreshRate", "frameRate"]
        .iter()
        .find_map(|k| f(sig, k))
        .map(|r| if r > 1000.0 { r / 1000.0 } else { r })
        .map(|r| if interlaced { r / 2.0 } else { r })
        .unwrap_or(0.0);
    Some(Format {
        width: w,
        height: h,
        rate,
        interlaced,
        name: get(sig, "formatName").and_then(Value::as_str).map(str::to_string),
    })
}

/// A store `inputNum`-style reference (`IN_3`, `STILL_1`, `PROGRAM_S2`,
/// `NONE`) → the show's source id, creating the source if the show lacks it.
fn source_for(show: &mut Show, key: &str) -> Option<String> {
    if key.is_empty() || key == "NONE" {
        return None;
    }
    let id = ids::source(key);
    if show.sources.iter().any(|s| s.id == id) {
        return Some(id);
    }
    let (kind, ref_id, label) = if let Some(nn) = key.strip_prefix("IN_") {
        (SourceKind::Input, Some(ids::input(key)), format!("IN {nn}"))
    } else if let Some(nn) = key.strip_prefix("STILL_") {
        (SourceKind::Still, Some(ids::still(key)), format!("Still {nn}"))
    } else if let Some(sc) = key.strip_prefix("PROGRAM_") {
        let r = if sc.starts_with('A') { ids::aux(sc) } else { ids::screen(sc) };
        (SourceKind::Screen, Some(r), format!("{sc} program"))
    } else if let Some(sc) = key.strip_prefix("PREVIEW_") {
        let r = if sc.starts_with('A') { ids::aux(sc) } else { ids::screen(sc) };
        (SourceKind::Screen, Some(r), format!("{sc} preview"))
    } else if key.starts_with("TIMER_") {
        (SourceKind::Unknown, None, key.replace('_', " "))
    } else if key.starts_with("BACKGROUND") || key.starts_with("BG_") {
        (SourceKind::Background, None, key.replace('_', " "))
    } else {
        (SourceKind::Unknown, None, key.to_string())
    };
    // Only keep references that resolve; otherwise leave the reference off
    // and keep the key in extra so validation stays honest.
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

pub fn parse(root: &Value) -> Result<Show> {
    let d = root.get("device").unwrap_or(root);
    if d.get("screenList").is_none() && d.get("inputList").is_none() {
        return Err(Error::NotAStore("no screenList/inputList at the root".into()));
    }
    // Midra 4K and Alta 4K carry the same wire protocol but a different tree
    // (no deviceList; `preset`, `transition` and `multiviewer` at the root).
    if get(d, "system/deviceList").is_none() && get(d, "system/pp/platformLabel").is_some() {
        return crate::store_mng::parse(d);
    }
    let mut show = Show::new("", Platform::AwLivePremier);
    let mut notes = vec![];

    // ---- system ------------------------------------------------------
    let devices = items(d, "system/deviceList");
    let dev = devices.first().map(|(_, v)| *v).cloned().unwrap_or(Value::Null);
    let dev_code = s(&dev, "pp/dev");
    show.system.model = model_name(&dev_code);
    show.system.firmware = s(&dev, "version/pp/updater");
    show.system.name = s(&dev, "pp/label");
    show.meta.name = if show.system.name.is_empty() { show.system.model.clone() } else { show.system.name.clone() };
    let mut sys_extra = Extra::new();
    sys_extra.insert("dev".into(), dev_code.clone().into());
    sys_extra.insert("serial".into(), s(&dev, "serial/pp/serialNumber").into());
    sys_extra.insert("isSimulated".into(), b(&dev, "pp/isSimulated").into());
    sys_extra.insert("chassis".into(), s(&dev, "hardware/device/pp/chassis").into());
    // Mixer allocation: who holds which VPU mixer.
    let mut mixers: Vec<Value> = vec![];
    for (dk, dv) in items(d, "preconfig/resources/current/status/mapping/deviceList") {
        for (mk, mv) in items(dv, "vpuMixerList") {
            let p = get(mv, "pp").cloned().unwrap_or(Value::Null);
            mixers.push(serde_json::json!({
                "device": dk, "mixer": mk,
                "enabled": b(&p, "isEnabled"), "available": b(&p, "isAvailable"),
                "usedInScreen": s(&p, "usedInScreen"), "usedInLayer": s(&p, "usedInLayer"),
                "capability": s(&p, "capability"), "slice": n(&p, "slice"),
            }));
        }
    }
    if !mixers.is_empty() {
        sys_extra.insert("mixers".into(), Value::Array(mixers));
    }
    show.system.extra = sys_extra;
    // The system rate is what the outputs framelock to; the first output's
    // master rate says it ("60HZ", "59_94HZ", "50HZ").
    show.system.native_rate = items(d, "outputList")
        .iter()
        .find(|(_, o)| b(o, "status/pp/isAvailable"))
        .map(|(_, o)| s(o, "format/control/pp/masterRate"))
        .and_then(|r| r.trim_end_matches("HZ").replace('_', ".").parse::<f64>().ok());

    // Frames: one per device that has hardware, slots = cards. A lone
    // device still lists four link slots; the empty ones carry no cards.
    for (di, (dk, dv)) in devices.iter().enumerate() {
        let plugged = items(dv, "hardware/cardList").iter().any(|(_, c)| b(c, "pp/isPlugged"));
        if !plugged && di > 0 {
            continue;
        }
        let mut frame = Frame {
            id: ids::frame(dk),
            label: { let l = s(dv, "pp/label"); if l.is_empty() { format!("Device {}", di + 1) } else { l } },
            model: model_name(&s(dv, "pp/dev")),
            address: None,
            slots: vec![],
        };
        for (ci, (ck, cv)) in items(dv, "hardware/cardList").iter().enumerate() {
            if !b(cv, "pp/isPlugged") {
                continue;
            }
            let febe: Vec<String> = items(cv, "febeList").iter().map(|(_, fe)| s(fe, "pp/type")).filter(|t| !t.is_empty() && t != "UNKNOWN").collect();
            let card = match s(cv, "pp/type").as_str() {
                "IN" => "Input card".to_string(),
                "OUT" => "Output card".to_string(),
                "PROC" => "VPU processing card".to_string(),
                "MOC" => "Multiviewer / OSD card".to_string(),
                "FRAME" => "Frame store".to_string(),
                "AUDIO" => "Audio card".to_string(),
                other => other.to_string(),
            };
            let card = if febe.is_empty() { card } else { format!("{card} ({})", febe.join(", ")) };
            frame.slots.push(Slot { index: ci as u32 + 1, card, label: ck.clone(), connectors: vec![] });
        }
        show.system.frames.push(frame);
    }
    if show.system.frames.is_empty() {
        show.system.frames.push(Frame { id: ids::frame(1), label: "Device 1".into(), model: show.system.model.clone(), address: None, slots: vec![] });
    }

    // ---- inputs ------------------------------------------------------
    for (ik, iv) in items(d, "inputList") {
        if !b(iv, "status/pp/isAvailable") {
            continue;
        }
        let plug_key = s(iv, "control/pp/plug");
        let plug = get(iv, &format!("plugList/items/{plug_key}")).cloned().unwrap_or(Value::Null);
        let ptype = s(&plug, "status/pp/type");
        let card = s(iv, "mapping/pp/card");
        let physical = s(iv, "mapping/pp/physical");
        let devk = { let x = s(iv, "mapping/pp/device"); if x.is_empty() { "1".to_string() } else { x } };
        // Name the plug after the input (IN 5), which is how the rear panel is
        // printed; the store's own physical index (a card-relative position) is kept in extra.
        let conn_id = connector_for(&mut show, &devk, &card, &ik, plug_kind(&ptype), Direction::In, &ptype);
        let mut extra = pp(iv, "status/pp");
        extra.insert("plug".into(), plug_key.into());
        extra.insert("plugType".into(), ptype.clone().into());
        extra.insert("signalType".into(), s(&plug, "status/pp/signalType").into());
        extra.insert("card".into(), card.into());
        extra.insert("physical".into(), physical.clone().into());
        let label = s(iv, "control/pp/label");
        let num = ik.trim_start_matches("IN_");
        show.inputs.push(Input {
            id: ids::input(&ik),
            label: if label.is_empty() { format!("IN {num}") } else { label },
            connector_ids: vec![conn_id],
            format: get(&plug, "status/signal/pp").and_then(signal_format),
            enabled: b(iv, "status/pp/isEnabled"),
            capacity: Some(s(iv, "status/pp/capability")).filter(|c| !c.is_empty()),
            hdcp: Some(s(iv, "status/pp/hdcp") == "ACTIVE"),
            extra,
        });
    }

    // ---- stills ------------------------------------------------------
    for (sk, sv) in items(d, "stillList") {
        let label = s(sv, "control/pp/label");
        let mode = s(sv, "control/pp/mode");
        if label.is_empty() && (mode.is_empty() || mode == "NONE") {
            continue;
        }
        show.stills.push(Still {
            id: ids::still(format!("STILL_{sk}")),
            label: if label.is_empty() { format!("Still {sk}") } else { label },
            size: None,
            file: None,
        });
    }

    // ---- outputs -----------------------------------------------------
    let pre_outputs: Map<String, Value> = items(d, "preconfig/resources/current/outputList").into_iter().map(|(k, v)| (k, v.clone())).collect();
    for (ok, ov) in items(d, "outputList") {
        if !b(ov, "status/pp/isAvailable") {
            continue;
        }
        let pre = pre_outputs.get(&ok).cloned().unwrap_or(Value::Null);
        let mode = s(&pre, "status/pp/mode");
        let used_in = s(&pre, "status/pp/usedInScreenAux");
        let plug = items(ov, "plugList").first().map(|(_, v)| (*v).clone()).unwrap_or(Value::Null);
        let ptype = s(&plug, "status/pp/type");
        let card = s(ov, "mapping/pp/card");
        let physical = s(ov, "mapping/pp/physical");
        let devk = { let x = s(ov, "mapping/pp/device"); if x.is_empty() { "1".to_string() } else { x } };
        let conn_id = connector_for(&mut show, &devk, &card, &physical, plug_kind(&ptype), Direction::Out, &ptype);
        let role = match mode.as_str() {
            "SCREEN" => OutputRole::Screen,
            "AUX" | "AUXILIARY" => OutputRole::Aux,
            _ => OutputRole::Unassigned,
        };
        let label = s(ov, "control/pp/label");
        let mut extra = pp(&pre, "status/pp");
        extra.insert("plugType".into(), ptype.into());
        extra.insert("formatMode".into(), s(ov, "status/pp/mode").into());
        extra.insert("card".into(), card.into());
        extra.insert("physical".into(), physical.into());
        let pattern = if !b(ov, "pattern/control/pp/inhibit") {
            Some(s(ov, "pattern/control/pp/type")).filter(|t| !t.is_empty() && t != "NO_PATTERN")
        } else {
            None
        };
        show.outputs.push(Output {
            id: ids::output(&ok),
            label: if label.is_empty() { format!("OUT {ok}") } else { label },
            connector_ids: vec![conn_id],
            format: get(ov, "status/pp").and_then(format_of),
            role,
            test_pattern: pattern,
            extra,
        });
        let _ = used_in;
    }

    // Multiviewer outputs.
    for (mk, mv) in items(d, "monitoringList") {
        if !b(mv, "status/pp/isAvailable") {
            continue;
        }
        let plug = items(mv, "plugList").first().map(|(_, v)| (*v).clone()).unwrap_or(Value::Null);
        let ptype = s(&plug, "status/pp/type");
        let card = s(mv, "mapping/pp/card");
        let physical = s(mv, "mapping/pp/physical");
        let devk = { let x = s(mv, "mapping/pp/device"); if x.is_empty() { "1".to_string() } else { x } };
        let conn_id = connector_for(&mut show, &devk, &card, &format!("MV_{physical}"), plug_kind(&ptype), Direction::Out, &ptype);
        let label = s(mv, "control/pp/label");
        show.outputs.push(Output {
            id: ids::output(format!("MV_{mk}")),
            label: if label.is_empty() { format!("Multiviewer {mk} output") } else { format!("{label} output") },
            connector_ids: vec![conn_id],
            format: get(mv, "status/pp").and_then(format_of),
            role: OutputRole::Multiviewer,
            test_pattern: None,
            extra: Extra::new(),
        });
    }

    // ---- screens and auxes -------------------------------------------
    let pre_screens: Map<String, Value> = items(d, "preconfig/resources/current/screenList").into_iter().map(|(k, v)| (k, v.clone())).collect();
    let group_of = |key: &str| get(d, &format!("screenAuxGroupList/items/{key}")).cloned().unwrap_or(Value::Null);

    for (sk, sv) in items(d, "screenList") {
        let group = group_of(&sk);
        let used = b(&group, "status/pp/isUsed") || n(&pre_screens.get(&sk).cloned().unwrap_or(Value::Null), "status/pp/outputCount").unwrap_or(0) > 0;
        if !used {
            continue;
        }
        let screen_id = ids::screen(&sk);
        let label = s(sv, "control/pp/label");
        let size = Size { w: n(sv, "status/size/pp/sizeH").unwrap_or(0) as u32, h: n(sv, "status/size/pp/sizeV").unwrap_or(0) as u32 };
        // Layers: NATIVE plus every layer with a capability.
        let mut layers = vec![LayerDef { id: ids::layer("NATIVE"), label: "Native background".into(), kind: LayerKind::Background, z: 0, capacity: None, extra: Extra::new() }];
        for (lk, lv) in items(sv, "layerList") {
            let cap = s(lv, "status/pp/capability");
            if cap.is_empty() || cap == "OFF" {
                continue;
            }
            let mut extra = pp(lv, "status/pp");
            extra.remove("capability");
            layers.push(LayerDef {
                id: ids::layer(&lk),
                label: format!("Layer {lk}"),
                kind: if b(lv, "status/pp/isMixerModeEnabled") { LayerKind::Mixer } else { LayerKind::Key },
                z: lk.parse::<u32>().unwrap_or(0),
                capacity: Some(cap),
                extra,
            });
        }
        // Outputs on this screen and where they sit on the canvas.
        let canvas_mode = s(sv, "control/canvas/pp/mode");
        let mut outputs = vec![];
        for (ok, pre) in pre_outputs.iter() {
            if s(pre, "status/pp/usedInScreenAux") != sk {
                continue;
            }
            let out_id = ids::output(ok);
            let Some(out) = show.outputs.iter().find(|o| o.id == out_id) else { continue };
            let fmt = out.format.clone().unwrap_or_default();
            let (x, y) = if canvas_mode == "GRID" {
                let col = n(sv, &format!("control/canvas/grid/control/outputList/items/{ok}/pp/column")).unwrap_or(1).max(1) - 1;
                let row = n(sv, &format!("control/canvas/grid/control/outputList/items/{ok}/pp/row")).unwrap_or(1).max(1) - 1;
                let cw = n(sv, "control/canvas/grid/control/size/pp/emptyCellWidth").unwrap_or(fmt.width as i64);
                let ch = n(sv, "control/canvas/grid/control/size/pp/emptyCellHeight").unwrap_or(fmt.height as i64);
                ((col * cw.max(fmt.width as i64)) as f64, (row * ch.max(fmt.height as i64)) as f64)
            } else {
                (
                    n(sv, &format!("control/canvas/free/control/outputList/items/{ok}/pp/left")).unwrap_or(0) as f64,
                    n(sv, &format!("control/canvas/free/control/outputList/items/{ok}/pp/top")).unwrap_or(0) as f64,
                )
            };
            outputs.push(OutputMap { output_id: out_id, rect: Rect::new(x, y, fmt.width as f64, fmt.height as f64) });
        }
        outputs.sort_by(|a, b| a.output_id.cmp(&b.output_id));
        let transition = f(&group, "control/pp/takeUpTime").map(|t| Transition { duration_ms: Some((t * 100.0) as u32), kind: Some("mix".into()) });
        let mut extra = pp(sv, "status/pp");
        extra.insert("canvasMode".into(), canvas_mode.into());
        extra.insert("presetUp".into(), s(&group, "control/pp/presetUp").into());
        extra.insert("presetDown".into(), s(&group, "control/pp/presetDown").into());
        extra.insert("transitionState".into(), s(&group, "status/pp/transition").into());
        show.screens.push(Screen {
            id: screen_id.clone(),
            label: if label.is_empty() { sk.clone() } else { label },
            kind: ScreenKind::Screen,
            size,
            outputs,
            layers,
            transition,
            extra,
        });
    }
    for (ak, av) in items(d, "auxiliaryList") {
        let mode = s(av, "status/pp/mode");
        let group = group_of(&ak);
        if (mode.is_empty() || mode == "DISABLED")
            && !b(&group, "status/pp/isUsed") {
                continue;
            }
        let label = s(av, "control/pp/label");
        let size = Size { w: n(av, "status/size/pp/sizeH").unwrap_or(0) as u32, h: n(av, "status/size/pp/sizeV").unwrap_or(0) as u32 };
        let mut layers = vec![LayerDef { id: ids::layer("NATIVE"), label: "Native background".into(), kind: LayerKind::Background, z: 0, capacity: None, extra: Extra::new() }];
        for (lk, lv) in items(av, "layerList") {
            let cap = s(lv, "status/pp/capability");
            if cap.is_empty() || cap == "OFF" {
                continue;
            }
            layers.push(LayerDef { id: ids::layer(&lk), label: format!("Layer {lk}"), kind: LayerKind::Mixer, z: lk.parse().unwrap_or(0), capacity: Some(cap), extra: Extra::new() });
        }
        let mut outputs = vec![];
        for (ok, pre) in pre_outputs.iter() {
            if s(pre, "status/pp/usedInScreenAux") != ak {
                continue;
            }
            let out_id = ids::output(ok);
            if let Some(out) = show.outputs.iter().find(|o| o.id == out_id) {
                let fmt = out.format.clone().unwrap_or_default();
                outputs.push(OutputMap { output_id: out_id, rect: Rect::new(0.0, 0.0, fmt.width as f64, fmt.height as f64) });
            }
        }
        let mut extra = pp(av, "status/pp");
        extra.insert("presetUp".into(), s(&group, "control/pp/presetUp").into());
        extra.insert("presetDown".into(), s(&group, "control/pp/presetDown").into());
        extra.insert("transitionState".into(), s(&group, "status/pp/transition").into());
        show.screens.push(Screen {
            id: ids::aux(&ak),
            label: if label.is_empty() { ak.clone() } else { label },
            kind: ScreenKind::Aux,
            size,
            outputs,
            layers,
            transition: f(&group, "control/pp/takeUpTime").map(|t| Transition { duration_ms: Some((t * 100.0) as u32), kind: Some("mix".into()) }),
            extra,
        });
    }

    // Program state per screen/aux, read from the letter on air.
    let screen_keys: Vec<(String, ScreenKind)> = show.screens.iter().map(|sc| (ids::tail(&sc.id).to_string(), sc.kind)).collect();
    for (key, kind) in screen_keys {
        let list = if kind == ScreenKind::Screen { "screenList" } else { "auxiliaryList" };
        let sv = get(d, &format!("{list}/items/{key}")).cloned().unwrap_or(Value::Null);
        let group = group_of(&key);
        let letters = openrcs_awj::Letters {
            down: s(&group, "control/pp/presetDown").chars().next().unwrap_or('A'),
            up: s(&group, "control/pp/presetUp").chars().next().unwrap_or('B'),
        };
        let transition = openrcs_awj::Transition::parse(&s(&group, "status/pp/transition")).unwrap_or(openrcs_awj::Transition::AtDown);
        let pgm = letters.letter(transition, openrcs_awj::Preset::Program);
        let pvw = letters.letter(transition, openrcs_awj::Preset::Preview);
        let screen_id = if kind == ScreenKind::Screen { ids::screen(&key) } else { ids::aux(&key) };
        let layer_ids: Vec<String> = show.screens.iter().find(|x| x.id == screen_id).map(|x| x.layers.iter().map(|l| l.id.clone()).collect()).unwrap_or_default();
        for (which, letter) in [("programState", pgm), ("previewState", pvw)] {
            let preset = get(&sv, &format!("presetList/items/{letter}")).cloned().unwrap_or(Value::Null);
            if preset.is_null() {
                continue;
            }
            let mut target = PresetTarget { screen_id: screen_id.clone(), background: None, layers: vec![], transition: None };
            for (lk, lv) in items(&preset, "layerList") {
                let lid = ids::layer(&lk);
                if !layer_ids.contains(&lid) {
                    continue;
                }
                let src = source_for(&mut show, &s(lv, "source/pp/inputNum"));
                if lk == "NATIVE" {
                    target.background = src.clone();
                }
                target.layers.push(layer_state(lv, &lid, src));
            }
            if let Some(sc) = show.screens.iter_mut().find(|x| x.id == screen_id) {
                sc.extra.insert(which.into(), serde_json::to_value(&target).unwrap_or_default());
                sc.extra.insert(if which == "programState" { "programLetter" } else { "previewLetter" }.into(), letter.to_string().into());
            }
        }
    }

    // ---- presets (memories) ---------------------------------------------
    for (pk, pv) in items(d, "presetBank/bankList") {
        if !b(pv, "status/pp/isValid") {
            continue;
        }
        let label = s(pv, "control/pp/label");
        let mut extra = pp(pv, "status/pp");
        extra.remove("isValid");
        let slot: u32 = pk.parse().unwrap_or(0);
        show.presets.push(Preset {
            id: ids::preset(slot),
            number: Some(slot),
            label: if label.is_empty() { format!("Memory {slot}") } else { label },
            notes: String::new(),
            targets: vec![],
            extra,
        });
    }
    if !show.presets.is_empty() {
        notes.push(Note {
            level: NoteLevel::Info,
            path: "presets".into(),
            message: "the memory bank lists each memory's label, canvas size, filters and transition time; the layer values inside a memory live only on the device (use a deep capture to read them)".into(),
        });
    }
    for (mk, mv) in items(d, "masterPresetBank/bankList") {
        if !b(mv, "status/pp/isValid") {
            continue;
        }
        let label = s(mv, "control/pp/label");
        let slot: u32 = mk.parse().unwrap_or(0);
        let mut entries = vec![];
        let filters: Vec<String> = get(mv, "status/pp/screenFilter").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        for (sk, sv) in items(mv, "status/screenList") {
            if !filters.is_empty() && !filters.contains(&sk) {
                continue;
            }
            let bank_slot = n(sv, "pp/presetBankSlot").unwrap_or(0) as u32;
            let sid = ids::screen(&sk);
            let pid = ids::preset(bank_slot);
            if show.screens.iter().any(|x| x.id == sid) && show.presets.iter().any(|p| p.id == pid) {
                entries.push(MasterEntry { screen_id: sid, preset_id: pid });
            }
        }
        let aux_filters: Vec<String> = get(mv, "status/pp/auxFilter").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        for (ak, av) in items(mv, "status/auxiliaryList") {
            if !aux_filters.contains(&ak) {
                continue;
            }
            let bank_slot = n(av, "pp/presetBankSlot").unwrap_or(0) as u32;
            let sid = ids::aux(&ak);
            let pid = ids::preset(bank_slot);
            if show.screens.iter().any(|x| x.id == sid) && show.presets.iter().any(|p| p.id == pid) {
                entries.push(MasterEntry { screen_id: sid, preset_id: pid });
            }
        }
        let mut extra = Extra::new();
        extra.insert("screenFilter".into(), filters.into());
        show.master_presets.push(MasterPreset { id: ids::master(slot), number: Some(slot), label: if label.is_empty() { format!("Master {slot}") } else { label }, entries, extra });
    }

    // ---- multiviewers -----------------------------------------------------
    for (mk, mv) in items(d, "monitoringList") {
        if !b(mv, "status/pp/isAvailable") {
            continue;
        }
        let label = s(mv, "control/pp/label");
        let size = Size { w: n(mv, "status/pp/sizeH").unwrap_or(1920) as u32, h: n(mv, "status/pp/sizeV").unwrap_or(1080) as u32 };
        let mut widgets = vec![];
        for (wk, wv) in items(mv, "layout/widgetList") {
            if !b(wv, "control/pp/enable") {
                continue;
            }
            let rect = Rect::new(
                f(wv, "control/pp/posH").unwrap_or(0.0),
                f(wv, "control/pp/posV").unwrap_or(0.0),
                f(wv, "control/pp/sizeH").unwrap_or(0.0),
                f(wv, "control/pp/sizeV").unwrap_or(0.0),
            );
            if rect.w <= 0.0 || rect.h <= 0.0 {
                continue;
            }
            let src = source_for(&mut show, &s(wv, "control/pp/source"));
            let mut extra = pp(wv, "control/pp");
            extra.retain(|k, _| !matches!(k.as_str(), "posH" | "posV" | "sizeH" | "sizeV" | "source" | "enable"));
            widgets.push(Widget {
                id: format!("w:{wk}"),
                rect,
                source_id: src,
                label: None,
                show_label: s(wv, "control/pp/displayOsd") != "NONE",
                tally: false,
                extra,
            });
        }
        show.multiviewers.push(Multiviewer {
            id: ids::multiviewer(&mk),
            label: if label.is_empty() { format!("Multiviewer {mk}") } else { label },
            output_ids: vec![ids::output(format!("MV_{mk}"))],
            layouts: vec![MvLayout { id: ids::layout(&mk, 1), label: "Current layout".into(), size, widgets }],
            active_layout: Some(ids::layout(&mk, 1)),
            extra: pp(mv, "status/pp"),
        });
    }

    show.notes = notes;
    show.system.frames.iter_mut().for_each(|fr| fr.slots.sort_by_key(|sl| sl.index));
    Ok(show)
}

/// Find or add the connector for a card/physical plug and return its id.
pub(crate) fn connector_for(show: &mut Show, dev: &str, card: &str, physical: &str, kind: ConnectorKind, dir: Direction, standard: &str) -> String {
    let id = ids::connector(dev, card, physical);
    if show.connector(&id).is_some() {
        return id;
    }
    let frame_id = ids::frame(dev);
    let frame = match show.system.frames.iter_mut().find(|f| f.id == frame_id) {
        Some(f) => f,
        None => {
            show.system.frames.push(Frame { id: frame_id.clone(), label: format!("Device {dev}"), model: String::new(), address: None, slots: vec![] });
            show.system.frames.last_mut().unwrap()
        }
    };
    let next_index = frame.slots.iter().map(|s| s.index).max().unwrap_or(0) + 1;
    let slot = match frame.slots.iter_mut().find(|s| s.label == card) {
        Some(s) => s,
        None => {
            frame.slots.push(Slot { index: next_index, card: card.to_string(), label: card.to_string(), connectors: vec![] });
            frame.slots.last_mut().unwrap()
        }
    };
    let index = slot.connectors.len() as u32 + 1;
    let pretty = physical.replace('_', " ");
    slot.connectors.push(Connector {
        id: id.clone(),
        kind,
        direction: dir,
        index,
        label: format!("{pretty} · {standard}"),
        standard: Some(standard.to_string()).filter(|s| !s.is_empty()),
    });
    id
}

/// One layer of a lettered preset → a layer state. Positions are
/// anchor-relative in the store; the model wants the top-left rect.
fn layer_state(lv: &Value, layer_id: &str, source_id: Option<String>) -> LayerState {
    let anchor = s(lv, "position/pp/anchor");
    let w = f(lv, "position/pp/sizeH").unwrap_or(0.0);
    let h = f(lv, "position/pp/sizeV").unwrap_or(0.0);
    let px = f(lv, "position/pp/posH").unwrap_or(0.0);
    let py = f(lv, "position/pp/posV").unwrap_or(0.0);
    let (ax, ay) = anchor_factors(&anchor);
    let rect = Rect::new(px - ax * w, py - ay * h, w, h);
    let opacity = f(lv, "opacity/pp/opacity").map(|o| (o / 256.0).clamp(0.0, 1.0));
    let crop = {
        let t = f(lv, "cropping/classic/pp/top").unwrap_or(0.0);
        let bo = f(lv, "cropping/classic/pp/bottom").unwrap_or(0.0);
        let l = f(lv, "cropping/classic/pp/left").unwrap_or(0.0);
        let r = f(lv, "cropping/classic/pp/right").unwrap_or(0.0);
        if t > 0.0 || bo > 0.0 || l > 0.0 || r > 0.0 {
            // The store gives edge insets; keep them as a rect of insets
            // (x=left, y=top, w=right, h=bottom) and say so in extra.
            Some(Rect::new(l, t, r, bo))
        } else {
            None
        }
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
    extra.insert("anchor".into(), anchor.into());
    if crop.is_some() {
        extra.insert("cropIsInsets".into(), true.into());
    }
    for (k, path) in [
        ("keying", "keying/pp"),
        ("transition", "transition/pp"),
        ("opening", "transition/opening/pp"),
        ("closing", "transition/closing/pp"),
        ("flying", "flying/pp"),
        ("speed", "speed/pp"),
        ("timing", "timing/pp"),
        ("effects", "effects/pp"),
        ("cutNFill", "cutNFill/pp"),
        ("mask", "cropping/mask/pp"),
        ("shadow", "border/shadow/pp"),
    ] {
        if let Some(v) = get(lv, path) {
            extra.insert(k.into(), v.clone());
        }
    }
    LayerState { layer_id: layer_id.to_string(), source_id: source_id.clone(), visible: source_id.is_some(), rect: Some(rect), crop, opacity, border, extra }
}

pub(crate) fn anchor_factors(anchor: &str) -> (f64, f64) {
    let x = if anchor.ends_with("LEFT") { 0.0 } else if anchor.ends_with("RIGHT") { 1.0 } else { 0.5 };
    let y = if anchor.starts_with("TOP") { 0.0 } else if anchor.starts_with("BOTTOM") { 1.0 } else { 0.5 };
    (x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/aw/livepremier-sim-6.2.73-store.json");
        serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
    }

    #[test]
    fn simulator_store() {
        let show = parse(&fixture()).unwrap();
        assert_eq!(show.system.model, "Aquilon C max");
        assert_eq!(show.system.firmware, "6.2.73");
        assert_eq!(show.system.extra["serial"], "ZZ9999");
        // Eight available inputs in the fixture; IN_33 is not available.
        assert_eq!(show.inputs.len(), 8);
        let cam = show.inputs.iter().find(|i| i.id == "in:IN_3").unwrap();
        assert_eq!(cam.label, "Camera 1");
        assert_eq!(cam.format.as_ref().map(|f| (f.width, f.height)), Some((1920, 1080)));
        assert_eq!(show.connector(&cam.connector_ids[0]).unwrap().kind, ConnectorKind::Hdmi);
        let sdi = show.inputs.iter().find(|i| i.id == "in:IN_13").unwrap();
        assert_eq!(show.connector(&sdi.connector_ids[0]).unwrap().kind, ConnectorKind::Sdi);
        // Six available outputs (21 is not fitted) + one multiviewer output; output 1 drives S1.
        assert_eq!(show.outputs.iter().filter(|o| o.role != OutputRole::Multiviewer).count(), 6);
        assert_eq!(show.outputs.iter().filter(|o| o.role == OutputRole::Multiviewer).count(), 1);
        let o1 = show.output("out:1").unwrap();
        assert_eq!(o1.role, OutputRole::Screen);
        assert_eq!(o1.format.as_ref().unwrap().describe(), "1920x1080p60");
        // One used screen, S1, with output 1 and a native + one layer.
        assert_eq!(show.screens.iter().filter(|s| s.kind == ScreenKind::Screen).count(), 1);
        let s1 = show.screen("scr:S1").unwrap();
        assert_eq!(s1.label, "Main wall");
        assert_eq!((s1.size.w, s1.size.h), (1920, 1080));
        assert_eq!(s1.outputs.len(), 1);
        assert_eq!(s1.layers.len(), 2);
        // Program is letter B (T-bar AT_UP, presetUp = B) — the fixture put
        // camera 1 on layer 1 of preset A, so A is preview here.
        assert_eq!(s1.extra["programLetter"], "B");
        let pvw: PresetTarget = serde_json::from_value(s1.extra["previewState"].clone()).unwrap();
        let l1 = pvw.layers.iter().find(|l| l.layer_id == "layer:1").unwrap();
        assert_eq!(l1.source_id.as_deref(), Some("src:IN_3"));
        // MIDDLE_CENTER anchor at (960,540) with 960x540 → top-left (480,270).
        let r = l1.rect.unwrap();
        assert_eq!((r.x, r.y, r.w, r.h), (480.0, 270.0, 960.0, 540.0));
        assert_eq!(pvw.background.as_deref(), Some("src:STILL_1"));
        // Memories and master.
        assert_eq!(show.presets.len(), 2);
        assert_eq!(show.presets[0].label, "Walk in");
        assert_eq!(show.master_presets.len(), 1);
        assert_eq!(show.master_presets[0].entries.len(), 1);
        assert_eq!(show.master_presets[0].entries[0].preset_id, "pre:2");
        // Multiviewer with widgets on an input and on S1 program.
        assert_eq!(show.multiviewers.len(), 1);
        let ws = &show.multiviewers[0].layouts[0].widgets;
        assert_eq!(ws.len(), 4);
        assert_eq!(ws[0].source_id.as_deref(), Some("src:IN_1"));
        assert_eq!(ws[1].source_id.as_deref(), Some("src:PROGRAM_S1"));
        assert!(show.stills.iter().any(|s| s.label == "Logo"));
        assert!(show.validate().is_empty(), "{:?}", show.validate());
    }
}
