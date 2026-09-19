/**
 * Show documentation as a PDF, drawn with pdf-lib: cover, chassis and patch
 * tables, one page per screen and per preset with the layers drawn to
 * scale, multiviewer layouts, cues, a glossary explaining the settings the
 * document mentions, the import/conversion notes, and the version history.
 */
import { PDFDocument, PDFFont, PDFPage, StandardFonts, rgb, type RGB } from 'pdf-lib';

import { PLATFORM_LABEL, describeFormat, tail, type Commit, type LayerState, type PresetTarget, type Screen, type Show } from '../types';
import { GLOSSARY } from './glossary';

const A4 = { w: 595.28, h: 841.89 };
const M = 42;
const LAYER_COLORS: RGB[] = [rgb(0.18, 0.62, 0.88), rgb(0.23, 0.65, 0.46), rgb(0.96, 0.65, 0.14), rgb(0.88, 0.36, 0.62), rgb(0.55, 0.44, 0.88), rgb(0.75, 0.48, 0.18)];

function clean(s: unknown): string {
  return String(s ?? '')
    .replace(/[→]/g, '->')
    .replace(/[–—]/g, '-')
    .replace(/[×]/g, 'x')
    .replace(/[…]/g, '...')
    .replace(/[^\x20-\x7e\xa0-\xff]/g, '?');
}

class Doc {
  doc!: PDFDocument;
  font!: PDFFont;
  bold!: PDFFont;
  mono!: PDFFont;
  page!: PDFPage;
  y = 0;
  pageNo = 0;
  title = '';

  static async create(title: string): Promise<Doc> {
    const d = new Doc();
    d.doc = await PDFDocument.create();
    d.font = await d.doc.embedFont(StandardFonts.Helvetica);
    d.bold = await d.doc.embedFont(StandardFonts.HelveticaBold);
    d.mono = await d.doc.embedFont(StandardFonts.Courier);
    d.title = title;
    d.doc.setTitle(title);
    d.doc.setProducer('Showbook');
    d.newPage();
    return d;
  }

  newPage() {
    this.page = this.doc.addPage([A4.w, A4.h]);
    this.pageNo += 1;
    this.y = A4.h - M;
    this.page.drawText(clean(this.title), { x: M, y: A4.h - 24, size: 8, font: this.font, color: rgb(0.5, 0.5, 0.5) });
    const pn = `${this.pageNo}`;
    this.page.drawText(pn, { x: A4.w - M - this.font.widthOfTextAtSize(pn, 8), y: A4.h - 24, size: 8, font: this.font, color: rgb(0.5, 0.5, 0.5) });
    this.page.drawLine({ start: { x: M, y: A4.h - 30 }, end: { x: A4.w - M, y: A4.h - 30 }, thickness: 0.5, color: rgb(0.8, 0.8, 0.8) });
  }

  need(h: number) {
    if (this.y - h < M) this.newPage();
  }

  h1(t: string) {
    this.need(40);
    this.y -= 22;
    this.page.drawText(clean(t), { x: M, y: this.y, size: 18, font: this.bold });
    this.y -= 10;
  }

  h2(t: string) {
    this.need(30);
    this.y -= 18;
    this.page.drawText(clean(t), { x: M, y: this.y, size: 13, font: this.bold });
    this.y -= 6;
  }

  wrap(text: string, size: number, font: PDFFont, width: number): string[] {
    const out: string[] = [];
    for (const para of clean(text).split('\n')) {
      let line = '';
      for (const word of para.split(' ')) {
        const cand = line ? `${line} ${word}` : word;
        if (font.widthOfTextAtSize(cand, size) > width && line) {
          out.push(line);
          line = word;
        } else line = cand;
      }
      out.push(line);
    }
    return out;
  }

  p(text: string, opts: { size?: number; color?: RGB; font?: PDFFont; indent?: number } = {}) {
    const size = opts.size ?? 9.5;
    const font = opts.font ?? this.font;
    const indent = opts.indent ?? 0;
    for (const line of this.wrap(text, size, font, A4.w - 2 * M - indent)) {
      this.need(size + 4);
      this.y -= size + 3;
      this.page.drawText(line, { x: M + indent, y: this.y, size, font, color: opts.color ?? rgb(0.1, 0.1, 0.1) });
    }
  }

  gap(h = 8) {
    this.y -= h;
  }

