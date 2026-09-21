//! `presets/*.xml`, `cues/*.xml` and `userkey/*.xml`.
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
use showbook_model::{ids, Cue, CueStep, CueStepKind, Extra, LayerMemory, Note, NoteLevel, Preset, PresetTarget, Show, Size};

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


/// `userkey/*.xml` — Event Master **user keys**: a stored look applied to a
/// layer, which is what a LivePremier calls a layer memory.
///
/// `settings.xml` shows both ends of the feature: every `Layer` carries
/// `LastAppliedUserKeyIdx` beside `LastAppliedSrcIdx`, and every `Source`
/// carries `UserKeyIndex`, so a user key is applied to a layer and may be
/// bound to a source. The files themselves are **unverified** — neither
/// simulator had one saved — so this reads them the same way the preset
/// parser reads a preset: the name is the label, a `Layer`-shaped subtree
/// is read as the look, and everything else is kept in `extra` and reported.
pub(crate) fn parse_user_keys(store: &Store, show: &mut Show, ctx: &mut Ctx) {
    for (path, bytes) in store.under("userkey") {
        let text_ = String::from_utf8_lossy(bytes);
        let doc = match Document::parse(&text_) {
            Ok(d) => d,
            Err(e) => {
                ctx.notes.push(Note { level: NoteLevel::Dropped, path: path.into(), message: format!("unreadable user key file: {e}") });
                continue;
            }
        };
        let root = doc.root_element();
        let file_no: Option<u32> = path.rsplit('/').next().and_then(|f| f.trim_end_matches(".xml").parse().ok());
        let number = id_num(root).or(file_no);
        let label = ["Name", "UserKeyName"]
            .iter()
            .find_map(|k| text(root, k))
            .map(str::to_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("User key {}", number.map(|n| n + 1).unwrap_or(0)));
        // The look: the first Layer-shaped node, or the root itself when the
        // file is one layer's worth of adjustments with no wrapper.
        let layer_node = root
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "Layer")
            .or_else(|| root.descendants().find(|n| n.is_element() && n.tag_name().name() == "LayerCfg").and_then(|n| n.parent()))
            .unwrap_or(root);
        let st = layer_state_from_node(layer_node, "", ctx.native_rate);
        let read_anything = st.source_id.is_some() || st.rect.is_some() || st.crop.is_some() || st.opacity.is_some() || st.border.is_some();
        if !read_anything {
            ctx.notes.push(Note {
                level: NoteLevel::Adapted,
                path: format!("userKeys/{label}"),
                message: format!("{path}: no layer adjustments recognised; the user key is kept by name only"),
            });
        }
        let canvas = match (index(root, "HSize"), index(root, "VSize")) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some(Size { w, h }),
            _ => None,
        };
        let mut extra: Extra = scalars(root);
        extra.retain(|k, _| k != "Name");
        extra.insert("file".into(), path.into());
        show.layer_memories.push(LayerMemory {
            id: ids::layer_memory(number.unwrap_or(show.layer_memories.len() as u32)),
            number: number.map(|n| n + 1),
            label,
            state: read_anything.then_some(st),
            categories: vec![],
            canvas,
            source_id: None,
            extra,
        });
    }
    // Which sources carry a user key: `Source/UserKeyIndex` on the settings
    // side is read in `settings.rs` and left in the source's `extra`.
    let by_number: Vec<(String, Option<u32>)> = show.layer_memories.iter().map(|m| (m.id.clone(), m.number)).collect();
    let mut bound: Vec<(String, String)> = vec![];
    for src in &show.sources {
        let Some(idx) = src.extra.get("UserKeyIndex").and_then(|v| v.as_i64()) else { continue };
        if idx < 0 {
            continue;
        }
        if let Some((id, _)) = by_number.iter().find(|(_, n)| *n == Some(idx as u32 + 1)) {
            bound.push((id.clone(), src.id.clone()));
        }
    }
    for (mem_id, src_id) in bound {
        if let Some(m) = show.layer_memories.iter_mut().find(|m| m.id == mem_id) {
            m.source_id = Some(src_id);
        }
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

#[cfg(test)]
mod tests {
    use showbook_model::{Platform, Show};

    use crate::archive::Store;
    use crate::settings::Ctx;

    /// Structural, like the parser: no real user key file has been seen, so
    /// this feeds it the shapes `settings.xml` uses for a layer and checks
    /// that the look comes out the other side.
    #[test]
    fn reads_a_user_key_shaped_like_a_layer() {
        let xml = r#"<UserKey id="2"><Name>Lower third</Name><HSize>1920</HSize><VSize>1080</VSize>
            <Layer id="0"><LayerCfg id="0"><ActiveState>0</ActiveState>
              <LayerState id="0"><WinAdjust id="0">
                <OWIN id="0"><HPos>0</HPos><VPos>540</VPos><HSize>960</HSize><VSize>540</VSize></OWIN>
              </WinAdjust></LayerState>
            </LayerCfg></Layer></UserKey>"#;
        let mut store = Store::default();
        store.files.insert("userkey/2.xml".into(), xml.as_bytes().to_vec());
        let mut show = Show::new("t", Platform::BarcoEm);
        let mut ctx = Ctx::default();
        super::parse_user_keys(&store, &mut show, &mut ctx);
        assert_eq!(show.layer_memories.len(), 1);
        let m = &show.layer_memories[0];
        assert_eq!(m.label, "Lower third");
        assert_eq!(m.number, Some(3));
        let st = m.state.as_ref().expect("the look was read");
        let r = st.rect.expect("a rectangle");
        assert_eq!((r.x, r.y, r.w, r.h), (0.0, 540.0, 960.0, 540.0));
        assert_eq!(m.canvas.map(|c| (c.w, c.h)), Some((1920, 1080)));
    }

    /// A file it cannot read is kept by name, with a note — never silently.
    #[test]
    fn an_unreadable_user_key_is_kept_by_name() {
        let mut store = Store::default();
        store.files.insert("userkey/5.xml".into(), br#"<UserKey id="4"><Name>Odd</Name></UserKey>"#.to_vec());
        let mut show = Show::new("t", Platform::BarcoEm);
        let mut ctx = Ctx::default();
        super::parse_user_keys(&store, &mut show, &mut ctx);
        assert_eq!(show.layer_memories.len(), 1);
        assert_eq!(show.layer_memories[0].label, "Odd");
        assert!(show.layer_memories[0].state.is_none());
        assert!(ctx.notes.iter().any(|n| n.path.contains("userKeys/Odd")));
    }
}
