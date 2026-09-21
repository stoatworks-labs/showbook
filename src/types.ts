/**
 * The JSON shapes shared with Rust — a mirror of `showbook-model`. Read this
 * first: every view draws from these and nothing in the UI computes a value
 * Rust has not already put here.
 */

export type Platform =
  | 'barco-em'
  | 'aw-live-premier'
  | 'aw-midra4k'
  | 'aw-alta4k'
  | 'aw-live-core'
  | 'barco-pds4k'
  | 'pixelhue'
  | 'generic';

export const PLATFORM_LABEL: Record<Platform, string> = {
  'barco-em': 'Barco Event Master',
  'aw-live-premier': 'Analog Way LivePremier',
  'aw-midra4k': 'Analog Way Midra 4K',
  'aw-alta4k': 'Analog Way Alta 4K',
  'aw-live-core': 'Analog Way LiveCore',
  'barco-pds4k': 'Barco PDS-4K',
  pixelhue: 'PixelHue',
  generic: 'Generic',
};

export type Extra = Record<string, unknown>;

export interface Format {
  width: number;
  height: number;
  rate: number;
  interlaced: boolean;
  name?: string;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Size {
  w: number;
  h: number;
}

export type ConnectorKind = 'sdi' | 'hdmi' | 'display-port' | 'dvi' | 'fibre' | 'ip' | 'link' | 'other';

export interface Connector {
  id: string;
  kind: ConnectorKind;
  direction: 'in' | 'out';
  index: number;
  label: string;
  standard?: string;
}

export interface Slot {
  index: number;
  card: string;
  label: string;
  connectors: Connector[];
}

export interface Frame {
  id: string;
  label: string;
  model: string;
  address?: string;
  slots: Slot[];
}

export interface Genlock {
  source: string;
  inputId?: string;
  locked: boolean;
}

export interface System {
  model: string;
  firmware: string;
  name: string;
  nativeRate?: number;
  genlock?: Genlock;
  frames: Frame[];
  extra?: Extra;
}

export interface Input {
  id: string;
  label: string;
  connectorIds: string[];
  format?: Format;
  enabled: boolean;
  capacity?: string;
  hdcp?: boolean;
  extra?: Extra;
}

export type SourceKind = 'input' | 'still' | 'screen' | 'aux' | 'multiviewer' | 'color' | 'background' | 'unknown';

export interface Source {
  id: string;
  label: string;
  kind: SourceKind;
  refId?: string;
  aoi?: Rect;
  format?: Format;
  extra?: Extra;
}

export type OutputRole = 'screen' | 'aux' | 'multiviewer' | 'unassigned';

export interface Output {
  id: string;
  label: string;
  connectorIds: string[];
  format?: Format;
  role: OutputRole;
  testPattern?: string;
  extra?: Extra;
}

export interface OutputMap {
  outputId: string;
  rect: Rect;
}

export type LayerKind = 'background' | 'mixer' | 'key' | 'other';

export interface LayerDef {
  id: string;
  label: string;
  kind: LayerKind;
  z: number;
  capacity?: string;
  extra?: Extra;
}

export interface Transition {
  durationMs?: number;
  kind?: string;
}

export interface Screen {
  id: string;
  label: string;
  kind: 'screen' | 'aux';
  size: Size;
  outputs: OutputMap[];
  layers: LayerDef[];
  transition?: Transition;
  extra?: Extra;
}

export interface Border {
  width: number;
  color: string;
}

export interface LayerState {
  layerId: string;
  sourceId?: string;
  visible: boolean;
  rect?: Rect;
  crop?: Rect;
  opacity?: number;
  border?: Border;
  extra?: Extra;
}

export interface PresetTarget {
  screenId: string;
  background?: string;
  layers: LayerState[];
  transition?: Transition;
}

export interface Preset {
  id: string;
  number?: number;
  label: string;
  notes: string;
  targets: PresetTarget[];
  extra?: Extra;
}

export interface MasterEntry {
  screenId: string;
  presetId: string;
}

export interface MasterPreset {
  id: string;
  number?: number;
  label: string;
  entries: MasterEntry[];
  extra?: Extra;
}

export type CueStepKind = 'recall-preset' | 'recall-master' | 'take' | 'wait' | 'other';

export interface CueStep {
  kind: CueStepKind;
  presetId?: string;
  masterId?: string;
  screenIds: string[];
  delayMs?: number;
  extra?: Extra;
}

export interface Cue {
  id: string;
  number?: number;
  label: string;
  steps: CueStep[];
  extra?: Extra;
}

export interface Widget {
  id: string;
  rect: Rect;
  sourceId?: string;
  label?: string;
  showLabel: boolean;
  tally: boolean;
  extra?: Extra;
}

export interface MvLayout {
  id: string;
  label: string;
  size: Size;
  widgets: Widget[];
}

export interface Multiviewer {
  id: string;
  label: string;
  outputIds: string[];
  layouts: MvLayout[];
  activeLayout?: string;
  extra?: Extra;
}

export interface Still {
  id: string;
  label: string;
  size?: Size;
  file?: string;
}

export interface VendorBlob {
  platform: Platform;
  kind: string;
  sha256: string;
  file: string;
  size: number;
  capturedAt: string;
  note: string;
}

export type NoteLevel = 'info' | 'adapted' | 'dropped';

export interface Note {
  level: NoteLevel;
  path: string;
  message: string;
}

export interface SourceInfo {
  kind: string;
  origin: string;
  at: string;
  firmware?: string;
}

/** Who the show is for and who is running it — printed on the documentation. */
export interface Production {
  /** The show or event name, if it differs from the file name. */
  event?: string;
  client?: string;
  company?: string;
  venue?: string;
  /** Show date(s), free text: "12–14 March 2026", "Fri 3 Oct". */
  date?: string;
  operator?: string;
  /** Phone or email for whoever is on the desk. */
  contact?: string;
}

export interface Meta {
  name: string;
  notes: string;
  tags: string[];
  created: string;
  modified: string;
  author?: string;
  source?: SourceInfo;
  production?: Production;
}

export interface Show {
  schema: string;
  id: string;
  meta: Meta;
  platform: Platform;
  system: System;
  inputs: Input[];
  sources: Source[];
  outputs: Output[];
  screens: Screen[];
  presets: Preset[];
  masterPresets: MasterPreset[];
  cues: Cue[];
  multiviewers: Multiviewer[];
  stills: Still[];
  vendor: VendorBlob[];
  notes: Note[];
}

// ---- library ----

export interface Summary {
  id: string;
  name: string;
  platform: Platform;
  model: string;
  firmware: string;
  modified: string;
  inputs: number;
  outputs: number;
  screens: number;
  auxes: number;
  presets: number;
  masterPresets: number;
  cues: number;
  multiviewers: number;
  notesDropped: number;
}

export interface Entry {
  summary: Summary;
  dir: string;
  commits: number;
  vendorFiles: number;
}

export interface Commit {
  id: string;
  parent?: string;
  at: string;
  message: string;
  author?: string;
  hash: string;
  summary: Summary;
  changes: number;
}

export interface Change {
  kind: 'added' | 'removed' | 'changed';
  path: string;
  before?: unknown;
  after?: unknown;
}

// ---- settings / devices ----

export interface DeviceEntry {
  name: string;
  platform: Platform;
  host: string;
  awjHost?: string;
}

export interface Tokens {
  accessToken: string;
  refreshToken?: string;
  expiresAt?: number;
  scope: string;
  account: string;
}

export interface SyncSettings {
  provider: string;
  root: string;
  clientId: string;
  clientSecret?: string;
  tokens?: Tokens;
  lastRun: string;
}

export interface Settings {
  libraryPath: string;
  author: string;
  devices: DeviceEntry[];
  sync: SyncSettings;
  companionHost: string;
}

export interface AppInfo {
  version: string;
  configDir: string;
  platforms: { id: Platform; label: string; models: string[] }[];
}

export interface Capabilities {
  platform: Platform;
  model: string;
  inputs: number;
  outputs: number;
  screens: number;
  auxes: number;
  layers4k: number;
  layersPerScreen: number;
  auxLayers: number;
  auxLayersCost: boolean;
  backgroundLayer: boolean;
  dsk: boolean;
  border: boolean;
  shadow: boolean;
  crop: boolean;
  opacity: boolean;
  keying: boolean;
  presetSlots: number;
  presetMultiScreen: boolean;
  masterSlots: number;
  cues: boolean;
  stillSlots: number;
  multiviewers: number;
  widgetsPerMv: number;
  mvLayouts: number;
  notes: string[];
}

export interface SyncReport {
  uploaded: string[];
  downloaded: string[];
  unchanged: number;
  errors: string[];
}

export interface ImportedButton {
  page: number;
  row: number;
  column: number;
  text: string;
  module: string;
  definition: string;
  options: Record<string, unknown>;
  matched?: string;
  problem?: string;
}

export interface CompanionImportReport {
  fileVersion: number;
  kind: string;
  pages: string[];
  connections: string[];
  buttons: ImportedButton[];
  unmatched: number;
}

export interface CompanionExportOptions {
  connectionLabel: string;
  host: string;
  toProgram: boolean;
  includePresets: boolean;
  includeMasters: boolean;
  includeCues: boolean;
  includeTakes: boolean;
  pageName: string;
}

export const tail = (id: string): string => (id.includes(':') ? id.slice(id.indexOf(':') + 1) : id);

export function describeFormat(f?: Format): string {
  if (!f) return '—';
  const rate = Math.abs(f.rate - Math.round(f.rate)) < 0.005 ? String(Math.round(f.rate)) : f.rate.toFixed(2);
  return `${f.width}×${f.height}${f.interlaced ? 'i' : 'p'}${f.rate ? rate : ''}`;
}
