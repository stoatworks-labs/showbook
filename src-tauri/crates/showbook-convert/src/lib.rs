//! Converting a show between platforms.
//!
//! A conversion never invents hardware. It takes the show's logical graph —
//! inputs, sources, outputs, screens, layers, presets, master presets, layer
//! memories, cues, multiviewers, stills — re-keys it in the target platform's
//! own spelling,
//! and holds every part of it up against the target's [`Capabilities`]. What
//! fits is carried; what the target does differently is adapted to the
//! nearest thing it has and said so; what the target has no equivalent for is
//! dropped and said so. The report is the point: a converted show is a
//! starting point for the operator, not a claim of equivalence.
//!
//! Capability figures come from the vendors' published spec sheets (the same
//! ones the fleet's *Will my show fit?* cites); where a figure is a family
//! convention rather than a printed number it is marked in `notes`.
//!
//! ## What answers to what
//!
//! The families store the same handful of ideas under different names and in
//! different shapes. These are the equivalences the conversion works to;
//! every slot count was read from the device's own store (the LivePremier,
//! Midra 4K and Alta 4K simulators) or from the vendor's sheet.
//!
//! | Idea | Event Master | LivePremier | Midra 4K / Alta 4K |
//! |---|---|---|---|
//! | a destination's whole state | **preset** — may hold several destinations at once | **screen memory** — exactly one screen; auxes share the bank | **screen memory** + a separate **aux memory** bank |
//! | recalling several destinations together | the preset itself | **master memory** naming a memory per screen | **master memory** |
//! | a stored look for one layer | **user key** — `xml/userkey/`, applied to a layer, bindable to a source | **layer memory** — `layerBank`, 50 slots | none: a look travels inside a screen memory |
//! | a stored multiviewer layout | layouts on the multiviewer itself | 1 live + 50 in `monitoringBank` | 1 live + 20 in the multiviewer bank |
//! | a sequence | **cue** on the frame | none — Companion or a timeline | none |
//!
//! So a preset that touches three destinations becomes three memories and a
//! master memory going one way, and three memories plus a master collapse
//! back into one preset going the other; a user key and a layer memory are
//! the same object with two names; and an aux memory has to be counted
//! against its own bank on a Midra 4K, not against the screen bank.

pub mod capabilities;

use std::collections::BTreeMap;

use showbook_model::{
    ids, Extra, LayerDef, LayerKind, MasterEntry, MasterPreset, Note, NoteLevel, Platform, Preset, PresetTarget, ScreenKind,
    Show, SourceKind,
};

pub use capabilities::{capabilities, models, Capabilities};

#[derive(Debug, Clone)]
pub struct Conversion {
    pub show: Show,
    pub notes: Vec<Note>,
    /// Old id → new id, for anyone holding references (the UI's selection).
    pub id_map: BTreeMap<String, String>,
}

fn note(notes: &mut Vec<Note>, level: NoteLevel, path: impl Into<String>, message: impl Into<String>) {
    notes.push(Note { level, path: path.into(), message: message.into() });
}

