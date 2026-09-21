/**
 * The show documentation as a `Document`: cover with the production
 * details, the chassis drawn as a frame map, the signal flow, the patch
 * tables, every screen and preset drawn to scale, the preset/screen
 * matrix, cues on a timeline, multiviewer layouts, a glossary, the
 * import notes and the version history. `pdf.ts` and `html.ts` render it.
 */
import { flowDiagram, legend, type FlowLink, type FlowNode } from './doc/diagrams';
import { DocBuilder, approxWidth, fitText, type CellLike, type Diagram, type DiagramItem, type Document, type Tile } from './doc/ir';
import { contentWidth, type PaperId, type ThemeId } from './doc/theme';
import { GLOSSARY } from './glossary';
import { PLATFORM_LABEL, describeFormat, tail, type Commit, type LayerState, type PresetTarget, type Screen, type Show } from '../types';

export interface DocOptions {
  includePresets: boolean;
  includeMultiviewers: boolean;
  includeGlossary: boolean;
  includeHistory: boolean;
  /** "Prepared for" on the cover. */
  preparedBy: string;
  /** "Prepared by" on the cover — the library's author from Settings. */
  author?: string;
  theme?: ThemeId;
  paper?: PaperId;
}

export const DEFAULT_DOC_OPTIONS: DocOptions = { includePresets: true, includeMultiviewers: true, includeGlossary: true, includeHistory: true, preparedBy: '', theme: 'light', paper: 'a4' };

const fmtDate = (iso: string) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString();
};

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

const srcLabel = (show: Show, id?: string) => (id ? (show.sources.find((s) => s.id === id)?.label ?? id) : '');

/** A screen to scale with its outputs and the given layers. */
function screenDiagram(show: Show, screen: Screen, layers: LayerState[], cw: number, background?: string, maxH = 220): Diagram {
  const W = Math.max(screen.size.w, 16);
  const H = Math.max(screen.size.h, 9);
  let scale = cw / W;
  if (H * scale > maxH) scale = maxH / H;
  const dw = W * scale;
  const dh = H * scale;
  const items: DiagramItem[] = [];
  // Labels drawn inside the canvas, so later ones can dodge earlier ones.
  const placed: { x: number; y: number; w: number }[] = [];
  const place = (x: number, y: number, w: number): number => {
    let yy = y;
    while (placed.some((p) => Math.abs(p.y - yy) < 8 && x < p.x + p.w && p.x < x + w)) yy += 8;
    placed.push({ x, y: yy, w });
    return yy;
  };
  for (const om of screen.outputs) {
    const o = show.outputs.find((x) => x.id === om.outputId);
    items.push({ t: 'rect', x: om.rect.x * scale, y: om.rect.y * scale, w: om.rect.w * scale, h: om.rect.h * scale, stroke: 'canvasLine', sw: 0.6, dash: [3, 2] });
    const text = `${o?.label ?? om.outputId}${o?.format ? ` ${o.format.width}×${o.format.height}` : ''}`;
    const x = om.rect.x * scale + 3;
    items.push({ t: 'text', x, y: place(x, om.rect.y * scale + 9, approxWidth(text, 6.5)), text, size: 6.5, color: 'canvasText' });
  }
  if (background) {
    const text = `background: ${srcLabel(show, background)}`;
    items.push({ t: 'text', x: 4, y: dh - 4, text: fitText(text, 6.5, dw - 8), size: 6.5, color: 'canvasText', opacity: 0.8 });
    placed.push({ x: 4, y: dh - 4, w: approxWidth(text, 6.5) });
  }
  const defs = screen.layers;
  const ordered = [...layers].filter((l) => l.rect).sort((a, b) => (defs.find((d) => d.id === a.layerId)?.z ?? 0) - (defs.find((d) => d.id === b.layerId)?.z ?? 0));
  // Layers that start at the same corner as another label drop below it.
  ordered.forEach((l, i) => {
    const r = l.rect!;
    const c = `palette:${i}`;
    items.push({ t: 'rect', x: r.x * scale, y: r.y * scale, w: r.w * scale, h: r.h * scale, fill: c, opacity: l.visible ? 0.3 : 0.1, stroke: c, sw: l.visible ? 1 : 0.5, strokeOpacity: l.visible ? 1 : 0.5 });
    const label = `${defs.find((d) => d.id === l.layerId)?.label ?? l.layerId}${l.sourceId ? ` – ${srcLabel(show, l.sourceId)}` : ''}${l.visible ? '' : ' (off)'}`;
    const text = fitText(label, 7, Math.max(30, r.w * scale - 6), true);
    const x = r.x * scale + 3;
    items.push({ t: 'text', x, y: place(x, r.y * scale + 9, approxWidth(text, 7, true)), text, size: 7, bold: true, color: 'canvasText' });
  });
  return { w: dw, h: dh, items, canvas: true };
}

