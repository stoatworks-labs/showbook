import { useEffect } from 'react';

import { ConvertView } from './components/ConvertView';
import { DevicesView } from './components/DevicesView';
import { LibraryView } from './components/LibraryView';
import { SettingsView } from './components/SettingsView';
import { ShowView } from './components/ShowView';
import { SyncView } from './components/SyncView';
import { FULL_APP_URL, inTauri, isLite } from './lib/ipc';
import { useStore, type View } from './store';

const ALL_NAV: { id: View; label: string }[] = [
  { id: 'library', label: 'Library' },
  { id: 'devices', label: 'Devices' },
  { id: 'convert', label: 'Convert' },
  { id: 'sync', label: 'Sync' },
  { id: 'settings', label: 'Settings' },
];
// showbook-lite has no sockets: no devices to talk to and no drive to sync
// with. The views exist; the navigation to them does not.
const NAV = ALL_NAV.filter((n) => !isLite || (n.id !== 'devices' && n.id !== 'sync'));


export function App() {
  const load = useStore((s) => s.load);
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const show = useStore((s) => s.show);
  const dirty = useStore((s) => s.dirty);
  const busy = useStore((s) => s.busy);
  const toasts = useStore((s) => s.toasts);
  const dismiss = useStore((s) => s.dismiss);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <div className="app">
      <header className="top">
        <h1>
          Show<span>book</span>
        </h1>
        <span className="tag">{isLite ? 'lite — show files in the browser' : 'show file library for video switchers'}</span>
        <span className="ver">{__APP_VERSION__}</span>
        <div className="spacer" />
        {busy ? <span className="pill busy">{busy}…</span> : null}
        {isLite ? (
          <a className="btn small primary" href={FULL_APP_URL} target="_blank" rel="noreferrer" title="Devices, cloud sync and a library on disk: the desktop app, free">
            Get the full app
          </a>
        ) : !inTauri ? (
          <span className="pill warn">browser demo — two simulator captures in memory; files and devices need the desktop app</span>
        ) : null}
        <button type="button" className="btn small" data-stoatworks-about>
          About
        </button>
      </header>
      <div className="main">
        <nav className="sidebar">
          {NAV.map((n) => (
            <button key={n.id} type="button" className={`nav${view === n.id ? ' nav--on' : ''}`} onClick={() => setView(n.id)}>
              {n.label}
            </button>
          ))}
          {show ? (
            <>
              <div className="nav-sep">open show</div>
              <button type="button" className={`nav nav-show${view === 'show' ? ' nav--on' : ''}`} onClick={() => setView('show')} title={show.meta.name}>
                {show.meta.name}
                {dirty ? <span className="dot" title="unsaved changes" /> : null}
              </button>
            </>
          ) : null}
        </nav>
        <section className="content">
          {view === 'library' ? <LibraryView /> : null}
          {view === 'show' ? show ? <ShowView /> : <LibraryView /> : null}
          {view === 'devices' ? <DevicesView /> : null}
          {view === 'convert' ? <ConvertView /> : null}
          {view === 'sync' ? <SyncView /> : null}
          {view === 'settings' ? <SettingsView /> : null}
        </section>
      </div>
      <div className="toasts">
        {toasts.map((t) => (
          <div key={t.id} className={`toast toast--${t.kind}`} onClick={() => dismiss(t.id)}>
            {t.text}
          </div>
        ))}
      </div>
    </div>
  );
}
