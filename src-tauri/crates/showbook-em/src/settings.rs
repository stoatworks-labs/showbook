//! `settings.xml` → [`Show`].
//!
//! The store is one big `<System>` element. The parts this driver maps, with
//! the paths they were read from in the Encore3 10.0.2 and E2 9.2 simulator
//! stores:
//!
//! | model | store |
//! |---|---|
//! | system, firmware, native rate | `System/{Name,Version,NativeRate}` |
//! | frames, slots, connectors | `FrameCollection/Frame/Slot/Card/{CardIn/In,CardOut/Out,CardExp}` + `hwconfig.xml` |
//! | inputs | `InputCfgCol/InputCfg` (wherever it appears; none in the simulator stores) |
//! | sources | `SrcMgr/SourceCol/Source` |
//! | outputs | `OutCfgMgr/OutputCfg` (+ E2 `Frame/MultiViewer/MVOutputCfgCollection/OutputCfg`) |
//! | screens | `DestMgr/ScreenDestCol/ScreenDest` with `DestOutMapCol/DestOutMap/{LayerCollection,BGLayer,DSKLayer}` |
//! | auxes | `DestMgr/AuxDestCol/AuxDest` |
//! | multiviewers | `MultiViewerCollection/MultiViewer/MVLayout/MVWinCollection/MVWin` (+ E2 `Frame/MultiViewer`) |
//! | presets, cues | `presets/*.xml`, `cues/*.xml` — see [`crate::presets`] |
//!
//! Numbers the frame keeps as codes are decoded where the code is known
//! (card types, frame types, capacities) and otherwise kept verbatim in the
//! entity's `extra` bag under the vendor's own element name.

use std::collections::BTreeMap;

use roxmltree::{Document, Node};
use showbook_model::{
    ids, Connector, ConnectorKind, Direction, Extra, Format, Frame, Genlock, Input, LayerDef, LayerKind, LayerState,
    Multiviewer, MvLayout, Note, NoteLevel, Output, OutputMap, OutputRole, Platform, Rect, Screen, ScreenKind, Show,
    Size, Slot, Source, SourceKind, Transition, Widget,
};

use crate::archive::Store;
use crate::cards;
use crate::xml::*;
use crate::{Error, Result};

/// Everything the mapping needs to remember across sections.
#[derive(Default)]
pub(crate) struct Ctx {
    /// (frame vpid, slot) → card type code, from hwconfig.xml and ConnMaps.
    pub card_codes: BTreeMap<(u32, u32), i64>,
    /// output id → its format, for screen output maps.
    pub output_formats: BTreeMap<String, Format>,
    pub native_rate: f64,
    pub notes: Vec<Note>,
}

pub fn parse_store(store: &Store) -> Result<Show> {
    let settings = store
        .text("settings.xml")
        .ok_or_else(|| Error::NotAStore("store has no settings.xml".into()))?;
    let doc = Document::parse(&settings).map_err(|e| Error::Xml(e.to_string()))?;
    let root = doc.root_element();
    if root.tag_name().name() != "System" {
        return Err(Error::NotAStore(format!("settings.xml root is <{}>, not <System>", root.tag_name().name())));
    }

    let mut ctx = Ctx::default();
    let mut show = Show::new(&string(root, "Name"), Platform::BarcoEm);

    if let Some(hw) = store.text("hwconfig.xml") {
        if let Ok(hwdoc) = Document::parse(&hw) {
            read_hwconfig(hwdoc.root_element(), &mut ctx);
        }
    }

    parse_system(root, &mut show, &mut ctx);
    parse_frames(root, &mut show, &mut ctx);
    parse_inputs(root, &mut show, &mut ctx);
    parse_outputs(root, &mut show, &mut ctx);
    parse_sources(root, &mut show, &mut ctx);
    parse_screens(root, &mut show, &mut ctx);
    parse_auxes(root, &mut show, &mut ctx);
    parse_multiviewers(root, &mut show, &mut ctx);
    crate::presets::parse_presets(store, &mut show, &mut ctx);
    crate::presets::parse_cues(store, &mut show, &mut ctx);
    crate::presets::parse_user_keys(store, &mut show, &mut ctx);

    if show.meta.name.is_empty() {
        show.meta.name = "Event Master show".into();
    }
    show.notes.append(&mut ctx.notes);
    Ok(show)
}

fn read_hwconfig(root: Node, ctx: &mut Ctx) {
    // <simsyshw><simhw vpid="0"><frametype type="8"/><card id type slot><name>…
    for simhw in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "simhw") {
        let vpid: u32 = simhw.attribute("vpid").and_then(|v| v.parse().ok()).unwrap_or(0);
        for card in children(simhw, "card") {
            let (Some(t), Some(s)) = (card.attribute("type"), card.attribute("slot")) else { continue };
            if let (Ok(t), Ok(s)) = (t.parse::<i64>(), s.parse::<u32>()) {
                ctx.card_codes.insert((vpid, s), t);
            }
        }
    }
}

