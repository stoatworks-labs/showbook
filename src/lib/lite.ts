/**
 * showbook-lite: the same commands the desktop app answers, answered in the
 * browser. The parsers, the conversion and the Companion export are the Rust
 * core compiled to WebAssembly (lite/pkg, from crates/showbook-wasm); the
 * library is IndexedDB in this browser, with the same content-addressed
 * history the desktop keeps on disk. Files come in through a file picker and
 * go out as downloads. Anything that needs a device or a cloud account says
 * so and points at the full app.
 */
import type { AppInfo, Commit, Entry, Settings, Show, Summary } from '../types';
import type { api as realApi } from './ipc';
import { take as takePicked } from './picked';

type Api = typeof realApi;


// ------------------------------------------------------------------ the core

type Wasm = typeof import('showbook-core');
let core: Promise<Wasm> | null = null;

function wasm(): Promise<Wasm> {
  if (!core) {
    core = (async () => {
      const mod = await import('showbook-core');
      await mod.default();
      return mod;
    })();
  }
  return core;
}

// --------------------------------------------------------------- the library

const DB = 'showbook-lite';
const STORES = ['shows', 'history', 'snapshots', 'vendor', 'settings'] as const;
type StoreName = (typeof STORES)[number];

let db: Promise<IDBDatabase> | null = null;

function open(): Promise<IDBDatabase> {
  if (!db) {
    db = new Promise((resolve, reject) => {
      const req = indexedDB.open(DB, 1);
      req.onupgradeneeded = () => {
        for (const s of STORES) if (!req.result.objectStoreNames.contains(s)) req.result.createObjectStore(s);
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error ?? new Error('IndexedDB is not available in this browser'));
    });
  }
  return db;
}

function tx<T>(store: StoreName, mode: IDBTransactionMode, fn: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
  return open().then(
    (d) =>
      new Promise<T>((resolve, reject) => {
        const t = d.transaction(store, mode);
        const req = fn(t.objectStore(store));
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
      }),
  );
}

const get = <T>(store: StoreName, key: string) => tx<T | undefined>(store, 'readonly', (s) => s.get(key) as IDBRequest<T | undefined>);
const put = (store: StoreName, key: string, value: unknown) => tx(store, 'readwrite', (s) => s.put(value, key));
const del = (store: StoreName, key: string) => tx(store, 'readwrite', (s) => s.delete(key));
const all = <T>(store: StoreName) => tx<T[]>(store, 'readonly', (s) => s.getAll() as IDBRequest<T[]>);

interface VendorRecord {
  name: string;
  bytes: Uint8Array;
}

// ------------------------------------------------------------------ helpers