function layerRows(show: Show, screen: Screen, t: PresetTarget): CellLike[][] {
  const rows: CellLike[][] = screen.layers
    .filter((d) => d.kind !== 'background')
    .map((d, i) => {
      const l = t.layers.find((x) => x.layerId === d.id);
      if (!l) return [{ text: d.label, swatch: `palette:${i}` }, { text: '(not set)', muted: true }, '', '', ''];
      return [
        { text: d.label, swatch: `palette:${i}` },
        l.sourceId ? srcLabel(show, l.sourceId) : '–',
        { text: l.visible ? 'on' : 'off', dot: l.visible ? 'good' : 'muted' },
        l.rect ? `${Math.round(l.rect.x)},${Math.round(l.rect.y)}  ${Math.round(l.rect.w)}×${Math.round(l.rect.h)}` : '',
        [l.opacity !== undefined && l.opacity < 1 ? `${Math.round(l.opacity * 100)}% opacity` : '', l.border ? `border ${l.border.width}px` : '', l.crop ? 'cropped' : ''].filter(Boolean).join(', '),
      ];
    });
  if (t.background) rows.unshift(['Background', srcLabel(show, t.background), { text: 'on', dot: 'good' }, '', '']);
  return rows;
}

/** Every frame as a row of slots, each connector a square coloured by what it carries. */
function frameMap(show: Show, frame: Show['system']['frames'][number], cw: number): Diagram {
  const items: DiagramItem[] = [];
  const labelW = 150;
  const sq = 10;
  const gap = 3;
  const perRow = Math.max(1, Math.floor((cw - labelW - 6) / (sq + gap)));
  const leg = legend(
    [
      { label: 'input, patched', color: 'palette:0' },
      { label: 'output, in use', color: 'palette:1' },
      { label: 'output, unassigned', color: 'palette:2' },
      { label: 'empty', color: 'rule' },
    ],
    cw,
  );
  items.push(...leg.items);
  let y = leg.h + 6;
  for (const s of frame.slots) {
    const rows = Math.max(1, Math.ceil(s.connectors.length / perRow));
    const rowH = Math.max(20, rows * (sq + gap) + 6);
    items.push({ t: 'rect', x: 0, y, w: cw, h: rowH, stroke: 'rule', sw: 0.4 });
    items.push({ t: 'text', x: 4, y: y + 9, text: `Slot ${s.index}`, size: 6.5, bold: true, color: 'text' });
    items.push({ t: 'text', x: 4, y: y + rowH - 4, text: fitText(s.card || s.label || '(empty)', 6, labelW - 8), size: 6, color: 'muted' });
    s.connectors.forEach((c, i) => {
      const cx = labelW + (i % perRow) * (sq + gap);
      const cy = y + 3 + Math.floor(i / perRow) * (sq + gap);
      const input = show.inputs.find((x) => x.connectorIds.includes(c.id));
      const output = show.outputs.find((x) => x.connectorIds.includes(c.id));
      const used = c.direction === 'in' ? Boolean(input) : output ? output.role !== 'unassigned' : false;
      const fill = c.direction === 'in' ? (input ? 'palette:0' : undefined) : output ? (used ? 'palette:1' : 'palette:2') : undefined;
      items.push({ t: 'rect', x: cx, y: cy, w: sq, h: sq, fill, stroke: fill ? undefined : 'rule', sw: 0.5, r: 1.5 });
      items.push({ t: 'text', x: cx + sq / 2, y: cy + sq - 2.6, text: String(c.index), size: 4.8, anchor: 'middle', color: fill ? '#ffffff' : 'muted', bold: Boolean(fill) });
    });
    y += rowH + 2;
  }
  return { w: cw, h: y, items };
}

