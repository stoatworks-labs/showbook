
import { pickSave } from '../lib/dialogs';
import { api } from '../lib/ipc';
import { useStore } from '../store';
import { bytes, fmtDate } from '../lib/format';
import { Panel } from './ui';

export function VendorTab() {
  const show = useStore((s) => s.show)!;
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);

  const exportBlob = async (sha: string, suggested: string) => {
    const path = await pickSave('Export vendor file', suggested, ['*']);
    if (!path) return;
    const r = await run('Exporting', () => api.vendorExport(show.id, sha, path));
    if (r !== undefined) toast(`Wrote ${path}`);
  };

  return (
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
  );
}
