import { useState } from 'react';

import { pickFile, pickFolder, pickSave } from '../lib/dialogs';
import { api } from '../lib/ipc';
import { outputPattern, safeName, screenMap, toPngBase64, type PatternKind } from '../lib/patterns';
import { DEFAULT_DOC_OPTIONS, DOC_FORMATS, buildDoc, type DocFormat, type DocOptions } from '../lib/pdf';
import { PAPERS, THEME_LIST, type PaperId, type ThemeId } from '../lib/doc/theme';
import { useStore } from '../store';
import type { CompanionImportReport } from '../types';
import { Field, Panel } from './ui';

function b64(bytes: Uint8Array): string {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(s);
}

export function ExportTab() {
  const show = useStore((s) => s.show)!;
  const settings = useStore((s) => s.settings);
  const dirty = useStore((s) => s.dirty);
  const run = useStore((s) => s.run);
  const toast = useStore((s) => s.toast);
  const [pdfOpts, setPdfOpts] = useState<DocOptions>({ ...DEFAULT_DOC_OPTIONS, author: settings?.author ?? '' });
  const [format, setFormat] = useState<DocFormat>('pdf');
  const [pattern, setPattern] = useState<PatternKind>('alignment');
  const [comp, setComp] = useState({ connectionLabel: show.platform.startsWith('aw') ? 'aquilon' : 'e3', host: settings?.devices.find((d) => d.platform === show.platform)?.host ?? '192.168.0.175', toProgram: false, includePresets: true, includeMasters: true, includeCues: true, includeTakes: true, pageName: '' });
  const [report, setReport] = useState<CompanionImportReport | null>(null);

  const doc = async () => {
    const f = DOC_FORMATS.find((x) => x.id === format)!;
    const path = await pickSave(`Save ${f.label}`, `${safeName(show.meta.name)}.${f.ext}`, [f.ext]);
    if (!path) return;
    const r = await run(`Building ${f.label}`, async () => {
      const commits = await api.showHistory(show.id).catch(() => []);
      const built = await buildDoc(show, commits, { ...pdfOpts, author: pdfOpts.author || settings?.author || '' }, format);
      if (built.text !== undefined) {
        await api.writeText(path, built.text);
        return built.text.length;
      }
      await api.writeFile(path, b64(built.bytes!));
      return built.bytes!.length;
    });
    if (r) toast(`Wrote ${path} (${Math.round(r / 1024)} KB)`);
  };

  const patterns = async () => {
    const folder = await pickFolder('Folder for the test pattern PNGs', `${safeName(show.meta.name)}-patterns`);
    if (!folder) return;
    const r = await run('Rendering patterns', async () => {
      let n = 0;
      for (const o of show.outputs) {
        const c = outputPattern(show, o.id, pattern);
        if (!c) continue;
        await api.writeFile(`${folder}/${safeName(`${show.meta.name}-out-${o.label}`)}.png`, await toPngBase64(c));
        n++;
      }
      for (const sc of show.screens) {
        await api.writeFile(`${folder}/${safeName(`${show.meta.name}-screen-${sc.label}`)}.png`, await toPngBase64(screenMap(show, sc)));
        n++;
      }
      await api.finishFolder(folder);
      return n;
    });
    if (r) toast(`Wrote ${r} PNGs to ${folder}`);
  };

  const companionOut = async () => {
    const path = await pickSave('Companion page', `${safeName(show.meta.name)}.companionconfig`, ['companionconfig', 'json']);
    if (!path) return;
    const r = await run('Exporting Companion page', () => api.companionExport(show.id, comp, path));
    if (r) toast(`Wrote ${r.pages} page${r.pages === 1 ? '' : 's'} to ${path}`);
  };

  const companionIn = async () => {
    const path = await pickFile('Companion config', ['companionconfig', 'json']);
    if (!path) return;
    const r = await run('Reading Companion page', () => api.companionImport(show.id, path));
    if (r) setReport(r);
  };

  const json = async () => {
    const path = await pickSave('Showbook show', `${safeName(show.meta.name)}.showbook.json`, ['json']);
    if (!path) return;
    const r = await run('Exporting', () => api.exportShowJson(show.id, path));
    if (r !== undefined) toast(`Wrote ${path}`);
  };

  return (
    <div className="grid-2">
      <Panel title="Documentation">
        <p className="muted small">Cover with the production details; the chassis as a frame map; the signal flow; the patch tables; every screen and preset drawn to scale; the preset matrix; cues on a timeline; multiviewer layouts; a glossary of the settings; the import notes and the version history.{dirty ? ' Unsaved edits are included; the history table shows saved versions only.' : ''}</p>
        <div className="row wrap">
          <Field label="Format">
            <select value={format} onChange={(e) => setFormat(e.target.value as DocFormat)}>
              {DOC_FORMATS.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Theme">
            <select value={pdfOpts.theme} onChange={(e) => setPdfOpts({ ...pdfOpts, theme: e.target.value as ThemeId })}>
              {THEME_LIST.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.label}
                </option>
              ))}
            </select>
          </Field>
          {format === 'pdf' ? (
            <Field label="Paper">
              <select value={pdfOpts.paper} onChange={(e) => setPdfOpts({ ...pdfOpts, paper: e.target.value as PaperId })}>
                {Object.values(PAPERS).map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </Field>
          ) : null}
        </div>
        <div className="muted small">{THEME_LIST.find((t) => t.id === pdfOpts.theme)?.description}</div>
        <div className="row wrap">
          <label className="check"><input type="checkbox" checked={pdfOpts.includePresets} onChange={(e) => setPdfOpts({ ...pdfOpts, includePresets: e.target.checked })} /> presets</label>
          <label className="check"><input type="checkbox" checked={pdfOpts.includeMultiviewers} onChange={(e) => setPdfOpts({ ...pdfOpts, includeMultiviewers: e.target.checked })} /> multiviewers</label>
          <label className="check"><input type="checkbox" checked={pdfOpts.includeGlossary} onChange={(e) => setPdfOpts({ ...pdfOpts, includeGlossary: e.target.checked })} /> glossary</label>
          <label className="check"><input type="checkbox" checked={pdfOpts.includeHistory} onChange={(e) => setPdfOpts({ ...pdfOpts, includeHistory: e.target.checked })} /> history</label>
        </div>
        <div className="row wrap">
          <Field label="Prepared for (optional)">
            <input value={pdfOpts.preparedBy} onChange={(e) => setPdfOpts({ ...pdfOpts, preparedBy: e.target.value })} />
          </Field>
          <Field label="Prepared by (optional)">
            <input value={pdfOpts.author ?? ''} placeholder={settings?.author ?? ''} onChange={(e) => setPdfOpts({ ...pdfOpts, author: e.target.value })} />
          </Field>
        </div>
        <p className="muted small">The cover reads the event, client, venue, date and operator from the show's production details (Overview tab).</p>
        <button type="button" className="btn primary" onClick={() => void doc()}>
          Save {DOC_FORMATS.find((f) => f.id === format)!.label}…
        </button>
      </Panel>
      <Panel title="Test patterns">
        <p className="muted small">One PNG per output at the output's raster, labelled with the output, its connector, the screen it belongs to and its position on the canvas, with arrows to its neighbours; plus one PNG per screen showing the whole canvas with every output's region. Play them from a media server or a laptop to prove the patch before content arrives.</p>
        <Field label="Pattern">
          <select value={pattern} onChange={(e) => setPattern(e.target.value as PatternKind)}>
            <option value="alignment">Alignment (grid, centre, safe areas)</option>
            <option value="grid">Grid</option>
            <option value="bars">75% bars over grid</option>
          </select>
        </Field>
        <button type="button" className="btn primary" onClick={() => void patterns()}>
          Render PNGs to a folder…
        </button>
        <div className="muted small">{show.outputs.length} outputs + {show.screens.length} screen maps</div>
      </Panel>
      <Panel title="Companion page">
        <p className="muted small">A Bitfocus Companion page: TAKE per screen, one button per preset{show.platform.startsWith('aw') ? ', master memory' : ' and cue'}, wired to the {show.platform.startsWith('aw') ? 'analogway-awj' : 'barco-eventmaster'} module. Import it in Companion (Import / Export → Import, pick the page). Reading a page back checks which of its buttons still match this show.</p>
        <div className="row wrap">
          <Field label="Connection label">
            <input value={comp.connectionLabel} onChange={(e) => setComp({ ...comp, connectionLabel: e.target.value })} />
          </Field>
          <Field label="Device address">
            <input value={comp.host} onChange={(e) => setComp({ ...comp, host: e.target.value })} />
          </Field>
          <Field label="Page name">
            <input value={comp.pageName} placeholder={show.meta.name} onChange={(e) => setComp({ ...comp, pageName: e.target.value })} />
          </Field>
        </div>
        <div className="row wrap">
          <label className="check"><input type="checkbox" checked={comp.includeTakes} onChange={(e) => setComp({ ...comp, includeTakes: e.target.checked })} /> takes</label>
          <label className="check"><input type="checkbox" checked={comp.includePresets} onChange={(e) => setComp({ ...comp, includePresets: e.target.checked })} /> presets</label>
          <label className="check"><input type="checkbox" checked={comp.includeMasters} onChange={(e) => setComp({ ...comp, includeMasters: e.target.checked })} /> masters</label>
          <label className="check"><input type="checkbox" checked={comp.includeCues} onChange={(e) => setComp({ ...comp, includeCues: e.target.checked })} /> cues</label>
          <label className="check"><input type="checkbox" checked={comp.toProgram} onChange={(e) => setComp({ ...comp, toProgram: e.target.checked })} /> recall to program</label>
        </div>
        <div className="row">
          <button type="button" className="btn primary" onClick={() => void companionOut()}>
            Export page…
          </button>
          <button type="button" className="btn" onClick={() => void companionIn()}>
            Read a page back…
          </button>
        </div>
        {report ? (
          <div className="report">
            <div>
              {report.kind} export v{report.fileVersion} · {report.pages.length} page{report.pages.length === 1 ? '' : 's'} ({report.pages.join(', ')}) · connections: {report.connections.join(', ') || '—'} · {report.buttons.length} recall/take buttons, {report.unmatched} not in this show
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Page</th>
                  <th>Pos</th>
                  <th>Button</th>
                  <th>Action</th>
                  <th>Matches</th>
                </tr>
              </thead>
              <tbody>
                {report.buttons.map((b, i) => (
                  <tr key={i} className={b.problem ? 'bad' : ''}>
                    <td>{b.page}</td>
                    <td className="mono">{b.row},{b.column}</td>
                    <td>{b.text.replace(/\n/g, ' ')}</td>
                    <td className="mono muted">{b.module}:{b.definition}</td>
                    <td>{b.problem ?? (b.matched ? (show.presets.find((p) => p.id === b.matched)?.label ?? show.cues.find((c) => c.id === b.matched)?.label ?? show.masterPresets.find((m) => m.id === b.matched)?.label ?? b.matched) : '—')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : null}
      </Panel>
      <Panel title="Showbook JSON">
        <p className="muted small">The brand-neutral model as a file, for another Showbook, a script, or a diff outside the app.</p>
        <button type="button" className="btn" onClick={() => void json()}>
          Save JSON…
        </button>
      </Panel>
    </div>
  );
}