/** All destinations at one scale, so their relative sizes are visible. */
function canvasOverview(show: Show, cw: number): Diagram {
  const maxW = Math.max(16, ...show.screens.map((s) => s.size.w));
  const maxH = Math.max(9, ...show.screens.map((s) => s.size.h));
  const s = Math.min((cw / 2 - 10) / maxW, 110 / maxH);
  const items: DiagramItem[] = [];
  let x = 0;
  let y = 0;
  let rowH = 0;
  for (const sc of show.screens) {
    const w = Math.max(sc.size.w * s, 40);
    const h = Math.max(sc.size.h * s, 12);
    if (x + w > cw && x > 0) {
      x = 0;
      y += rowH + 18;
      rowH = 0;
    }
    items.push({ t: 'text', x, y: y + 8, text: fitText(`${sc.kind === 'aux' ? 'Aux ' : ''}${sc.label} · ${sc.size.w}×${sc.size.h}`, 6.5, w, true), size: 6.5, bold: true, color: 'text' });
    items.push({ t: 'rect', x, y: y + 11, w, h, fill: 'canvas', stroke: 'canvasLine', sw: 0.5 });
    for (const om of sc.outputs) {
      const o = show.outputs.find((z) => z.id === om.outputId);
      items.push({ t: 'rect', x: x + om.rect.x * s, y: y + 11 + om.rect.y * s, w: om.rect.w * s, h: om.rect.h * s, stroke: 'canvasLine', sw: 0.5, dash: [2, 1.5] });
      items.push({ t: 'text', x: x + om.rect.x * s + 2, y: y + 11 + om.rect.y * s + 7, text: fitText(o?.label ?? om.outputId, 5.5, om.rect.w * s - 4), size: 5.5, color: 'canvasText' });
    }
    rowH = Math.max(rowH, h + 11);
    x += w + 10;
  }
  return { w: cw, h: y + rowH + 4, items };
}

/** Inputs → destinations (screens, auxes, multiviewers) → outputs. */
function signalFlow(show: Show, cw: number): Diagram {
  const inputNode = new Map<string, FlowNode>();
  for (const i of show.inputs) inputNode.set(i.id, { id: i.id, label: i.label, sub: describeFormat(i.format), color: 'palette:0' });
  const dests: FlowNode[] = [
    ...show.screens.map((s) => ({ id: s.id, label: `${s.kind === 'aux' ? 'Aux ' : ''}${s.label}`, sub: `${s.size.w}×${s.size.h}`, color: s.kind === 'aux' ? 'palette:2' : 'palette:1' })),
    ...show.multiviewers.map((m) => ({ id: m.id, label: m.label, sub: 'multiviewer', color: 'palette:4' })),
  ];
  const outputs: FlowNode[] = show.outputs.map((o) => ({ id: o.id, label: o.label, sub: describeFormat(o.format), color: o.role === 'unassigned' ? 'muted' : 'palette:1' }));
  const links = new Map<string, FlowLink>();
  const add = (from: string, to: string, color?: string) => {
    if (from && to && !links.has(`${from}>${to}`)) links.set(`${from}>${to}`, { from, to, color });
  };
  const inputOfSource = (srcId?: string) => {
    const s = srcId ? show.sources.find((x) => x.id === srcId) : undefined;
    if (!s) return undefined;
    if (s.kind === 'input' && s.refId && inputNode.has(s.refId)) return s.refId;
    return undefined;
  };
  const addLayers = (screenId: string, layers: LayerState[], background?: string) => {
    for (const l of layers) {
      const i = inputOfSource(l.sourceId);
      if (i) add(i, screenId, 'palette:0');
    }
    const b = inputOfSource(background);
    if (b) add(b, screenId, 'palette:0');
  };
  for (const sc of show.screens) {
    const pgm = sc.extra?.programState as PresetTarget | undefined;
    if (pgm) addLayers(sc.id, pgm.layers, pgm.background);
    for (const om of sc.outputs) add(sc.id, om.outputId, sc.kind === 'aux' ? 'palette:2' : 'palette:1');
  }
  for (const p of show.presets) for (const t of p.targets) addLayers(t.screenId, t.layers, t.background);
  for (const mv of show.multiviewers) {
    for (const lay of mv.layouts) for (const w of lay.widgets) {
      const i = inputOfSource(w.sourceId);
      if (i) add(i, mv.id, 'palette:4');
    }
    for (const o of mv.outputIds) add(mv.id, o, 'palette:4');
  }
  return flowDiagram(
    [
      { title: 'Inputs', nodes: [...inputNode.values()] },
      { title: 'Destinations', nodes: dests },
      { title: 'Outputs', nodes: outputs },
    ],
    [...links.values()],
    cw,
  );
}

