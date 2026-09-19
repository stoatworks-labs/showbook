import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';

import { useStore } from '../store';
import type { Rect, Widget } from '../types';
import { sourceLabel } from '../lib/format';
import { Field, Panel } from './ui';

export function MultiviewersTab() {
  const show = useStore((s) => s.show)!;
  const update = useStore((s) => s.update);
  const [mvIdx, setMvIdx] = useState(0);
  const [layIdx, setLayIdx] = useState(0);
  const [sel, setSel] = useState<string | null>(null);
  const [grid, setGrid] = useState({ cols: 4, rows: 3 });
  const svg = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState<{ id: string; mode: 'move' | 'resize'; start: { x: number; y: number }; rect: Rect } | null>(null);

  const mv = show.multiviewers[mvIdx];
  const layout = mv?.layouts[layIdx] ?? mv?.layouts[0];
  const W = layout?.size.w ?? 1920;
  const H = layout?.size.h ?? 1080;
  const width = 720;
  const scale = width / W;

  const setWidget = (id: string, fn: (w: Widget) => void) =>
    update((s) => {
      const w = s.multiviewers[mvIdx].layouts[layIdx].widgets.find((x) => x.id === id);
      if (w) fn(w);
    });

  const toCanvas = (e: ReactPointerEvent) => {
    const r = svg.current!.getBoundingClientRect();
    return { x: (e.clientX - r.left) / scale, y: (e.clientY - r.top) / scale };
  };
  const down = (e: ReactPointerEvent, w: Widget, mode: 'move' | 'resize') => {
    setSel(w.id);
    e.preventDefault();
    setDrag({ id: w.id, mode, start: toCanvas(e), rect: { ...w.rect } });
  };
  const move = (e: ReactPointerEvent) => {
    if (!drag) return;
    const p = toCanvas(e);
    const dx = p.x - drag.start.x;
    const dy = p.y - drag.start.y;
    const snap = (v: number) => Math.round(v / 4) * 4;
    if (drag.mode === 'move') setWidget(drag.id, (w) => void (w.rect = { ...drag.rect, x: snap(drag.rect.x + dx), y: snap(drag.rect.y + dy) }));
    else setWidget(drag.id, (w) => void (w.rect = { ...drag.rect, w: Math.max(32, snap(drag.rect.w + dx)), h: Math.max(18, snap(drag.rect.h + dy)) }));
  };

  const addMv = () =>
    update((s) => {
      const n = s.multiviewers.length + 1;
      s.multiviewers.push({ id: `mv:${n}`, label: `Multiviewer ${n}`, outputIds: [], layouts: [{ id: `mvl:${n}.1`, label: 'Layout 1', size: { w: 1920, h: 1080 }, widgets: [] }], activeLayout: `mvl:${n}.1` });
    });

  const makeGrid = () =>
    update((s) => {
      const lay = s.multiviewers[mvIdx].layouts[layIdx];
      const cw = Math.floor(lay.size.w / grid.cols);
      const ch = Math.floor(lay.size.h / grid.rows);
      lay.widgets = [];
      let k = 0;
      for (let r = 0; r < grid.rows; r++)
        for (let c = 0; c < grid.cols; c++) {
          lay.widgets.push({ id: `w:${k}`, rect: { x: c * cw, y: r * ch, w: cw - 4, h: ch - 4 }, sourceId: s.sources[k]?.id, showLabel: true, tally: false });
          k++;
        }
    });

  if (!mv) {
    return (
      <Panel title="Multiviewers" actions={<button type="button" className="btn small" onClick={addMv}>Add multiviewer</button>}>
        <div className="muted">No multiviewers in this show.</div>
      </Panel>
    );
  }
  const selected = layout?.widgets.find((w) => w.id === sel);

  return (
    <div className="split">
      <div className="side">
        <Panel title="Multiviewers" actions={<button type="button" className="btn small" onClick={addMv}>Add</button>}>
          <ul className="list">
            {show.multiviewers.map((m, i) => (
              <li key={m.id} className={`list-item${i === mvIdx ? ' list-item--on' : ''}`} onClick={() => { setMvIdx(i); setLayIdx(0); setSel(null); }}>
                {m.label} <span className="muted small">· {m.layouts.length} layout{m.layouts.length === 1 ? '' : 's'}</span>
              </li>
            ))}
          </ul>
          <Field label="Label">
            <input value={mv.label} onChange={(e) => update((s) => void (s.multiviewers[mvIdx].label = e.target.value))} />
          </Field>
          <Field label="Output">
            <select value={mv.outputIds[0] ?? ''} onChange={(e) => update((s) => void (s.multiviewers[mvIdx].outputIds = e.target.value ? [e.target.value] : []))}>
              <option value="">—</option>
              {show.outputs.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </Field>
        </Panel>
        <Panel
          title="Layouts"
          actions={
            <button
              type="button"
              className="btn small"
              onClick={() =>
                update((s) => {
                  const m = s.multiviewers[mvIdx];
                  const n = m.layouts.length + 1;
                  m.layouts.push({ id: `mvl:${mvIdx + 1}.${n}`, label: `Layout ${n}`, size: { ...(m.layouts[0]?.size ?? { w: 1920, h: 1080 }) }, widgets: [] });
                })
              }
            >
              Add
            </button>
          }
        >
          <ul className="list">
            {mv.layouts.map((l, i) => (
              <li key={l.id} className={`list-item${i === layIdx ? ' list-item--on' : ''}`} onClick={() => { setLayIdx(i); setSel(null); }}>
                {l.label} <span className="muted small">· {l.widgets.length} windows{mv.activeLayout === l.id ? ' · active' : ''}</span>
              </li>
            ))}
          </ul>
          {layout ? (
            <>
              <Field label="Layout label">
                <input value={layout.label} onChange={(e) => update((s) => void (s.multiviewers[mvIdx].layouts[layIdx].label = e.target.value))} />
              </Field>
              <div className="row">
                <Field label="Width">
                  <input type="number" value={layout.size.w} onChange={(e) => update((s) => void (s.multiviewers[mvIdx].layouts[layIdx].size.w = Number(e.target.value)))} />
                </Field>
                <Field label="Height">
                  <input type="number" value={layout.size.h} onChange={(e) => update((s) => void (s.multiviewers[mvIdx].layouts[layIdx].size.h = Number(e.target.value)))} />
                </Field>
              </div>
              <div className="row tight">
                <span className="field-label">Fill with a grid</span>
                <input className="num" type="number" min={1} max={12} value={grid.cols} onChange={(e) => setGrid({ ...grid, cols: Number(e.target.value) })} />
                <span>×</span>
                <input className="num" type="number" min={1} max={12} value={grid.rows} onChange={(e) => setGrid({ ...grid, rows: Number(e.target.value) })} />
                <button type="button" className="btn small" onClick={makeGrid}>
                  Make grid
                </button>
              </div>
              <div className="row tight">
                <button type="button" className="btn small" onClick={() => update((s) => void (s.multiviewers[mvIdx].activeLayout = layout.id))}>
                  Set active
                </button>
                <button
                  type="button"
                  className="btn small"
                  onClick={() =>
                    update((s) => {
                      const lay = s.multiviewers[mvIdx].layouts[layIdx];
                      const n = lay.widgets.length;
                      lay.widgets.push({ id: `w:${Date.now()}`, rect: { x: 40 * (n % 8), y: 40 * (n % 8), w: Math.round(lay.size.w / 4), h: Math.round(lay.size.h / 4) }, showLabel: true, tally: false });
                    })
                  }
                >
                  Add window
                </button>
                <button type="button" className="btn small danger" onClick={() => { update((s) => void s.multiviewers[mvIdx].layouts.splice(layIdx, 1)); setLayIdx(0); }}>
                  Delete layout
                </button>
              </div>
            </>
          ) : null}
        </Panel>
      </div>
      <div className="grow">
        {layout ? (
          <Panel title={`${mv.label} · ${layout.label} (${layout.size.w}×${layout.size.h})`}>
            <svg ref={svg} className="canvas" width={width} height={H * scale} viewBox={`0 0 ${W} ${H}`} onPointerMove={move} onPointerUp={() => setDrag(null)} onPointerLeave={() => setDrag(null)}>
              <rect x={0} y={0} width={W} height={H} fill="#0b0d12" stroke="#262c38" strokeWidth={2 / scale} />
              {layout.widgets.map((w) => {
                const on = sel === w.id;
                return (
                  <g key={w.id}>
                    <rect x={w.rect.x} y={w.rect.y} width={w.rect.w} height={w.rect.h} fill="#1b202b" stroke={on ? '#2f9ee0' : '#55606f'} strokeWidth={(on ? 3 : 1.5) / scale} style={{ cursor: 'move' }} onPointerDown={(e) => down(e, w, 'move')} />
                    {w.showLabel ? (
                      <rect x={w.rect.x} y={w.rect.y + w.rect.h - 22 / scale} width={w.rect.w} height={22 / scale} fill="#262c38" pointerEvents="none" />
                    ) : null}
                    <text x={w.rect.x + w.rect.w / 2} y={w.rect.y + w.rect.h - 7 / scale} textAnchor="middle" fill="#e6ebf2" fontSize={14 / scale} pointerEvents="none">
                      {w.label ?? sourceLabel(show, w.sourceId) ?? ''}
                    </text>
                    <rect x={w.rect.x + w.rect.w - 12 / scale} y={w.rect.y + w.rect.h - 12 / scale} width={12 / scale} height={12 / scale} fill="#2f9ee0" style={{ cursor: 'nwse-resize' }} onPointerDown={(e) => down(e, w, 'resize')} />
                  </g>
                );
              })}
            </svg>
            {selected ? (
              <div className="row wrap">
                <Field label="Source">
                  <select value={selected.sourceId ?? ''} onChange={(e) => setWidget(selected.id, (w) => void (w.sourceId = e.target.value || undefined))}>
                    <option value="">— none —</option>
                    {show.sources.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Label (UMD)">
                  <input value={selected.label ?? ''} placeholder={sourceLabel(show, selected.sourceId)} onChange={(e) => setWidget(selected.id, (w) => void (w.label = e.target.value || undefined))} />
                </Field>
                {(['x', 'y', 'w', 'h'] as const).map((k) => (
                  <Field key={k} label={k.toUpperCase()}>
                    <input className="num" type="number" value={Math.round(selected.rect[k])} onChange={(e) => setWidget(selected.id, (w) => void (w.rect[k] = Number(e.target.value)))} />
                  </Field>
                ))}
                <label className="check">
                  <input type="checkbox" checked={selected.showLabel} onChange={(e) => setWidget(selected.id, (w) => void (w.showLabel = e.target.checked))} /> label
                </label>
                <label className="check">
                  <input type="checkbox" checked={selected.tally} onChange={(e) => setWidget(selected.id, (w) => void (w.tally = e.target.checked))} /> tally
                </label>
                <button type="button" className="btn small danger" onClick={() => { update((s) => void (s.multiviewers[mvIdx].layouts[layIdx].widgets = s.multiviewers[mvIdx].layouts[layIdx].widgets.filter((w) => w.id !== selected.id))); setSel(null); }}>
                  Remove window
                </button>
              </div>
            ) : (
              <div className="muted small">Click a window to edit it; drag to move, corner handle to resize.</div>
            )}
          </Panel>
        ) : null}
      </div>
    </div>
  );
}
