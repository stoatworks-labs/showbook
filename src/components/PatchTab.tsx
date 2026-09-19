import { useStore } from '../store';
import { describeFormat, type Format, type Show } from '../types';
import { Panel } from './ui';

function connectorLabel(show: Show, ids: string[]): string {
  return ids
    .map((id) => {
      for (const f of show.system.frames) for (const s of f.slots) for (const c of s.connectors) if (c.id === id) return c.label;
      return id;
    })
    .join(', ');
}

function allConnectors(show: Show, direction: 'in' | 'out') {
  return show.system.frames.flatMap((f) => f.slots.flatMap((s) => s.connectors.filter((c) => c.direction === direction)));
}

function parseFormat(text: string, prev?: Format): Format | undefined {
  const m = text.trim().match(/^(\d+)\s*[x×]\s*(\d+)\s*([pi])?\s*@?\s*([\d.]+)?/i);
  if (!m) return prev;
  return { width: Number(m[1]), height: Number(m[2]), interlaced: (m[3] ?? 'p').toLowerCase() === 'i', rate: m[4] ? Number(m[4]) : (prev?.rate ?? 0), name: undefined };
}

export function PatchTab() {
  const show = useStore((s) => s.show)!;
  const update = useStore((s) => s.update);
  const ins = allConnectors(show, 'in');
  const outs = allConnectors(show, 'out');

  return (
    <div className="grid-2">
      <Panel
        title={`Inputs (${show.inputs.length})`}
        actions={
          <button
            type="button"
            className="btn small"
            onClick={() =>
              update((s) => {
                const n = s.inputs.length + 1;
                s.inputs.push({ id: `in:new${Date.now()}`, label: `Input ${n}`, connectorIds: [], enabled: true });
              })
            }
          >
            Add input
          </button>
        }
      >
        <table className="table">
          <thead>
            <tr>
              <th>#</th>
              <th>Label</th>
              <th>Connector</th>
              <th>Format</th>
              <th>Cap.</th>
              <th>HDCP</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {show.inputs.map((i, idx) => (
              <tr key={i.id} className={i.enabled ? '' : 'dim'}>
                <td className="mono">{i.id.replace(/^in:/, '')}</td>
                <td>
                  <input value={i.label} onChange={(e) => update((s) => void (s.inputs[idx].label = e.target.value))} />
                </td>
                <td>
                  <select value={i.connectorIds[0] ?? ''} onChange={(e) => update((s) => void (s.inputs[idx].connectorIds = e.target.value ? [e.target.value] : []))}>
                    <option value="">— unpatched —</option>
                    {ins.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.label}
                      </option>
                    ))}
                  </select>
                </td>
                <td>
                  <input className="mono" defaultValue={describeFormat(i.format) === '—' ? '' : describeFormat(i.format)} placeholder="1920x1080p60" onBlur={(e) => update((s) => void (s.inputs[idx].format = parseFormat(e.target.value, s.inputs[idx].format)))} />
                </td>
                <td>{i.capacity ?? ''}</td>
                <td>{i.hdcp === undefined ? '' : i.hdcp ? 'yes' : 'no'}</td>
                <td>
                  <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.inputs.splice(idx, 1))}>
                    ×
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Panel>
      <Panel
        title={`Outputs (${show.outputs.length})`}
        actions={
          <button
            type="button"
            className="btn small"
            onClick={() =>
              update((s) => {
                const n = s.outputs.length + 1;
                s.outputs.push({ id: `out:new${Date.now()}`, label: `Output ${n}`, connectorIds: [], role: 'unassigned' });
              })
            }
          >
            Add output
          </button>
        }
      >
        <table className="table">
          <thead>
            <tr>
              <th>#</th>
              <th>Label</th>
              <th>Connector</th>
              <th>Format</th>
              <th>Role</th>
              <th>Feeds</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {show.outputs.map((o, idx) => {
              const feeds = show.screens.filter((sc) => sc.outputs.some((m) => m.outputId === o.id)).map((sc) => sc.label);
              const mv = show.multiviewers.filter((m) => m.outputIds.includes(o.id)).map((m) => m.label);
              return (
                <tr key={o.id}>
                  <td className="mono">{o.id.replace(/^out:/, '')}</td>
                  <td>
                    <input value={o.label} onChange={(e) => update((s) => void (s.outputs[idx].label = e.target.value))} />
                  </td>
                  <td>
                    <select value={o.connectorIds[0] ?? ''} onChange={(e) => update((s) => void (s.outputs[idx].connectorIds = e.target.value ? [e.target.value] : []))}>
                      <option value="">— unpatched —</option>
                      {outs.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    <input className="mono" defaultValue={describeFormat(o.format) === '—' ? '' : describeFormat(o.format)} placeholder="1920x1080p60" onBlur={(e) => update((s) => void (s.outputs[idx].format = parseFormat(e.target.value, s.outputs[idx].format)))} />
                  </td>
                  <td>
                    <select value={o.role} onChange={(e) => update((s) => void (s.outputs[idx].role = e.target.value as typeof o.role))}>
                      <option value="screen">screen</option>
                      <option value="aux">aux</option>
                      <option value="multiviewer">multiviewer</option>
                      <option value="unassigned">unassigned</option>
                    </select>
                  </td>
                  <td className="muted">{[...feeds, ...mv].join(', ')}</td>
                  <td>
                    <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.outputs.splice(idx, 1))}>
                      ×
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Panel>
      <Panel title={`Sources (${show.sources.length})`} className="span-2">
        <table className="table">
          <thead>
            <tr>
              <th>#</th>
              <th>Label</th>
              <th>Kind</th>
              <th>Built on</th>
              <th>AOI</th>
              <th>Format</th>
            </tr>
          </thead>
          <tbody>
            {show.sources.map((src, idx) => {
              const ref = src.refId ? (show.inputs.find((i) => i.id === src.refId)?.label ?? show.screens.find((sc) => sc.id === src.refId)?.label ?? show.stills.find((st) => st.id === src.refId)?.label ?? show.multiviewers.find((m) => m.id === src.refId)?.label ?? src.refId) : '';
              return (
                <tr key={src.id}>
                  <td className="mono">{src.id.replace(/^src:/, '')}</td>
                  <td>
                    <input value={src.label} onChange={(e) => update((s) => void (s.sources[idx].label = e.target.value))} />
                  </td>
                  <td>{src.kind}</td>
                  <td className="muted">{ref}</td>
                  <td className="mono muted">{src.aoi ? `${src.aoi.x},${src.aoi.y} ${src.aoi.w}×${src.aoi.h}` : ''}</td>
                  <td className="mono muted">{describeFormat(src.format)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div className="muted small">
          Unpatched: {show.inputs.filter((i) => i.connectorIds.length === 0).length} inputs, {show.outputs.filter((o) => o.connectorIds.length === 0).length} outputs · patched to{' '}
          {connectorLabel(show, [...show.inputs, ...show.outputs].flatMap((x) => x.connectorIds).slice(0, 6))}
          {[...show.inputs, ...show.outputs].flatMap((x) => x.connectorIds).length > 6 ? ', …' : ''}
        </div>
      </Panel>
    </div>
  );
}