fn parse_system(root: Node, show: &mut Show, ctx: &mut Ctx) {
    show.system.name = string(root, "Name");
    show.system.firmware = string(root, "Version");
    show.system.native_rate = float(root, "NativeRate");
    ctx.native_rate = show.system.native_rate.unwrap_or(60.0);
    if let Some(g) = child(root, "Genlock") {
        let lock = int(g, "LockSrc").unwrap_or(0);
        show.system.genlock = Some(Genlock {
            source: match lock {
                0 => "internal".into(),
                1 => "input".into(),
                2 => "external".into(),
                n => format!("mode {n}"),
            },
            input_id: index(g, "InputCfgIndex").map(ids::input),
            locked: flag(g, "LockMode"),
        });
    }
    let mut extra = Extra::new();
    for key in ["MacAddress", "Simulator", "CanvasMode", "AutoSave", "AutoTake", "Colorimetry", "FlexLayerMode", "LinkPref"] {
        if let Some(t) = text(root, key) {
            extra.insert(key.into(), t.into());
        }
    }
    show.system.extra = extra;
}

fn parse_frames(root: Node, show: &mut Show, ctx: &mut Ctx) {
    let Some(fc) = child(root, "FrameCollection") else { return };
    let mut model = String::new();
    for (i, f) in children(fc, "Frame").enumerate() {
        let vpid = index(f, "VPID").unwrap_or(i as u32);
        let ftype = int(f, "FrameType").unwrap_or(-1);
        let fmodel = cards::frame_model(ftype).to_string();
        if model.is_empty() {
            model = fmodel.clone();
        }
        let mut frame = Frame {
            id: ids::frame(vpid + 1),
            label: {
                let n = string(f, "Name");
                if n.is_empty() { format!("Frame {}", vpid + 1) } else { n }
            },
            model: fmodel,
            address: at(f, "Enet/IP").and_then(|n| n.text()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
            slots: vec![],
        };
        // A frame's own multiviewer card (E2 9.2 keeps it here).
        for slot in children(f, "Slot") {
            let sidx = id_num(slot).unwrap_or(0);
            let Some(card) = child(slot, "Card") else { continue };
            let code = ctx.card_codes.get(&(vpid, sidx)).copied();
            let info = code.and_then(cards::card);
            let mut connectors = vec![];
            let mut kind_label = String::new();
            if let Some(ci) = child(card, "CardIn") {
                kind_label = "input".into();
                for (k, inp) in children(ci, "In").enumerate() {
                    let n = id_num(inp).unwrap_or(k as u32);
                    let (kind, std) = connector_kind_of(inp, Direction::In, info.as_ref(), n as usize);
                    connectors.push(Connector {
                        id: ids::connector(vpid + 1, sidx + 1, n + 1),
                        kind,
                        direction: Direction::In,
                        index: n + 1,
                        label: format!("Slot {} · {} {}", sidx + 1, kind_name(kind), n + 1),
                        standard: std,
                    });
                }
            }
            if let Some(co) = child(card, "CardOut") {
                kind_label = "output".into();
                for (k, out) in children(co, "Out").enumerate() {
                    let n = id_num(out).unwrap_or(k as u32);
                    let (kind, std) = connector_kind_of(out, Direction::Out, info.as_ref(), n as usize);
                    connectors.push(Connector {
                        id: ids::connector(vpid + 1, sidx + 1, n + 1),
                        kind,
                        direction: Direction::Out,
                        index: n + 1,
                        label: format!("Slot {} · {} {}", sidx + 1, kind_name(kind), n + 1),
                        standard: std,
                    });
                }
            }
            if let Some(ce) = child(card, "CardExp") {
                kind_label = "link".into();
                for (k, lc) in children(ce, "LinkConnector").enumerate() {
                    let n = id_num(lc).unwrap_or(k as u32);
                    connectors.push(Connector {
                        id: ids::connector(vpid + 1, sidx + 1, n + 1),
                        kind: ConnectorKind::Link,
                        direction: Direction::In,
                        index: n + 1,
                        label: format!("Slot {} · Link {}", sidx + 1, n + 1),
                        standard: None,
                    });
                }
            }
            let card_name = match info {
                Some(i) => i.name.to_string(),
                None => match code {
                    Some(c) => cards::card_name(c),
                    None => match kind_label.as_str() {
                        "input" => format!("Input card ({} connectors)", connectors.len()),
                        "output" => format!("Output card ({} connectors)", connectors.len()),
                        "link" => "Link card".into(),
                        _ => "Card".into(),
                    },
                },
            };
            frame.slots.push(Slot { index: sidx + 1, card: card_name, label: String::new(), connectors });
        }
        // E2/S3: the multiviewer lives on the frame, with its own output configs.
        if let Some(mv) = child(f, "MultiViewer") {
            if let Some(oc) = child(mv, "MVOutputCfgCollection") {
                for o in children(oc, "OutputCfg") {
                    if let Some(out) = output_from_cfg(o, OutputRole::Multiviewer, "mvr", ctx) {
                        ensure_connectors_for(&mut frame, &out, vpid + 1);
                        ctx.output_formats.insert(out.id.clone(), out.format.clone().unwrap_or_default());
                        show.outputs.push(out);
                    }
                }
            }
        }
        show.system.frames.push(frame);
    }
    show.system.model = model;
}

/// An output config's ConnMap can name a slot that has no `Slot` entry in
/// the frame (the E2 multiviewer card); add the connector so the reference resolves.
fn ensure_connectors_for(frame: &mut Frame, out: &Output, frame_no: u32) {
    for cid in &out.connector_ids {
        if frame.slots.iter().any(|s| s.connectors.iter().any(|c| &c.id == cid)) {
            continue;
        }
        // conn:<frame>.<slot>.<n>
        let tail = ids::tail(cid);
        let parts: Vec<&str> = tail.split('.').collect();
        if parts.len() != 3 || parts[0] != frame_no.to_string() {
            continue;
        }
        let (Ok(slot), Ok(n)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) else { continue };
        let kind = out
            .extra
            .get("CardType")
            .and_then(|v| v.as_i64())
            .and_then(cards::card)
            .and_then(|c| c.connectors.get((n as usize).saturating_sub(1)).map(|c| (c.0, c.2)));
        let conn = Connector {
            id: cid.clone(),
            kind: kind.map(|k| k.0).unwrap_or(ConnectorKind::Other),
            direction: Direction::Out,
            index: n,
            label: format!("Slot {} · {} {}", slot, kind.map(|k| kind_name(k.0)).unwrap_or("Output"), n),
            standard: kind.map(|k| k.1.to_string()),
        };
        match frame.slots.iter_mut().find(|s| s.index == slot) {
            Some(s) => s.connectors.push(conn),
            None => frame.slots.push(Slot {
                index: slot,
                card: out
                    .extra
                    .get("CardType")
                    .and_then(|v| v.as_i64())
                    .map(cards::card_name)
                    .unwrap_or_else(|| "Multiviewer output card".into()),
                label: String::new(),
                connectors: vec![conn],
            }),
        }
        frame.slots.sort_by_key(|s| s.index);
    }
}

fn kind_name(k: ConnectorKind) -> &'static str {
    match k {
        ConnectorKind::Sdi => "SDI",
        ConnectorKind::Hdmi => "HDMI",
        ConnectorKind::DisplayPort => "DP",
        ConnectorKind::Dvi => "DVI",
        ConnectorKind::Fibre => "Fibre",
        ConnectorKind::Ip => "IP",
        ConnectorKind::Link => "Link",
        ConnectorKind::Other => "Connector",
    }
}

