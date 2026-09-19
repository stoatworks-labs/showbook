/**
 * A browser-only stand-in for the Tauri commands, so the UI can be looked at
 * (and the PDF / pattern generators exercised) in a plain tab. It serves two
 * demo shows captured from the vendors' simulators and keeps edits in
 * memory; everything that needs the disk or a device says so.
 */
import type { AppInfo, Commit, Entry, Settings, Show } from '../types';
import type { api as realApi } from './ipc';
import { outputPattern, screenMap } from './patterns';

type Api = typeof realApi;

const shows = new Map<string, Show>();
const history = new Map<string, Commit[]>();
let loaded: Promise<void> | null = null;

const info: AppInfo = {
  version: 'demo',
  configDir: '(browser demo — nothing is written)',
  platforms: [
    { id: 'barco-em', label: 'Barco Event Master', models: ['Encore3', 'E2 Gen 2', 'E2', 'S3-4K', 'EX'] },
    { id: 'aw-live-premier', label: 'Analog Way LivePremier', models: ['Aquilon C', 'Aquilon C max', 'Aquilon RS4'] },
    { id: 'aw-midra4k', label: 'Analog Way Midra 4K', models: ['Pulse 4K', 'QuickVu 4K'] },
    { id: 'aw-alta4k', label: 'Analog Way Alta 4K', models: ['Zenith 100', 'Zenith 200'] },
    { id: 'aw-live-core', label: 'Analog Way LiveCore', models: ['Ascender 16'] },
    { id: 'barco-pds4k', label: 'Barco PDS-4K', models: ['PDS-4K'] },
    { id: 'pixelhue', label: 'PixelHue', models: ['P20', 'F8'] },
    { id: 'generic', label: 'Generic', models: ['Generic'] },
  ],
};

let settings: Settings = { libraryPath: '(demo library in memory)', author: 'demo', devices: [{ name: 'Aquilon simulator', platform: 'aw-live-premier', host: '127.0.0.1:3000' }], sync: { provider: 'none', root: '', clientId: '', lastRun: '' }, companionHost: '' };

function summary(s: Show) {
  return {
    id: s.id,
    name: s.meta.name,
    platform: s.platform,
    model: s.system.model,
    firmware: s.system.firmware,
    modified: s.meta.modified,
    inputs: s.inputs.length,
    outputs: s.outputs.length,
    screens: s.screens.filter((x) => x.kind === 'screen').length,
    auxes: s.screens.filter((x) => x.kind === 'aux').length,
    presets: s.presets.length,
    masterPresets: s.masterPresets.length,
    cues: s.cues.length,
    multiviewers: s.multiviewers.length,
    notesDropped: s.notes.filter((n) => n.level === 'dropped').length,
  };
}

async function ensure() {
  if (!loaded) {
    loaded = (async () => {
      for (const f of ['encore3-simulator', 'aquilon-cmax-simulator']) {
        const s = (await (await fetch(`./demo/${f}.json`)).json()) as Show;
        shows.set(s.id, s);
        history.set(s.id, [{ id: 'demo0', at: s.meta.modified, message: 'Captured from the simulator', hash: 'demo', changes: 0, summary: summary(s) }]);
      }
    })();
  }
  await loaded;
}

const notDesktop = (what: string) => Promise.reject(new Error(`${what} needs the desktop app (this is the browser demo)`));

function download(name: string, bytes: Uint8Array | string, type: string) {
  const blob = new Blob([bytes as BlobPart], { type });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

export const mockApi: Api = {
  appInfo: async () => info,
  settingsGet: async () => settings,
  settingsSet: async (s) => (settings = s),
  libraryList: async () => {
    await ensure();
    return [...shows.values()].map<Entry>((s) => ({ summary: summary(s), dir: '(memory)', commits: history.get(s.id)?.length ?? 0, vendorFiles: s.vendor.length }));
  },
  showLoad: async (id) => {
    await ensure();
    const s = shows.get(id);
    if (!s) throw new Error(`no show ${id}`);
    return structuredClone(s);
  },
  showSave: async (show, message) => {
    show.meta.modified = new Date().toISOString();
    shows.set(show.id, structuredClone(show));
    const h = history.get(show.id) ?? [];
    const commit: Commit = { id: `demo${h.length}`, parent: h[h.length - 1]?.id, at: show.meta.modified, message, hash: String(h.length), changes: 1, summary: summary(show) };
    h.push(commit);
    history.set(show.id, h);
    return { show, commit };
  },
  showNew: async (name, platform, model) => {
    const now = new Date().toISOString();
    const s: Show = { schema: 'showbook/1', id: `demo-${Date.now()}`, meta: { name, notes: '', tags: [], created: now, modified: now }, platform, system: { model, firmware: '', name: '', frames: [] }, inputs: [], sources: [], outputs: [], screens: [], presets: [], masterPresets: [], cues: [], multiviewers: [], stills: [], vendor: [], notes: [] };
    shows.set(s.id, s);
    history.set(s.id, [{ id: 'demo0', at: now, message: 'Created', hash: '0', changes: 0, summary: summary(s) }]);
    return s;
  },
  showHistory: async (id) => history.get(id) ?? [],
  showSnapshot: async (id) => shows.get(id)!,
  showDiff: async () => [],
  showRestore: () => notDesktop('Restore'),
  showDelete: async (id) => void shows.delete(id),
  showDuplicate: async (id, name) => {
    const s = structuredClone(shows.get(id)!);
    s.id = `demo-${Date.now()}`;
    s.meta.name = name;
    shows.set(s.id, s);
    history.set(s.id, []);
    return s;
  },
  showValidate: async () => [],
  importPath: () => notDesktop('Import'),
  exportShowJson: async (id) => download(`${shows.get(id)?.meta.name ?? 'show'}.showbook.json`, JSON.stringify(shows.get(id), null, 2), 'application/json'),
  vendorExport: () => notDesktop('Vendor export'),
  writeFile: async (path, base64) => download(path.split('/').pop() ?? 'file', Uint8Array.from(atob(base64), (c) => c.charCodeAt(0)), 'application/octet-stream'),
  writeText: async (path, text) => download(path.split('/').pop() ?? 'file', text, 'text/plain'),
  readText: () => notDesktop('Reading a file'),
  deviceProbe: () => notDesktop('Probing a device'),
  devicePull: () => notDesktop('Pulling from a device'),
  devicePush: () => notDesktop('Pushing to a device'),
  deviceRecall: () => notDesktop('Recalling on a device'),
  deviceTake: () => notDesktop('TAKE'),
  convertShow: () => notDesktop('Conversion'),
  capabilities: () => notDesktop('Capabilities'),
  companionExport: () => notDesktop('Companion export'),
  companionImport: () => notDesktop('Companion import'),
  syncRun: () => notDesktop('Sync'),
  oauthBegin: () => notDesktop('Sign-in'),
  oauthFinish: () => notDesktop('Sign-in'),
  oauthRefresh: () => notDesktop('Sign-in'),
};

// For looking at the renderers from the browser console in the demo.
(window as unknown as { __showbookDemo: unknown }).__showbookDemo = { shows, outputPattern, screenMap };