  table(headers: string[], rows: string[][], widths?: number[]) {
    const total = A4.w - 2 * M;
    const ws = widths ?? headers.map(() => total / headers.length);
    const size = 8.5;
    const drawRow = (cells: string[], font: PDFFont, bg?: RGB) => {
      const lines = cells.map((c, i) => this.wrap(c, size, font, ws[i] - 6));
      const h = Math.max(...lines.map((l) => l.length)) * (size + 3) + 5;
      this.need(h);
      if (bg) this.page.drawRectangle({ x: M, y: this.y - h, width: total, height: h, color: bg });
      let x = M;
      lines.forEach((ls, i) => {
        ls.forEach((l, k) => this.page.drawText(l, { x: x + 3, y: this.y - (k + 1) * (size + 3) + 1, size, font }));
        x += ws[i];
      });
      this.y -= h;
      this.page.drawLine({ start: { x: M, y: this.y }, end: { x: A4.w - M, y: this.y }, thickness: 0.3, color: rgb(0.75, 0.75, 0.75) });
    };
    drawRow(headers, this.bold, rgb(0.92, 0.93, 0.95));
    for (const r of rows) drawRow(r, this.font);
    this.gap(6);
  }

  /** A screen drawn to scale with its outputs and the given layers. */
  screenDiagram(show: Show, screen: Screen, layers: LayerState[], background?: string, maxH = 220) {
    const W = Math.max(screen.size.w, 16);
    const H = Math.max(screen.size.h, 9);
    const width = A4.w - 2 * M;
    let scale = width / W;
    if (H * scale > maxH) scale = maxH / H;
    const dw = W * scale;
    const dh = H * scale;
    this.need(dh + 14);
    const x0 = M;
    const y0 = this.y - dh;
    this.page.drawRectangle({ x: x0, y: y0, width: dw, height: dh, color: rgb(0.11, 0.12, 0.15), borderColor: rgb(0.4, 0.4, 0.4), borderWidth: 0.6 });
    for (const om of screen.outputs) {
      const o = show.outputs.find((x) => x.id === om.outputId);
      this.page.drawRectangle({ x: x0 + om.rect.x * scale, y: y0 + dh - (om.rect.y + om.rect.h) * scale, width: om.rect.w * scale, height: om.rect.h * scale, borderColor: rgb(0.45, 0.5, 0.58), borderWidth: 0.6, borderDashArray: [3, 2] });
      this.page.drawText(clean(`${o?.label ?? om.outputId}${o?.format ? ` ${o.format.width}x${o.format.height}` : ''}`), { x: x0 + om.rect.x * scale + 3, y: y0 + dh - om.rect.y * scale - 9, size: 6.5, font: this.font, color: rgb(0.7, 0.72, 0.78) });
    }
    if (background) {
      const src = show.sources.find((s) => s.id === background)?.label ?? background;
      this.page.drawText(clean(`background: ${src}`), { x: x0 + 4, y: y0 + 4, size: 6.5, font: this.font, color: rgb(0.6, 0.6, 0.6) });
    }
    const defs = screen.layers;
    const ordered = [...layers].filter((l) => l.rect).sort((a, b) => (defs.find((d) => d.id === a.layerId)?.z ?? 0) - (defs.find((d) => d.id === b.layerId)?.z ?? 0));
    ordered.forEach((l, i) => {
      const r = l.rect!;
      const c = LAYER_COLORS[i % LAYER_COLORS.length];
      const op = l.visible ? 0.28 : 0.1;
      this.page.drawRectangle({ x: x0 + r.x * scale, y: y0 + dh - (r.y + r.h) * scale, width: r.w * scale, height: r.h * scale, color: c, opacity: op, borderColor: c, borderWidth: l.visible ? 1 : 0.5, borderOpacity: l.visible ? 1 : 0.5 });
      const label = `${defs.find((d) => d.id === l.layerId)?.label ?? l.layerId}${l.sourceId ? ` - ${show.sources.find((s) => s.id === l.sourceId)?.label ?? l.sourceId}` : ''}`;
      this.page.drawText(clean(label), { x: x0 + r.x * scale + 3, y: y0 + dh - r.y * scale - 9, size: 7, font: this.bold, color: rgb(1, 1, 1) });
    });
    this.y = y0 - 8;
  }

  layerTable(show: Show, screen: Screen, t: PresetTarget) {
    const rows = screen.layers
      .filter((d) => d.kind !== 'background')
      .map((d) => {
        const l = t.layers.find((x) => x.layerId === d.id);
        if (!l) return [d.label, '(not set)', '', '', ''];
        return [
          d.label,
          l.sourceId ? (show.sources.find((s) => s.id === l.sourceId)?.label ?? l.sourceId) : '-',
          l.visible ? 'on' : 'off',
          l.rect ? `${Math.round(l.rect.x)},${Math.round(l.rect.y)}  ${Math.round(l.rect.w)}x${Math.round(l.rect.h)}` : '',
          [l.opacity !== undefined && l.opacity < 1 ? `${Math.round(l.opacity * 100)}% opacity` : '', l.border ? `border ${l.border.width}px` : '', l.crop ? 'cropped' : ''].filter(Boolean).join(', '),
        ];
      });
    if (t.background) rows.unshift(['Background', show.sources.find((s) => s.id === t.background)?.label ?? t.background, 'on', '', '']);
    this.table(['Layer', 'Source', 'On', 'Position / size', 'Notes'], rows, [90, 150, 30, 120, 121]);
  }
}