/// The connector's physical kind: from the `In`/`Out` element's child
/// (`HDMIIn`, `DPIn`, `SDIIn`, `SDIOut`, …), else from the card table.
fn connector_kind_of(
    n: Node,
    dir: Direction,
    info: Option<&cards::CardInfo>,
    index: usize,
) -> (ConnectorKind, Option<String>) {
    for c in n.children().filter(|c| c.is_element()) {
        let t = c.tag_name().name();
        let k = match t {
            "HDMIIn" | "HDMIOut" => Some(ConnectorKind::Hdmi),
            "DPIn" | "DPOut" => Some(ConnectorKind::DisplayPort),
            "SDIIn" | "SDIOut" => Some(ConnectorKind::Sdi),
            "DVIIn" | "DVIOut" => Some(ConnectorKind::Dvi),
            _ => None,
        };
        if let Some(k) = k {
            let std = match (k, t) {
                (ConnectorKind::Sdi, "SDIOut") | (ConnectorKind::Sdi, "SDIIn") => match int(c, "Type3G") {
                    Some(2) => Some("12G-SDI".to_string()),
                    Some(1) => Some("6G-SDI".to_string()),
                    Some(0) => Some("3G-SDI".to_string()),
                    _ => None,
                },
                _ => info.and_then(|i| i.connectors.get(index)).filter(|c| c.0 == k).map(|c| c.2.to_string()),
            };
            return (k, std);
        }
    }
    if let Some(c) = info.and_then(|i| i.connectors.get(index)) {
        if c.1 == dir {
            return (c.0, Some(c.2.to_string()));
        }
    }
    (ConnectorKind::Other, None)
}

pub(crate) fn video_format(n: Node) -> Option<Format> {
    let vf = child(n, "VideoFormat")?;
    let width = index(vf, "HActive")?;
    let height = index(vf, "VActive")?;
    let rate = float(vf, "VFreq").unwrap_or(0.0);
    // The store writes `Interlaced` as 80/81-style flags on some firmware and
    // 0/1 on others; treat odd as interlaced.
    let interlaced = int(vf, "Interlaced").map(|v| v % 2 == 1).unwrap_or(false);
    Some(Format { width, height, rate, interlaced, name: text(vf, "Name").map(str::to_string) })
}

/// `FormatName` like `1920x1080p @59.94` / `1920x1080 @59.94` / `3840x2160p @60`.
pub(crate) fn parse_format_name(s: &str) -> Option<Format> {
    let s = s.trim();
    let (res, rate) = s.split_once('@').map(|(a, b)| (a.trim(), b.trim())).unwrap_or((s, ""));
    let (w, rest) = res.split_once('x')?;
    let width: u32 = w.trim().parse().ok()?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let height: u32 = digits.parse().ok()?;
    let interlaced = rest[digits.len()..].trim_start().starts_with('i');
    let rate: f64 = rate.trim_end_matches("Hz").trim().parse().unwrap_or(0.0);
    Some(Format { width, height, rate, interlaced, name: Some(s.to_string()) })
}