/// Convert `show` for `target`/`model`. The source show is not modified.
pub fn convert(show: &Show, target: Platform, model: &str) -> Conversion {
    let cap = capabilities(target, model);
    let mut notes = vec![];
    let mut out = show.clone();
    let mut id_map: BTreeMap<String, String> = BTreeMap::new();

    out.id = uuid::Uuid::new_v4().to_string();
    out.platform = target;
    out.meta.name = format!("{} ({})", show.meta.name, cap.model);
    out.meta.created = showbook_model::now();
    out.meta.source = None;
    out.system.model = cap.model.clone();
    out.system.firmware = String::new();
    out.system.name = String::new();
    out.system.frames.clear();
    out.system.extra = Extra::new();
    out.system.extra.insert("convertedFrom".into(), serde_json::json!({
        "platform": show.platform, "model": show.system.model, "firmware": show.system.firmware, "showId": show.id, "at": showbook_model::now(),
    }));
    out.vendor.clear();
    out.notes.clear();
    if !show.vendor.is_empty() {
        note(&mut notes, NoteLevel::Dropped, "vendor", format!("{} vendor file(s) belong to the {} and do not come across; the target gets a new one when it is first saved to hardware", show.vendor.len(), show.platform.label()));
    }
    note(&mut notes, NoteLevel::Adapted, "system", format!("chassis, cards and connectors are the {}'s; inputs and outputs keep their labels and formats and need patching to the {}'s connectors", show.platform.label(), cap.model));
    for i in &mut out.inputs {
        i.connector_ids.clear();
    }
    for o in &mut out.outputs {
        o.connector_ids.clear();
        if o.test_pattern.take().is_some() {
            note(&mut notes, NoteLevel::Adapted, format!("outputs/{}", o.id), "output test pattern reset; pattern names differ between platforms");
        }
    }

    // ---- capacity checks ------------------------------------------------
    let screens: Vec<String> = out.screens.iter().filter(|s| s.kind == ScreenKind::Screen).map(|s| s.id.clone()).collect();
    let auxes: Vec<String> = out.screens.iter().filter(|s| s.kind == ScreenKind::Aux).map(|s| s.id.clone()).collect();
    if screens.len() as u32 > cap.screens {
        note(&mut notes, NoteLevel::Dropped, "screens", format!("{} screens; the {} has {}. The last {} are dropped.", screens.len(), cap.model, cap.screens, screens.len() as u32 - cap.screens));
        let keep: Vec<String> = screens.iter().take(cap.screens as usize).cloned().collect();
        out.screens.retain(|s| s.kind != ScreenKind::Screen || keep.contains(&s.id));
    }
    if auxes.len() as u32 > cap.auxes {
        note(&mut notes, NoteLevel::Dropped, "auxes", format!("{} auxiliary screens; the {} has {}. The last {} are dropped.", auxes.len(), cap.model, cap.auxes, auxes.len() as u32 - cap.auxes));
        let keep: Vec<String> = auxes.iter().take(cap.auxes as usize).cloned().collect();
        out.screens.retain(|s| s.kind != ScreenKind::Aux || keep.contains(&s.id));
    }
    if out.inputs.len() as u32 > cap.inputs {
        note(&mut notes, NoteLevel::Dropped, "inputs", format!("{} inputs; the {} takes up to {}. Inputs beyond that are kept in the model but have nowhere to plug in.", out.inputs.len(), cap.model, cap.inputs));
    }
    if out.outputs.len() as u32 > cap.outputs {
        note(&mut notes, NoteLevel::Dropped, "outputs", format!("{} outputs; the {} has up to {}. Outputs beyond that are kept in the model but have nowhere to plug in.", out.outputs.len(), cap.model, cap.outputs));
    }
    // Sources whose kind the target cannot offer.
    let kept_screen_ids: Vec<String> = out.screens.iter().map(|s| s.id.clone()).collect();
    out.sources.retain(|s| match (&s.kind, &s.ref_id) {
        (SourceKind::Screen, Some(r)) | (SourceKind::Aux, Some(r)) => kept_screen_ids.contains(r),
        _ => true,
    });

    // ---- layers per screen and layer features -----------------------------
    let mut dropped_layers: Vec<(String, String)> = vec![];
    for sc in &mut out.screens {
        let mut mixers = 0u32;
        let mut keep: Vec<LayerDef> = vec![];
        for l in sc.layers.drain(..) {
            match l.kind {
                LayerKind::Background => {
                    if cap.background_layer {
                        keep.push(l);
                    } else {
                        dropped_layers.push((sc.id.clone(), l.id.clone()));
                    }
                }
                LayerKind::Key if l.id == ids::layer("dsk") => {
                    if cap.dsk {
                        keep.push(l);
                    } else {
                        dropped_layers.push((sc.id.clone(), l.id.clone()));
                    }
                }
                _ => {
                    let per = if sc.kind == ScreenKind::Aux { cap.aux_layers } else { cap.layers_per_screen };
                    if mixers < per {
                        mixers += 1;
                        keep.push(l);
                    } else {
                        dropped_layers.push((sc.id.clone(), l.id.clone()));
                    }
                }
            }
        }
        sc.layers = keep;
        sc.extra.remove("programState");
        sc.extra.remove("previewState");
        sc.extra.remove("programLetter");
        sc.extra.remove("previewLetter");
    }
    for (sid, lid) in &dropped_layers {
        let sc_label = out.screen(sid).map(|s| s.label.clone()).unwrap_or_default();
        let what = if lid == "layer:dsk" {
            format!("{} has no DSK layer", cap.model)
        } else if lid == "layer:bg" || lid == "layer:NATIVE" {
            format!("{} has no background layer", cap.model)
        } else {
            format!("{} allows {} mixing layers on a screen", cap.model, cap.layers_per_screen)
        };
        note(&mut notes, NoteLevel::Dropped, format!("screens/{sid}/layers/{lid}"), format!("{sc_label}: {lid} dropped — {what}"));
    }
    // Layer budget in 4K units.
    if cap.layers_4k > 0.0 {
        let cost: f64 = out
            .screens
            .iter()
            .flat_map(|s| s.layers.iter().map(move |l| (s.kind, l)))
            .filter(|(k, l)| l.kind == LayerKind::Mixer && (*k == ScreenKind::Screen || cap.aux_layers_cost))
            .map(|(_, l)| capabilities::layer_cost(l.capacity.as_deref()))
            .sum();
        if cost > cap.layers_4k {
            note(&mut notes, NoteLevel::Dropped, "layers", format!("the show's mixing layers add up to {cost:.1} 4K-layer units; the {} has {:.0}. Some screens will not get all their layers — reallocate on the target.", cap.model, cap.layers_4k));
        } else {
            note(&mut notes, NoteLevel::Info, "layers", format!("mixing layers add up to {cost:.1} of the {}'s {:.0} 4K-layer units", cap.model, cap.layers_4k));
        }
    }

    // ---- presets -----------------------------------------------------------
    let mut feature_drops: BTreeMap<&str, usize> = BTreeMap::new();
    let screen_layers: BTreeMap<String, Vec<String>> = out.screens.iter().map(|s| (s.id.clone(), s.layers.iter().map(|l| l.id.clone()).collect())).collect();
    for p in &mut out.presets {
        p.targets.retain(|t| screen_layers.contains_key(&t.screen_id));
        for t in &mut p.targets {
            let allowed = &screen_layers[&t.screen_id];
            let before = t.layers.len();
            t.layers.retain(|l| allowed.contains(&l.layer_id));
            if t.layers.len() < before {
                *feature_drops.entry("layer states on dropped layers").or_default() += before - t.layers.len();
            }
            if t.background.is_some() && !cap.background_layer {
                t.background = None;
                *feature_drops.entry("background source").or_default() += 1;
            }
            for l in &mut t.layers {
                if !cap.border && l.border.take().is_some() {
                    *feature_drops.entry("layer border").or_default() += 1;
                }
                if !cap.crop && l.crop.take().is_some() {
                    *feature_drops.entry("layer crop").or_default() += 1;
                }
                if !cap.opacity && l.opacity.take().is_some() {
                    *feature_drops.entry("layer opacity").or_default() += 1;
                }
                if l.extra.remove("keying").is_some() && !cap.keying {
                    *feature_drops.entry("keying").or_default() += 1;
                }
                if l.extra.remove("shadow").is_some() && !cap.shadow {
                    *feature_drops.entry("drop shadow").or_default() += 1;
                }
                // Vendor-specific leftovers never carry.
                l.extra.clear();
                if let Some(r) = &l.rect {
                    if let Some(sc) = out.screens.iter().find(|s| s.id == t.screen_id) {
                        if r.x + r.w > sc.size.w as f64 + 0.5 || r.y + r.h > sc.size.h as f64 + 0.5 || r.x < -0.5 || r.y < -0.5 {
                            *feature_drops.entry("layer partly off canvas").or_default() += 1;
                        }
                    }
                }
            }
        }
        p.extra.clear();
    }
    for (what, n) in &feature_drops {
        let level = if what.ends_with("off canvas") { NoteLevel::Adapted } else { NoteLevel::Dropped };
        note(&mut notes, level, "presets", format!("{n} × {what}: the {} does not carry this over", cap.model));
    }

    // Preset numbering and the master/multi-target shape.
    reshape_presets(&mut out, &cap, &mut notes);
    check_preset_slots(&mut out, &cap, &mut notes);
    reshape_layer_memories(&mut out, &cap, show.platform, &mut notes);
    if !cap.cues && !out.cues.is_empty() {
        note(&mut notes, NoteLevel::Dropped, "cues", format!("{} cues: the {} has no cue list. Their preset order is kept in the notes of the show.", out.cues.len(), cap.model));
        let listing: Vec<String> = out.cues.iter().map(|c| format!("{}: {}", c.label, c.steps.iter().filter_map(|s| s.preset_id.clone()).collect::<Vec<_>>().join(", "))).collect();
        if !listing.is_empty() {
            out.meta.notes = format!("{}\n\nCues from the {} show:\n{}", out.meta.notes, show.platform.label(), listing.join("\n")).trim().to_string();
        }
        out.cues.clear();
    }

    // ---- multiviewers and stills ---------------------------------------------
    if out.multiviewers.len() as u32 > cap.multiviewers {
        note(&mut notes, NoteLevel::Dropped, "multiviewers", format!("{} multiviewers; the {} has {}.", out.multiviewers.len(), cap.model, cap.multiviewers));
        out.multiviewers.truncate(cap.multiviewers as usize);
        let kept: Vec<String> = out.multiviewers.iter().map(|m| m.id.clone()).collect();
        out.sources.retain(|s| !(s.kind == SourceKind::Multiviewer && s.ref_id.as_ref().map(|r| !kept.contains(r)).unwrap_or(false)));
    }
    for mv in &mut out.multiviewers {
        mv.output_ids.clear();
        let room = cap.mv_layouts + cap.mv_memories;
        if mv.layouts.len() as u32 > room {
            note(&mut notes, NoteLevel::Dropped, format!("multiviewers/{}", mv.id), format!("{} layouts; the {} holds {} live and {} in its bank.", mv.layouts.len(), cap.model, cap.mv_layouts, cap.mv_memories));
            mv.layouts.truncate(room as usize);
            if let Some(a) = &mv.active_layout {
                if !mv.layouts.iter().any(|l| &l.id == a) {
                    mv.active_layout = mv.layouts.first().map(|l| l.id.clone());
                }
            }
        } else if mv.layouts.len() as u32 > cap.mv_layouts && cap.mv_memories > 0 {
            // Not a loss: the layouts past the live one become memories in
            // the target's multiviewer bank and are recalled from there.
            note(
                &mut notes,
                NoteLevel::Adapted,
                format!("multiviewers/{}", mv.id),
                format!(
                    "{} layouts: the {} shows one at a time, so {} of them become multiviewer memories in its bank of {}",
                    mv.layouts.len(),
                    cap.model,
                    mv.layouts.len() as u32 - cap.mv_layouts,
                    cap.mv_memories
                ),
            );
        }
        for lay in &mut mv.layouts {
            if lay.widgets.len() as u32 > cap.widgets_per_mv {
                note(&mut notes, NoteLevel::Dropped, format!("multiviewers/{}/{}", mv.id, lay.id), format!("{} windows; the {} draws up to {}.", lay.widgets.len(), cap.model, cap.widgets_per_mv));
                lay.widgets.truncate(cap.widgets_per_mv as usize);
            }
            for w in &mut lay.widgets {
                w.extra.clear();
            }
        }
        mv.extra.clear();
    }
    if out.stills.len() as u32 > cap.still_slots {
        note(&mut notes, NoteLevel::Dropped, "stills", format!("{} stills; the {} holds {}.", out.stills.len(), cap.model, cap.still_slots));
    }
    if !out.stills.is_empty() {
        note(&mut notes, NoteLevel::Adapted, "stills", "still images are listed by name; the files themselves have to be loaded on the target");
    }

    // ---- re-key everything in the target's spelling -----------------------------
    rekey(&mut out, target, &mut id_map);
    for n in &cap.notes {
        note(&mut notes, NoteLevel::Info, "target", n.clone());
    }
    out.notes = notes.clone();
    Conversion { show: out, notes, id_map }
}

