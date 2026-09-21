import { useEffect, useState } from 'react';

import { api } from '../lib/ipc';
import { useStore } from '../store';
import { PLATFORM_LABEL, type Capabilities, type Note, type Platform } from '../types';
import { Field, Panel } from './ui';

export function ConvertView() {
  const entries = useStore((s) => s.entries);
  const info = useStore((s) => s.info);
  const show = useStore((s) => s.show);
  const refresh = useStore((s) => s.refreshLibrary);
  const openShow = useStore((s) => s.openShow);
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const [source, setSource] = useState(show?.id ?? entries[0]?.summary.id ?? '');
  const [target, setTarget] = useState<Platform>('aw-live-premier');
  const [model, setModel] = useState('Aquilon C');
  const [caps, setCaps] = useState<Capabilities | null>(null);
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [converted, setConverted] = useState<string | null>(null);

  const models = info?.platforms.find((p) => p.id === target)?.models ?? [];
  useEffect(() => {
    void api.capabilities(target, model).then(setCaps).catch(() => setCaps(null));
  }, [target, model]);

  const preview = async () => {
    if (!source) return;
    const r = await run('Converting', () => api.convertShow(source, target, model, false));
    if (r) {
      setNotes(r.notes);
      setConverted(null);
    }
  };
  const commit = async () => {
    if (!source) return;
    const r = await run('Converting', () => api.convertShow(source, target, model, true));
    if (r) {
      setNotes(r.notes);
      setConverted(r.show.id);
      toast(`Saved "${r.show.meta.name}" to the library`);
      await refresh();
    }
  };

  const dropped = notes?.filter((n) => n.level === 'dropped') ?? [];
  const adapted = notes?.filter((n) => n.level === 'adapted') ?? [];
  const infos = notes?.filter((n) => n.level === 'info') ?? [];

  return (
    <div className="view">
      <div className="view-head">
        <h2>Convert</h2>
        <span className="muted">a show for a different switcher, with an honest report</span>
      </div>
      <div className="grid-2">
        <Panel title="From → to">
          <Field label="Show">
            <select value={source} onChange={(e) => setSource(e.target.value)}>
              <option value="">—</option>
              {entries.map((e) => (
                <option key={e.summary.id} value={e.summary.id}>
                  {e.summary.name} ({PLATFORM_LABEL[e.summary.platform]} {e.summary.model})
                </option>
              ))}
            </select>
          </Field>
          <div className="row">
            <Field label="Target platform">
              <select
                value={target}
                onChange={(e) => {
                  const p = e.target.value as Platform;
                  setTarget(p);
                  setModel(info?.platforms.find((x) => x.id === p)?.models[0] ?? '');
                }}
              >
                {(info?.platforms ?? []).filter((p) => p.id !== 'generic').map((p) => (
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
          </div>
          <div className="row">
            <button type="button" className="btn" onClick={() => void preview()} disabled={!source}>
              Preview the report
            </button>
            <button type="button" className="btn primary" onClick={() => void commit()} disabled={!source}>
              Convert and save
            </button>
            {converted ? (
              <button type="button" className="btn" onClick={() => void openShow(converted)}>
                Open the converted show
              </button>
            ) : null}
          </div>
        </Panel>
        <Panel title={caps ? `${caps.model} holds` : 'Target'}>
          {caps ? (
            <>
              <div className="stats">
                <div className="stat"><div className="stat-value">{caps.inputs}</div><div className="stat-label">inputs</div></div>
                <div className="stat"><div className="stat-value">{caps.outputs}</div><div className="stat-label">outputs</div></div>
                <div className="stat"><div className="stat-value">{caps.screens}</div><div className="stat-label">screens</div></div>
                <div className="stat"><div className="stat-value">{caps.auxes}</div><div className="stat-label">aux</div></div>
                <div className="stat"><div className="stat-value">{caps.layers4k || '—'}</div><div className="stat-label">4K layers</div></div>
                <div className="stat"><div className="stat-value">{caps.layersPerScreen}</div><div className="stat-label">per screen</div></div>
                <div className="stat"><div className="stat-value">{caps.presetSlots}</div><div className="stat-label">presets</div></div>
                <div className="stat"><div className="stat-value">{caps.masterSlots}</div><div className="stat-label">masters</div></div>
                <div className="stat"><div className="stat-value">{caps.layerMemorySlots === 0 ? '—' : (caps.layerMemorySlots ?? '∞')}</div><div className="stat-label">layer memories</div></div>
                <div className="stat"><div className="stat-value">{caps.auxPresetSlots ?? '—'}</div><div className="stat-label">aux bank</div></div>
              </div>
              <div className="muted small">
                {[caps.backgroundLayer ? 'background layer' : 'no background layer', caps.dsk ? 'DSK' : 'no DSK', caps.cues ? 'cues' : 'no cues', caps.presetMultiScreen ? 'multi-screen presets' : 'one screen per memory', `${caps.multiviewers} multiviewer${caps.multiviewers === 1 ? '' : 's'} × ${caps.widgetsPerMv} windows`, caps.mvMemories ? `${caps.mvMemories} multiviewer memories` : 'no multiviewer bank', caps.layerMemorySlots === 0 ? 'no layer bank (a look lives inside a memory)' : 'layer memories (Event Master calls them user keys)', caps.auxPresetSlots ? 'aux memories in their own bank' : 'auxes share the memory bank'].join(' · ')}
              </div>
              <ul className="notes">
                {caps.notes.map((n) => (
                  <li key={n} className="note note--info note--plain">
                    <span>{n}</span>
                  </li>
                ))}
              </ul>
            </>
          ) : null}
        </Panel>
        {notes ? (
          <Panel title={`Report: ${dropped.length} dropped, ${adapted.length} adapted, ${infos.length} notes`} className="span-2">
            <ul className="notes">
              {[...dropped, ...adapted, ...infos].map((n, i) => (
                <li key={i} className={`note note--${n.level}`}>
                  <span className="note-level">{n.level}</span>
                  <span className="note-path">{n.path}</span>
                  <span>{n.message}</span>
                </li>
              ))}
            </ul>
          </Panel>
        ) : null}
      </div>
    </div>
  );
}