fn parse_inputs(root: Node, show: &mut Show, ctx: &mut Ctx) {
    // The input config collection has not been seen in a simulator store, so
    // find it wherever it lives and map the fields the output config uses.
    let cfgs: Vec<Node> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "InputCfg" && n.attribute("id").is_some())
        .collect();
    for (k, cfg) in cfgs.iter().enumerate() {
        let id = id_num(*cfg).unwrap_or(k as u32);
        let mut connector_ids = vec![];
        if let Some(coll) = at(*cfg, "Config/ConnMapColl").or_else(|| child(*cfg, "ConnMapColl")) {
            for cm in children(coll, "ConnMap") {
                if let Some(cid) = connmap_id(cm, ctx) {
                    connector_ids.push(cid);
                }
            }
        }
        let format = at(*cfg, "Config").and_then(video_format).or_else(|| video_format(*cfg));
        let mut extra = scalars(*cfg);
        extra.retain(|k, _| k != "Name");
        let name = string(*cfg, "Name");
        show.inputs.push(Input {
            id: ids::input(id),
            label: if name.is_empty() { format!("Input {}", id + 1) } else { name },
            connector_ids,
            format,
            enabled: true,
            capacity: int(*cfg, "Capacity").map(capacity_name),
            hdcp: int(*cfg, "HdcpMode").map(|v| v != 0),
            extra,
        });
    }
    if cfgs.is_empty() {
        ctx.notes.push(Note {
            level: NoteLevel::Info,
            path: "inputs".into(),
            message: "settings.xml holds no InputCfg entries (no inputs configured, or a firmware that stores them elsewhere)".into(),
        });
    }
}

pub(crate) fn capacity_name(c: i64) -> String {
    match c {
        1 => "SL".into(),
        2 => "DL".into(),
        4 => "4K".into(),
        8 => "8K".into(),
        n => format!("capacity {n}"),
    }
}

/// `ConnMap` → `conn:<frame>.<slot>.<connector>`, remembering the card type.
fn connmap_id(cm: Node, ctx: &mut Ctx) -> Option<String> {
    if int(cm, "InUse") == Some(0) {
        return None;
    }
    let slot = index(cm, "SlotIndex")?;
    let conn = index(cm, "ConnectorIndex")?;
    let vpid = index(cm, "VPID").unwrap_or(0);
    if let Some(code) = int(cm, "CardType") {
        ctx.card_codes.entry((vpid, slot)).or_insert(code);
    }
    Some(ids::connector(vpid + 1, slot + 1, conn + 1))
}

fn output_from_cfg(o: Node, role: OutputRole, prefix: &str, ctx: &mut Ctx) -> Option<Output> {
    let id = id_num(o)?;
    let mut connector_ids = vec![];
    let mut card_type = None;
    if let Some(coll) = at(o, "Config/ConnMapColl") {
        for cm in children(coll, "ConnMap") {
            card_type = card_type.or_else(|| int(cm, "CardType"));
            if let Some(cid) = connmap_id(cm, ctx) {
                connector_ids.push(cid);
            }
        }
    } else if let Some(cm) = at(o, "Config/ConnMap") {
        card_type = int(cm, "CardType");
        if let Some(cid) = connmap_id(cm, ctx) {
            connector_ids.push(cid);
        }
    }
    let format = at(o, "Config").and_then(video_format);
    let mut extra = scalars(o);
    extra.retain(|k, _| k != "Name");
    if let Some(ct) = card_type {
        extra.insert("CardType".into(), ct.into());
    }
    if let Some(tp) = child(o, "TestPattern") {
        for (k, v) in scalars(tp) {
            extra.insert(format!("TestPattern.{k}"), v);
        }
    }
    // `TestPatternMode` 0 is taken as off. The list order below follows the
    // toolset's output test-pattern menu; it is not documented in the store
    // and has not been checked against a frame, so the raw code is kept in
    // `extra` too.
    let test_pattern = child(o, "TestPattern")
        .and_then(|tp| int(tp, "TestPatternMode"))
        .filter(|m| *m > 0)
        .map(test_pattern_name);
    let name = string(o, "Name");
    Some(Output {
        id: if prefix.is_empty() { ids::output(id) } else { ids::output(format!("{prefix}{id}")) },
        label: if name.is_empty() { format!("Output {}", id + 1) } else { name },
        connector_ids,
        format,
        role,
        test_pattern,
        extra,
    })
}

fn test_pattern_name(mode: i64) -> String {
    let name = match mode {
        1 => "H ramp",
        2 => "V ramp",
        3 => "100% colour bars",
        4 => "16x16 grid",
        5 => "32x32 grid",
        6 => "burst",
        7 => "75% colour bars",
        8 => "50% grey",
        9 => "grey steps 16",
        10 => "grey steps 32",
        11 => "white",
        12 => "black",
        13 => "SMPTE bars",
        14 => "H alignment",
        15 => "V alignment",
        16 => "HV alignment",
        17 => "custom grid",
        _ => "test pattern",
    };
    if mode > 17 {
        format!("{name} (mode {mode})")
    } else {
        name.to_string()
    }
}