/** A cue's steps along a time axis. */
function cueTimeline(show: Show, cue: Show['cues'][number], cw: number): Diagram {
  const items: DiagramItem[] = [];
  const steps = cue.steps;
  const times: number[] = [];
  let t = 0;
  for (const s of steps) {
    t += s.delayMs ?? 0;
    times.push(t);
  }
  const total = t;
  const x0 = 12;
  const x1 = cw - 12;
  const y = 26;
  items.push({ t: 'line', x1: x0, y1: y, x2: x1, y2: y, stroke: 'rule', sw: 0.8 });
  const color = (k: string) => (k === 'recall-preset' ? 'palette:0' : k === 'recall-master' ? 'palette:4' : k === 'take' ? 'palette:2' : k === 'wait' ? 'muted' : 'palette:3');
  steps.forEach((s, i) => {
    const x = steps.length === 1 ? (x0 + x1) / 2 : total > 0 ? x0 + ((x1 - x0) * times[i]) / total : x0 + ((x1 - x0) * i) / (steps.length - 1);
    const target = s.presetId ? (show.presets.find((p) => p.id === s.presetId)?.label ?? s.presetId) : s.masterId ? (show.masterPresets.find((p) => p.id === s.masterId)?.label ?? s.masterId) : s.screenIds.map((id) => show.screens.find((sc) => sc.id === id)?.label ?? id).join(', ');
    items.push({ t: 'circle', cx: x, cy: y, r: 4, fill: color(s.kind), stroke: 'page', sw: 1 });
    items.push({ t: 'text', x, y: y - 9, text: fitText(`${i + 1}. ${s.kind}${target ? ` ${target}` : ''}`, 6, 90, true), size: 6, anchor: 'middle', bold: true, color: 'text' });
    items.push({ t: 'text', x, y: y + 14, text: total > 0 ? `${times[i]} ms` : '', size: 5.5, anchor: 'middle', color: 'muted' });
  });
  return { w: cw, h: 44, items };
}

