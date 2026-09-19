//! The canonical show model.
//!
//! Every platform driver (`showbook-em`, `showbook-aw`, …) reads a vendor show
//! file or a live device into a [`Show`], and writes one back out. Everything
//! above the drivers — the inspector, the documentation, the conversion engine,
//! the history — only ever sees this model. That is what makes conversion
//! possible: a show is a graph of inputs, sources, outputs, screens, layers,
//! presets and cues, and the vendor spellings are pushed down into `extra` bags
//! so nothing is lost on a round trip but nothing vendor-specific leaks up.
//!
//! Three rules the model keeps:
//!
//! - **IDs are stable strings**, scoped by kind (`in:…`, `src:…`, `out:…`,
//!   `scr:…`, `pre:…`) and chosen by the driver so that re-importing the same
//!   vendor file yields the same IDs and the history diff stays readable.
//! - **Geometry is in pixels of the thing it is measured on**: a layer's `rect`
//!   is in screen canvas pixels, a `crop` is in source pixels, an output map is
//!   in canvas pixels.
//! - **`extra` is a bag, not a model.** Anything a driver cannot express in the
//!   shared fields goes in `extra` keyed by the vendor's own name. Conversion
//!   never reads `extra`; the same driver can write it back.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub mod diff;
pub mod ids;
pub mod summary;

/// Schema tag written into every show file. Bump when a field changes meaning.
pub const SCHEMA: &str = "showbook/1";

pub type Extra = Map<String, Value>;

fn extra_is_empty(e: &Extra) -> bool {
    e.is_empty()
}