fn parse_outputs(root: Node, show: &mut Show, ctx: &mut Ctx) {
    let Some(mgr) = child(root, "OutCfgMgr") else { return };
    for o in children(mgr, "OutputCfg") {
        // Role is decided when screens and auxes claim their outputs.
        if let Some(out) = output_from_cfg(o, OutputRole::Unassigned, "", ctx) {
            ctx.output_formats.insert(out.id.clone(), out.format.clone().unwrap_or_default());
            show.outputs.push(out);
        }
    }
}

/// A `Source` element → (kind, reference), by which of its indices is set.
pub(crate) fn source_ref(src: Node) -> (SourceKind, Option<String>) {
    let src_type = int(src, "SrcType").unwrap_or(-1);
    if let Some(i) = index(src, "InputCfgIndex") {
        return (SourceKind::Input, Some(ids::input(i)));
    }
    if let Some(i) = index(src, "StillIndex") {
        return (SourceKind::Still, Some(ids::still(i)));
    }
    if let Some(i) = index(src, "DestIndex") {
        return match src_type {
            3 => (SourceKind::Aux, Some(ids::aux(i))),
            _ => (SourceKind::Screen, Some(ids::screen(i))),
        };
    }
    if index(src, "MvrIndex").is_some() {
        return (SourceKind::Multiviewer, index(src, "MvrIndex").map(|i| ids::multiviewer(i + 1)));
    }
    if index(src, "OutCfgIndex").is_some() && src_type == 6 {
        return (SourceKind::Multiviewer, None);
    }
    match src_type {
        0 => (SourceKind::Input, None),
        1 => (SourceKind::Background, None),
        2 => (SourceKind::Screen, None),
        3 => (SourceKind::Aux, None),
        4 => (SourceKind::Still, None),
        _ => (SourceKind::Unknown, None),
    }
}

fn parse_sources(root: Node, show: &mut Show, _ctx: &mut Ctx) {
    let Some(col) = at(root, "SrcMgr/SourceCol") else { return };
    for s in children(col, "Source") {
        let Some(id) = id_num(s) else { continue };
        let (kind, ref_id) = source_ref(s);
        let aoi = child(s, "AOIRect").and_then(|r| {
            let w = float(r, "HSize")?;
            let h = float(r, "VSize")?;
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            Some(Rect::new(float(r, "HPos").unwrap_or(0.0), float(r, "VPos").unwrap_or(0.0), w, h))
        });
        let mut extra = scalars(s);
        extra.retain(|k, _| k != "Name");
        let name = string(s, "Name");
        show.sources.push(Source {
            id: ids::source(id),
            label: if name.is_empty() { format!("Source {}", id + 1) } else { name },
            kind,
            ref_id,
            aoi,
            format: text(s, "FormatName").and_then(parse_format_name),
            extra,
        });
    }
}

/// Map a `Layer` element (in the destination or in a preset) to a
/// definition and its current state.
pub(crate) fn layer_from_node(layer: Node, n: u32, native_rate: f64) -> (LayerDef, LayerState) {
    let name = string(layer, "Name");
    let mut def_extra = Extra::new();
    for k in ["Capacity", "CapacityLock", "LayerTransparentMode", "AlphaMaskable", "FollowDestOutMapIndex", "LockMode"] {
        if let Some(t) = text(layer, k) {
            def_extra.insert(k.into(), t.into());
        }
    }
    let def = LayerDef {
        id: ids::layer(n),
        label: if name.is_empty() { format!("Layer {n}") } else { name },
        kind: LayerKind::Mixer,
        z: index(layer, "PgmZOrder").unwrap_or(n),
        capacity: int(layer, "Capacity").map(capacity_name),
        extra: def_extra,
    };
    let state = layer_state_from_node(layer, &def.id, native_rate);
    (def, state)
}

