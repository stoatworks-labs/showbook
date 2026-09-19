/**
 * Test patterns from the show: one PNG per output at the output's raster,
 * labelled with what the output is and where it sits; and one per screen
 * showing the whole canvas with the output boundaries. Drawn on a canvas in
 * the webview; the bytes go to disk through the Rust side.
 */
import type { Screen, Show } from '../types';

export type PatternKind = 'grid' | 'bars' | 'alignment';

const BARS_75 = ['#c0c0c0', '#c0c000', '#00c0c0', '#00c000', '#c000c0', '#c00000', '#0000c0', '#000000'];

function canvas(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  return c;
}

function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number, pitch: number) {
  ctx.strokeStyle = 'rgba(255,255,255,0.28)';
  ctx.lineWidth = 1;
  for (let x = pitch; x < w; x += pitch) {
    ctx.beginPath();
    ctx.moveTo(x + 0.5, 0);
    ctx.lineTo(x + 0.5, h);
    ctx.stroke();
  }
  for (let y = pitch; y < h; y += pitch) {
    ctx.beginPath();
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(w, y + 0.5);
    ctx.stroke();
  }
  ctx.strokeStyle = 'rgba(255,255,255,0.7)';
  for (let x = pitch * 5; x < w; x += pitch * 5) {
    ctx.beginPath();
    ctx.moveTo(x + 0.5, 0);
    ctx.lineTo(x + 0.5, h);
    ctx.stroke();
  }
  for (let y = pitch * 5; y < h; y += pitch * 5) {
    ctx.beginPath();
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(w, y + 0.5);
    ctx.stroke();
  }
}

function drawFrameMarks(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 1;
  ctx.strokeRect(0.5, 0.5, w - 1, h - 1);
  const arm = Math.round(Math.min(w, h) * 0.06);
  for (const [x, y, sx, sy] of [
    [0, 0, 1, 1],
    [w, 0, -1, 1],
    [0, h, 1, -1],
    [w, h, -1, -1],
  ]) {
    ctx.beginPath();
    ctx.moveTo(x + sx * 8, y + sy * 8);
    ctx.lineTo(x + sx * (8 + arm), y + sy * 8);
    ctx.moveTo(x + sx * 8, y + sy * 8);
    ctx.lineTo(x + sx * 8, y + sy * (8 + arm));
    ctx.lineWidth = 3;
    ctx.stroke();
  }
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(w / 2, 0);
  ctx.lineTo(w / 2, h);
  ctx.moveTo(0, h / 2);
  ctx.lineTo(w, h / 2);
  ctx.stroke();
  const r = Math.round(Math.min(w, h) * 0.2);
  ctx.beginPath();
  ctx.arc(w / 2, h / 2, r, 0, Math.PI * 2);
  ctx.stroke();
  // Safe areas.
  ctx.setLineDash([12, 8]);
  ctx.strokeStyle = 'rgba(255,255,255,0.6)';
  ctx.strokeRect(w * 0.05 + 0.5, h * 0.05 + 0.5, w * 0.9, h * 0.9);
  ctx.strokeRect(w * 0.1 + 0.5, h * 0.1 + 0.5, w * 0.8, h * 0.8);
  ctx.setLineDash([]);
}

function drawBars(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number) {
  const bw = w / BARS_75.length;
  BARS_75.forEach((c, i) => {
    ctx.fillStyle = c;
    ctx.fillRect(x + i * bw, y, Math.ceil(bw), h);
  });
}

