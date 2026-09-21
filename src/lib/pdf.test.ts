import { readFileSync } from 'node:fs';

import { PDFDocument } from 'pdf-lib';
import { describe, expect, it } from 'vitest';

import type { Commit, Show } from '../types';
import { buildDocument } from './document';
import { clean } from './doc/render-pdf';
import { THEME_LIST, THEMES } from './doc/theme';
import { DEFAULT_DOC_OPTIONS, buildHtml, buildPdf } from './pdf';

const demo = (name: string) => JSON.parse(readFileSync(new URL(`../../public/demo/${name}.json`, import.meta.url), 'utf8')) as Show;

function sample(): Show {
  return {
    schema: 'showbook/1',
    id: 'x',
    meta: { name: 'Gala — main hall', notes: 'Notes here', tags: [], created: '2026-09-19T10:00:00Z', modified: '2026-09-19T10:00:00Z' },
    platform: 'barco-em',
    system: {
      model: 'Encore3',
      firmware: '10.0.2',
      name: 'E3',
      nativeRate: 59.94,
      frames: [{ id: 'frame:1', label: 'Frame 1', model: 'Encore3', slots: [{ index: 1, card: '4x HDMI 2.0 input card', label: '', connectors: [{ id: 'conn:1.1.1', kind: 'hdmi', direction: 'in', index: 1, label: 'Slot 1 · HDMI 1', standard: 'HDMI 2.0' }] }] }],
    },
    inputs: [{ id: 'in:0', label: 'Camera 1', connectorIds: ['conn:1.1.1'], format: { width: 1920, height: 1080, rate: 50, interlaced: false }, enabled: true }],
    sources: [{ id: 'src:0', label: 'Camera 1', kind: 'input', refId: 'in:0' }],
    outputs: [{ id: 'out:0', label: 'LED wall L', connectorIds: [], format: { width: 1920, height: 1080, rate: 50, interlaced: false }, role: 'screen' }],
    screens: [
      {
        id: 'scr:0',
        label: 'Main',
        kind: 'screen',
        size: { w: 3840, h: 1080 },
        outputs: [{ outputId: 'out:0', rect: { x: 0, y: 0, w: 1920, h: 1080 } }],
        layers: [
          { id: 'layer:bg', label: 'Background', kind: 'background', z: 0 },
          { id: 'layer:1', label: 'Layer 1', kind: 'mixer', z: 1, capacity: '4K' },
        ],
        transition: { durationMs: 500, kind: 'mix' },
      },
    ],
    presets: [{ id: 'pre:0', number: 1, label: 'Walk in', notes: '', targets: [{ screenId: 'scr:0', background: 'src:0', layers: [{ layerId: 'layer:1', sourceId: 'src:0', visible: true, rect: { x: 100, y: 100, w: 1280, h: 720 }, opacity: 0.8 }] }] }],
    masterPresets: [],
    layerMemories: [],
    cues: [{ id: 'cue:0', number: 1, label: 'Open', steps: [{ kind: 'recall-preset', presetId: 'pre:0', screenIds: [], delayMs: 0 }, { kind: 'take', screenIds: ['scr:0'], delayMs: 500 }] }],
    multiviewers: [{ id: 'mv:1', label: 'MV', outputIds: [], layouts: [{ id: 'mvl:1.1', label: 'Layout 1', size: { w: 1920, h: 1080 }, widgets: [{ id: 'w:0', rect: { x: 0, y: 0, w: 640, h: 360 }, sourceId: 'src:0', showLabel: true, tally: false }] }] }],
    stills: [],
    vendor: [],
    notes: [{ level: 'info', path: 'inputs', message: 'sample' }],
  };
}

const commit: Commit = {
  id: 'abc',
  at: '2026-09-19T10:00:00Z',
  message: 'first',
  hash: 'h',
  changes: 0,
  summary: { id: 'x', name: 'Gala', platform: 'barco-em', model: 'Encore3', firmware: '', modified: '', inputs: 1, outputs: 1, screens: 1, auxes: 0, presets: 1, masterPresets: 0, layerMemories: 0, cues: 1, multiviewers: 1, notesDropped: 0 },
};

