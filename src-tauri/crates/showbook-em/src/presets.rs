//! `presets/*.xml` and `cues/*.xml`.
//!
//! **Unverified against a real file.** Neither simulator store on this machine
//! had a preset or cue saved, so the parser is structural: a preset is
//! whatever the file's root element is, its `Name` is the label, and every
//! `Layer` element below a destination-looking ancestor (`ScreenDest`,
//! `AuxDest`, anything with `Dest` in its name and an `id`) becomes a layer
//! state on that screen, read with the same code that reads the live layers
//! in `settings.xml`. Anything else is kept in `extra` and reported in the
//! show's notes, so a first real backup will say exactly what it did not
//! understand.

use roxmltree::{Document, Node};
use showbook_model::{ids, Cue, CueStep, CueStepKind, Extra, Note, NoteLevel, Preset, PresetTarget, Show};

use crate::archive::Store;
use crate::settings::{layer_state_from_node, Ctx};
use crate::xml::*;

pub(crate) fn parse_presets(store: &Store, show: &mut Show, ctx: &mut Ctx) {
    for (path, bytes) in store.under("presets") {
        let text_ = String::from_utf8_lossy(bytes);
        let doc = match Document::parse(&text_) {
            Ok(d) => d,
            Err(e) => {
                ctx.notes.push(Note { level: NoteLevel::Dropped, path: path.into(), message: format!("unreadable preset file: {e}") });
                continue;
            }
        };
        let root = doc.root_element();
        let file_no: Option<u32> = path.rsplit('/').next().and_then(|f| f.trim_end_matches(".xml").parse().ok());
        let number = id_num(root).or(file_no);
        let label = ["Name", "PresetName", "presetName"]
            .iter()
            .find_map(|k| text(root, k))
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Preset {}", number.map(|n| n + 1).unwrap_or(0)));
        let mut targets: Vec<PresetTarget> = vec![];
        let mut unplaced = 0usize;
        for layer in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "Layer") {
            let Some(dest) = dest_ancestor(layer) else {
                unplaced += 1;
                continue;
            };
            let dest_id = dest_screen_id(dest);
            let Some(dest_id) = dest_id else {
                unplaced += 1;
                continue;
            };
            let n = id_num(layer).map(|i| i + 1).unwrap_or(1);
            let st = layer_state_from_node(layer, &ids::layer(n), ctx.native_rate);
            let t = match targets.iter_mut().find(|t| t.screen_id == dest_id) {
                Some(t) => t,
                None => {
                    targets.push(PresetTarget { screen_id: dest_id.clone(), background: None, layers: vec![], transition: None });
                    targets.last_mut().unwrap()
                }
            };
            t.layers.push(st);
        }
        // Background layers per destination.
        for bg in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "BGLayer") {
            if let Some(dest) = dest_ancestor(bg) {
                if let Some(dest_id) = dest_screen_id(dest) {
                    if let Some(t) = targets.iter_mut().find(|t| t.screen_id == dest_id) {
                        t.background = index(bg, "LastAppliedSrcIdx").map(ids::source);
                    }
                }
            }
        }
        if unplaced > 0 {
            ctx.notes.push(Note {
                level: NoteLevel::Dropped,
                path: format!("presets/{}", label),
                message: format!("{unplaced} layer entries in {path} had no destination ancestor and were skipped"),
            });
        }
        if targets.is_empty() {
            ctx.notes.push(Note {
                level: NoteLevel::Adapted,
                path: format!("presets/{}", label),
                message: format!("{path}: no layer content recognised; the preset is kept by name only"),
            });
        }
        let mut extra: Extra = scalars(root);
        extra.retain(|k, _| k != "Name");
        extra.insert("file".into(), path.into());
        show.presets.push(Preset { id: ids::preset(number.unwrap_or(show.presets.len() as u32)), number: number.map(|n| n + 1), label, notes: String::new(), targets, extra });
    }
}

fn dest_ancestor<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    n.ancestors().skip(1).find(|a| {
        let t = a.tag_name().name();
        a.is_element() && (t == "ScreenDest" || t == "AuxDest" || (t.contains("Dest") && !t.ends_with("Col") && a.attribute("id").is_some()))
    })
}

fn dest_screen_id(dest: Node) -> Option<String> {
    let idx = index(dest, "DestIndex").or_else(|| id_num(dest))?;
    Some(if dest.tag_name().name().starts_with("Aux") { ids::aux(idx) } else { ids::screen(idx) })
}

pub(crate) fn parse_cues(store: &Store, show: &mut Show, ctx: &mut Ctx) {
    for (path, bytes) in store.under("cues") {
        let text_ = String::from_utf8_lossy(bytes);
        let doc = match Document::parse(&text_) {
            Ok(d) => d,
            Err(e) => {
                ctx.notes.push(Note { level: NoteLevel::Dropped, path: path.into(), message: format!("unreadable cue file: {e}") });
                continue;
            }
        };
        let root = doc.root_element();
        let file_no: Option<u32> = path.rsplit('/').next().and_then(|f| f.trim_end_matches(".xml").parse().ok());
        let number = id_num(root).or(file_no);
        let label = ["Name", "CueName"]
            .iter()
            .find_map(|k| text(root, k))
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("Cue {}", number.map(|n| n + 1).unwrap_or(0)));
        let mut steps = vec![];
        for item in root.descendants().filter(|n| n.is_element() && n != &root) {
            let has_preset = index(item, "PresetIndex").is_some() || index(item, "PresetId").is_some();
            let delay = ["Delay", "DelayTime", "Wait"].iter().find_map(|k| float(item, k));
            if !has_preset && delay.is_none() {
                continue;
            }
            let mut extra = scalars(item);
            extra.insert("element".into(), item.tag_name().name().into());
            if has_preset {
                let p = index(item, "PresetIndex").or_else(|| index(item, "PresetId")).unwrap();
                steps.push(CueStep {
                    kind: CueStepKind::RecallPreset,
                    preset_id: Some(ids::preset(p)),
                    master_id: None,
                    screen_ids: vec![],
                    delay_ms: delay.map(|d| (d * 1000.0) as u32),
                    extra,
                });
            } else {
                steps.push(CueStep { kind: CueStepKind::Wait, preset_id: None, master_id: None, screen_ids: vec![], delay_ms: delay.map(|d| (d * 1000.0) as u32), extra });
            }
        }
        if steps.is_empty() {
            ctx.notes.push(Note { level: NoteLevel::Adapted, path: format!("cues/{label}"), message: format!("{path}: no steps recognised; the cue is kept by name only") });
        }
        let mut extra: Extra = scalars(root);
        extra.retain(|k, _| k != "Name");
        extra.insert("file".into(), path.into());
        show.cues.push(Cue { id: ids::cue(number.unwrap_or(show.cues.len() as u32)), number: number.map(|n| n + 1), label, steps, extra });
    }
}