/// The platform a show was authored for. This is the family that decides
/// which driver reads and writes it; the specific chassis is `System::model`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    /// Barco Event Master: E2 (Gen 1/2), S3-4K, EX, and Encore3 (E3).
    BarcoEm,
    /// Analog Way LivePremier: Aquilon C / RS.
    AwLivePremier,
    /// Analog Way Midra 4K: QuickVu 4K, Pulse 4K, Eikos 4K, QuickMatrix 4K.
    AwMidra4k,
    /// Analog Way Alta 4K: Zenith 100/200.
    AwAlta4k,
    /// Analog Way LiveCore: Ascender, NeXtage, SmartMatriX Ultra.
    AwLiveCore,
    /// Barco PDS-4K.
    BarcoPds4k,
    /// PixelHue P/Q series (NovaStar).
    Pixelhue,
    /// No vendor: a show authored from scratch in Showbook.
    Generic,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Platform::BarcoEm => "Barco Event Master",
            Platform::AwLivePremier => "Analog Way LivePremier",
            Platform::AwMidra4k => "Analog Way Midra 4K",
            Platform::AwAlta4k => "Analog Way Alta 4K",
            Platform::AwLiveCore => "Analog Way LiveCore",
            Platform::BarcoPds4k => "Barco PDS-4K",
            Platform::Pixelhue => "PixelHue",
            Platform::Generic => "Generic",
        }
    }

    pub fn all() -> &'static [Platform] {
        &[
            Platform::BarcoEm,
            Platform::AwLivePremier,
            Platform::AwMidra4k,
            Platform::AwAlta4k,
            Platform::AwLiveCore,
            Platform::BarcoPds4k,
            Platform::Pixelhue,
            Platform::Generic,
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Show {
    pub schema: String,
    pub id: String,
    pub meta: Meta,
    pub platform: Platform,
    pub system: System,
    #[serde(default)]
    pub inputs: Vec<Input>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub screens: Vec<Screen>,
    #[serde(default)]
    pub presets: Vec<Preset>,
    #[serde(default)]
    pub master_presets: Vec<MasterPreset>,
    #[serde(default)]
    pub cues: Vec<Cue>,
    #[serde(default)]
    pub multiviewers: Vec<Multiviewer>,
    #[serde(default)]
    pub stills: Vec<Still>,
    /// Vendor files kept beside the model — an `.awc`, an `E3Backup.tar.gz`,
    /// the raw XML — so a show can go back to its own hardware byte-exact.
    #[serde(default)]
    pub vendor: Vec<VendorBlob>,
    /// What an import or a conversion could not express faithfully.
    #[serde(default)]
    pub notes: Vec<Note>,
}

impl Show {
    pub fn new(name: &str, platform: Platform) -> Show {
        Show {
            schema: SCHEMA.to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            meta: Meta {
                name: name.to_string(),
                notes: String::new(),
                tags: vec![],
                created: now(),
                modified: now(),
                author: None,
                source: None,
            },
            platform,
            system: System::default(),
            inputs: vec![],
            sources: vec![],
            outputs: vec![],
            screens: vec![],
            presets: vec![],
            master_presets: vec![],
            cues: vec![],
            multiviewers: vec![],
            stills: vec![],
            vendor: vec![],
            notes: vec![],
        }
    }

    pub fn touch(&mut self) {
        self.meta.modified = now();
    }

    pub fn input(&self, id: &str) -> Option<&Input> {
        self.inputs.iter().find(|x| x.id == id)
    }
    pub fn source(&self, id: &str) -> Option<&Source> {
        self.sources.iter().find(|x| x.id == id)
    }
    pub fn output(&self, id: &str) -> Option<&Output> {
        self.outputs.iter().find(|x| x.id == id)
    }
    pub fn screen(&self, id: &str) -> Option<&Screen> {
        self.screens.iter().find(|x| x.id == id)
    }
    pub fn preset(&self, id: &str) -> Option<&Preset> {
        self.presets.iter().find(|x| x.id == id)
    }
    pub fn connector(&self, id: &str) -> Option<&Connector> {
        self.system
            .frames
            .iter()
            .flat_map(|f| f.slots.iter())
            .flat_map(|s| s.connectors.iter())
            .find(|c| c.id == id)
    }

    /// Validate the reference graph: every ID a field points at must exist.
    /// Returns human-readable problems; an empty list is a consistent show.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = vec![];
        let has_conn = |id: &str| self.connector(id).is_some();
        for i in &self.inputs {
            for c in &i.connector_ids {
                if !has_conn(c) {
                    problems.push(format!("input {} points at missing connector {}", i.id, c));
                }
            }
        }
        for o in &self.outputs {
            for c in &o.connector_ids {
                if !has_conn(c) {
                    problems.push(format!("output {} points at missing connector {}", o.id, c));
                }
            }
        }
        for s in &self.sources {
            if let Some(r) = &s.ref_id {
                let ok = match s.kind {
                    SourceKind::Input => self.input(r).is_some(),
                    SourceKind::Still => self.stills.iter().any(|x| &x.id == r),
                    SourceKind::Screen | SourceKind::Aux => self.screen(r).is_some(),
                    SourceKind::Multiviewer => self.multiviewers.iter().any(|x| &x.id == r),
                    _ => true,
                };
                if !ok {
                    problems.push(format!("source {} points at missing {:?} {}", s.id, s.kind, r));
                }
            }
        }
        for sc in &self.screens {
            for om in &sc.outputs {
                if self.output(&om.output_id).is_none() {
                    problems.push(format!("screen {} maps missing output {}", sc.id, om.output_id));
                }
            }
        }
        for p in &self.presets {
            for t in &p.targets {
                let Some(sc) = self.screen(&t.screen_id) else {
                    problems.push(format!("preset {} targets missing screen {}", p.id, t.screen_id));
                    continue;
                };
                for l in &t.layers {
                    if !sc.layers.iter().any(|d| d.id == l.layer_id) {
                        problems.push(format!(
                            "preset {} on {} sets layer {} the screen does not have",
                            p.id, sc.id, l.layer_id
                        ));
                    }
                    if let Some(s) = &l.source_id {
                        if self.source(s).is_none() {
                            problems.push(format!("preset {} layer {} uses missing source {}", p.id, l.layer_id, s));
                        }
                    }
                }
            }
        }
        for m in &self.master_presets {
            for e in &m.entries {
                if self.screen(&e.screen_id).is_none() {
                    problems.push(format!("master preset {} names missing screen {}", m.id, e.screen_id));
                }
                if self.preset(&e.preset_id).is_none() {
                    problems.push(format!("master preset {} names missing preset {}", m.id, e.preset_id));
                }
            }
        }
        problems
    }
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created: String,
    pub modified: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Where the model came from, when it was imported or captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    /// `file` or `device`.
    pub kind: String,
    /// File name, or host address.
    pub origin: String,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct System {
    /// Chassis model as the vendor names it: "E3", "E2 Gen 2", "S3-4K", "Aquilon C", "Aquilon RS4".
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub firmware: String,
    /// The device's own name.
    #[serde(default)]
    pub name: String,
    /// System native frame rate, Hz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genlock: Option<Genlock>,
    #[serde(default)]
    pub frames: Vec<Frame>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Genlock {
    /// `internal`, `input`, `blackburst`, `tri-level` — as the vendor reports it.
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_id: Option<String>,
    #[serde(default)]
    pub locked: bool,
}

/// One chassis (Event Master frames link; a LivePremier is one frame).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default)]
    pub slots: Vec<Slot>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub index: u32,
    /// Vendor card type name, e.g. "In HDMI2.0 Card", "Out TriCombo Card", "VPU".
    pub card: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub connectors: Vec<Connector>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorKind {
    Sdi,
    Hdmi,
    DisplayPort,
    Dvi,
    Fibre,
    Ip,
    Link,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    In,
    Out,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Connector {
    pub id: String,
    pub kind: ConnectorKind,
    pub direction: Direction,
    /// Connector number on the card, 1-based, as printed on the panel.
    pub index: u32,
    pub label: String,
    /// What the plug can carry, as the vendor names it: "12G-SDI", "HDMI 2.0", "DP 1.2".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Format {
    pub width: u32,
    pub height: u32,
    /// Frames per second (fields per second / 2 for interlaced).
    pub rate: f64,
    #[serde(default)]
    pub interlaced: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Format {
    pub fn describe(&self) -> String {
        format!(
            "{}x{}{}{}",
            self.width,
            self.height,
            if self.interlaced { "i" } else { "p" },
            trim_rate(self.rate)
        )
    }
}

fn trim_rate(r: f64) -> String {
    if (r - r.round()).abs() < 0.005 {
        format!("{}", r.round() as i64)
    } else {
        format!("{:.2}", r)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect { x, y, w, h }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub connector_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Vendor capacity class, e.g. "4K", "2K", "SL", "DL".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hdcp: Option<bool>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Input,
    Still,
    Screen,
    Aux,
    Multiviewer,
    Color,
    Background,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub label: String,
    pub kind: SourceKind,
    /// The input / still / screen this source is built on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<String>,
    /// Area of interest in the referenced input's pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aoi: Option<Rect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum OutputRole {
    Screen,
    Aux,
    Multiviewer,
    Unassigned,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub connector_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
    pub role: OutputRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ScreenKind {
    Screen,
    Aux,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputMap {
    pub output_id: String,
    /// Where the output's raster sits on the screen canvas, canvas pixels.
    pub rect: Rect,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum LayerKind {
    /// The native background / matte / background set.
    Background,
    /// A mixing layer (PIP).
    Mixer,
    /// A key / DSK layer.
    Key,
    /// A layer whose kind the driver could not classify.
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayerDef {
    pub id: String,
    pub label: String,
    pub kind: LayerKind,
    /// Stacking order, 0 at the bottom.
    pub z: u32,
    /// Vendor capacity class of the layer ("4K", "2K", "SL", "DL", "DUAL").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Duration in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u32>,
    /// `mix`, `cut`, `wipe`, or the vendor's own name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screen {
    pub id: String,
    pub label: String,
    pub kind: ScreenKind,
    pub size: Size,
    #[serde(default)]
    pub outputs: Vec<OutputMap>,
    #[serde(default)]
    pub layers: Vec<LayerDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Border {
    pub width: u32,
    /// `#rrggbb`
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayerState {
    pub layer_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default = "yes")]
    pub visible: bool,
    /// Position and size on the screen canvas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<Rect>,
    /// Crop in source pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Rect>,
    /// 0.0 transparent … 1.0 opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Border>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PresetTarget {
    pub screen_id: String,
    /// Source on the background layer, if the preset sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default)]
    pub layers: Vec<LayerState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    /// Slot / preset number as the operator sees it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    pub label: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub targets: Vec<PresetTarget>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MasterEntry {
    pub screen_id: String,
    pub preset_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MasterPreset {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    pub label: String,
    #[serde(default)]
    pub entries: Vec<MasterEntry>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CueStepKind {
    RecallPreset,
    RecallMaster,
    Take,
    Wait,
    Other,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CueStep {
    pub kind: CueStepKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_id: Option<String>,
    #[serde(default)]
    pub screen_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    pub label: String,
    #[serde(default)]
    pub steps: Vec<CueStep>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    /// In layout pixels.
    pub rect: Rect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "yes")]
    pub show_label: bool,
    #[serde(default)]
    pub tally: bool,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MvLayout {
    pub id: String,
    pub label: String,
    pub size: Size,
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Multiviewer {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub output_ids: Vec<String>,
    #[serde(default)]
    pub layouts: Vec<MvLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_layout: Option<String>,
    #[serde(default, skip_serializing_if = "extra_is_empty")]
    pub extra: Extra,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Still {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    /// Path relative to the show's assets directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VendorBlob {
    pub platform: Platform,
    /// `awc`, `e3-backup`, `em-xml`, `em-backup`.
    pub kind: String,
    pub sha256: String,
    /// Path relative to the show directory.
    pub file: String,
    pub size: u64,
    pub captured_at: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum NoteLevel {
    /// Carried over as-is.
    Info,
    /// Carried over with an adaptation — the nearest thing the target has.
    Adapted,
    /// Could not be carried; the target has nothing like it.
    Dropped,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub level: NoteLevel,
    /// Which entity: `screens/scr:1/layers/layer:2`.
    pub path: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let mut show = Show::new("Test", Platform::Generic);
        show.system.frames.push(Frame {
            id: "frame:1".into(),
            label: "Frame 1".into(),
            model: "E3".into(),
            address: None,
            slots: vec![Slot {
                index: 1,
                card: "In HDMI2.0 Card".into(),
                label: "".into(),
                connectors: vec![Connector {
                    id: "conn:1.1".into(),
                    kind: ConnectorKind::Hdmi,
                    direction: Direction::In,
                    index: 1,
                    label: "HDMI 1".into(),
                    standard: Some("HDMI 2.0".into()),
                }],
            }],
        });
        show.inputs.push(Input {
            id: "in:1".into(),
            label: "Camera".into(),
            connector_ids: vec!["conn:1.1".into()],
            format: Some(Format { width: 1920, height: 1080, rate: 59.94, interlaced: false, name: None }),
            enabled: true,
            capacity: None,
            hdcp: None,
            extra: Extra::new(),
        });
        let json = serde_json::to_string_pretty(&show).unwrap();
        let back: Show = serde_json::from_str(&json).unwrap();
        assert_eq!(show, back);
        assert!(show.validate().is_empty());
    }

    #[test]
    fn validate_reports_dangling_references() {
        let mut show = Show::new("Test", Platform::Generic);
        show.inputs.push(Input {
            id: "in:1".into(),
            label: "x".into(),
            connector_ids: vec!["conn:missing".into()],
            format: None,
            enabled: true,
            capacity: None,
            hdcp: None,
            extra: Extra::new(),
        });
        let problems = show.validate();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("conn:missing"));
    }

    #[test]
    fn format_describe() {
        let f = Format { width: 1920, height: 1080, rate: 59.94, interlaced: false, name: None };
        assert_eq!(f.describe(), "1920x1080p59.94");
        let f = Format { width: 3840, height: 2160, rate: 60.0, interlaced: false, name: None };
        assert_eq!(f.describe(), "3840x2160p60");
        let f = Format { width: 1920, height: 1080, rate: 25.0, interlaced: true, name: None };
        assert_eq!(f.describe(), "1920x1080i25");
    }
}