/// Presets and master presets have different shapes per family:
/// - Event Master: a preset can hold several destinations; no master bank.
/// - LivePremier: a memory is one screen's layers; a master memory names a
///   memory per screen.
fn reshape_presets(out: &mut Show, cap: &Capabilities, notes: &mut Vec<Note>) {
    let multi_target = cap.preset_multi_screen;
    if multi_target {
        // Masters become multi-target presets built from their members.
        if !out.master_presets.is_empty() && cap.master_slots == 0 {
            let mut made = 0;
            for m in out.master_presets.clone() {
                let mut targets: Vec<PresetTarget> = vec![];
                for e in &m.entries {
                    if let Some(p) = out.presets.iter().find(|p| p.id == e.preset_id) {
                        for t in &p.targets {
                            if t.screen_id == e.screen_id {
                                targets.push(t.clone());
                            }
                        }
                        if !p.targets.iter().any(|t| t.screen_id == e.screen_id) {
                            targets.push(PresetTarget { screen_id: e.screen_id.clone(), background: None, layers: vec![], transition: None });
                        }
                    }
                }
                out.presets.push(Preset {
                    id: ids::preset(format!("m{}", ids::tail(&m.id))),
                    number: None,
                    label: format!("{} (master)", m.label),
                    notes: format!("Built from master memory {}: {}", m.label, m.entries.iter().map(|e| format!("{} ← {}", e.screen_id, e.preset_id)).collect::<Vec<_>>().join(", ")),
                    targets,
                    extra: Extra::new(),
                });
                made += 1;
            }
            note(notes, NoteLevel::Adapted, "masterPresets", format!("{made} master memories became presets that recall several screens at once — the {}'s way of doing the same thing", cap.model));
            out.master_presets.clear();
        }
    } else {
        // A preset that touches several screens becomes one memory per
        // screen plus a master memory that recalls them together.
        let multi: Vec<Preset> = out.presets.iter().filter(|p| p.targets.len() > 1).cloned().collect();
        if !multi.is_empty() {
            let mut next = out.presets.iter().filter_map(|p| p.number).max().unwrap_or(0);
            for p in multi {
                let mut entries = vec![];
                for t in &p.targets {
                    next += 1;
                    let id = ids::preset(format!("{}.{}", ids::tail(&p.id), ids::tail(&t.screen_id)));
                    out.presets.push(Preset {
                        id: id.clone(),
                        number: Some(next),
                        label: format!("{} · {}", p.label, out.screen(&t.screen_id).map(|s| s.label.clone()).unwrap_or_default()),
                        notes: format!("Split from preset {}", p.label),
                        targets: vec![t.clone()],
                        extra: Extra::new(),
                    });
                    entries.push(MasterEntry { screen_id: t.screen_id.clone(), preset_id: id });
                }
                if cap.master_slots > 0 {
                    out.master_presets.push(MasterPreset { id: ids::master(format!("p{}", ids::tail(&p.id))), number: None, label: p.label.clone(), entries, extra: Extra::new() });
                }
                out.presets.retain(|x| x.id != p.id);
            }
            note(notes, NoteLevel::Adapted, "presets", format!("presets that recalled several screens at once were split into one memory per screen{}", if cap.master_slots > 0 { " and a master memory that recalls them together" } else { "" }));
        }
        if out.master_presets.len() as u32 > cap.master_slots {
            note(notes, NoteLevel::Dropped, "masterPresets", format!("{} master memories; the {} has {} slots.", out.master_presets.len(), cap.model, cap.master_slots));
            out.master_presets.truncate(cap.master_slots as usize);
        }
    }
}