export function buildDocument(show: Show, commits: Commit[], opts: DocOptions): Document {
  const cw = contentWidth(opts.paper ?? 'a4');
  const d = new DocBuilder(cw);
  const screens = show.screens.filter((s) => s.kind === 'screen');
  const auxes = show.screens.filter((s) => s.kind === 'aux');
  const prod = show.meta.production ?? {};
  const platform = `${PLATFORM_LABEL[show.platform]} – ${show.system.model || 'model not set'}${show.system.firmware ? ` – firmware ${show.system.firmware}` : ''}`;
  const title = prod.event?.trim() || show.meta.name;

  // ---- cover
  const production: [string, string][] = [];
  if (prod.event && prod.event !== show.meta.name) production.push(['Show file', show.meta.name]);
  if (prod.client) production.push(['Client', prod.client]);
  if (prod.company) production.push(['Production company', prod.company]);
  if (prod.venue) production.push(['Venue', prod.venue]);
  if (prod.date) production.push(['Date', prod.date]);
  if (prod.operator) production.push(['Operator', prod.operator]);
  if (prod.contact) production.push(['Contact', prod.contact]);
  const tiles: Tile[] = [
    { label: 'inputs', value: String(show.inputs.length) },
    { label: 'outputs', value: String(show.outputs.length) },
    { label: 'screens', value: String(screens.length), hint: auxes.length ? `+ ${auxes.length} aux` : undefined },
    { label: 'presets', value: String(show.presets.length), hint: show.masterPresets.length ? `+ ${show.masterPresets.length} master` : undefined },
    { label: 'cues', value: String(show.cues.length) },
    { label: 'multiviewers', value: String(show.multiviewers.length) },
    { label: 'native rate', value: show.system.nativeRate ? `${show.system.nativeRate} Hz` : '–' },
    { label: 'genlock', value: show.system.genlock ? show.system.genlock.source : '–' },
  ];
  const lines = [`Generated by Showbook on ${new Date().toLocaleString()}${opts.preparedBy ? ` for ${opts.preparedBy}` : ''}${opts.author ? `, prepared by ${opts.author}` : ''}.`];
  if (show.meta.source) lines.push(`Model captured from ${show.meta.source.kind} ${show.meta.source.origin} at ${fmtDate(show.meta.source.at)}.`);
  d.cover({ title, subtitle: title === show.meta.name ? platform : `${show.meta.name} · ${platform}`, production, tiles, lines, notes: show.meta.notes || undefined });

  // ---- system
  d.pagebreak();
  d.h1('System and chassis');
  d.p(`Device name: ${show.system.name || '–'}. ${show.system.frames.length} frame(s).`);
  for (const f of show.system.frames) {
    d.h2(`${f.label} – ${f.model}${f.address ? ` – ${f.address}` : ''}`);
    if (f.slots.length) d.diagram(frameMap(show, f, cw), 'Each square is a connector, numbered as on the card; colour says whether the show uses it.');
    d.table(
      ['Slot', 'Card', 'Connectors and what is on them'],
      f.slots.map((s) => [
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
      ]),
      { widths: [40, 150, 321] },
    );
  }

  // ---- signal flow
  if (show.inputs.length || show.screens.length || show.outputs.length) {
    d.h1('Signal flow');
    d.p('Inputs that a preset or the captured program state puts on a destination, the destinations, and the outputs each one drives. Multiviewers are destinations too.', true);
    d.diagram(signalFlow(show, cw));
  }

  // ---- patch
  d.pagebreak();
  d.h1('Patch');
  d.h2('Inputs');
  d.table(
    ['#', 'Label', 'Connector', 'Format', 'Capacity', 'HDCP'],
    show.inputs.map((i) => [tail(i.id), i.label, connectorLabel(show, i.connectorIds), describeFormat(i.format), i.capacity ?? '', i.hdcp === undefined ? '' : i.hdcp ? 'yes' : 'no']),
    { widths: [50, 150, 140, 90, 50, 31] },
  );
  d.h2('Outputs');
  d.table(
    ['#', 'Label', 'Connector', 'Format', 'Role', 'Feeds'],
    show.outputs.map((o) => [
      tail(o.id),
      o.label,
      connectorLabel(show, o.connectorIds),
      describeFormat(o.format),
      { text: o.role, muted: o.role === 'unassigned' },
      [...show.screens.filter((sc) => sc.outputs.some((m) => m.outputId === o.id)).map((sc) => sc.label), ...show.multiviewers.filter((m) => m.outputIds.includes(o.id)).map((m) => m.label)].join(', '),
    ]),
    { widths: [50, 130, 130, 80, 60, 61] },
  );
  if (show.sources.length) {
    d.h2('Sources');
    d.table(
      ['#', 'Label', 'Kind', 'Built on', 'Format'],
      show.sources.map((s) => [tail(s.id), s.label, s.kind, s.refId ? refLabel(show, s.refId) : '', describeFormat(s.format)]),
      { widths: [60, 170, 70, 130, 81] },
    );
  }

  // ---- screens
  if (show.screens.length) {
    d.pagebreak();
    d.h1('Destinations');
    d.p(`${screens.length} screen(s)${auxes.length ? ` and ${auxes.length} aux` : ''}, drawn at one scale with each output's region.`, true);
    d.diagram(canvasOverview(show, cw));
    d.table(
      ['Destination', 'Kind', 'Canvas', 'Outputs', 'Mixing layers', 'Transition'],
      show.screens.map((sc) => [
        sc.label,
        sc.kind,
        `${sc.size.w}×${sc.size.h}`,
        sc.outputs.map((om) => show.outputs.find((o) => o.id === om.outputId)?.label ?? om.outputId).join(', '),
        { text: String(sc.layers.filter((l) => l.kind === 'mixer').length), align: 'r' },
        sc.transition?.durationMs !== undefined ? `${sc.transition.durationMs} ms ${sc.transition.kind ?? ''}` : '–',
      ]),
      { widths: [110, 45, 70, 160, 60, 66], align: ['l', 'l', 'l', 'l', 'r', 'l'] },
    );
  }
  for (const sc of show.screens) {
    d.pagebreak();
    d.h1(`${sc.kind === 'aux' ? 'Aux ' : 'Screen '}${sc.label}`);
    d.p(`${sc.size.w} × ${sc.size.h} pixels, ${sc.outputs.length} output(s), ${sc.layers.filter((l) => l.kind === 'mixer').length} mixing layer(s)${sc.layers.some((l) => l.kind === 'background') ? ', native background' : ''}${sc.layers.some((l) => l.id === 'layer:dsk') ? ', DSK' : ''}. Transition: ${sc.transition?.durationMs !== undefined ? `${sc.transition.durationMs} ms ${sc.transition.kind ?? ''}` : 'not recorded'}.`);
    const pgm = sc.extra?.programState as PresetTarget | undefined;
    d.diagram(screenDiagram(show, sc, pgm?.layers ?? [], cw, pgm?.background), pgm ? `Layers as captured on program${sc.extra?.programLetter ? ` (preset ${sc.extra.programLetter})` : ''}.` : 'No captured program state; the outline shows the outputs only.');
    d.table(
      ['Output', 'Position on canvas', 'Raster', 'Connector'],
      sc.outputs.map((om) => {
        const o = show.outputs.find((x) => x.id === om.outputId);
        return [o?.label ?? om.outputId, `${Math.round(om.rect.x)},${Math.round(om.rect.y)}`, describeFormat(o?.format), connectorLabel(show, o?.connectorIds ?? [])];
      }),
      { widths: [140, 110, 110, 151] },
    );
    d.table(
      ['Layer', 'Kind', 'Capacity', 'Order'],
      sc.layers.map((l, i) => [{ text: l.label, swatch: l.kind === 'background' ? 'muted' : `palette:${i}` }, l.kind, l.capacity ?? '', { text: String(l.z), align: 'r' }]),
      { widths: [200, 100, 100, 111], align: ['l', 'l', 'l', 'r'] },
    );
    if (pgm) {
      d.h3('On program');
      d.table(['Layer', 'Source', 'On', 'Position / size', 'Notes'], layerRows(show, sc, pgm), { widths: [90, 150, 30, 120, 121] });
    }
  }

  // ---- presets
  if (opts.includePresets && show.presets.length) {
    d.pagebreak();
    d.h1('Presets');
    d.table(
      ['#', 'Label', 'Screens', 'Notes'],
      show.presets.map((p) => [String(p.number ?? tail(p.id)), p.label, p.targets.map((t) => show.screens.find((s) => s.id === t.screenId)?.label ?? t.screenId).join(', '), p.notes]),
      { widths: [40, 160, 200, 111] },
    );
    if (show.screens.length > 1) {
      d.h2('Which preset touches which destination');
      d.p('A dot with the number of layers the preset switches on there.', true);
      d.table(
        ['Preset', ...show.screens.map((s) => s.label)],
        show.presets.map((p) => [
          `${p.number ?? tail(p.id)} ${p.label}`,
          ...show.screens.map((s) => {
            const t = p.targets.find((x) => x.screenId === s.id);
            if (!t || (!t.layers.length && !t.background)) return '';
            const on = t.layers.filter((l) => l.visible).length;
            return { text: on ? String(on) : '', dot: true, align: 'c' as const };
          }),
        ]),
        { widths: [130, ...show.screens.map(() => Math.max(30, 381 / show.screens.length))], size: 'small' },
      );
    }
    for (const p of show.presets) {
      if (!p.targets.some((t) => t.layers.length || t.background)) continue;
      d.pagebreak();
      d.h1(`Preset ${p.number ?? tail(p.id)} – ${p.label}`);
      if (p.notes) d.p(p.notes);
      for (const t of p.targets) {
        const sc = show.screens.find((s) => s.id === t.screenId);
        if (!sc) continue;
        d.h2(sc.label);
        d.diagram(screenDiagram(show, sc, t.layers, cw, t.background, 170));
        d.table(['Layer', 'Source', 'On', 'Position / size', 'Notes'], layerRows(show, sc, t), { widths: [90, 150, 30, 120, 121] });
      }
    }
    if (show.masterPresets.length) {
      d.pagebreak();
      d.h1('Master presets');
      d.p('Each master recalls one preset per destination; the table shows which.', true);
      const cols = show.screens.filter((s) => show.masterPresets.some((m) => m.entries.some((e) => e.screenId === s.id)));
      d.table(
        ['#', 'Master', ...cols.map((s) => s.label)],
        show.masterPresets.map((m) => [
          String(m.number ?? tail(m.id)),
          m.label,
          ...cols.map((s) => {
            const e = m.entries.find((x) => x.screenId === s.id);
            return e ? (show.presets.find((p) => p.id === e.presetId)?.label ?? e.presetId) : { text: '', muted: true };
          }),
        ]),
        { widths: [30, 120, ...cols.map(() => Math.max(40, 361 / Math.max(1, cols.length)))], size: cols.length > 6 ? 'small' : 'normal' },
      );
    }
  }

  // ---- cues
  if (show.cues.length) {
    d.pagebreak();
    d.h1('Cues');
    for (const c of show.cues) {
      d.h2(`Cue ${c.number ?? tail(c.id)} – ${c.label}`);
      if (c.steps.length === 0) d.p('(no steps recorded)', true);
      else {
        d.diagram(cueTimeline(show, c, cw));
        d.table(
          ['Step', 'Action', 'Target', 'Delay'],
          c.steps.map((s, i) => [String(i + 1), s.kind, s.presetId ? (show.presets.find((p) => p.id === s.presetId)?.label ?? s.presetId) : s.masterId ? (show.masterPresets.find((p) => p.id === s.masterId)?.label ?? s.masterId) : s.screenIds.map((id) => show.screens.find((sc) => sc.id === id)?.label ?? id).join(', '), s.delayMs !== undefined ? `${s.delayMs} ms` : '']),
          { widths: [40, 120, 271, 80] },
        );
      }
    }
  }

  // ---- multiviewers
  if (opts.includeMultiviewers && show.multiviewers.length) {
    for (const mv of show.multiviewers) {
      for (const lay of mv.layouts) {
        d.pagebreak();
        d.h1(`${mv.label} – ${lay.label}${mv.activeLayout === lay.id ? ' (active)' : ''}`);
        d.p(`${lay.size.w} × ${lay.size.h}, ${lay.widgets.length} window(s)${mv.outputIds.length ? `, on ${mv.outputIds.map((o) => show.outputs.find((x) => x.id === o)?.label ?? o).join(', ')}` : ''}.`);
        const W = lay.size.w;
        const H = lay.size.h;
        let scale = cw / W;
        if (H * scale > 260) scale = 260 / H;
        const items: DiagramItem[] = [];
        for (const w of lay.widgets) {
          items.push({ t: 'rect', x: w.rect.x * scale, y: w.rect.y * scale, w: w.rect.w * scale, h: w.rect.h * scale, fill: 'canvasLine', opacity: 0.25, stroke: 'canvasLine', sw: 0.5 });
          const label = w.label ?? srcLabel(show, w.sourceId);
          items.push({ t: 'text', x: w.rect.x * scale + 2, y: (w.rect.y + w.rect.h) * scale - 3, text: fitText(label, 6, w.rect.w * scale - 4), size: 6, color: 'canvasText' });
        }
        d.diagram({ w: W * scale, h: H * scale, items, canvas: true });
        d.table(
          ['Window', 'Source', 'Position', 'Size', 'Label'],
          lay.widgets.map((w) => [tail(w.id), w.sourceId ? srcLabel(show, w.sourceId) : '–', `${Math.round(w.rect.x)},${Math.round(w.rect.y)}`, `${Math.round(w.rect.w)}×${Math.round(w.rect.h)}`, w.showLabel ? (w.label ?? '(source name)') : 'off']),
          { widths: [60, 170, 80, 80, 121] },
        );
      }
    }
  }

  // ---- glossary
  if (opts.includeGlossary) {
    d.pagebreak();
    d.h1('What the settings mean');
    d.glossary(GLOSSARY.filter((g) => !g.platforms || g.platforms.includes(show.platform)).map((g) => ({ term: g.term, text: g.text })));
  }

  // ---- notes and history
  if (show.notes.length) {
    d.pagebreak();
    d.h1('Import and conversion notes');
    d.table(
      ['Level', 'Where', 'Note'],
      show.notes.map((n) => [{ text: n.level, dot: n.level === 'dropped' ? 'bad' : n.level === 'adapted' ? 'warn' : 'muted' }, n.path, n.message]),
      { widths: [60, 130, 321] },
    );
  }
  if (opts.includeHistory && commits.length) {
    if (!show.notes.length) d.pagebreak();
    d.h1('Version history');
    d.table(
      ['When', 'Version', 'Message', 'Changes', 'Author'],
      [...commits]
        .reverse()
        .slice(0, 40)
        .map((c) => [c.at.replace('T', ' ').replace('Z', ''), c.id, c.message, { text: String(c.changes), align: 'r' }, c.author ?? '']),
      { widths: [120, 70, 200, 50, 71], align: ['l', 'l', 'l', 'r', 'l'] },
    );
  }

  return { title: `${title} – ${PLATFORM_LABEL[show.platform]} ${show.system.model}`.trim(), subtitle: platform, producer: 'Showbook', blocks: d.blocks };
}
