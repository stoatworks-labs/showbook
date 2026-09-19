import { useState } from 'react';

import { api, toRef } from '../lib/ipc';
import { useStore } from '../store';
import { tail, type Preset } from '../types';
import { sourceLabel } from '../lib/format';
import { ScreenCanvas } from './ScreenCanvas';
import { Field, Panel } from './ui';

export function PresetsTab() {
  const show = useStore((s) => s.show)!;
  const update = useStore((s) => s.update);
  const settings = useStore((s) => s.settings);
  const selectedPreset = useStore((s) => s.selectedPreset);
  const selectPreset = useStore((s) => s.selectPreset);
  const setTab = useStore((s) => s.setTab);
  const selectScreen = useStore((s) => s.selectScreen);
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const [deviceIdx, setDeviceIdx] = useState(0);
  const [toProgram, setToProgram] = useState(false);

  const preset = show.presets.find((p) => p.id === selectedPreset) ?? show.presets[0];
  const pIdx = show.presets.findIndex((p) => p.id === preset?.id);
  const devices = (settings?.devices ?? []).filter((d) => d.platform === show.platform);
  const device = devices[deviceIdx];
  const isEm = show.platform === 'barco-em' || show.platform === 'barco-pds4k';

  const recall = async (p: Preset, screen?: string) => {
    if (!device) return;
    const number = isEm ? Number(tail(p.id)) : (p.number ?? 0);
    const r = await run('Recalling', () => api.deviceRecall(toRef(device), { kind: 'preset', number, screen, program: toProgram }));
    if (r !== undefined) toast(`Recalled ${p.label} to ${toProgram ? 'program' : 'preview'} on ${device.name}`);
  };
  const recallMaster = async (number: number, label: string) => {
    if (!device) return;
    const r = await run('Recalling', () => api.deviceRecall(toRef(device), { kind: 'master', number, program: toProgram }));
    if (r !== undefined) toast(`Recalled master ${label} on ${device.name}`);
  };
  const take = async () => {
    if (!device) return;
    const r = await run('Take', () => api.deviceTake(toRef(device), []));
    if (r !== undefined) toast(`TAKE on ${device.name}`);
  };

  const addPreset = () => {
    const number = (show.presets.reduce((m, p) => Math.max(m, p.number ?? 0), 0) || 0) + 1;
    const id = isEm ? `pre:${number - 1}` : `pre:${number}`;
    update((s) => void s.presets.push({ id: s.presets.some((p) => p.id === id) ? `pre:new${Date.now()}` : id, number, label: `Preset ${number}`, notes: '', targets: [] }));
    selectPreset(id);
  };

  return (
    <div className="split">
      <div className="side">
        <Panel
          title={`Presets (${show.presets.length})`}
          actions={
            <button type="button" className="btn small" onClick={addPreset}>
              Add
            </button>
          }
        >
          <ul className="list">
            {show.presets.map((p) => (
              <li key={p.id} className={`list-item${p.id === preset?.id ? ' list-item--on' : ''}`} onClick={() => selectPreset(p.id)}>
                <span className="mono muted">{p.number ?? tail(p.id)}</span> {p.label}
                <span className="muted small"> · {p.targets.map((t) => show.screens.find((s) => s.id === t.screenId)?.label ?? t.screenId).join(', ') || 'no screens'}</span>
              </li>
            ))}
          </ul>
        </Panel>
        <Panel
          title={`Master presets (${show.masterPresets.length})`}
          actions={
            <button
              type="button"
              className="btn small"
              onClick={() =>
                update((s) => {
                  const n = s.masterPresets.reduce((m, p) => Math.max(m, p.number ?? 0), 0) + 1;
                  s.masterPresets.push({ id: `master:${n}`, number: n, label: `Master ${n}`, entries: [] });
                })
              }
            >
              Add
            </button>
          }
        >
          {show.masterPresets.map((m, mi) => (
            <div key={m.id} className="master">
              <div className="row tight">
                <span className="mono muted">{m.number ?? tail(m.id)}</span>
                <input className="grow" value={m.label} onChange={(e) => update((s) => void (s.masterPresets[mi].label = e.target.value))} />
                {device && !isEm ? (
                  <button type="button" className="btn tiny" onClick={() => void recallMaster(m.number ?? 0, m.label)}>
                    Recall
                  </button>
                ) : null}
                <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.masterPresets.splice(mi, 1))}>
                  ×
                </button>
              </div>
              {m.entries.map((e, ei) => (
                <div key={ei} className="row tight indent">
                  <span className="muted small">{show.screens.find((s) => s.id === e.screenId)?.label ?? e.screenId} ←</span>
                  <select value={e.presetId} onChange={(ev) => update((s) => void (s.masterPresets[mi].entries[ei].presetId = ev.target.value))}>
                    {show.presets.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.number ?? tail(p.id)} {p.label}
                      </option>
                    ))}
                  </select>
                  <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.masterPresets[mi].entries.splice(ei, 1))}>
                    ×
                  </button>
                </div>
              ))}
              <select
                value=""
                onChange={(ev) => {
                  const sid = ev.target.value;
                  if (!sid) return;
                  update((s) => void s.masterPresets[mi].entries.push({ screenId: sid, presetId: s.presets[0]?.id ?? '' }));
                }}
              >
                <option value="">+ screen…</option>
                {show.screens
                  .filter((s) => !m.entries.some((e) => e.screenId === s.id))
                  .map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.label}
                    </option>
                  ))}
              </select>
            </div>
          ))}
        </Panel>
      </div>
      <div className="grow">
        {preset ? (
          <>
            <Panel
              title={
                <span>
                  Preset <span className="mono">{preset.number ?? tail(preset.id)}</span>
                </span>
              }
              actions={
                <>
                  {devices.length ? (
                    <>
                      <select value={deviceIdx} onChange={(e) => setDeviceIdx(Number(e.target.value))}>
                        {devices.map((d, i) => (
                          <option key={d.name} value={i}>
                            {d.name}
                          </option>
                        ))}
                      </select>
                      <label className="check">
                        <input type="checkbox" checked={toProgram} onChange={(e) => setToProgram(e.target.checked)} /> to program
                      </label>
                      {isEm ? (
                        <button type="button" className="btn small" onClick={() => void recall(preset)}>
                          Recall on {device?.name}
                        </button>
                      ) : (
                        preset.targets.map((t) => (
                          <button key={t.screenId} type="button" className="btn small" onClick={() => void recall(preset, tail(t.screenId))}>
                            Recall → {show.screens.find((s) => s.id === t.screenId)?.label ?? t.screenId}
                          </button>
                        ))
                      )}
                      <button type="button" className="btn small danger" onClick={() => void take()}>
                        TAKE
                      </button>
                    </>
                  ) : (
                    <span className="muted small">add a {show.platform} device under Devices to recall from here</span>
                  )}
                  <button type="button" className="btn small danger" onClick={() => update((s) => void s.presets.splice(pIdx, 1))}>
                    Delete preset
                  </button>
                </>
              }
            >
              <div className="row">
                <Field label="Number">
                  <input className="num" type="number" value={preset.number ?? ''} onChange={(e) => update((s) => void (s.presets[pIdx].number = e.target.value ? Number(e.target.value) : undefined))} />
                </Field>
                <Field label="Label">
                  <input value={preset.label} onChange={(e) => update((s) => void (s.presets[pIdx].label = e.target.value))} />
                </Field>
                <Field label="Notes">
                  <input value={preset.notes} onChange={(e) => update((s) => void (s.presets[pIdx].notes = e.target.value))} />
                </Field>
              </div>
              <div className="row tight">
                <span className="field-label">Screens in this preset:</span>
                {preset.targets.map((t, ti) => (
                  <span key={t.screenId} className="chip">
                    {show.screens.find((s) => s.id === t.screenId)?.label ?? t.screenId}
                    <button type="button" className="chip-x" onClick={() => update((s) => void s.presets[pIdx].targets.splice(ti, 1))}>
                      ×
                    </button>
                  </span>
                ))}
                <select
                  value=""
                  onChange={(e) => {
                    const sid = e.target.value;
                    if (!sid) return;
                    update((s) => {
                      const sc = s.screens.find((x) => x.id === sid)!;
                      s.presets[pIdx].targets.push({ screenId: sid, layers: sc.layers.filter((l) => l.kind !== 'background').map((l) => ({ layerId: l.id, visible: false, rect: { x: 0, y: 0, w: Math.round(sc.size.w / 2), h: Math.round(sc.size.h / 2) } })) });
                    });
                  }}
                >
                  <option value="">+ add screen…</option>
                  {show.screens
                    .filter((s) => !preset.targets.some((t) => t.screenId === s.id))
                    .map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.label}
                      </option>
                    ))}
                </select>
              </div>
            </Panel>
            {preset.targets.map((t) => {
              const sc = show.screens.find((s) => s.id === t.screenId);
              if (!sc) return null;
              return (
                <Panel
                  key={t.screenId}
                  title={sc.label}
                  actions={
                    <button
                      type="button"
                      className="btn small"
                      onClick={() => {
                        selectScreen(sc.id);
                        setTab('screens');
                      }}
                    >
                      Edit layers on the Screens tab
                    </button>
                  }
                >
                  <div className="row">
                    <ScreenCanvas show={show} screen={sc} layers={t.layers} background={t.background} width={420} />
                    <div className="grow">
                      <table className="table">
                        <tbody>
                          {t.background ? (
                            <tr>
                              <td className="muted">background</td>
                              <td>{sourceLabel(show, t.background)}</td>
                              <td />
                            </tr>
                          ) : null}
                          {t.layers.map((l) => (
                            <tr key={l.layerId} className={l.visible ? '' : 'dim'}>
                              <td className="muted">{sc.layers.find((d) => d.id === l.layerId)?.label ?? l.layerId}</td>
                              <td>{l.sourceId ? sourceLabel(show, l.sourceId) : '—'}</td>
                              <td className="mono muted">{l.rect ? `${Math.round(l.rect.x)},${Math.round(l.rect.y)} ${Math.round(l.rect.w)}×${Math.round(l.rect.h)}` : ''}</td>
                            </tr>
                          ))}
                          {t.layers.length === 0 && !t.background ? (
                            <tr>
                              <td className="muted" colSpan={3}>
                                No layer content recorded for this screen (a live capture lists which screens a preset touches; the layers come from the backup archive or a deep capture).
                              </td>
                            </tr>
                          ) : null}
                        </tbody>
                      </table>
                    </div>
                  </div>
                </Panel>
              );
            })}
          </>
        ) : (
          <Panel title="Presets">
            <div className="muted">No presets. Add one, or pull the show from a device.</div>
          </Panel>
        )}
      </div>
    </div>
  );
}