/// The layer's current (program) state: source, window, crop, opacity, border.
pub(crate) fn layer_state_from_node(layer: Node, layer_id: &str, native_rate: f64) -> LayerState {
    let cfg = child(layer, "LayerCfg");
    let source_id = cfg.and_then(|c| child(c, "Source")).and_then(|s| {
        // In the destination the layer's Source is a copy of a SrcMgr entry
        // with its indices; resolve it to that entry's id by index if the
        // store gives one, else by the source's own reference.
        let (_, r) = source_ref(s);
        r
    });
    let active = cfg.and_then(|c| index(c, "ActiveState")).unwrap_or(0);
    let state = cfg.and_then(|c| children(c, "LayerState").find(|s| id_num(*s) == Some(active)).or_else(|| child(c, "LayerState")));
    let mut rect = None;
    let mut crop = None;
    let mut opacity = None;
    let mut border = None;
    let mut extra = Extra::new();
    if let Some(st) = state {
        if let Some(wa) = child(st, "WinAdjust") {
            if let Some(o) = child(wa, "OWIN") {
                rect = Some(Rect::new(
                    float(o, "HPos").unwrap_or(0.0),
                    float(o, "VPos").unwrap_or(0.0),
                    float(o, "HSize").unwrap_or(0.0),
                    float(o, "VSize").unwrap_or(0.0),
                ));
            }
            if let Some(i) = child(wa, "IWIN") {
                let r = Rect::new(
                    float(i, "HPos").unwrap_or(0.0),
                    float(i, "VPos").unwrap_or(0.0),
                    float(i, "HSize").unwrap_or(0.0),
                    float(i, "VSize").unwrap_or(0.0),
                );
                if r.w > 0.0 && r.h > 0.0 {
                    crop = Some(r);
                }
            }
            if let Some(m) = child(wa, "Mask") {
                for (k, v) in scalars(m) {
                    extra.insert(format!("Mask.{k}"), v);
                }
            }
        }
        if let Some(pip) = child(st, "PIP") {
            opacity = float(pip, "Opacity").map(|o| (o / 100.0).clamp(0.0, 1.0));
            if let Some(b) = child(pip, "Border") {
                let typ = int(b, "BrdrTyp").unwrap_or(0);
                let px = float(b, "BrdrPixel").unwrap_or(0.0);
                if typ != 0 && px > 0.0 {
                    border = Some(showbook_model::Border { width: px as u32, color: colour_of(child(b, "Color")) });
                }
                for (k, v) in scalars(b) {
                    extra.insert(format!("Border.{k}"), v);
                }
            }
            if let Some(sh) = child(pip, "Shadow") {
                for (k, v) in scalars(sh) {
                    extra.insert(format!("Shadow.{k}"), v);
                }
            }
        }
        if let Some(key) = child(st, "Key") {
            for (k, v) in scalars(key) {
                extra.insert(format!("Key.{k}"), v);
            }
        }
        for (k, v) in scalars(st) {
            extra.insert(k, v);
        }
    }
    if let Some(c) = cfg {
        for k in ["LayerMode", "FlightCurve", "FrzMode", "AuxScalingMode"] {
            if let Some(t) = text(c, k) {
                extra.insert(k.into(), t.into());
            }
        }
        if let Some(tr) = children(c, "Transition").find(|t| id_num(*t) == Some(active)).or_else(|| child(c, "Transition")) {
            if let Some(frames) = float(tr, "TransTime") {
                extra.insert("TransTime".into(), frames.into());
                extra.insert("transitionMs".into(), ((frames / native_rate.max(1.0)) * 1000.0).round().into());
            }
        }
    }
    let pgm = flag(layer, "PgmMode");
    let pvw = flag(layer, "PvwMode");
    let active_flag = flag(layer, "IsActive");
    for k in ["PgmMode", "PvwMode", "IsActive", "PgmZOrder", "PvwZOrder", "Eye3D", "SrcSwitch"] {
        if let Some(t) = text(layer, k) {
            extra.insert(k.into(), t.into());
        }
    }
    LayerState { layer_id: layer_id.to_string(), source_id, visible: pgm || pvw || active_flag, rect, crop, opacity, border, extra }
}