export interface PdfOptions {
  includePresets: boolean;
  includeMultiviewers: boolean;
  includeGlossary: boolean;
  includeHistory: boolean;
  preparedBy: string;
}

export async function buildPdf(show: Show, commits: Commit[], opts: PdfOptions): Promise<Uint8Array> {
  const d = await Doc.create(`${show.meta.name} - ${PLATFORM_LABEL[show.platform]} ${show.system.model}`);
  const screens = show.screens.filter((s) => s.kind === 'screen');
  const auxes = show.screens.filter((s) => s.kind === 'aux');

  // ---- cover
  d.y -= 120;
  d.page.drawText(clean(show.meta.name), { x: M, y: d.y, size: 28, font: d.bold });
  d.y -= 30;
  d.page.drawText(clean(`${PLATFORM_LABEL[show.platform]} - ${show.system.model || 'model not set'}${show.system.firmware ? ` - firmware ${show.system.firmware}` : ''}`), { x: M, y: d.y, size: 13, font: d.font, color: rgb(0.3, 0.3, 0.3) });
  d.y -= 40;
  const facts = [
    ['Inputs', String(show.inputs.length)],
    ['Outputs', String(show.outputs.length)],
    ['Screens', `${screens.length}${auxes.length ? ` + ${auxes.length} aux` : ''}`],
    ['Presets', `${show.presets.length}${show.masterPresets.length ? ` + ${show.masterPresets.length} master` : ''}`],
    ['Cues', String(show.cues.length)],
    ['Multiviewers', String(show.multiviewers.length)],
    ['Native rate', show.system.nativeRate ? `${show.system.nativeRate} Hz` : '-'],
    ['Genlock', show.system.genlock ? show.system.genlock.source : '-'],
  ];
  for (const [k, v] of facts) {
    d.page.drawText(clean(k), { x: M, y: d.y, size: 10, font: d.bold });
    d.page.drawText(clean(v), { x: M + 110, y: d.y, size: 10, font: d.font });
    d.y -= 15;
  }
  d.y -= 20;
  d.p(`Generated by Showbook on ${new Date().toLocaleString()}${opts.preparedBy ? ` for ${opts.preparedBy}` : ''}.`, { color: rgb(0.4, 0.4, 0.4) });
  if (show.meta.source) d.p(`Model captured from ${show.meta.source.kind} ${show.meta.source.origin} at ${show.meta.source.at}.`, { color: rgb(0.4, 0.4, 0.4) });
  if (show.meta.notes) {
    d.gap(10);
    d.h2('Show notes');
    d.p(show.meta.notes);
  }

  // ---- system
  d.newPage();
  d.h1('System and chassis');
  d.p(`Device name: ${show.system.name || '-'}. ${show.system.frames.length} frame(s).`);
  for (const f of show.system.frames) {
    d.h2(`${f.label} - ${f.model}${f.address ? ` - ${f.address}` : ''}`);
    const rows = f.slots.map((s) => [
      String(s.index),
      s.card,
      s.connectors
        .map((c) => {
          const i = show.inputs.find((x) => x.connectorIds.includes(c.id));
          const o = show.outputs.find((x) => x.connectorIds.includes(c.id));
          const use = i ? `in: ${i.label}` : o ? `out: ${o.label}` : '';
          return `${c.label}${use ? ` = ${use}` : ''}`;
        })
        .join('; '),
    ]);
    d.table(['Slot', 'Card', 'Connectors and what is on them'], rows, [40, 150, 321]);
  }

  // ---- patch
  d.h1('Patch');
  d.h2('Inputs');
  d.table(
    ['#', 'Label', 'Connector', 'Format', 'Capacity', 'HDCP'],
    show.inputs.map((i) => [tail(i.id), i.label, connectorLabel(show, i.connectorIds), describeFormat(i.format), i.capacity ?? '', i.hdcp === undefined ? '' : i.hdcp ? 'yes' : 'no']),
    [50, 150, 140, 90, 50, 31],
  );
  d.h2('Outputs');
  d.table(
    ['#', 'Label', 'Connector', 'Format', 'Role', 'Feeds'],
    show.outputs.map((o) => [
      tail(o.id),
      o.label,
      connectorLabel(show, o.connectorIds),
      describeFormat(o.format),
      o.role,
      [...show.screens.filter((sc) => sc.outputs.some((m) => m.outputId === o.id)).map((sc) => sc.label), ...show.multiviewers.filter((m) => m.outputIds.includes(o.id)).map((m) => m.label)].join(', '),
    ]),
    [50, 130, 130, 80, 60, 61],
  );
  if (show.sources.length) {
    d.h2('Sources');
    d.table(
      ['#', 'Label', 'Kind', 'Built on', 'Format'],
      show.sources.map((s) => [tail(s.id), s.label, s.kind, s.refId ? refLabel(show, s.refId) : '', describeFormat(s.format)]),
      [60, 170, 70, 130, 81],
    );
  }

  // ---- screens
  for (const sc of show.screens) {
    d.newPage();
    d.h1(`${sc.kind === 'aux' ? 'Aux ' : 'Screen '}${sc.label}`);
    d.p(`${sc.size.w} x ${sc.size.h} pixels, ${sc.outputs.length} output(s), ${sc.layers.filter((l) => l.kind === 'mixer').length} mixing layer(s)${sc.layers.some((l) => l.kind === 'background') ? ', native background' : ''}${sc.layers.some((l) => l.id === 'layer:dsk') ? ', DSK' : ''}. Transition: ${sc.transition?.durationMs !== undefined ? `${sc.transition.durationMs} ms ${sc.transition.kind ?? ''}` : 'not recorded'}.`);
    const pgm = sc.extra?.programState as PresetTarget | undefined;
    d.screenDiagram(show, sc, pgm?.layers ?? [], pgm?.background);
    d.p(pgm ? `Layers as captured on program${sc.extra?.programLetter ? ` (preset ${sc.extra.programLetter})` : ''}.` : 'No captured program state; the outline shows the outputs only.', { color: rgb(0.4, 0.4, 0.4) });
    d.table(
      ['Output', 'Position on canvas', 'Raster', 'Connector'],
      sc.outputs.map((om) => {
        const o = show.outputs.find((x) => x.id === om.outputId);
        return [o?.label ?? om.outputId, `${Math.round(om.rect.x)},${Math.round(om.rect.y)}`, describeFormat(o?.format), connectorLabel(show, o?.connectorIds ?? [])];
      }),
      [140, 110, 110, 151],
    );
    d.table(
      ['Layer', 'Kind', 'Capacity', 'Order'],
      sc.layers.map((l) => [l.label, l.kind, l.capacity ?? '', String(l.z)]),
      [200, 100, 100, 111],
    );
    if (pgm) d.layerTable(show, sc, pgm);
  }

  // ---- presets
  if (opts.includePresets && show.presets.length) {
    d.newPage();
    d.h1('Presets');
    d.table(['#', 'Label', 'Screens', 'Notes'], show.presets.map((p) => [String(p.number ?? tail(p.id)), p.label, p.targets.map((t) => show.screens.find((s) => s.id === t.screenId)?.label ?? t.screenId).join(', '), p.notes]), [40, 160, 200, 111]);
    for (const p of show.presets) {
      if (!p.targets.some((t) => t.layers.length || t.background)) continue;
      d.newPage();
      d.h1(`Preset ${p.number ?? tail(p.id)} - ${p.label}`);
      if (p.notes) d.p(p.notes);
      for (const t of p.targets) {
        const sc = show.screens.find((s) => s.id === t.screenId);
        if (!sc) continue;
        d.h2(sc.label);
        d.screenDiagram(show, sc, t.layers, t.background, 170);
        d.layerTable(show, sc, t);
      }
    }
    if (show.masterPresets.length) {
      d.newPage();
      d.h1('Master presets');
      d.table(['#', 'Label', 'Recalls'], show.masterPresets.map((m) => [String(m.number ?? tail(m.id)), m.label, m.entries.map((e) => `${show.screens.find((s) => s.id === e.screenId)?.label ?? e.screenId} <- ${show.presets.find((p) => p.id === e.presetId)?.label ?? e.presetId}`).join('; ')]), [40, 160, 311]);
    }
  }

  // ---- cues
  if (show.cues.length) {
    d.newPage();
    d.h1('Cues');
    for (const c of show.cues) {
      d.h2(`Cue ${c.number ?? tail(c.id)} - ${c.label}`);
      if (c.steps.length === 0) d.p('(no steps recorded)', { color: rgb(0.4, 0.4, 0.4) });
      else
        d.table(
          ['Step', 'Action', 'Target', 'Delay'],
          c.steps.map((s, i) => [String(i + 1), s.kind, s.presetId ? (show.presets.find((p) => p.id === s.presetId)?.label ?? s.presetId) : s.masterId ? (show.masterPresets.find((p) => p.id === s.masterId)?.label ?? s.masterId) : s.screenIds.join(', '), s.delayMs !== undefined ? `${s.delayMs} ms` : '']),
          [40, 120, 271, 80],
        );
    }
  }

  // ---- multiviewers
  if (opts.includeMultiviewers && show.multiviewers.length) {
    for (const mv of show.multiviewers) {
      for (const lay of mv.layouts) {
        d.newPage();
        d.h1(`${mv.label} - ${lay.label}${mv.activeLayout === lay.id ? ' (active)' : ''}`);
        d.p(`${lay.size.w} x ${lay.size.h}, ${lay.widgets.length} window(s)${mv.outputIds.length ? `, on ${mv.outputIds.map((o) => show.outputs.find((x) => x.id === o)?.label ?? o).join(', ')}` : ''}.`);
        const W = lay.size.w;
        const H = lay.size.h;
        let scale = (A4.w - 2 * M) / W;
        if (H * scale > 260) scale = 260 / H;
        d.need(H * scale + 12);
        const x0 = M;
        const y0 = d.y - H * scale;
        d.page.drawRectangle({ x: x0, y: y0, width: W * scale, height: H * scale, color: rgb(0.11, 0.12, 0.15), borderColor: rgb(0.4, 0.4, 0.4), borderWidth: 0.6 });
        for (const w of lay.widgets) {
          d.page.drawRectangle({ x: x0 + w.rect.x * scale, y: y0 + H * scale - (w.rect.y + w.rect.h) * scale, width: w.rect.w * scale, height: w.rect.h * scale, color: rgb(0.16, 0.18, 0.22), borderColor: rgb(0.55, 0.6, 0.68), borderWidth: 0.5 });
          const label = w.label ?? (w.sourceId ? (show.sources.find((s) => s.id === w.sourceId)?.label ?? w.sourceId) : '');
          d.page.drawText(clean(label), { x: x0 + w.rect.x * scale + 2, y: y0 + H * scale - (w.rect.y + w.rect.h) * scale + 3, size: 6, font: d.font, color: rgb(0.9, 0.9, 0.9) });
        }
        d.y = y0 - 8;
        d.table(['Window', 'Source', 'Position', 'Size', 'Label'], lay.widgets.map((w) => [tail(w.id), w.sourceId ? (show.sources.find((s) => s.id === w.sourceId)?.label ?? w.sourceId) : '-', `${Math.round(w.rect.x)},${Math.round(w.rect.y)}`, `${Math.round(w.rect.w)}x${Math.round(w.rect.h)}`, w.showLabel ? (w.label ?? '(source name)') : 'off']), [60, 170, 80, 80, 121]);
      }
    }
  }

  // ---- glossary
  if (opts.includeGlossary) {
    d.newPage();
    d.h1('What the settings mean');
    const entries = GLOSSARY.filter((g) => !g.platforms || g.platforms.includes(show.platform));
    for (const g of entries) {
      d.need(40);
      d.gap(6);
      d.p(g.term, { font: d.bold, size: 10 });
      d.p(g.text, { indent: 10 });
    }
  }

  // ---- notes and history
  if (show.notes.length) {
    d.newPage();
    d.h1('Import and conversion notes');
    d.table(['Level', 'Where', 'Note'], show.notes.map((n) => [n.level, n.path, n.message]), [60, 130, 321]);
  }
  if (opts.includeHistory && commits.length) {
    d.h1('Version history');
    d.table(['When', 'Version', 'Message', 'Changes', 'Author'], [...commits].reverse().slice(0, 40).map((c) => [c.at.replace('T', ' ').replace('Z', ''), c.id, c.message, String(c.changes), c.author ?? '']), [120, 70, 200, 50, 71]);
  }

  return d.doc.save();
}

function connectorLabel(show: Show, ids: string[]): string {
  return ids
    .map((id) => {
      for (const f of show.system.frames) for (const s of f.slots) for (const c of s.connectors) if (c.id === id) return c.label;
      return id;
    })
    .join(', ');
}

function refLabel(show: Show, id: string): string {
  return show.inputs.find((i) => i.id === id)?.label ?? show.screens.find((s) => s.id === id)?.label ?? show.stills.find((s) => s.id === id)?.label ?? show.multiviewers.find((m) => m.id === id)?.label ?? id;
}
