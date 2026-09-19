//! What each switcher can hold. Figures are from the vendors' spec sheets
//! and user manuals; `notes` carries the caveats that a single number hides.
//! A figure marked "family convention" in a comment is a limit the software
//! imposes uniformly and the sheets do not print.

use serde::{Deserialize, Serialize};
use showbook_model::Platform;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub platform: Platform,
    pub model: String,
    pub inputs: u32,
    pub outputs: u32,
    pub screens: u32,
    pub auxes: u32,
    /// System pool of mixing layers in 4K-layer units (0 = not modelled).
    pub layers_4k: f64,
    /// Mixing layers one screen may hold.
    pub layers_per_screen: u32,
    pub aux_layers: u32,
    /// Whether aux layers draw on the same pool.
    pub aux_layers_cost: bool,
    pub background_layer: bool,
    pub dsk: bool,
    pub border: bool,
    pub shadow: bool,
    pub crop: bool,
    pub opacity: bool,
    pub keying: bool,
    pub preset_slots: u32,
    /// A preset can hold several screens (Event Master) or exactly one
    /// (Analog Way memories, which use master memories to group them).
    pub preset_multi_screen: bool,
    pub master_slots: u32,
    pub cues: bool,
    pub still_slots: u32,
    pub multiviewers: u32,
    pub widgets_per_mv: u32,
    pub mv_layouts: u32,
    pub notes: Vec<String>,
}

/// Cost of a layer in 4K units by its vendor capacity class.
pub fn layer_cost(capacity: Option<&str>) -> f64 {
    match capacity.map(|c| c.to_ascii_uppercase()).as_deref() {
        Some("4K") | Some("4K60") => 1.0,
        Some("5K") => 1.5,
        Some("8K") => 4.0,
        Some("DL") | Some("DUAL") | Some("2K") | Some("SPLIT") => 0.5,
        Some("SL") | Some("SINGLE") | Some("HD") => 0.25,
        _ => 1.0,
    }
}

pub fn models(platform: Platform) -> Vec<&'static str> {
    match platform {
        Platform::BarcoEm => vec!["Encore3", "E2 Gen 2", "E2", "E2 Jr", "S3-4K", "S3-4K Jr", "EX"],
        Platform::BarcoPds4k => vec!["PDS-4K"],
        Platform::AwLivePremier => vec![
            "Aquilon C mini", "Aquilon C", "Aquilon C+", "Aquilon C max", "Aquilon RS alpha", "Aquilon RS1", "Aquilon RS2",
            "Aquilon RS3", "Aquilon RS4", "Aquilon RS5", "Aquilon RS6",
        ],
        Platform::AwMidra4k => vec!["QuickVu 4K", "Pulse 4K", "Eikos 4K", "QuickMatrix 4K"],
        Platform::AwAlta4k => vec!["Zenith 100", "Zenith 200"],
        Platform::AwLiveCore => vec!["NeXtage 08", "NeXtage 16", "Ascender 16", "Ascender 32", "Ascender 48", "SmartMatriX Ultra"],
        Platform::Pixelhue => vec!["P20", "P10", "Q8", "F8", "F4"],
        Platform::Generic => vec!["Generic"],
    }
}

fn em(model: &str, inputs: u32, outputs: u32, screens: u32, layers_4k: f64, per_screen: u32, mvr: u32) -> Capabilities {
    Capabilities {
        platform: Platform::BarcoEm,
        model: model.into(),
        inputs,
        outputs,
        screens,
        auxes: outputs,
        layers_4k,
        layers_per_screen: per_screen,
        aux_layers: 1,
        aux_layers_cost: false,
        background_layer: true,
        dsk: true,
        border: true,
        shadow: true,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 1000,
        preset_multi_screen: true,
        master_slots: 0,
        cues: true,
        still_slots: 100,
        multiviewers: mvr,
        widgets_per_mv: 64,
        mv_layouts: 10,
        notes: vec!["Event Master presets can recall several destinations at once; there is no separate master bank".into(), "layer capacity is a pool of mixable 4K layers: a DL layer costs half, a 2K layer a quarter (Barco's own arithmetic)".into()],
    }
}