fn colour_of(c: Option<Node>) -> String {
    let Some(c) = c else { return "#000000".into() };
    // Colours are 0–1000 in the store.
    let ch = |k: &str| (float(c, k).unwrap_or(0.0) / 1000.0 * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", ch("Red"), ch("Green"), ch("Blue"))
}

fn transition_of(n: Node, native_rate: f64) -> Option<Transition> {
    let tr = child(n, "Transition")?;
    let frames = float(tr, "TransTime")?;
    Some(Transition {
        duration_ms: Some(((frames / native_rate.max(1.0)) * 1000.0).round() as u32),
        kind: Some(match int(tr, "Mode").unwrap_or(0) {
            0 => "mix".into(),
            1 => "cut".into(),
            n => format!("mode {n}"),
        }),
    })
}

fn parse_screens(root: Node, show: &mut Show, ctx: &mut Ctx) {
    let Some(col) = at(root, "DestMgr/ScreenDestCol") else { return };
    for sd in children(col, "ScreenDest") {
        let Some(id) = id_num(sd) else { continue };
        let name = string(sd, "Name");
        let format = video_format(sd);
        let mut outputs = vec![];
        let mut layers: Vec<LayerDef> = vec![];
        let mut states: Vec<LayerState> = vec![];
        let mut background = None;
        let mut extra = scalars(sd);
        extra.retain(|k, _| k != "Name");
        if let Some(maps) = child(sd, "DestOutMapCol") {
            for (mi, m) in children(maps, "DestOutMap").enumerate() {
                if let Some(oi) = index(m, "OutCfgIndex") {
                    let out_id = ids::output(oi);
                    let f = ctx.output_formats.get(&out_id).cloned().or_else(|| format.clone()).unwrap_or_default();
                    outputs.push(OutputMap {
                        output_id: out_id.clone(),
                        rect: Rect::new(
                            float(m, "HPos").unwrap_or(0.0),
                            float(m, "VPos").unwrap_or(0.0),
                            f.width as f64,
                            f.height as f64,
                        ),
                    });
                    if let Some(o) = show.outputs.iter_mut().find(|o| o.id == out_id) {
                        o.role = OutputRole::Screen;
                    }
                }
                // Layers follow the map they were defined on; a second map's
                // `FollowDestOutMapIndex` layers are the same layers seen again.
                if let Some(lc) = child(m, "LayerCollection") {
                    for (li, l) in children(lc, "Layer").enumerate() {
                        let follows = index(l, "FollowDestOutMapIndex").unwrap_or(mi as u32);
                        if follows != mi as u32 {
                            continue;
                        }
                        let n = id_num(l).unwrap_or(li as u32) + 1;
                        if layers.iter().any(|d| d.id == ids::layer(n)) {
                            continue;
                        }
                        let (def, st) = layer_from_node(l, n, ctx.native_rate);
                        layers.push(def);
                        states.push(st);
                    }
                }
                if mi == 0 {
                    if let Some(bg) = child(m, "BGLayer") {
                        let name = string(bg, "Name");
                        layers.push(LayerDef {
                            id: ids::layer("bg"),
                            label: if name.is_empty() || name == "EmptyNativeLayer" { "Background".into() } else { name },
                            kind: LayerKind::Background,
                            z: 0,
                            capacity: None,
                            extra: scalars(bg),
                        });
                        background = index(bg, "LastAppliedSrcIdx").map(ids::source);
                    }
                    if let Some(dsk) = child(m, "DSKLayer") {
                        let name = string(dsk, "Name");
                        layers.push(LayerDef {
                            id: ids::layer("dsk"),
                            label: if name.is_empty() || name == "EmptyNativeLayer" { "DSK".into() } else { name },
                            kind: LayerKind::Key,
                            z: 1000,
                            capacity: None,
                            extra: scalars(dsk),
                        });
                    }
                }
            }
        }
        // Canvas: what the output maps cover, else the destination's own raster.
        let mut w = 0.0f64;
        let mut h = 0.0f64;
        for om in &outputs {
            w = w.max(om.rect.x + om.rect.w);
            h = h.max(om.rect.y + om.rect.h);
        }
        if w == 0.0 || h == 0.0 {
            if let Some(f) = &format {
                let hd = index(sd, "HDimension").unwrap_or(1).max(1);
                let vd = index(sd, "VDimension").unwrap_or(1).max(1);
                w = (f.width * hd) as f64;
                h = (f.height * vd) as f64;
            }
        }
        if let Some(f) = &format {
            extra.insert("rasterFormat".into(), f.describe().into());
        }
        layers.sort_by_key(|l| l.z);
        let screen_id = ids::screen(id);
        // Remember the live program state as the "current" preset target so
        // the inspector can draw the screen even with no presets saved.
        if !states.is_empty() || background.is_some() {
            extra.insert(
                "programState".into(),
                serde_json::to_value(showbook_model::PresetTarget {
                    screen_id: screen_id.clone(),
                    background: background.clone(),
                    layers: states,
                    transition: None,
                })
                .unwrap_or_default(),
            );
        }
        show.screens.push(Screen {
            id: screen_id,
            label: if name.is_empty() { format!("Screen {}", id + 1) } else { name },
            kind: ScreenKind::Screen,
            size: Size { w: w as u32, h: h as u32 },
            outputs,
            layers,
            transition: transition_of(sd, ctx.native_rate),
            extra,
        });
    }
}

fn parse_auxes(root: Node, show: &mut Show, ctx: &mut Ctx) {
    let Some(col) = at(root, "DestMgr/AuxDestCol") else { return };
    for ad in children(col, "AuxDest") {
        let Some(id) = id_num(ad) else { continue };
        let name = string(ad, "Name");
        let mut outputs = vec![];
        let mut extra = scalars(ad);
        extra.retain(|k, _| k != "Name");
        let format = video_format(ad);
        // An aux has one output; it may be named directly or through an output map.
        let mut out_indices: Vec<u32> = index(ad, "OutCfgIndex").into_iter().collect();
        if let Some(maps) = child(ad, "DestOutMapCol") {
            out_indices.extend(children(maps, "DestOutMap").filter_map(|m| index(m, "OutCfgIndex")));
        }
        for oi in out_indices {
            let out_id = ids::output(oi);
            let f = ctx.output_formats.get(&out_id).cloned().or_else(|| format.clone()).unwrap_or_default();
            outputs.push(OutputMap { output_id: out_id.clone(), rect: Rect::new(0.0, 0.0, f.width as f64, f.height as f64) });
            if let Some(o) = show.outputs.iter_mut().find(|o| o.id == out_id) {
                o.role = OutputRole::Aux;
            }
        }
        let size = outputs
            .first()
            .map(|o| Size { w: o.rect.w as u32, h: o.rect.h as u32 })
            .or_else(|| format.as_ref().map(|f| Size { w: f.width, h: f.height }))
            .unwrap_or_default();
        let pgm_src = ["PgmSrcIndex", "PgmSourceIndex", "LastAppliedSrcIdx", "SrcIndex"]
            .iter()
            .find_map(|k| index(ad, k))
            .map(ids::source);
        let mut layers = vec![LayerDef {
            id: ids::layer("aux"),
            label: "Aux".into(),
            kind: LayerKind::Mixer,
            z: 0,
            capacity: None,
            extra: Extra::new(),
        }];
        if let Some(s) = &pgm_src {
            extra.insert(
                "programState".into(),
                serde_json::to_value(showbook_model::PresetTarget {
                    screen_id: ids::aux(id),
                    background: None,
                    layers: vec![LayerState {
                        layer_id: ids::layer("aux"),
                        source_id: Some(s.clone()),
                        visible: true,
                        rect: None,
                        crop: None,
                        opacity: None,
                        border: None,
                        extra: Extra::new(),
                    }],
                    transition: None,
                })
                .unwrap_or_default(),
            );
        }
        layers.truncate(1);
        show.screens.push(Screen {
            id: ids::aux(id),
            label: if name.is_empty() { format!("Aux {}", id + 1) } else { name },
            kind: ScreenKind::Aux,
            size,
            outputs,
            layers,
            transition: transition_of(ad, ctx.native_rate),
            extra,
        });
    }
}

fn parse_multiviewers(root: Node, show: &mut Show, ctx: &mut Ctx) {
    let mut mvs: Vec<(u32, Node)> = vec![];
    if let Some(col) = child(root, "MultiViewerCollection") {
        for (k, mv) in children(col, "MultiViewer").enumerate() {
            mvs.push((id_num(mv).unwrap_or(k as u32), mv));
        }
    }
    // E2/S3 keep the multiviewer on the frame.
    if let Some(fc) = child(root, "FrameCollection") {
        for (fi, f) in children(fc, "Frame").enumerate() {
            for (k, mv) in children(f, "MultiViewer").enumerate() {
                mvs.push((100 * (fi as u32 + 1) + k as u32, mv));
            }
        }
    }
    for (n, mv) in mvs {
        let name = {
            let s = string(mv, "MVName");
            if s.is_empty() { format!("Multiviewer {}", n + 1) } else { s }
        };
        let mut output_ids = vec![];
        if let Some(oi) = index(mv, "OutCfgIndex") {
            let out_id = ids::output(oi);
            if let Some(o) = show.outputs.iter_mut().find(|o| o.id == out_id) {
                o.role = OutputRole::Multiviewer;
            }
            output_ids.push(out_id);
        }
        // E2: the outputs are the card's own configs, mapped as out:mvrN above.
        for o in &show.outputs {
            if o.role == OutputRole::Multiviewer && o.id.starts_with("out:mvr") && !output_ids.contains(&o.id) {
                output_ids.push(o.id.clone());
            }
        }
        let size = Size { w: index(mv, "HSize").unwrap_or(1920), h: index(mv, "VSize").unwrap_or(1080) };
        let mut layouts = vec![];
        for (li, lay) in children(mv, "MVLayout").enumerate() {
            let lid = id_num(lay).unwrap_or(li as u32);
            let mut widgets = vec![];
            if let Some(wc) = child(lay, "MVWinCollection") {
                for (wi, w) in children(wc, "MVWin").enumerate() {
                    let wid = id_num(w).unwrap_or(wi as u32);
                    let Some(r) = child(w, "Rect") else { continue };
                    let rect = Rect::new(
                        float(r, "HPos").unwrap_or(0.0),
                        float(r, "VPos").unwrap_or(0.0),
                        float(r, "HSize").unwrap_or(0.0),
                        float(r, "VSize").unwrap_or(0.0),
                    );
                    if rect.w <= 0.0 || rect.h <= 0.0 {
                        continue;
                    }
                    let source_id = widget_source(w, show);
                    let umd = text(w, "UMDText").map(str::to_string).filter(|s| !s.is_empty());
                    let mut extra = scalars(w);
                    extra.retain(|k, _| k != "UMDText");
                    widgets.push(Widget {
                        id: format!("w:{wid}"),
                        rect,
                        source_id,
                        label: umd,
                        show_label: int(w, "UmdEnable").map(|v| v != 0).unwrap_or(true),
                        tally: int(w, "TallyColor").map(|v| v != 0).unwrap_or(false),
                        extra,
                    });
                }
            }
            if widgets.is_empty() && li > 0 {
                continue;
            }
            layouts.push(MvLayout { id: ids::layout(n + 1, lid + 1), label: format!("Layout {}", lid + 1), size, widgets });
        }
        let active = index(mv, "LayoutSelect").map(|s| ids::layout(n + 1, s + 1));
        let mut extra = scalars(mv);
        extra.retain(|k, _| k != "MVName");
        show.multiviewers.push(Multiviewer { id: ids::multiviewer(n + 1), label: name, output_ids, layouts, active_layout: active, extra });
    }
    let _ = ctx;
}

/// A multiviewer window names what it shows by index; resolve that to the
/// matching source entry so the model has one spelling for "camera 1".
fn widget_source(w: Node, show: &Show) -> Option<String> {
    if let Some(i) = index(w, "InputCfgIndex") {
        let want = ids::input(i);
        if let Some(s) = show.sources.iter().find(|s| s.kind == SourceKind::Input && s.ref_id.as_deref() == Some(&want)) {
            return Some(s.id.clone());
        }
        return None;
    }
    if let Some(d) = index(w, "DestIndex") {
        let is_aux = int(w, "InputType") == Some(4);
        let want = if is_aux { ids::aux(d) } else { ids::screen(d) };
        if let Some(s) = show.sources.iter().find(|s| s.ref_id.as_deref() == Some(&want)) {
            return Some(s.id.clone());
        }
        return None;
    }
    None
}
