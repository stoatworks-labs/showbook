import { useState } from 'react';

import { api, toRef } from '../lib/ipc';
import { useStore } from '../store';
import { PLATFORM_LABEL, type DeviceEntry, type Platform } from '../types';
import { Empty, Field, Panel } from './ui';

const LIVE: Platform[] = ['barco-em', 'barco-pds4k', 'aw-live-premier', 'aw-midra4k', 'aw-alta4k'];

export function DevicesView() {
  const settings = useStore((s) => s.settings);
  const saveSettings = useStore((s) => s.saveSettings);
  const entries = useStore((s) => s.entries);
  const openShow = useStore((s) => s.openShow);
  const refresh = useStore((s) => s.refreshLibrary);
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const [draft, setDraft] = useState<DeviceEntry>({ name: '', platform: 'barco-em', host: '' });
  const [sel, setSel] = useState<number>(0);
  const [probe, setProbe] = useState<Record<string, unknown> | null>(null);
  const [pull, setPull] = useState({ vendorFile: true, deepCapture: false, intoShow: '', name: '' });
  const [push, setPush] = useState({ showId: '', labels: true, vendorFile: false });
  const [log, setLog] = useState<string[]>([]);

  const devices = settings?.devices ?? [];
  const device = devices[sel];

  const add = async () => {
    if (!settings || !draft.name || !draft.host) return;
    await saveSettings({ ...settings, devices: [...settings.devices, { ...draft }] });
    setDraft({ name: '', platform: draft.platform, host: '' });
  };
  const remove = async (i: number) => {
    if (!settings) return;
    await saveSettings({ ...settings, devices: settings.devices.filter((_, k) => k !== i) });
    setSel(0);
  };

  const doProbe = async () => {
    if (!device) return;
    const r = await run(`Probing ${device.name}`, () => api.deviceProbe(toRef(device)));
    if (r) setProbe(r);
  };

  const doPull = async () => {
    if (!device) return;
    if (pull.deepCapture && !window.confirm('Deep capture recalls every memory into the preview of every screen to read its layers. It changes the device\'s preview buffers. Continue?')) return;
    const r = await run(`Pulling from ${device.name}`, () => api.devicePull(toRef(device), { vendorFile: pull.vendorFile, deepCapture: pull.deepCapture, intoShow: pull.intoShow || undefined, name: pull.name || undefined }));
    if (r) {
      toast(`Pulled ${r.summary.name}: ${r.summary.screens} screens, ${r.summary.presets} presets${r.show.vendor.length ? ', vendor file kept' : ''}`);
      await refresh();
      await openShow(r.show.id);
    }
  };

  const doPush = async () => {
    if (!device || !push.showId) return;
    const what = [push.labels ? 'labels' : '', push.vendorFile ? 'the vendor file (the device will apply it and reboot)' : ''].filter(Boolean).join(' and ');
    if (!window.confirm(`Push ${what} to ${device.name} (${device.host})? This writes to a live device.`)) return;
    const r = await run(`Pushing to ${device.name}`, () => api.devicePush(toRef(device), push.showId, { vendorFile: push.vendorFile, modules: [], labels: push.labels }));
    if (r) {
      setLog(r.log);
      toast(`Push to ${device.name} finished`);
    }
  };

  const showsFor = entries.filter((e) => !device || e.summary.platform === device.platform);

  return (
    <div className="view">
      <div className="view-head">
        <h2>Devices</h2>
        <span className="muted">live switchers on the network</span>
      </div>
      <div className="grid-2">
        <Panel title="Known devices">
          {devices.length === 0 ? <Empty>No devices yet. Add one below.</Empty> : null}
          <ul className="list">
            {devices.map((d, i) => (
              <li key={`${d.name}${i}`} className={`list-item${i === sel ? ' list-item--on' : ''}`} onClick={() => { setSel(i); setProbe(null); setLog([]); }}>
                <strong>{d.name}</strong> <span className="muted small">{PLATFORM_LABEL[d.platform]} · {d.host}</span>
                <button type="button" className="btn tiny danger" style={{ float: 'right' }} onClick={(e) => { e.stopPropagation(); void remove(i); }}>
                  ×
                </button>
              </li>
            ))}
          </ul>
          <div className="row wrap">
            <Field label="Name">
              <input value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
            </Field>
            <Field label="Platform">
              <select value={draft.platform} onChange={(e) => setDraft({ ...draft, platform: e.target.value as Platform })}>
                {LIVE.map((p) => (
                  <option key={p} value={p}>
                    {PLATFORM_LABEL[p]}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="Address" hint={draft.platform.startsWith('aw') ? 'Web RCS host[:port] — a device is :80, the simulator :3000; AWJ is assumed on :10606' : 'frame IP; JSON-RPC on :9999'}>
              <input value={draft.host} placeholder="192.168.0.175" onChange={(e) => setDraft({ ...draft, host: e.target.value })} />
            </Field>
            <button type="button" className="btn" onClick={() => void add()}>
              Add device
            </button>
          </div>
        </Panel>
        <Panel title={device ? `${device.name} · ${device.host}` : 'Select a device'} actions={device ? <button type="button" className="btn small" onClick={() => void doProbe()}>Probe</button> : null}>
          {probe ? <pre className="pre">{JSON.stringify(probe, null, 1).slice(0, 1500)}</pre> : null}
          {device ? (
            <>
              <h4>Pull the show from the device</h4>
              <div className="row wrap">
                <label className="check"><input type="checkbox" checked={pull.vendorFile} onChange={(e) => setPull({ ...pull, vendorFile: e.target.checked })} /> keep the vendor file ({device.platform.startsWith('aw') ? '.awc' : 'backup archive'})</label>
                {device.platform.startsWith('aw') ? (
                  <label className="check"><input type="checkbox" checked={pull.deepCapture} onChange={(e) => setPull({ ...pull, deepCapture: e.target.checked })} /> deep capture memory layers (writes preview)</label>
                ) : null}
              </div>
              <div className="row wrap">
                <Field label="Into an existing show (optional)">
                  <select value={pull.intoShow} onChange={(e) => setPull({ ...pull, intoShow: e.target.value })}>
                    <option value="">— new show —</option>
                    {showsFor.map((e) => (
                      <option key={e.summary.id} value={e.summary.id}>
                        {e.summary.name}
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="Name for a new show">
                  <input value={pull.name} placeholder={device.name} onChange={(e) => setPull({ ...pull, name: e.target.value })} />
                </Field>
                <button type="button" className="btn primary" onClick={() => void doPull()}>
                  Pull
                </button>
              </div>
              <h4>Push a show to the device</h4>
              <div className="row wrap">
                <Field label="Show">
                  <select value={push.showId} onChange={(e) => setPush({ ...push, showId: e.target.value })}>
                    <option value="">—</option>
                    {showsFor.map((e) => (
                      <option key={e.summary.id} value={e.summary.id}>
                        {e.summary.name}
                      </option>
                    ))}
                  </select>
                </Field>
                {device.platform.startsWith('aw') ? (
                  <>
                    <label className="check"><input type="checkbox" checked={push.labels} onChange={(e) => setPush({ ...push, labels: e.target.checked })} /> labels (screens, inputs, outputs, memories) over AWJ</label>
                    <label className="check"><input type="checkbox" checked={push.vendorFile} onChange={(e) => setPush({ ...push, vendorFile: e.target.checked })} /> restore the .awc (upload, apply, reboot)</label>
                  </>
                ) : (
                  <span className="muted small">Event Master has no API for writing configuration. Recall presets and cues from the Presets tab; restore a backup archive from the Event Master Toolset (export it from the show's Vendor files tab).</span>
                )}
                <button type="button" className="btn danger" disabled={!push.showId || !device.platform.startsWith('aw')} onClick={() => void doPush()}>
                  Push
                </button>
              </div>
              {log.length ? <pre className="pre">{log.join('\n')}</pre> : null}
            </>
          ) : null}
        </Panel>
      </div>
    </div>
  );
}