fn aw_lp(model: &str, inputs: u32, outputs: u32, layers_4k: f64) -> Capabilities {
    Capabilities {
        platform: Platform::AwLivePremier,
        model: model.into(),
        inputs,
        outputs,
        screens: 24,
        auxes: 32,
        layers_4k,
        layers_per_screen: 16,
        aux_layers: 8,
        aux_layers_cost: false,
        background_layer: true,
        dsk: false,
        border: true,
        shadow: true,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 1000,
        preset_multi_screen: false,
        master_slots: 500,
        cues: false,
        still_slots: 192,
        multiviewers: 2,
        widgets_per_mv: 24,
        mv_layouts: 1,
        notes: vec!["a LivePremier memory holds one screen; master memories recall a memory per screen together".into(), "layers on auxiliary screens use output scalers, not the mixing-layer pool (spec sheet: 'unscaled background mixer per output')".into(), "a screen wider than four outputs takes a second layer link and costs each layer twice (User Manual v6 §5.5.4)".into(), "no cue list on the device; the fleet's livepremier-plus timeline or Companion carries sequencing".into(), "the multiviewer holds one live layout; multiviewer memories (MTVW bank) hold up to 50 more".into()],
    }
}

fn aw_midra(model: &str, screens: u32, layers: u32, inputs: u32, outputs: u32) -> Capabilities {
    Capabilities {
        platform: Platform::AwMidra4k,
        model: model.into(),
        inputs,
        outputs,
        screens,
        auxes: 1,
        layers_4k: layers as f64,
        layers_per_screen: layers,
        aux_layers: 1,
        aux_layers_cost: false,
        background_layer: true,
        dsk: false,
        border: true,
        shadow: true,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 100,
        preset_multi_screen: false,
        master_slots: 100,
        cues: false,
        still_slots: 50,
        multiviewers: 1,
        widgets_per_mv: 16,
        mv_layouts: 1,
        notes: vec!["Midra 4K figures are for Mixer mode; Matrix mode gives two screens with fewer layers each".into(), "two split layers cost one mixing layer".into()],
    }
}

fn aw_alta(model: &str, layers: u32, inputs: u32, outputs: u32) -> Capabilities {
    Capabilities {
        platform: Platform::AwAlta4k,
        model: model.into(),
        inputs,
        outputs,
        screens: outputs,
        auxes: outputs,
        layers_4k: layers as f64,
        layers_per_screen: layers,
        aux_layers: 1,
        aux_layers_cost: true,
        background_layer: true,
        dsk: false,
        border: true,
        shadow: true,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 100,
        preset_multi_screen: false,
        master_slots: 100,
        cues: false,
        still_slots: 50,
        multiviewers: 1,
        widgets_per_mv: 16,
        mv_layouts: 1,
        notes: vec!["Alta 4K split layers double the count but cut or fade to black instead of cross-fading".into()],
    }
}

fn aw_livecore(model: &str, inputs: u32, outputs: u32, screens: u32, layers: u32) -> Capabilities {
    Capabilities {
        platform: Platform::AwLiveCore,
        model: model.into(),
        inputs,
        outputs,
        screens,
        auxes: outputs,
        layers_4k: layers as f64 * 0.25,
        layers_per_screen: 6,
        aux_layers: 1,
        aux_layers_cost: false,
        background_layer: true,
        dsk: false,
        border: true,
        shadow: true,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 144,
        preset_multi_screen: false,
        master_slots: 144,
        cues: false,
        still_slots: 12,
        multiviewers: 1,
        widgets_per_mv: 16,
        mv_layouts: 1,
        notes: vec!["LiveCore layers are 2K; a 4K layer takes four (the 4K models split a 4K source across links)".into()],
    }
}

fn pixelhue(model: &str, inputs: u32, outputs: u32, out_cards: u32, layers_4k: u32) -> Capabilities {
    Capabilities {
        platform: Platform::Pixelhue,
        model: model.into(),
        inputs,
        outputs,
        screens: out_cards * 2,
        auxes: outputs,
        layers_4k: layers_4k as f64,
        layers_per_screen: 8,
        aux_layers: 1,
        aux_layers_cost: false,
        background_layer: true,
        dsk: false,
        border: true,
        shadow: false,
        crop: true,
        opacity: true,
        keying: true,
        preset_slots: 128,
        preset_multi_screen: true,
        master_slots: 0,
        cues: false,
        still_slots: 50,
        multiviewers: 1,
        widgets_per_mv: 32,
        mv_layouts: 4,
        notes: vec!["PixelHue budgets mixing layers per output card (2x 4K, 4x DL or 8x SL each), not across the chassis".into()],
    }
}

