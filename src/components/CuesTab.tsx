import { useStore } from '../store';
import { tail, type CueStepKind } from '../types';
import { Panel } from './ui';

const KINDS: { id: CueStepKind; label: string }[] = [
  { id: 'recall-preset', label: 'Recall preset' },
  { id: 'recall-master', label: 'Recall master' },
  { id: 'take', label: 'Take' },
  { id: 'wait', label: 'Wait' },
  { id: 'other', label: 'Other' },
];

export function CuesTab() {
  const show = useStore((s) => s.show)!;
  const update = useStore((s) => s.update);
  return (
    <Panel
      title={`Cues (${show.cues.length})`}
      actions={
        <button type="button" className="btn small" onClick={() => update((s) => void s.cues.push({ id: `cue:${s.cues.length}`, number: s.cues.length + 1, label: `Cue ${s.cues.length + 1}`, steps: [] }))}>
          Add cue
        </button>
      }
    >
      {show.cues.length === 0 ? <div className="muted">No cues. Event Master keeps cues on the frame; LivePremier has no cue list — sequencing lives in Companion or the fleet's timeline tools.</div> : null}
      {show.cues.map((c, ci) => (
        <div key={c.id} className="cue">
          <div className="row tight">
            <span className="mono muted">{c.number ?? tail(c.id)}</span>
            <input className="grow" value={c.label} onChange={(e) => update((s) => void (s.cues[ci].label = e.target.value))} />
            <button type="button" className="btn tiny" onClick={() => update((s) => void s.cues[ci].steps.push({ kind: 'recall-preset', presetId: s.presets[0]?.id, screenIds: [] }))}>
              + step
            </button>
            <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.cues.splice(ci, 1))}>
              ×
            </button>
          </div>
          {c.steps.map((st, si) => (
            <div key={si} className="row tight indent">
              <span className="muted small">{si + 1}.</span>
              <select value={st.kind} onChange={(e) => update((s) => void (s.cues[ci].steps[si].kind = e.target.value as CueStepKind))}>
                {KINDS.map((k) => (
                  <option key={k.id} value={k.id}>
                    {k.label}
                  </option>
                ))}
              </select>
              {st.kind === 'recall-preset' ? (
                <select value={st.presetId ?? ''} onChange={(e) => update((s) => void (s.cues[ci].steps[si].presetId = e.target.value || undefined))}>
                  <option value="">—</option>
                  {show.presets.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.number ?? tail(p.id)} {p.label}
                    </option>
                  ))}
                </select>
              ) : null}
              {st.kind === 'recall-master' ? (
                <select value={st.masterId ?? ''} onChange={(e) => update((s) => void (s.cues[ci].steps[si].masterId = e.target.value || undefined))}>
                  <option value="">—</option>
                  {show.masterPresets.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.number ?? tail(p.id)} {p.label}
                    </option>
                  ))}
                </select>
              ) : null}
              <span className="muted small">delay ms</span>
              <input className="num" type="number" value={st.delayMs ?? ''} onChange={(e) => update((s) => void (s.cues[ci].steps[si].delayMs = e.target.value ? Number(e.target.value) : undefined))} />
              <button type="button" className="btn tiny danger" onClick={() => update((s) => void s.cues[ci].steps.splice(si, 1))}>
                ×
              </button>
            </div>
          ))}
        </div>
      ))}
    </Panel>
  );
}
