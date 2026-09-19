import { open } from '@tauri-apps/plugin-dialog';

import { useStore } from '../store';
import { Field, Panel } from './ui';

export function SettingsView() {
  const settings = useStore((s) => s.settings);
  const info = useStore((s) => s.info);
  const saveSettings = useStore((s) => s.saveSettings);
  if (!settings) return null;

  const pick = async () => {
    const picked = await open({ directory: true, title: 'Library folder' });
    if (picked) await saveSettings({ ...settings, libraryPath: Array.isArray(picked) ? picked[0] : picked });
  };

  return (
    <div className="view">
      <div className="view-head">
        <h2>Settings</h2>
      </div>
      <div className="grid-2">
        <Panel title="Library">
          <div className="row">
            <Field label="Folder" hint="plain files: shows/<id>/show.json, history/, vendor/. Put it inside a synced drive folder to share it.">
              <input value={settings.libraryPath} onChange={(e) => void saveSettings({ ...settings, libraryPath: e.target.value })} />
            </Field>
            <button type="button" className="btn" onClick={() => void pick()}>
              Choose…
            </button>
          </div>
          <Field label="Your name (recorded on each version)">
            <input value={settings.author} onChange={(e) => void saveSettings({ ...settings, author: e.target.value })} />
          </Field>
        </Panel>
        <Panel title="About">
          <p>
            Showbook {info?.version} — show file library and documentation generator for video switchers. Settings live in <span className="mono">{info?.configDir}</span>.
          </p>
          <p className="muted small">Not affiliated with Barco or Analog Way. Drivers were built against the vendors' own simulators and published protocol guides; see the README for what has and has not been checked on hardware.</p>
        </Panel>
      </div>
    </div>
  );
}
