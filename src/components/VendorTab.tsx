import { useEffect, useState } from 'react';

import { pickFile, pickSave } from '../lib/dialogs';
import { api } from '../lib/ipc';
import { useStore } from '../store';
import { bytes, fmtDate } from '../lib/format';
import { Panel } from './ui';
import type { LppSummary } from '../types';

export function VendorTab() {
  const show = useStore((s) => s.show)!;
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const openShow = useStore((s) => s.openShow);

  const [lpp, setLpp] = useState<LppSummary | null>(null);

  useEffect(() => {
    let live = true;
    api
      .lppSummary(show.id)
      .then((s) => live && setLpp(s))
      .catch(() => live && setLpp(null));
    return () => {
      live = false;
    };
  }, [show.id, show.vendor.length]);

  const slug = (show.meta.name || 'show').replace(/[^A-Za-z0-9._-]+/g, '-').replace(/^-|-$/g, '') || 'show';
  const awcs = show.vendor.filter((v) => v.kind === 'awc');

  const exportBlob = async (sha: string, suggested: string) => {
    const path = await pickSave('Export vendor file', suggested, ['*']);
    if (!path) return;
    const r = await run('Exporting', () => api.vendorExport(show.id, sha, path));
    if (r !== undefined) toast(`Wrote ${path}`);
  };

  const exportBundle = async () => {
    const path = await pickSave('Export a Showbook bundle', `${slug}.showbook`, ['showbook']);
    if (!path) return;
    const r = await run('Writing the bundle', () => api.bundleExport(show.id, path));
    if (r) toast(`Wrote ${path} (${bytes(r.size)})`);
  };

  const exportEmbedded = async (sha: string) => {
    const path = await pickSave('Export one .awc carrying the configuration', `${slug}.awc`, ['awc']);
    if (!path) return;
    const r = await run('Embedding the configuration', () => api.bundleExportAwc(show.id, sha, path));
    if (r) toast(`Wrote ${path} (${bytes(r.size)}) — the device will not keep the configuration when it re-exports.`);
  };

  const attach = async () => {
    const path = await pickFile('Choose a LivePremier Plus configuration', ['json']);
    if (!path) return;
    const r = await run('Attaching', () => api.lppAttach(show.id, path));
    if (r) {
      setLpp(r);
      toast(`Attached ${r.cues} cue${r.cues === 1 ? '' : 's'} from ${r.device || 'an unnamed device'}`);
      await openShow(show.id);
    }
  };

  const exportLpp = async () => {
    const path = await pickSave('Export the LivePremier Plus configuration', 'livepremier-plus.json', ['json']);
    if (!path) return;
    const r = await run('Exporting', () => api.lppExport(show.id, path));
    if (r !== undefined) toast(`Wrote ${path}`);
  };

  const detach = async () => {
    const r = await run('Removing', () => api.lppDetach(show.id));
    if (r !== undefined) {
      setLpp(null);
      toast('Removed the LivePremier Plus configuration');
      await openShow(show.id);
    }
  };

  return (
    <>
      <Panel title="Vendor files kept with this show">
        {show.vendor.length === 0 ? (
          <div className="muted">
            None. A vendor file is the switcher's own show file — an Event Master backup archive or a LivePremier .awc — kept byte-for-byte beside the model so the show can go back to its own hardware exactly. Pull from a device with "vendor file" ticked, or import one.
          </div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Kind</th>
                <th>Captured</th>
                <th>Size</th>
                <th>Note</th>
                <th>sha256</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {show.vendor.map((v) => (
                <tr key={v.sha256}>
                  <td>{v.kind}</td>
                  <td>{fmtDate(v.capturedAt)}</td>
                  <td>{bytes(v.size)}</td>
                  <td>{v.note}</td>
                  <td className="mono muted">{v.sha256.slice(0, 12)}…</td>
                  <td>
                    <button type="button" className="btn small" onClick={() => void exportBlob(v.sha256, v.note.split(' — ')[0] || `${v.kind}`)}>
                      Export…
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <div className="muted small" style={{ marginTop: 12 }}>
          Event Master: restore an exported backup from the Event Master Toolset (Configuration → Restore). LivePremier: push the .awc from Devices → Push (the device extracts it, applies the chosen modules and reboots).
        </div>
      </Panel>

      <Panel title="LivePremier Plus configuration">
        {lpp ? (
          <>
            <table className="table">
              <tbody>
                <tr>
                  <th>Written against</th>
                  <td className="mono">{lpp.device || '—'}</td>
                </tr>
                <tr>
                  <th>Exported</th>
                  <td>
                    {lpp.exported ? fmtDate(lpp.exported) : '—'}
                    {lpp.appVersion ? ` by LivePremier Plus ${lpp.appVersion}` : ''}
                  </td>
                </tr>
                <tr>
                  <th>Holds</th>
                  <td>
                    {[
                      lpp.cues ? `${lpp.cues} cue${lpp.cues === 1 ? '' : 's'}${lpp.stackName ? ` (${lpp.stackName})` : ''}` : null,
                      lpp.groups ? `${lpp.groups} layer group${lpp.groups === 1 ? '' : 's'}` : null,
                      lpp.names ? `${lpp.names} layer name${lpp.names === 1 ? '' : 's'}` : null,
                      lpp.patchEntries ? `${lpp.patchEntries} patch entr${lpp.patchEntries === 1 ? 'y' : 'ies'}` : null,
                      lpp.matrices ? `${lpp.matrices} router${lpp.matrices === 1 ? '' : 's'}` : null,
                      lpp.hasSettings ? 'app settings' : null,
                    ]
                      .filter(Boolean)
                      .join(', ') || 'nothing'}
                  </td>
                </tr>
              </tbody>
            </table>
            <div className="row" style={{ gap: 8, marginTop: 12 }}>
              <button type="button" className="btn small" onClick={() => void exportLpp()}>
                Export the configuration…
              </button>
              <button type="button" className="btn small" onClick={() => void attach()}>
                Replace…
              </button>
              <button type="button" className="btn small" onClick={() => void detach()}>
                Remove
              </button>
            </div>
          </>
        ) : (
          <div className="muted">
            None. LivePremier Plus holds the other half of a rig — the cue stack, layer groups, layer names, the patch to the external routers — and a .awc holds none of it. Export one from LivePremier Plus (Setup → Configuration) and attach it here so the two travel together.
            <div style={{ marginTop: 12 }}>
              <button type="button" className="btn small" onClick={() => void attach()}>
                Attach a configuration…
              </button>
            </div>
          </div>
        )}
      </Panel>

      <Panel title="Export both halves as one file">
        <div className="muted">
          A <strong>.showbook bundle</strong> holds this model, the vendor files exactly as the device wrote them, and the LivePremier Plus configuration. It is what to keep and what to send someone: Showbook imports it whole, and the .awc inside is untouched, so it can always be handed to the Web RCS on its own.
        </div>
        <div className="row" style={{ gap: 8, marginTop: 12 }}>
          <button type="button" className="btn" onClick={() => void exportBundle()}>
            Export a bundle…
          </button>
        </div>

        {awcs.length > 0 && (
          <>
            <div className="muted" style={{ marginTop: 20 }}>
              Or put the configuration <strong>inside the .awc itself</strong>, so one file restores both halves from the Web RCS with Showbook nowhere in sight. A LivePremier accepts this exactly as it accepts its own file.
              <div className="small" style={{ marginTop: 6 }}>
                One caveat, and it matters: the device does not keep the configuration. An .awc exported from the Web RCS afterwards will not contain it, so treat an embedded file as something you hand out, not as the copy you keep. Verified on LivePremier firmware 6.2.73; Midra 4K and Alta 4K are untested.
              </div>
            </div>
            <div className="row" style={{ gap: 8, marginTop: 12 }}>
              {awcs.map((v) => (
                <button
                  key={v.sha256}
                  type="button"
                  className="btn"
                  disabled={!lpp}
                  title={lpp ? undefined : 'Attach a LivePremier Plus configuration first'}
                  onClick={() => void exportEmbedded(v.sha256)}
                >
                  Export one .awc with the configuration{awcs.length > 1 ? ` (${v.sha256.slice(0, 8)}…)` : ''}…
                </button>
              ))}
            </div>
          </>
        )}
      </Panel>
    </>
  );
}