function label(ctx: CanvasRenderingContext2D, lines: string[], w: number, h: number, yFrac = 0.5 - 0.12) {
  const size = Math.max(18, Math.round(Math.min(w, h) / 22));
  ctx.font = `bold ${size}px -apple-system, Helvetica, Arial, sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  const boxW = Math.max(...lines.map((l) => ctx.measureText(l).width)) + size * 2;
  const boxH = lines.length * size * 1.4 + size;
  const bx = w / 2 - boxW / 2;
  const by = h * yFrac - boxH / 2;
  ctx.fillStyle = 'rgba(0,0,0,0.78)';
  ctx.fillRect(bx, by, boxW, boxH);
  ctx.strokeStyle = '#fff';
  ctx.lineWidth = 2;
  ctx.strokeRect(bx + 1, by + 1, boxW - 2, boxH - 2);
  ctx.fillStyle = '#fff';
  lines.forEach((l, i) => ctx.fillText(l, w / 2, by + size * 0.9 + i * size * 1.4));
}

export function outputPattern(show: Show, outputId: string, kind: PatternKind): HTMLCanvasElement | null {
  const o = show.outputs.find((x) => x.id === outputId);
  if (!o) return null;
  const w = o.format?.width ?? 1920;
  const h = o.format?.height ?? 1080;
  const c = canvas(w, h);
  const ctx = c.getContext('2d')!;
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, w, h);
  const screen = show.screens.find((s) => s.outputs.some((m) => m.outputId === o.id));
  const om = screen?.outputs.find((m) => m.outputId === o.id);
  if (kind === 'bars') {
    drawBars(ctx, 0, 0, w, Math.round(h * 0.67));
    drawGrid(ctx, w, h, 64);
  } else if (kind === 'grid') {
    drawGrid(ctx, w, h, 64);
  }
  drawFrameMarks(ctx, w, h);
  drawBars(ctx, w * 0.1, h * 0.86, w * 0.8, h * 0.06);
  const conn = o.connectorIds
    .map((id) => {
      for (const f of show.system.frames) for (const s of f.slots) for (const cc of s.connectors) if (cc.id === id) return cc.label;
      return id;
    })
    .join(', ');
  label(
    ctx,
    [
      `${o.label}  ·  ${w}×${h}${o.format?.rate ? `p${Math.round(o.format.rate)}` : ''}`,
      conn ? `connector: ${conn}` : 'unpatched',
      screen && om ? `${screen.label} @ ${Math.round(om.rect.x)},${Math.round(om.rect.y)} of ${screen.size.w}×${screen.size.h}` : o.role,
      show.meta.name,
    ],
    w,
    h,
  );
  // Direction arrows to neighbouring outputs of the same screen.
  if (screen && om) {
    ctx.font = `bold ${Math.round(Math.min(w, h) / 30)}px Helvetica, Arial, sans-serif`;
    ctx.fillStyle = '#ff0';
    ctx.textAlign = 'center';
    const n = (dx: number, dy: number) => screen.outputs.find((m) => m.outputId !== o.id && Math.abs(m.rect.x - (om.rect.x + dx)) < 2 && Math.abs(m.rect.y - (om.rect.y + dy)) < 2);
    const right = n(om.rect.w, 0);
    const left = n(-om.rect.w, 0);
    const below = n(0, om.rect.h);
    const above = n(0, -om.rect.h);
    if (right) ctx.fillText(`→ ${show.outputs.find((x) => x.id === right.outputId)?.label ?? ''}`, w * 0.85, h / 2);
    if (left) ctx.fillText(`${show.outputs.find((x) => x.id === left.outputId)?.label ?? ''} ←`, w * 0.15, h / 2);
    if (below) ctx.fillText(`↓ ${show.outputs.find((x) => x.id === below.outputId)?.label ?? ''}`, w / 2, h * 0.8);
    if (above) ctx.fillText(`↑ ${show.outputs.find((x) => x.id === above.outputId)?.label ?? ''}`, w / 2, h * 0.2);
  }
  return c;
}

/** The whole screen canvas with every output's region outlined and named. */
export function screenMap(show: Show, screen: Screen): HTMLCanvasElement {
  const w = Math.max(screen.size.w, 16);
  const h = Math.max(screen.size.h, 9);
  const c = canvas(w, h);
  const ctx = c.getContext('2d')!;
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, w, h);
  drawGrid(ctx, w, h, 64);
  screen.outputs.forEach((om, i) => {
    const o = show.outputs.find((x) => x.id === om.outputId);
    ctx.fillStyle = `hsla(${(i * 67) % 360}, 60%, 40%, 0.35)`;
    ctx.fillRect(om.rect.x, om.rect.y, om.rect.w, om.rect.h);
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 3;
    ctx.strokeRect(om.rect.x + 1.5, om.rect.y + 1.5, om.rect.w - 3, om.rect.h - 3);
    const size = Math.max(16, Math.round(Math.min(om.rect.w, om.rect.h) / 12));
    ctx.font = `bold ${size}px Helvetica, Arial, sans-serif`;
    ctx.fillStyle = '#fff';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(o?.label ?? om.outputId, om.rect.x + om.rect.w / 2, om.rect.y + om.rect.h / 2 - size * 0.7);
    ctx.font = `${Math.round(size * 0.7)}px Helvetica, Arial, sans-serif`;
    ctx.fillText(`${Math.round(om.rect.x)},${Math.round(om.rect.y)}  ${Math.round(om.rect.w)}×${Math.round(om.rect.h)}`, om.rect.x + om.rect.w / 2, om.rect.y + om.rect.h / 2 + size * 0.7);
  });
  drawFrameMarks(ctx, w, h);
  label(ctx, [`${screen.label}  ·  ${w}×${h}`, `${screen.outputs.length} output${screen.outputs.length === 1 ? '' : 's'}`, show.meta.name], w, h, 0.16);
  return c;
}

export function toPngBase64(c: HTMLCanvasElement): Promise<string> {
  return new Promise((resolve, reject) => {
    c.toBlob((blob) => {
      if (!blob) return reject(new Error('toBlob failed'));
      const r = new FileReader();
      r.onload = () => resolve(String(r.result).split(',')[1]);
      r.onerror = () => reject(r.error);
      r.readAsDataURL(blob);
    }, 'image/png');
  });
}

export function safeName(s: string): string {
  return s.replace(/[^A-Za-z0-9._-]+/g, '_').replace(/^_+|_+$/g, '') || 'x';
}
