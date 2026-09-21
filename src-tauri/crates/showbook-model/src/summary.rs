//! The numbers a library card shows without loading the whole show.

use serde::{Deserialize, Serialize};

use crate::{Platform, Show};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub model: String,
    pub firmware: String,
    pub modified: String,
    pub inputs: usize,
    pub outputs: usize,
    pub screens: usize,
    pub auxes: usize,
    pub presets: usize,
    pub master_presets: usize,
    #[serde(default)]
    pub layer_memories: usize,
    pub cues: usize,
    pub multiviewers: usize,
    pub notes_dropped: usize,
}

impl Summary {
    pub fn of(show: &Show) -> Summary {
        Summary {
            id: show.id.clone(),
            name: show.meta.name.clone(),
            platform: show.platform,
            model: show.system.model.clone(),
            firmware: show.system.firmware.clone(),
            modified: show.meta.modified.clone(),
            inputs: show.inputs.len(),
            outputs: show.outputs.len(),
            screens: show.screens.iter().filter(|s| s.kind == crate::ScreenKind::Screen).count(),
            auxes: show.screens.iter().filter(|s| s.kind == crate::ScreenKind::Aux).count(),
            presets: show.presets.len(),
            master_presets: show.master_presets.len(),
            layer_memories: show.layer_memories.len(),
            cues: show.cues.len(),
            multiviewers: show.multiviewers.len(),
            notes_dropped: show.notes.iter().filter(|n| n.level == crate::NoteLevel::Dropped).count(),
        }
    }
}