/// Presets against the target's banks. Most platforms keep one bank for
/// every destination; the Midra 4K and the Alta 4K keep auxiliary memories
/// in a bank of their own, so the two are counted separately or a show with
/// 150 screen memories and 150 aux memories reads as over capacity when it
/// fits twice over.
fn check_preset_slots(out: &mut Show, cap: &Capabilities, notes: &mut Vec<Note>) {
    let is_aux = |p: &Preset, out: &Show| p.targets.iter().all(|t| out.screen(&t.screen_id).map(|s| s.kind == ScreenKind::Aux).unwrap_or(false)) && !p.targets.is_empty();
    match cap.aux_preset_slots {
        None => {
            if out.presets.len() as u32 > cap.preset_slots {
                note(notes, NoteLevel::Dropped, "presets", format!("{} presets; the {} has {} slots. Presets past the last slot are dropped.", out.presets.len(), cap.model, cap.preset_slots));
                out.presets.truncate(cap.preset_slots as usize);
            }
        }
        Some(aux_slots) => {
            let screen_side: Vec<String> = out.presets.iter().filter(|p| !is_aux(p, out)).map(|p| p.id.clone()).collect();
            let aux_side: Vec<String> = out.presets.iter().filter(|p| is_aux(p, out)).map(|p| p.id.clone()).collect();
            let mut drop: Vec<String> = vec![];
            if screen_side.len() as u32 > cap.preset_slots {
                note(notes, NoteLevel::Dropped, "presets", format!("{} screen memories; the {} has {} screen slots. The last {} are dropped.", screen_side.len(), cap.model, cap.preset_slots, screen_side.len() as u32 - cap.preset_slots));
                drop.extend(screen_side.iter().skip(cap.preset_slots as usize).cloned());
            }
            if aux_side.len() as u32 > aux_slots {
                note(notes, NoteLevel::Dropped, "presets", format!("{} auxiliary memories; the {} keeps them in a bank of {}. The last {} are dropped.", aux_side.len(), cap.model, aux_slots, aux_side.len() as u32 - aux_slots));
                drop.extend(aux_side.iter().skip(aux_slots as usize).cloned());
            }
            if !drop.is_empty() {
                out.presets.retain(|p| !drop.contains(&p.id));
                for m in out.master_presets.iter_mut() {
                    m.entries.retain(|e| !drop.contains(&e.preset_id));
                }
            }
            if !aux_side.is_empty() {
                note(notes, NoteLevel::Info, "presets", format!("the {} keeps auxiliary memories in their own bank: {} of the show's memories are auxiliary and are counted against it, not against the {} screen slots", cap.model, aux_side.len(), cap.preset_slots));
            }
        }
    }
}

