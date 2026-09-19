import { useMemo, useState } from 'react';

import { useStore } from '../store';
import type { LayerState, PresetTarget, Rect, Screen } from '../types';
import { ScreenCanvas } from './ScreenCanvas';
import { Field, Panel } from './ui';

function programState(screen: Screen): PresetTarget | null {
  const ps = screen.extra?.programState as PresetTarget | undefined;
  return ps ?? null;
}

export function ScreensTab() {
  const show = useStore((s) => s.show)!;
  const update = useStore((s) => s.update);
  const selectedScreen = useStore((s) => s.selectedScreen);
  const selectScreen = useStore((s) => s.selectScreen);
  const [stateKey, setStateKey] = useState<string>('program');
  const [layerSel, setLayerSel] = useState<string | null>(null);

  const screen = show.screens.find((s) => s.id === selectedScreen) ?? show.screens[0];
  const presetsHere = useMemo(() => show.presets.filter((p) => p.targets.some((t) => t.screenId === screen?.id)), [show.presets, screen?.id]);

  if (!screen) {
    return (
      <Panel title="Screens" actions={<AddScreen />}>
        <div className="muted">No screens. Add one, or import a show.</div>
      </Panel>
    );
  }

  const pgm = programState(screen);
  const editingPreset = stateKey !== 'program' && stateKey !== 'preview' ? show.presets.find((p) => p.id === stateKey) : undefined;
  const target: PresetTarget | null = editingPreset ? (editingPreset.targets.find((t) => t.screenId === screen.id) ?? null) : stateKey === 'preview' ? ((screen.extra?.previewState as PresetTarget | undefined) ?? null) : pgm;
  const layers: LayerState[] = target?.layers ?? [];
  const editable = Boolean(editingPreset);
  const sIdx = show.screens.findIndex((s) => s.id === screen.id);

  const setLayer = (layerId: string, fn: (l: LayerState) => void) => {
    if (!editingPreset) return;
    update((s) => {
      const p = s.presets.find((x) => x.id === editingPreset.id)!;
      const t = p.targets.find((x) => x.screenId === screen.id)!;
      let l = t.layers.find((x) => x.layerId === layerId);
      if (!l) {
        l = { layerId, visible: true };
        t.layers.push(l);
      }
      fn(l);
    });
  };

  return (
    <div className="split">
      <div className="side">
        <Panel title="Screens" actions={<AddScreen />}>
          <ul className="list">
            {show.screens.map((s) => (
              <li key={s.id} className={`list-item${s.id === screen.id ? ' list-item--on' : ''}`} onClick={() => selectScreen(s.id)}>
                <span className="mono muted">{s.id.replace(/^(scr|aux):/, '')}</span> {s.label}
                <span className="muted small">
                  {' '}
                  {s.kind === 'aux' ? 'aux · ' : ''}
                  {s.size.w}×{s.size.h} · {s.outputs.length} out · {s.layers.filter((l) => l.kind === 'mixer').length} layers
                </span>
              </li>
            ))}
          </ul>
        </Panel>
        <Panel title="Screen">
          <Field label="Label">
            <input value={screen.label} onChange={(e) => update((s) => void (s.screens[sIdx].label = e.target.value))} />
          </Field>
          <div className="row wrap">
            <Field label="Width">
              <input className="num" type="number" value={screen.size.w} onChange={(e) => update((s) => void (s.screens[sIdx].size.w = Number(e.target.value)))} />
            </Field>
            <Field label="Height">
              <input className="num" type="number" value={screen.size.h} onChange={(e) => update((s) => void (s.screens[sIdx].size.h = Number(e.target.value)))} />
            </Field>
            <Field label="Kind">
              <select value={screen.kind} onChange={(e) => update((s) => void (s.screens[sIdx].kind = e.target.value as 'screen' | 'aux'))}>
                <option value="screen">screen</option>
                <option value="aux">aux</option>
              </select>
            </Field>
          </div>
          <Field label="Transition (ms)">
            <input type="number" value={screen.transition?.durationMs ?? ''} onChange={(e) => update((s) => void (s.screens[sIdx].transition = { ...(s.screens[sIdx].transition ?? {}), durationMs: e.target.value ? Number(e.target.value) : undefined }))} />
          </Field>
          <div className="field-label">Outputs on this screen</div>
          {screen.outputs.map((om, i) => {
            const o = show.outputs.find((x) => x.id === om.outputId);
            return (
              <div key={om.outputId} className="row tight">
                <span className="grow">{o?.label ?? om.outputId}</span>
                <input className="num" type="number" value={om.rect.x} title="x" onChange={(e) => update((s) => void (s.screens[sIdx].outputs[i].rect.x = Number(e.target.value)))} />
                <input className="num" type="number" value={om.rect.y} title="y" onChange={(e) => update((s) => void (s.screens[sIdx].outputs[i].rect.y = Number(e.target.value)))} />
                <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.screens[sIdx].outputs.splice(i, 1))}>
                  ×
                </button>
              </div>
            );
          })}
          <select
            value=""
            onChange={(e) => {
              const id = e.target.value;
              if (!id) return;
              update((s) => {
                const o = s.outputs.find((x) => x.id === id);
                const f = o?.format;
                s.screens[sIdx].outputs.push({ outputId: id, rect: { x: 0, y: 0, w: f?.width ?? s.screens[sIdx].size.w, h: f?.height ?? s.screens[sIdx].size.h } });
              });
            }}
          >
            <option value="">+ add output…</option>
            {show.outputs
              .filter((o) => !screen.outputs.some((m) => m.outputId === o.id))
              .map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
          </select>
          <div className="field-label">Layers</div>
          {screen.layers.map((l, i) => (
            <div key={l.id} className="row tight">
              <span className="mono muted">{l.id.replace(/^layer:/, '')}</span>
              <input className="grow" value={l.label} onChange={(e) => update((s) => void (s.screens[sIdx].layers[i].label = e.target.value))} />
              <span className="muted small">{l.kind}{l.capacity ? ` · ${l.capacity}` : ''}</span>
              <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.screens[sIdx].layers.splice(i, 1))}>
                ×
              </button>
            </div>
          ))}
          <button
            type="button"
            className="btn small"
            onClick={() =>
              update((s) => {
                const sc = s.screens[sIdx];
                const n = sc.layers.filter((l) => l.kind === 'mixer').length + 1;
                sc.layers.push({ id: `layer:${n}`, label: `Layer ${n}`, kind: 'mixer', z: n });
              })
            }
          >
            Add layer
          </button>
          <button type="button" className="btn small danger" onClick={() => update((s) => void s.screens.splice(sIdx, 1))} style={{ marginLeft: 8 }}>
            Delete screen
          </button>
        </Panel>
      </div>
      <div className="grow">
        <Panel
          title={
            <span>
              {screen.label} <span className="muted">{screen.size.w}×{screen.size.h}</span>
            </span>
          }
          actions={
            <select value={stateKey} onChange={(e) => setStateKey(e.target.value)}>
              <option value="program">{pgm ? `Program as captured${screen.extra?.programLetter ? ` (${screen.extra.programLetter})` : ''}` : 'Program (not captured)'}</option>
              {screen.extra?.previewState ? <option value="preview">Preview as captured ({String(screen.extra.previewLetter ?? '')})</option> : null}
              {presetsHere.map((p) => (
                <option key={p.id} value={p.id}>
                  Preset {p.number ?? ''} {p.label}
                </option>
              ))}
            </select>
          }
        >
          <ScreenCanvas show={show} screen={screen} layers={layers} background={target?.background} selected={layerSel} onSelect={setLayerSel} onChange={editable ? (id, rect: Rect) => setLayer(id, (l) => void (l.rect = rect)) : undefined} width={720} />
          {!target ? <div className="muted">Nothing to draw: this screen has no captured state and no preset targets it yet. Pick a preset on the Presets tab and add this screen to it.</div> : null}
          {editable ? <div className="muted small">Drag a layer to move it, the corner handle to resize. Edits go into preset "{editingPreset?.label}".</div> : target ? <div className="muted small">Captured state is read-only; choose a preset to edit.</div> : null}
        </Panel>
        {target ? (
          <Panel title="Layers in this state">
            <table className="table">
              <thead>
                <tr>
                  <th>Layer</th>
                  <th>Source</th>
                  <th>On</th>
                  <th>X</th>
                  <th>Y</th>
                  <th>W</th>
                  <th>H</th>
                  <th>Opacity</th>
                  <th>Border</th>
                </tr>
              </thead>
              <tbody>
                {screen.layers
                  .filter((d) => d.kind !== 'background')
                  .map((d) => {
                    const l = layers.find((x) => x.layerId === d.id) ?? { layerId: d.id, visible: false };
                    const r = l.rect ?? { x: 0, y: 0, w: 0, h: 0 };
                    const num = (k: keyof Rect) => (
                      <input className="num" type="number" value={Math.round(r[k])} disabled={!editable} onChange={(e) => setLayer(d.id, (x) => void (x.rect = { ...(x.rect ?? { x: 0, y: 0, w: screen.size.w / 2, h: screen.size.h / 2 }), [k]: Number(e.target.value) }))} />
                    );
                    return (
                      <tr key={d.id} className={layerSel === d.id ? 'row--on' : ''} onClick={() => setLayerSel(d.id)}>
                        <td>{d.label}</td>
                        <td>
                          <select value={l.sourceId ?? ''} disabled={!editable} onChange={(e) => setLayer(d.id, (x) => void (x.sourceId = e.target.value || undefined))}>
                            <option value="">— none —</option>
                            {show.sources.map((s) => (
                              <option key={s.id} value={s.id}>
                                {s.label}
                              </option>
                            ))}
                          </select>
                        </td>
                        <td>
                          <input type="checkbox" checked={l.visible} disabled={!editable} onChange={(e) => setLayer(d.id, (x) => void (x.visible = e.target.checked))} />
                        </td>
                        <td>{num('x')}</td>
                        <td>{num('y')}</td>
                        <td>{num('w')}</td>
                        <td>{num('h')}</td>
                        <td>
                          <input className="num" type="number" min={0} max={100} value={Math.round((l.opacity ?? 1) * 100)} disabled={!editable} onChange={(e) => setLayer(d.id, (x) => void (x.opacity = Number(e.target.value) / 100))} />
                        </td>
                        <td className="muted small">{l.border ? `${l.border.width}px ${l.border.color}` : ''}</td>
                      </tr>
                    );
                  })}
                {screen.layers.some((d) => d.kind === 'background') ? (
                  <tr>
                    <td>Background</td>
                    <td colSpan={8}>
                      <select
                        value={target.background ?? ''}
                        disabled={!editable}
                        onChange={(e) =>
                          update((s) => {
                            const p = s.presets.find((x) => x.id === editingPreset!.id)!;
                            const t = p.targets.find((x) => x.screenId === screen.id)!;
                            t.background = e.target.value || undefined;
                          })
                        }
                      >
                        <option value="">— none —</option>
                        {show.sources.map((s) => (
                          <option key={s.id} value={s.id}>
                            {s.label}
                          </option>
                        ))}
                      </select>
                    </td>
                  </tr>
                ) : null}
              </tbody>
            </table>
          </Panel>
        ) : null}
      </div>
    </div>
  );
}

function AddScreen() {
  const update = useStore((s) => s.update);
  const selectScreen = useStore((s) => s.selectScreen);
  return (
    <button
      type="button"
      className="btn small"
      onClick={() => {
        const id = `scr:new${Date.now()}`;
        update((s) => {
          const n = s.screens.filter((x) => x.kind === 'screen').length + 1;
          s.screens.push({
            id,
            label: `Screen ${n}`,
            kind: 'screen',
            size: { w: 1920, h: 1080 },
            outputs: [],
            layers: [
              { id: 'layer:bg', label: 'Background', kind: 'background', z: 0 },
              { id: 'layer:1', label: 'Layer 1', kind: 'mixer', z: 1 },
              { id: 'layer:2', label: 'Layer 2', kind: 'mixer', z: 2 },
            ],
          });
        });
        selectScreen(id);
      }}
    >
      Add screen
    </button>
  );
}
