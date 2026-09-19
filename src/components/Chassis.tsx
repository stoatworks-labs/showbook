import { connectorUse } from '../lib/format';
import type { Show } from '../types';

const KIND_COLOR: Record<string, string> = {
  sdi: '#3aa675',
  hdmi: '#2f9ee0',
  'display-port': '#8b6fe0',
  dvi: '#c07a2f',
  fibre: '#f5a524',
  ip: '#e05c9e',
  link: '#8e99ab',
  other: '#55606f',
};

const KIND_SHORT: Record<string, string> = {
  sdi: 'SDI',
  hdmi: 'HDMI',
  'display-port': 'DP',
  dvi: 'DVI',
  fibre: 'SFP',
  ip: 'IP',
  link: 'LINK',
  other: '?',
};

/** The chassis as a rack drawing: one row per slot, one box per connector. */
export function Chassis({ show }: { show: Show }) {
  const frames = show.system.frames;
  if (frames.length === 0) return <div className="muted">No chassis recorded — the show was authored without hardware, or imported from a file that does not describe it.</div>;
  return (
    <div className="chassis">
      {frames.map((f) => (
        <div key={f.id} className="frame">
          <div className="frame-title">
            {f.label} <span className="muted">{f.model}{f.address ? ` · ${f.address}` : ''}</span>
          </div>
          {f.slots.map((s) => (
            <div key={s.index} className="slot">
              <div className="slot-name">
                <span className="slot-no">{s.index}</span> {s.card}
              </div>
              <div className="slot-conns">
                {s.connectors.map((c) => {
                  const use = connectorUse(show, c);
                  return (
                    <div key={c.id} className={`conn conn--${c.direction}${use ? ' conn--used' : ''}`} title={`${c.label}${c.standard ? ` (${c.standard})` : ''}${use ? `\n${use.kind}: ${use.label}` : ''}`} style={{ borderColor: KIND_COLOR[c.kind] }}>
                      <span className="conn-kind" style={{ background: KIND_COLOR[c.kind] }}>
                        {KIND_SHORT[c.kind]} {c.index}
                      </span>
                      <span className="conn-use">{use ? use.label : ''}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