/// Layer memories — Event Master user keys, LivePremier layer memories — are
/// the same object under two names: one layer's look, applied to whichever
/// layer the operator picks. A platform either has the bank or it does not.
/// Where it does not, the look is not silently lost: it is written into the
/// show's notes so the operator can rebuild it, because the layers that used
/// it carry their own copy of the look inside each preset anyway.
fn reshape_layer_memories(out: &mut Show, cap: &Capabilities, from: Platform, notes: &mut Vec<Note>) {
    if out.layer_memories.is_empty() {
        return;
    }
    let name_here = layer_memory_name(cap.platform);
    let name_there = layer_memory_name(from);
    match cap.layer_memory_slots {
        Some(0) => {
            let listing: Vec<String> = out
                .layer_memories
                .iter()
                .map(|m| {
                    let what = if m.categories.is_empty() { String::new() } else { format!(" ({})", m.categories.join(", ").to_lowercase()) };
                    format!("{}{}", m.label, what)
                })
                .collect();
            note(
                notes,
                NoteLevel::Dropped,
                "layerMemories",
                format!(
                    "{}: the {} has no layer bank, so a look is saved inside a screen memory instead. {} kept in the show's notes.",
                    counted(out.layer_memories.len(), name_there),
                    cap.model,
                    if out.layer_memories.len() == 1 { "Its name is" } else { "Their names are" }
                ),
            );
            out.meta.notes = format!("{}\n\n{} from the {} show, which the {} has nowhere to keep:\n{}", out.meta.notes, counted(out.layer_memories.len(), name_there), from.label(), cap.model, listing.join("\n")).trim().to_string();
            out.layer_memories.clear();
        }
        slots => {
            if let Some(n) = slots {
                if out.layer_memories.len() as u32 > n {
                    note(notes, NoteLevel::Dropped, "layerMemories", format!("{}; the {} has {n} {name_here} slots. The last {} are dropped.", counted(out.layer_memories.len(), name_there), cap.model, out.layer_memories.len() as u32 - n));
                    out.layer_memories.truncate(n as usize);
                }
            }
            if cap.platform != from {
                note(
                    notes,
                    NoteLevel::Adapted,
                    "layerMemories",
                    format!(
                        "what the {} calls {name_there}, the {} calls {name_here}: the same idea — one layer's look, applied to whichever layer you pick. {} carried over.",
                        from.label(),
                        cap.model,
                        counted(out.layer_memories.len(), name_there)
                    ),
                );
            }
            // A memory read from a bank carries only what the device told us.
            let metadata_only = out.layer_memories.iter().filter(|m| m.state.is_none()).count();
            if metadata_only > 0 {
                note(notes, NoteLevel::Adapted, "layerMemories", format!("{} came off the device as a name and a category list only — the values live on the hardware, so the slots carry over empty and have to be saved again on the target", if metadata_only == 1 { "one of them".to_string() } else { format!("{metadata_only} of them") }));
            }
            for m in out.layer_memories.iter_mut() {
                // A look saved on one canvas lands somewhere else on another.
                if let Some(st) = &mut m.state {
                    if !cap.border {
                        st.border = None;
                    }
                    if !cap.crop {
                        st.crop = None;
                    }
                    if !cap.opacity {
                        st.opacity = None;
                    }
                    st.extra.clear();
                }
                // Binding a look to a source is an Event Master idea.
                if m.source_id.is_some() && !matches!(cap.platform, Platform::BarcoEm | Platform::BarcoPds4k | Platform::Generic) {
                    m.source_id = None;
                    note(notes, NoteLevel::Dropped, format!("layerMemories/{}", m.id), format!("the {} cannot bind a {name_here} to a source; the look stays, the binding does not", cap.model));
                }
                m.extra.clear();
            }
        }
    }
}

/// What the platform calls a layer memory, for the notes.
fn layer_memory_name(p: Platform) -> &'static str {
    match p {
        Platform::BarcoEm | Platform::BarcoPds4k => "user keys",
        _ => "layer memories",
    }
}

/// `1 user key`, `3 user keys` — the notes are read by people.
fn counted(n: usize, name: &str) -> String {
    let one = match name {
        "user keys" => "user key",
        "layer memories" => "layer memory",
        other => other,
    };
    if n == 1 { format!("1 {one}") } else { format!("{n} {name}") }
}

