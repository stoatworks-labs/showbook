import { open } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';

import { api } from '../lib/ipc';
import { useStore } from '../store';
import { PLATFORM_LABEL, type Platform } from '../types';
import { fmtDate } from '../lib/format';
import { Empty, Field, Panel } from './ui';

export function LibraryView() {
  const entries = useStore((s) => s.entries);
  const info = useStore((s) => s.info);
  const settings = useStore((s) => s.settings);
  const openShow = useStore((s) => s.openShow);
  const refresh = useStore((s) => s.refreshLibrary);
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('New show');
  const [platform, setPlatform] = useState<Platform>('barco-em');
  const [model, setModel] = useState('Encore3');
  const [filter, setFilter] = useState('');

  const models = info?.platforms.find((p) => p.id === platform)?.models ?? [];

  const importFile = async () => {
    const picked = await open({
      multiple: false,
      title: 'Import a show file',
      filters: [
        { name: 'Show files', extensions: ['awc', 'gz', 'tgz', 'zip', 'tar', 'xml', 'json'] },
        { name: 'All files', extensions: ['*'] },
      ],
    });
    if (!picked) return;
    const path = Array.isArray(picked) ? picked[0] : picked;
    const r = await run('Importing', () => api.importPath(path));
    if (r) {
      toast(`Imported ${r.summary.name} (${r.kind}): ${r.summary.screens} screens, ${r.summary.presets} presets`);
      await refresh();
      await openShow(r.show.id);
    }
  };

  const importFolder = async () => {
    const picked = await open({ directory: true, title: 'Import an Event Master store folder (holds settings.xml)' });
    if (!picked) return;
    const path = Array.isArray(picked) ? picked[0] : picked;
    const r = await run('Importing', () => api.importPath(path));
    if (r) {
      await refresh();
      await openShow(r.show.id);
    }
  };

  const create = async () => {
    const r = await run('Creating', () => api.showNew(name, platform, model));
    if (r) {
      setCreating(false);
      await refresh();
      await openShow(r.id);
    }
  };

  const visible = entries.filter((e) => !filter || `${e.summary.name} ${e.summary.model} ${PLATFORM_LABEL[e.summary.platform]}`.toLowerCase().includes(filter.toLowerCase()));

  return (
    <div className="view">
      <div className="view-head">
        <h2>Library</h2>
        <span className="muted">{settings?.libraryPath}</span>
        <div className="spacer" />
        <input className="search" placeholder="filter" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <button type="button" className="btn" onClick={() => void importFile()}>
          Import file…
        </button>
        <button type="button" className="btn" onClick={() => void importFolder()}>
          Import folder…
        </button>
        <button type="button" className="btn primary" onClick={() => setCreating((c) => !c)}>
          New show
        </button>
      </div>
      {creating ? (
        <Panel title="New show">
          <div className="row">
            <Field label="Name">
              <input value={name} onChange={(e) => setName(e.target.value)} />
            </Field>
            <Field label="Platform">
              <select
                value={platform}
                onChange={(e) => {
                  const p = e.target.value as Platform;
                  setPlatform(p);
                  setModel(info?.platforms.find((x) => x.id === p)?.models[0] ?? '');
                }}
              >
                {(info?.platforms ?? []).map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Model">
              <select value={model} onChange={(e) => setModel(e.target.value)}>
                {models.map((m) => (
                  <option key={m}>{m}</option>
                ))}
              </select>
            </Field>
            <button type="button" className="btn primary" onClick={() => void create()}>
              Create
            </button>
          </div>
        </Panel>
      ) : null}
      {visible.length === 0 ? (
        <Empty>
          {entries.length === 0
            ? 'No shows yet. Import an Event Master backup (.tar.gz), a folder holding settings.xml, a LivePremier .awc, or pull one from a device.'
            : 'Nothing matches the filter.'}
        </Empty>
      ) : (
        <div className="cards">
          {visible.map((e) => (
            <button key={e.summary.id} type="button" className="card" onClick={() => void openShow(e.summary.id)}>
              <div className="card-title">{e.summary.name}</div>
              <div className="card-sub">
                {PLATFORM_LABEL[e.summary.platform]} · {e.summary.model || '—'}
                {e.summary.firmware ? ` · fw ${e.summary.firmware}` : ''}
              </div>
              <div className="card-stats">
                <span>{e.summary.inputs} in</span>
                <span>{e.summary.outputs} out</span>
                <span>{e.summary.screens} scr</span>
                {e.summary.auxes ? <span>{e.summary.auxes} aux</span> : null}
                <span>{e.summary.presets} presets</span>
                {e.summary.masterPresets ? <span>{e.summary.masterPresets} masters</span> : null}
                {e.summary.cues ? <span>{e.summary.cues} cues</span> : null}
                {e.summary.multiviewers ? <span>{e.summary.multiviewers} MV</span> : null}
              </div>
              <div className="card-foot">
                <span>{fmtDate(e.summary.modified)}</span>
                <span>
                  {e.commits} version{e.commits === 1 ? '' : 's'}
                  {e.vendorFiles ? ` · ${e.vendorFiles} vendor file${e.vendorFiles === 1 ? '' : 's'}` : ''}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