function nowIso(): string {
  return new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** The desktop library's commit id: the first ten hex of a hash over parent + hash + time. */
async function shortId(seed: string): Promise<string> {
  const w = await wasm();
  return w.sha256(new TextEncoder().encode(seed)).slice(0, 10);
}

const MIME: Record<string, string> = { pdf: 'application/pdf', png: 'image/png', json: 'application/json', zip: 'application/zip', companionconfig: 'application/json', txt: 'text/plain' };

function download(name: string, bytes: Uint8Array | string, type?: string): void {
  const blob = new Blob([bytes as BlobPart], { type: type ?? MIME[name.split('.').pop() ?? ''] ?? 'application/octet-stream' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

const basename = (path: string) => path.split('/').pop() || 'file';

const needsApp = (what: string) =>
  Promise.reject(new Error(`${what} needs the desktop app — it is free at https://stoatworks-labs.com/software/showbook/`));

async function readPicked(token: string): Promise<{ name: string; bytes: Uint8Array }> {
  const file = takePicked(token);
  if (!file) throw new Error('no file was chosen');
  return { name: file.name, bytes: new Uint8Array(await file.arrayBuffer()) };
}

async function loadShow(id: string): Promise<Show> {
  const s = await get<Show>('shows', id);
  if (!s) throw new Error(`no show ${id} in this browser's library`);
  return s;
}

async function history(id: string): Promise<Commit[]> {
  return (await get<Commit[]>('history', id)) ?? [];
}

/** Record a version, the way the desktop library does: content-addressed, and not at all if nothing changed. */
async function commit(show: Show, message: string): Promise<Commit | null> {
  const w = await wasm();
  const settings = await liteApi.settingsGet();
  show.meta.modified = nowIso();
  const json = JSON.stringify(show);
  const hash = w.hash_show(json);
  const index = await history(show.id);
  const last = index[index.length - 1];
  let made: Commit | null = null;
  if (!last || last.hash !== hash) {
    const parent = last?.id;
    let changes = 0;
    if (last) {
      const before = await get<string>('snapshots', last.hash);
      if (before) changes = (w.diff(before, json) as unknown[]).length;
    }
    await put('snapshots', hash, json);
    const at = show.meta.modified;
    made = {
      id: await shortId(`${parent ?? ''}${hash}${at}`),
      parent,
      at,
      message,
      author: settings.author || undefined,
      hash,
      summary: w.summary(json) as Summary,
      changes,
    };
    index.push(made);
    await put('history', show.id, index);
  }
  await put('shows', show.id, show);
  return made;
}

/** Keep a vendor file with the show, content-addressed, as the desktop does under shows/<id>/vendor/. */
async function addVendor(show: Show, kind: string, name: string, bytes: Uint8Array, note: string): Promise<void> {
  const w = await wasm();
  const sha = w.sha256(bytes);
  const ext = name.includes('.') ? `.${name.split('.').pop()}` : '';
  await put('vendor', `${show.id}/${sha}`, { name, bytes } satisfies VendorRecord);
  show.vendor.push({ platform: show.platform, kind, sha256: sha, file: `vendor/${sha}${ext}`, size: bytes.length, capturedAt: nowIso(), note: `${name} — ${note}` });
}

// -------------------------------------------------------- the zip "folders"

/** Files written into a `zip://` folder, until the folder is finished. */
const folders = new Map<string, { name: string; bytes: Uint8Array }[]>();

const DEFAULT_SETTINGS: Settings = {
  libraryPath: "This browser's storage (IndexedDB) — export a show as JSON to take it elsewhere",
  author: '',
  devices: [],
  sync: { provider: 'none', root: '', clientId: '', lastRun: '' },
  companionHost: '',
};

/** The two simulator captures the desktop demo ships, as a starting point. */
export async function loadDemoShows(): Promise<number> {
  let n = 0;
  for (const f of ['encore3-simulator', 'aquilon-cmax-simulator']) {
    const s = (await (await fetch(`./demo/${f}.json`)).json()) as Show;
    if (await get<Show>('shows', s.id)) continue;
    await commit(s, 'Captured from the simulator');
    n++;
  }
  return n;
}

export const liteApi: Api = {
  appInfo: async () => {
    const w = await wasm();
    return { version: __APP_VERSION__, configDir: "this browser's storage", platforms: w.platforms() } as AppInfo;
  },
  settingsGet: async () => ({ ...DEFAULT_SETTINGS, ...((await get<Partial<Settings>>('settings', 'settings')) ?? {}), libraryPath: DEFAULT_SETTINGS.libraryPath, devices: [] }),
  settingsSet: async (s) => {
    await put('settings', 'settings', { author: s.author });
    return liteApi.settingsGet();
  },

  libraryList: async () => {
    const w = await wasm();
    const shows = await all<Show>('shows');
    const entries: Entry[] = [];
    for (const s of shows) {
      entries.push({ summary: w.summary(JSON.stringify(s)) as Summary, dir: `indexeddb://${DB}/shows/${s.id}`, commits: (await history(s.id)).length, vendorFiles: s.vendor.length });
    }
    return entries.sort((a, b) => (a.summary.modified < b.summary.modified ? 1 : -1));
  },
  showLoad: (id) => loadShow(id),
  showSave: async (show, message) => {
    const c = await commit(show, message);
    return { show, commit: c };
  },
  showNew: async (name, platform, model) => {
    const w = await wasm();
    const show = w.new_show(name, platform, model) as Show;
    await commit(show, 'Created');
    return show;
  },
  showHistory: (id) => history(id),
  showSnapshot: async (id, commitId) => {
    const c = (await history(id)).find((x) => x.id === commitId);
    const json = c && (await get<string>('snapshots', c.hash));
    if (!json) throw new Error(`no version ${commitId}`);
    return JSON.parse(json) as Show;
  },
  showDiff: async (id, from, to) => {
    const w = await wasm();
    const index = await history(id);
    const a = index.find((x) => x.id === from);
    const b = index.find((x) => x.id === to);
    const ja = a && (await get<string>('snapshots', a.hash));
    const jb = b && (await get<string>('snapshots', b.hash));
    if (!ja || !jb) throw new Error('a version is missing from this browser');
    return w.diff(ja, jb);
  },
  showRestore: async (id, commitId) => {
    const snap = await liteApi.showSnapshot(id, commitId);
    const c = await commit(snap, `Restored ${commitId}`);
    if (!c) throw new Error('that version is already the current one');
    return c;
  },
  showDelete: async (id) => {
    const index = await history(id);
    for (const c of index) await del('snapshots', c.hash);
    const show = await get<Show>('shows', id);
    for (const v of show?.vendor ?? []) await del('vendor', `${id}/${v.sha256}`);
    await del('history', id);
    await del('shows', id);
  },
  showDuplicate: async (id, name) => {
    const src = await loadShow(id);
    const copy = structuredClone(src);
    copy.id = crypto.randomUUID();
    copy.meta.name = name;
    copy.meta.created = nowIso();
    for (const v of src.vendor) {
      const rec = await get<VendorRecord>('vendor', `${id}/${v.sha256}`);
      if (rec) await put('vendor', `${copy.id}/${v.sha256}`, rec);
    }
    await commit(copy, `Duplicated from ${src.meta.name}`);
    return copy;
  },
  showValidate: async (show) => (await wasm()).validate(JSON.stringify(show)),

  importPath: async (token) => {
    const w = await wasm();
    const { name, bytes } = await readPicked(token);
    const r = w.import_file(name, bytes) as { show: Show; kind: string; summary: Summary };
    if (r.kind === 'awc' || r.kind === 'em-backup') {
      await commit(r.show, 'Imported');
      await addVendor(r.show, r.kind, name, bytes, 'imported file');
    }
    await commit(r.show, `Imported ${name}`);
    return { ...r, summary: w.summary(JSON.stringify(r.show)) as Summary };
  },
  exportShowJson: async (id, path) => download(basename(path), JSON.stringify(await loadShow(id), null, 2), 'application/json'),
  vendorExport: async (id, sha256, path) => {
    const rec = await get<VendorRecord>('vendor', `${id}/${sha256}`);
    if (!rec) throw new Error('that vendor file is not in this browser');
    download(basename(path) || rec.name, rec.bytes);
  },
  writeFile: async (path, base64) => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    const zip = path.match(/^(zip:\/\/[^/]+)\/(.+)$/);
    if (zip) {
      const list = folders.get(zip[1]) ?? [];
      list.push({ name: zip[2], bytes });
      folders.set(zip[1], list);
      return;
    }
    download(basename(path), bytes);
  },
  finishFolder: async (folder) => {
    const list = folders.get(folder);
    if (!list?.length) return;
    folders.delete(folder);
    const w = await wasm();
    const bytes = w.zip_files(
      list.map((f) => f.name),
      list.map((f) => f.bytes),
    );
    download(`${folder.replace(/^zip:\/\//, '')}.zip`, bytes, 'application/zip');
  },
  writeText: async (path, text) => download(basename(path), text, 'text/plain'),
  readText: async (token) => new TextDecoder().decode((await readPicked(token)).bytes),

  bundleExport: () => needsApp('Exporting a bundle'),
  bundleExportAwc: () => needsApp('Embedding a configuration in an .awc'),
  lppSummary: async () => null,
  lppAttach: () => needsApp('Attaching a LivePremier Plus configuration'),
  lppExport: () => needsApp('Exporting a LivePremier Plus configuration'),
  lppDetach: () => needsApp('Editing a LivePremier Plus configuration'),

  deviceProbe: () => needsApp('Talking to a device'),
  devicePull: () => needsApp('Pulling from a device'),
  devicePush: () => needsApp('Pushing to a device'),
  deviceRecall: () => needsApp('Recalling on a device'),
  deviceTake: () => needsApp('TAKE'),

  convertShow: async (id, target, model, save) => {
    const w = await wasm();
    const src = await loadShow(id);
    const r = w.convert(JSON.stringify(src), target, model) as { show: Show; notes: import('../types').Note[] };
    if (save) await commit(r.show, `Converted from ${src.meta.name} (${src.system.model})`);
    return { show: r.show, notes: r.notes, saved: save };
  },
  capabilities: async (platform, model) => (await wasm()).capabilities(platform, model),

  companionExport: async (id, opts, path) => {
    const w = await wasm();
    const text = w.companion_export(JSON.stringify(await loadShow(id)), JSON.stringify(opts));
    const doc = JSON.parse(text) as { type: string; pages?: Record<string, unknown> };
    download(basename(path), text, 'application/json');
    return { path: basename(path), type: doc.type, pages: doc.pages ? Object.keys(doc.pages).length : 1 };
  },
  companionImport: async (id, token) => {
    const w = await wasm();
    const { bytes } = await readPicked(token);
    return w.companion_import(bytes, JSON.stringify(await loadShow(id)));
  },

  syncRun: () => needsApp('Sync'),
  oauthBegin: () => needsApp('Signing in to a drive'),
  oauthFinish: () => needsApp('Signing in to a drive'),
  oauthRefresh: () => needsApp('Signing in to a drive'),
};