describe('documents', () => {
  it('builds a PDF with every section', async () => {
    const bytes = await buildPdf(sample(), [commit], DEFAULT_DOC_OPTIONS);
    expect(bytes.length).toBeGreaterThan(5000);
    expect(String.fromCharCode(...bytes.subarray(0, 5))).toBe('%PDF-');
    const doc = await PDFDocument.load(bytes);
    // Cover, system, patch, destinations, screen, presets, preset page, cues, MV, glossary, notes.
    expect(doc.getPageCount()).toBeGreaterThanOrEqual(9);
  });

  it('builds a PDF for both demo shows', async () => {
    for (const name of ['encore3-simulator', 'aquilon-cmax-simulator']) {
      const bytes = await buildPdf(demo(name), [], DEFAULT_DOC_OPTIONS);
      expect(String.fromCharCode(...bytes.subarray(0, 5))).toBe('%PDF-');
      expect(bytes.length).toBeGreaterThan(10_000);
    }
  });

  it('renders every theme and both papers', async () => {
    const show = sample();
    for (const t of THEME_LIST) {
      const bytes = await buildPdf(show, [], { ...DEFAULT_DOC_OPTIONS, theme: t.id });
      expect(String.fromCharCode(...bytes.subarray(0, 5))).toBe('%PDF-');
      expect(buildHtml(show, [], { ...DEFAULT_DOC_OPTIONS, theme: t.id })).toContain(`--accent:${t.accent}`);
    }
    const letter = await PDFDocument.load(await buildPdf(show, [], { ...DEFAULT_DOC_OPTIONS, paper: 'letter' }));
    expect(Math.round(letter.getPage(0).getWidth())).toBe(612);
    const a4 = await PDFDocument.load(await buildPdf(show, [], { ...DEFAULT_DOC_OPTIONS, paper: 'a4' }));
    expect(Math.round(a4.getPage(0).getWidth())).toBe(595);
  });

  it('puts the production details on the cover of both formats', async () => {
    const show = sample();
    show.meta.production = { event: 'Autumn Launch', client: 'Northwind', company: 'Stage Co', venue: 'The Round', date: '3 October 2026', operator: 'A. Operator', contact: 'a@example.com' };
    const html = buildHtml(show, [], DEFAULT_DOC_OPTIONS);
    for (const v of ['Autumn Launch', 'Northwind', 'Stage Co', 'The Round', '3 October 2026', 'A. Operator']) expect(html).toContain(v);
    const doc = buildDocument(show, [], DEFAULT_DOC_OPTIONS);
    const cover = doc.blocks[0];
    expect(cover.kind).toBe('cover');
    if (cover.kind === 'cover') {
      expect(cover.title).toBe('Autumn Launch');
      expect(cover.production.map(([k]) => k)).toContain('Venue');
    }
    expect(doc.title.startsWith('Autumn Launch')).toBe(true);
  });

  it('writes a self-contained HTML page with a table of contents and diagrams', () => {
    const html = buildHtml(sample(), [commit], DEFAULT_DOC_OPTIONS);
    expect(html.startsWith('<!doctype html>')).toBe(true);
    expect(html).not.toMatch(/<(script|link|img)[^>]+\b(src|href)="(https?:)?\/\//);
    expect(html).toContain('<svg');
    expect(html).toContain('id="q"');
    for (const heading of ['System and chassis', 'Signal flow', 'Patch', 'Destinations', 'Cues']) expect(html).toContain(`>${heading}</h1>`);
    const ids = [...html.matchAll(/<section class="s" id="([^"]+)"/g)].map((m) => m[1]);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) expect(html).toContain(`href="#${id}"`);
  });

  it('documents the layer memories under the name the platform uses', () => {
    const show = sample();
    show.layerMemories = [
      { id: 'lmem:1', number: 1, label: 'Lower third', categories: ['SOURCE', 'POS'], canvas: { w: 1920, h: 1080 }, sourceId: 'src:0', state: { layerId: '', sourceId: 'src:0', visible: true, rect: { x: 0, y: 540, w: 960, h: 540 } } },
    ];
    const em = buildHtml(show, [], DEFAULT_DOC_OPTIONS);
    expect(em).toContain('>User keys</h1>');
    expect(em).toContain('Lower third');
    expect(em).toContain('bound to a source');
    const aw = buildHtml({ ...show, platform: 'aw-live-premier' }, [], DEFAULT_DOC_OPTIONS);
    expect(aw).toContain('>Layer memories</h1>');
    expect(aw).toContain('A LivePremier holds 50');
    // The look is drawn on the canvas it was saved for.
    expect(aw).toContain('on a 1920×1080 canvas');
  });

  it('reads a show saved before layer memories existed', async () => {
    const old = sample();
    delete (old as { layerMemories?: unknown }).layerMemories;
    const bytes = await buildPdf(old, [], DEFAULT_DOC_OPTIONS);
    expect(String.fromCharCode(...bytes.subarray(0, 5))).toBe('%PDF-');
    expect(buildHtml(old, [], DEFAULT_DOC_OPTIONS)).not.toContain('>User keys</h1>');
  });

  it('honours the section switches', () => {
    const show = sample();
    const off = buildHtml(show, [], { ...DEFAULT_DOC_OPTIONS, includePresets: false, includeMultiviewers: false, includeGlossary: false, includeHistory: false });
    expect(off).not.toContain('>Presets</h1>');
    expect(off).not.toContain('>What the settings mean</h1>');
    expect(buildHtml(show, [], DEFAULT_DOC_OPTIONS)).toContain('>Presets</h1>');
  });

  it('escapes HTML in show data', () => {
    const show = sample();
    show.meta.name = '<script>alert(1)</script>';
    show.meta.notes = 'a & b';
    const html = buildHtml(show, [], DEFAULT_DOC_OPTIONS);
    expect(html).not.toContain('<script>alert(1)</script>');
    expect(html).toContain('&lt;script&gt;');
    expect(html).toContain('a &amp; b');
  });

  it('maps characters pdf-lib cannot encode', () => {
    expect(clean('−∞ · 1920×1080 → out')).toBe('-inf - 1920x1080 -> out');
    expect(clean('naïve – dash')).toBe('naïve – dash');
  });

  it('the ink theme uses no coloured fills', () => {
    expect(THEMES.ink.ink).toBe(true);
    const html = buildHtml(sample(), [], { ...DEFAULT_DOC_OPTIONS, theme: 'ink' });
    expect(html).toContain('class="ink ');
    expect(html).not.toMatch(/<td[^>]*style="background:#/);
  });
});
