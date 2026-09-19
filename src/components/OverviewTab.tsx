import { useEffect, useState } from 'react';

import { api, inTauri } from '../lib/ipc';
import { useStore } from '../store';
import { PLATFORM_LABEL } from '../types';
import { Chassis } from './Chassis';
import { fmtDate } from '../lib/format';
import { Field, Panel, Stat } from './ui';

export function OverviewTab() {
  const show = useStore((s) => s.show)!;
  const info = useStore((s) => s.info);
  const update = useStore((s) => s.update);
  const [problems, setProblems] = useState<string[]>([]);

  useEffect(() => {
    if (!inTauri) return;
    let live = true;
    void api.showValidate(show).then((p) => live && setProblems(p)).catch(() => {});
    return () => {
      live = false;
    };
  }, [show]);

  const models = info?.platforms.find((p) => p.id === show.platform)?.models ?? [];
  const screens = show.screens.filter((s) => s.kind === 'screen');
  const auxes = show.screens.filter((s) => s.kind === 'aux');
  const mixers = show.system.extra?.mixers as { usedInScreen: string; enabled: boolean }[] | undefined;
  const dropped = show.notes.filter((n) => n.level === 'dropped');
  const adapted = show.notes.filter((n) => n.level === 'adapted');
  const infos = show.notes.filter((n) => n.level === 'info');

  return (
    <div className="grid-2">
      <Panel title="System">
        <div className="row wrap">
          <Field label="Platform">
            <input value={PLATFORM_LABEL[show.platform]} readOnly />
          </Field>
          <Field label="Model">
            <input list="models" value={show.system.model} onChange={(e) => update((s) => void (s.system.model = e.target.value))} />
            <datalist id="models">
              {models.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          </Field>
          <Field label="Firmware">
            <input value={show.system.firmware} onChange={(e) => update((s) => void (s.system.firmware = e.target.value))} />
          </Field>
          <Field label="Device name">
            <input value={show.system.name} onChange={(e) => update((s) => void (s.system.name = e.target.value))} />
          </Field>
          <Field label="Native rate">
            <input value={show.system.nativeRate ?? ''} onChange={(e) => update((s) => void (s.system.nativeRate = e.target.value ? Number(e.target.value) : undefined))} />
          </Field>
          <Field label="Genlock">
            <input value={show.system.genlock ? `${show.system.genlock.source}${show.system.genlock.locked ? ' (locked)' : ''}` : '—'} readOnly />
          </Field>
        </div>
        <Field label="Tags (comma separated)">
          <input value={show.meta.tags.join(', ')} onChange={(e) => update((s) => void (s.meta.tags = e.target.value.split(',').map((t) => t.trim()).filter(Boolean)))} />
        </Field>
        <Field label="Show notes">
          <textarea rows={6} value={show.meta.notes} onChange={(e) => update((s) => void (s.meta.notes = e.target.value))} />
        </Field>
        <div className="muted small">
          Created {fmtDate(show.meta.created)} · modified {fmtDate(show.meta.modified)}
          {show.meta.source ? ` · from ${show.meta.source.kind} ${show.meta.source.origin} at ${fmtDate(show.meta.source.at)}` : ''}
        </div>
      </Panel>
      <Panel title="At a glance">
        <div className="stats">
          <Stat label="inputs" value={show.inputs.length} />
          <Stat label="outputs" value={show.outputs.length} />
          <Stat label="screens" value={screens.length} />
          <Stat label="aux" value={auxes.length} />
          <Stat label="presets" value={show.presets.length} />
          <Stat label="masters" value={show.masterPresets.length} />
          <Stat label="cues" value={show.cues.length} />
          <Stat label="multiviewers" value={show.multiviewers.length} />
          <Stat label="stills" value={show.stills.length} />
          <Stat label="layers" value={screens.reduce((n, s) => n + s.layers.filter((l) => l.kind === 'mixer').length, 0)} />
        </div>
        {mixers ? (
          <div className="muted small">
            Mixer allocation: {mixers.filter((m) => m.enabled).length} of {mixers.length} mixers enabled ·{' '}
            {Object.entries(
              mixers.filter((m) => m.enabled).reduce<Record<string, number>>((acc, m) => ((acc[m.usedInScreen || '—'] = (acc[m.usedInScreen || '—'] ?? 0) + 1), acc), {}),
            )
              .map(([k, v]) => `${k}: ${v}`)
              .join(', ')}
          </div>
        ) : null}
        {problems.length ? (
          <div className="problems">
            <strong>{problems.length} reference problem{problems.length === 1 ? '' : 's'}</strong>
            <ul>
              {problems.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
          </div>
        ) : null}
      </Panel>
      <Panel title="Chassis" className="span-2">
        <Chassis show={show} />
      </Panel>
      {show.notes.length ? (
        <Panel title={`Import & conversion notes (${dropped.length} dropped, ${adapted.length} adapted, ${infos.length} info)`} className="span-2">
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
  );
}