pub fn capabilities(platform: Platform, model: &str) -> Capabilities {
    let m = model.trim();
    match platform {
        Platform::BarcoEm => match m {
            "Encore3" | "E3" | "ENCORE3" => {
                let mut c = em("Encore3", 16, 8, 8, 16.0, 16, 2);
                c.notes.push("Encore3 pre-assigns 4 mixable layers per output screen; up to 16 by reallocating from other screens".into());
                c
            }
            "E2 Gen 2" | "E2 GEN 2" => em("E2 Gen 2", 16, 8, 8, 16.0, 16, 1),
            "E2 Jr" => em("E2 Jr", 8, 4, 4, 8.0, 8, 1),
            "S3-4K" | "S3" => em("S3-4K", 8, 4, 4, 4.0, 8, 1),
            "S3-4K Jr" | "S3 Jr" => em("S3-4K Jr", 4, 4, 4, 4.0, 8, 1),
            "EX" => em("EX", 4, 4, 4, 0.0, 0, 0),
            _ => em("E2", 8, 8, 8, 16.0, 16, 1),
        },
        Platform::BarcoPds4k => {
            let mut c = em("PDS-4K", 4, 2, 1, 2.0, 2, 1);
            c.platform = Platform::BarcoPds4k;
            c.notes.push("PDS-4K runs Event Master software on a fixed chassis: one program screen, two mixing layers".into());
            c
        }
        Platform::AwLivePremier => match m {
            "Aquilon C mini" | "Cmini" => aw_lp("Aquilon C mini", 8, 12, 4.0),
            "Aquilon C" | "C" => aw_lp("Aquilon C", 16, 12, 8.0),
            "Aquilon C+" | "C+" => aw_lp("Aquilon C+", 24, 20, 12.0),
            "Aquilon C max" | "Cmax" | "CMAX" => aw_lp("Aquilon C max", 32, 24, 16.0),
            "Aquilon RS alpha" => aw_lp("Aquilon RS alpha", 8, 4, 4.0),
            "Aquilon RS1" => aw_lp("Aquilon RS1", 16, 8, 4.0),
            "Aquilon RS2" => aw_lp("Aquilon RS2", 16, 12, 8.0),
            "Aquilon RS3" => aw_lp("Aquilon RS3", 24, 12, 8.0),
            "Aquilon RS4" => aw_lp("Aquilon RS4", 24, 16, 12.0),
            "Aquilon RS5" => aw_lp("Aquilon RS5", 32, 16, 12.0),
            "Aquilon RS6" => aw_lp("Aquilon RS6", 32, 20, 16.0),
            _ => aw_lp("Aquilon C", 16, 12, 8.0),
        },
        Platform::AwMidra4k => match m {
            "QuickVu 4K" => aw_midra("QuickVu 4K", 1, 2, 4, 2),
            "Eikos 4K" => aw_midra("Eikos 4K", 2, 4, 8, 4),
            "QuickMatrix 4K" => aw_midra("QuickMatrix 4K", 2, 4, 8, 4),
            _ => aw_midra("Pulse 4K", 2, 4, 6, 3),
        },
        Platform::AwAlta4k => match m {
            "Zenith 200" => aw_alta("Zenith 200", 4, 8, 4),
            _ => aw_alta("Zenith 100", 3, 6, 3),
        },
        Platform::AwLiveCore => match m {
            "NeXtage 08" => aw_livecore("NeXtage 08", 8, 4, 2, 8),
            "NeXtage 16" => aw_livecore("NeXtage 16", 16, 4, 2, 8),
            "Ascender 32" => aw_livecore("Ascender 32", 32, 8, 4, 16),
            "Ascender 48" => aw_livecore("Ascender 48", 48, 12, 6, 24),
            "SmartMatriX Ultra" => aw_livecore("SmartMatriX Ultra", 16, 8, 4, 8),
            _ => aw_livecore("Ascender 16", 16, 4, 2, 8),
        },
        Platform::Pixelhue => match m {
            "F4" => pixelhue("F4", 24, 12, 6, 12),
            "P20" => pixelhue("P20", 32, 16, 8, 16),
            "P10" => pixelhue("P10", 24, 12, 6, 12),
            "Q8" => pixelhue("Q8", 16, 8, 4, 8),
            _ => pixelhue("F8", 32, 16, 8, 16),
        },
        Platform::Generic => Capabilities {
            platform: Platform::Generic,
            model: "Generic".into(),
            inputs: 999,
            outputs: 999,
            screens: 999,
            auxes: 999,
            layers_4k: 0.0,
            layers_per_screen: 999,
            aux_layers: 999,
            aux_layers_cost: false,
            background_layer: true,
            dsk: true,
            border: true,
            shadow: true,
            crop: true,
            opacity: true,
            keying: true,
            preset_slots: 9999,
            preset_multi_screen: true,
            master_slots: 9999,
            cues: true,
            still_slots: 9999,
            multiviewers: 99,
            widgets_per_mv: 999,
            mv_layouts: 99,
            notes: vec![],
        },
    }
}
