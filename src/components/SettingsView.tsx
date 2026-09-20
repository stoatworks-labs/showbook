
import { pickFolder } from '../lib/dialogs';
import { FULL_APP_URL, isLite } from '../lib/ipc';
import { useStore } from '../store';
import { Field, Panel } from './ui';

export function SettingsView() {
  const settings = useStore((s) => s.settings);
  const info = useStore((s) => s.info);
  const saveSettings = useStore((s) => s.saveSettings);
  if (!settings) return null;

  const pick = async () => {
    const picked = await pickFolder('Library folder');
    if (picked) await saveSettings({ ...settings, libraryPath: picked });
  };

  return (
    <div className="view">
      <div className="view-head">
        <h2>Settings</h2>
      </div>
      <div className="grid-2">
        <Panel title="Library">
          {isLite ? (
            <p className="muted small">
              Shows, their versions and their vendor files live in this browser's storage on this machine — nothing is uploaded anywhere. Clearing site data clears the library, so export a show as JSON (Documents &amp; export) to keep it elsewhere. The desktop app keeps the library as plain files in a folder of your choosing, and can mirror it with Dropbox, Google Drive or OneDrive.
            </p>
          ) : (
            <div className="row">
              <Field label="Folder" hint="plain files: shows/<id>/show.json, history/, vendor/. Put it inside a synced drive folder to share it.">
                <input value={settings.libraryPath} onChange={(e) => void saveSettings({ ...settings, libraryPath: e.target.value })} />
              </Field>
              <button type="button" className="btn" onClick={() => void pick()}>
                Choose…
              </button>
            </div>
          )}
          <Field label="Your name (recorded on each version)">
            <input value={settings.author} onChange={(e) => void saveSettings({ ...settings, author: e.target.value })} />
          </Field>
        </Panel>
        <Panel title="About">
          <p>
            Showbook {isLite ? 'lite ' : ''}{info?.version} — show file library and documentation generator for video switchers. Settings live in <span className="mono">{info?.configDir}</span>.
          </p>
          {isLite ? (
            <p>
              This is the browser build: import, inspect, edit, document, convert and export show files, with the same parsers as the desktop app. What it cannot do in a browser is talk to a switcher — pull, push, recall, TAKE — or sync a library with a cloud drive.{' '}
              <a href={FULL_APP_URL} target="_blank" rel="noreferrer">
                The full app is free, for macOS, Windows and Linux.
              </a>
            </p>
          ) : null}
          <p className="muted small">Not affiliated with Barco or Analog Way. Drivers were built against the vendors' own simulators and published protocol guides; see the README for what has and has not been checked on hardware.</p>
        </Panel>
      </div>
    </div>
  );
}