/// Renumber every entity the way the target driver keys it, and fix every
/// reference. Order is preserved.
fn rekey(out: &mut Show, target: Platform, id_map: &mut BTreeMap<String, String>) {
    let aw = matches!(target, Platform::AwLivePremier | Platform::AwMidra4k | Platform::AwAlta4k);
    fn map(id_map: &mut BTreeMap<String, String>, old: &str, new: String) -> String {
        id_map.insert(old.to_string(), new.clone());
        new
    }
    // Inputs
    for (i, inp) in out.inputs.iter_mut().enumerate() {
        let new = if aw { ids::input(format!("IN_{}", i + 1)) } else { ids::input(i) };
        inp.id = map(id_map, &inp.id, new);
    }
    for (i, o) in out.outputs.iter_mut().enumerate() {
        let new = if aw { ids::output(i + 1) } else { ids::output(i) };
        o.id = map(id_map, &o.id, new);
    }
    for (i, st) in out.stills.iter_mut().enumerate() {
        let new = if aw { ids::still(format!("STILL_{}", i + 1)) } else { ids::still(i) };
        st.id = map(id_map, &st.id, new);
    }
    let mut sn = 0;
    let mut an = 0;
    for sc in out.screens.iter_mut() {
        let new = if sc.kind == ScreenKind::Screen {
            sn += 1;
            if aw { ids::screen(format!("S{sn}")) } else { ids::screen(sn - 1) }
        } else {
            an += 1;
            if aw { ids::aux(format!("A{an}")) } else { ids::aux(an - 1) }
        };
        sc.id = map(id_map, &sc.id, new);
        let mut ln = 0;
        for l in sc.layers.iter_mut() {
            let new = match l.kind {
                LayerKind::Background => if aw { ids::layer("NATIVE") } else { ids::layer("bg") },
                LayerKind::Key if l.id == ids::layer("dsk") => ids::layer("dsk"),
                _ => {
                    ln += 1;
                    ids::layer(ln)
                }
            };
            // Layer ids are scoped to the screen: map them with the screen prefix.
            id_map.insert(format!("{}/{}", sc.id, l.id), new.clone());
            l.id = new;
        }
    }
    for (i, p) in out.presets.iter_mut().enumerate() {
        let n = i as u32 + if aw { 1 } else { 0 };
        p.number = Some(if aw { n } else { n + 1 });
        p.id = map(id_map, &p.id, ids::preset(n));
    }
    for (i, m) in out.master_presets.iter_mut().enumerate() {
        let n = i as u32 + if aw { 1 } else { 0 };
        m.number = Some(if aw { n } else { n + 1 });
        m.id = map(id_map, &m.id, ids::master(n));
    }
    for (i, m) in out.layer_memories.iter_mut().enumerate() {
        let n = i as u32 + 1;
        m.number = Some(n);
        m.id = map(id_map, &m.id, ids::layer_memory(n));
    }
    for (i, c) in out.cues.iter_mut().enumerate() {
        c.id = map(id_map, &c.id, ids::cue(i));
    }
    for (i, mv) in out.multiviewers.iter_mut().enumerate() {
        let n = i + 1;
        mv.id = map(id_map, &mv.id, ids::multiviewer(n));
        for (j, l) in mv.layouts.iter_mut().enumerate() {
            let new = ids::layout(n, j + 1);
            id_map.insert(l.id.clone(), new.clone());
            l.id = new;
        }
        if let Some(a) = &mv.active_layout {
            mv.active_layout = id_map.get(a).cloned().or_else(|| mv.layouts.first().map(|l| l.id.clone()));
        }
    }
    // Sources: keyed by what they reference where possible.
    for (i, s) in out.sources.iter_mut().enumerate() {
        if let Some(r) = &s.ref_id {
            if let Some(new_ref) = id_map.get(r) {
                s.ref_id = Some(new_ref.clone());
            }
        }
        let new = if aw {
            match (&s.kind, &s.ref_id) {
                (SourceKind::Input, Some(r)) => ids::source(ids::tail(r)),
                (SourceKind::Still, Some(r)) => ids::source(ids::tail(r)),
                (SourceKind::Screen, Some(r)) | (SourceKind::Aux, Some(r)) => ids::source(format!("PROGRAM_{}", ids::tail(r))),
                _ => ids::source(format!("X{}", i + 1)),
            }
        } else {
            ids::source(i)
        };
        s.id = map(id_map, &s.id, new);
    }
    // Now every reference.
    let m = id_map.clone();
    let fix = |id: &mut String| {
        if let Some(n) = m.get(id.as_str()) {
            *id = n.clone();
        }
    };
    for sc in out.screens.iter_mut() {
        for om in sc.outputs.iter_mut() {
            fix(&mut om.output_id);
        }
    }
    let fix_target = |t: &mut PresetTarget, m: &BTreeMap<String, String>| {
        let old_screen = t.screen_id.clone();
        if let Some(n) = m.get(&old_screen) {
            t.screen_id = n.clone();
        }
        if let Some(b) = &mut t.background {
            if let Some(n) = m.get(b.as_str()) {
                *b = n.clone();
            }
        }
        for l in t.layers.iter_mut() {
            if let Some(n) = m.get(&format!("{}/{}", t.screen_id, l.layer_id)) {
                l.layer_id = n.clone();
            }
            if let Some(s) = &mut l.source_id {
                if let Some(n) = m.get(s.as_str()) {
                    *s = n.clone();
                }
            }
        }
    };
    for p in out.presets.iter_mut() {
        for t in p.targets.iter_mut() {
            fix_target(t, &m);
        }
    }
    for mp in out.master_presets.iter_mut() {
        for e in mp.entries.iter_mut() {
            fix(&mut e.screen_id);
            fix(&mut e.preset_id);
        }
    }
    for c in out.cues.iter_mut() {
        for st in c.steps.iter_mut() {
            if let Some(p) = &mut st.preset_id {
                fix(p);
            }
            if let Some(p) = &mut st.master_id {
                fix(p);
            }
            for s in st.screen_ids.iter_mut() {
                fix(s);
            }
        }
    }
    for lm in out.layer_memories.iter_mut() {
        if let Some(src) = &mut lm.source_id {
            fix(src);
        }
        if let Some(st) = &mut lm.state {
            if let Some(src) = &mut st.source_id {
                fix(src);
            }
        }
    }
    for mv in out.multiviewers.iter_mut() {
        for l in mv.layouts.iter_mut() {
            for w in l.widgets.iter_mut() {
                if let Some(s) = &mut w.source_id {
                    fix(s);
                }
            }
        }
    }
    if let Some(g) = &mut out.system.genlock {
        if let Some(i) = &mut g.input_id {
            fix(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use showbook_model::*;

    fn em_show() -> Show {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/em/e3-sim-10.0.2");
        let mut show = showbook_em::import_path(&p).unwrap();
        // Give it a preset that touches two screens, with a border and a crop.
        let s0 = show.screens[0].id.clone();
        let s1 = show.screens[1].id.clone();
        let src = show.sources[0].id.clone();
        show.presets.push(Preset {
            id: ids::preset(0),
            number: Some(1),
            label: "Walk in".into(),
            notes: String::new(),
            targets: vec![
                PresetTarget {
                    screen_id: s0,
                    background: None,
                    layers: vec![LayerState {
                        layer_id: ids::layer(1),
                        source_id: Some(src.clone()),
                        visible: true,
                        rect: Some(Rect::new(0.0, 0.0, 960.0, 540.0)),
                        crop: Some(Rect::new(0.0, 0.0, 1920.0, 1080.0)),
                        opacity: Some(0.8),
                        border: Some(Border { width: 4, color: "#ffffff".into() }),
                        extra: Extra::new(),
                    }],
                    transition: None,
                },
                PresetTarget { screen_id: s1, background: None, layers: vec![], transition: None },
            ],
            extra: Extra::new(),
        });
        show
    }

    #[test]
    fn e3_to_aquilon_splits_multi_screen_presets() {
        let show = em_show();
        let c = convert(&show, Platform::AwLivePremier, "Aquilon C");
        assert_eq!(c.show.platform, Platform::AwLivePremier);
        // Six screens → S1..S6, 8 mixing layers each stays (Aquilon allows 16).
        assert!(c.show.screens.iter().all(|s| s.id.starts_with("scr:S")));
        assert_eq!(c.show.screens[0].layers.iter().filter(|l| l.kind == LayerKind::Mixer).count(), 8);
        // The DSK layer has no Aquilon equivalent.
        assert!(c.notes.iter().any(|n| n.level == NoteLevel::Dropped && n.path.contains("layer:dsk")));
        // The two-screen preset became two memories + one master.
        assert_eq!(c.show.presets.len(), 2);
        assert_eq!(c.show.master_presets.len(), 1);
        assert_eq!(c.show.master_presets[0].entries.len(), 2);
        assert!(c.show.validate().is_empty(), "{:?}", c.show.validate());
        // Ids are the target's spelling and references were rewritten.
        assert!(c.show.presets.iter().all(|p| p.targets.iter().all(|t| t.screen_id.starts_with("scr:S"))));
        assert!(c.show.inputs.iter().all(|i| i.id.starts_with("in:IN_")));
    }

    #[test]
    fn aquilon_to_e3_builds_multi_target_presets() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/aw/livepremier-sim-6.2.73-store.json");
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
        let mut show = showbook_aw::store::parse(&v).unwrap();
        // Give memory 2 some content so the master has something to gather.
        let s1 = show.screens[0].id.clone();
        show.presets[1].targets.push(PresetTarget { screen_id: s1, background: None, layers: vec![], transition: None });
        let c = convert(&show, Platform::BarcoEm, "Encore3");
        assert_eq!(c.show.platform, Platform::BarcoEm);
        assert!(c.show.master_presets.is_empty());
        // Two memories + one preset built from the master.
        assert_eq!(c.show.presets.len(), 3);
        assert!(c.show.presets.iter().any(|p| p.label.ends_with("(master)")));
        assert!(c.notes.iter().any(|n| n.path == "masterPresets" && n.level == NoteLevel::Adapted));
        assert!(c.show.validate().is_empty(), "{:?}", c.show.validate());
    }


    /// A user key and a layer memory are the same object with two names; a
    /// Midra 4K has neither, and says so instead of losing it quietly.
    #[test]
    fn user_keys_and_layer_memories_are_the_same_object() {
        let mut show = em_show();
        let src = show.sources[0].id.clone();
        show.layer_memories.push(LayerMemory {
            id: ids::layer_memory(0),
            number: Some(1),
            label: "Lower third".into(),
            state: Some(LayerState {
                layer_id: String::new(),
                source_id: Some(src.clone()),
                visible: true,
                rect: Some(Rect::new(0.0, 540.0, 960.0, 540.0)),
                crop: None,
                opacity: Some(0.9),
                border: Some(Border { width: 2, color: "#ffffff".into() }),
                extra: Extra::new(),
            }),
            categories: vec!["SOURCE".into(), "POS".into(), "SIZE".into()],
            canvas: Some(Size { w: 1920, h: 1080 }),
            source_id: Some(src),
            extra: Extra::new(),
        });

        // Event Master user key → LivePremier layer memory: kept, renamed,
        // renumbered into the bank, and the source binding is dropped
        // because only Event Master can bind a look to a source.
        let c = convert(&show, Platform::AwLivePremier, "Aquilon C");
        assert_eq!(c.show.layer_memories.len(), 1);
        let m = &c.show.layer_memories[0];
        assert_eq!(m.id, "lmem:1");
        assert_eq!(m.label, "Lower third");
        assert!(m.state.as_ref().unwrap().rect.is_some(), "the look itself carries over");
        assert!(m.source_id.is_none(), "LivePremier cannot bind a layer memory to a source");
        assert!(c.notes.iter().any(|n| n.path == "layerMemories" && n.level == NoteLevel::Adapted && n.message.contains("layer memories")));
        assert!(c.show.validate().is_empty(), "{:?}", c.show.validate());

        // Midra 4K has no layer bank at all: dropped, named in the notes and
        // written into the show's own notes so the operator can rebuild it.
        let c = convert(&show, Platform::AwMidra4k, "Pulse 4K");
        assert!(c.show.layer_memories.is_empty());
        assert!(c.notes.iter().any(|n| n.path == "layerMemories" && n.level == NoteLevel::Dropped));
        assert!(c.show.meta.notes.contains("Lower third"));
    }

    /// Over the bank's size, the extra memories go rather than silently
    /// overwriting slot 1.
    #[test]
    fn the_layer_bank_has_fifty_slots() {
        let mut show = em_show();
        for i in 0..60 {
            show.layer_memories.push(LayerMemory {
                id: ids::layer_memory(i),
                number: Some(i + 1),
                label: format!("Look {i}"),
                state: None,
                categories: vec![],
                canvas: None,
                source_id: None,
                extra: Extra::new(),
            });
        }
        let c = convert(&show, Platform::AwLivePremier, "Aquilon C max");
        assert_eq!(c.show.layer_memories.len(), 50);
        assert!(c.notes.iter().any(|n| n.path == "layerMemories" && n.level == NoteLevel::Dropped && n.message.contains("50")));
        // Slots that came off a device as metadata only say so.
        assert!(c.notes.iter().any(|n| n.message.contains("values live on the hardware")));
    }

    /// The Midra 4K keeps auxiliary memories in a bank of their own, so a
    /// show with more aux memories than screen slots is not over capacity.
    #[test]
    fn auxiliary_memories_are_counted_against_their_own_bank() {
        let mut show = em_show();
        // One aux destination and 120 memories on it; the Pulse 4K has 100
        // screen slots and 200 aux slots.
        let aux = Screen {
            id: ids::aux(0),
            label: "Record".into(),
            kind: ScreenKind::Aux,
            size: Size { w: 1920, h: 1080 },
            outputs: vec![],
            layers: vec![LayerDef { id: ids::layer(1), label: "Layer 1".into(), kind: LayerKind::Mixer, z: 1, capacity: None, extra: Extra::new() }],
            transition: None,
            extra: Extra::new(),
        };
        let aux_id = aux.id.clone();
        show.screens.push(aux);
        show.presets.clear();
        for i in 0..120 {
            show.presets.push(Preset {
                id: ids::preset(i),
                number: Some(i + 1),
                label: format!("Aux memory {i}"),
                notes: String::new(),
                targets: vec![PresetTarget { screen_id: aux_id.clone(), background: None, layers: vec![], transition: None }],
                extra: Extra::new(),
            });
        }
        let c = convert(&show, Platform::AwMidra4k, "Pulse 4K");
        assert_eq!(c.show.presets.len(), 120, "120 aux memories fit the 200-slot aux bank");
        assert!(c.notes.iter().any(|n| n.path == "presets" && n.level == NoteLevel::Info && n.message.contains("own bank")));
        // The same 120 against the screen bank would not fit.
        assert!(!c.notes.iter().any(|n| n.path == "presets" && n.level == NoteLevel::Dropped));
    }

    /// Event Master keeps ten layouts on a multiviewer; a LivePremier shows
    /// one and keeps fifty in its bank, so nothing is lost.
    #[test]
    fn extra_multiviewer_layouts_become_memories() {
        let mut show = em_show();
        let mv = &mut show.multiviewers[0];
        for i in 1..6 {
            let mut lay = mv.layouts[0].clone();
            lay.id = ids::layout(1, i + 1);
            lay.label = format!("Layout {}", i + 1);
            mv.layouts.push(lay);
        }
        let c = convert(&show, Platform::AwLivePremier, "Aquilon C");
        assert_eq!(c.show.multiviewers[0].layouts.len(), 6, "kept: one live, five in the bank");
        assert!(c.notes.iter().any(|n| n.level == NoteLevel::Adapted && n.message.contains("multiviewer memories")));
        // A LiveCore has no multiviewer bank: the extras go.
        let c = convert(&show, Platform::AwLiveCore, "Ascender 16");
        assert_eq!(c.show.multiviewers[0].layouts.len(), 1);
        assert!(c.notes.iter().any(|n| n.level == NoteLevel::Dropped && n.message.contains("live and 0 in its bank")));
    }

    #[test]
    fn over_capacity_is_reported() {
        let show = em_show();
        let c = convert(&show, Platform::AwMidra4k, "Pulse 4K");
        // Pulse 4K: 2 screens, 4 layers.
        assert_eq!(c.show.screens.iter().filter(|s| s.kind == ScreenKind::Screen).count(), 2);
        assert!(c.notes.iter().any(|n| n.path == "screens" && n.level == NoteLevel::Dropped));
        assert!(c.show.validate().is_empty(), "{:?}", c.show.validate());
    }
}
