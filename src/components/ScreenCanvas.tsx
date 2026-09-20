import { useRef, useState, type PointerEvent as ReactPointerEvent } from 'react';

import { sourceLabel } from '../lib/format';
import type { LayerState, Rect, Screen, Show } from '../types';

export interface CanvasProps {
  show: Show;
  screen: Screen;
  /** Layer states to draw (a preset target's, or the program state's). */
  layers: LayerState[];
  background?: string;
  selected?: string | null;
  onSelect?: (layerId: string) => void;
  /** Called with the moved/resized rect while dragging; absent = read-only. */
  onChange?: (layerId: string, rect: Rect) => void;
  width?: number;
}

const LAYER_COLORS = ['#2f9ee0', '#3aa675', '#f5a524', '#e05c9e', '#8b6fe0', '#c07a2f', '#4fc3f7', '#a3e635'];

/** A screen drawn to scale: output regions, then the layer rects. */
export function ScreenCanvas({ show, screen, layers, background, selected, onSelect, onChange, width = 640 }: CanvasProps) {
  const W = Math.max(screen.size.w, 16);
  const H = Math.max(screen.size.h, 9);
  const scale = width / W;
  const height = H * scale;
  const svg = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState<{ id: string; mode: 'move' | 'resize'; start: { x: number; y: number }; rect: Rect } | null>(null);

  const toCanvas = (e: ReactPointerEvent) => {
    const r = svg.current!.getBoundingClientRect();
    return { x: (e.clientX - r.left) / scale, y: (e.clientY - r.top) / scale };
  };

  const down = (e: ReactPointerEvent, l: LayerState, mode: 'move' | 'resize') => {
    onSelect?.(l.layerId);
    if (!onChange || !l.rect) return;
    e.preventDefault();
    (e.target as Element).setPointerCapture?.(e.pointerId);
    setDrag({ id: l.layerId, mode, start: toCanvas(e), rect: { ...l.rect } });
  };
  const move = (e: ReactPointerEvent) => {
    if (!drag || !onChange) return;
    const p = toCanvas(e);
    const dx = p.x - drag.start.x;
    const dy = p.y - drag.start.y;
    const snap = (v: number) => Math.round(v / 2) * 2;
    if (drag.mode === 'move') onChange(drag.id, { ...drag.rect, x: snap(drag.rect.x + dx), y: snap(drag.rect.y + dy) });
    else onChange(drag.id, { ...drag.rect, w: Math.max(16, snap(drag.rect.w + dx)), h: Math.max(9, snap(drag.rect.h + dy)) });
  };
  const up = () => setDrag(null);

  const layerDefs = screen.layers;
  const ordered = [...layers].sort((a, b) => (layerDefs.find((d) => d.id === a.layerId)?.z ?? 0) - (layerDefs.find((d) => d.id === b.layerId)?.z ?? 0));

  return (
    <svg ref={svg} className="canvas" width={width} height={height} viewBox={`0 0 ${W} ${H}`} onPointerMove={move} onPointerUp={up} onPointerLeave={up}>
      <rect x={0} y={0} width={W} height={H} fill="#0b0d12" stroke="#262c38" strokeWidth={2 / scale} />
      {background ? (
        <text x={W / 2} y={H - 12 / scale} textAnchor="middle" fill="#55606f" fontSize={18 / scale}>
          background: {sourceLabel(show, background)}
        </text>
      ) : null}
      {screen.outputs.map((om) => {
        const o = show.outputs.find((x) => x.id === om.outputId);
        return (
          <g key={om.outputId}>
            <rect x={om.rect.x} y={om.rect.y} width={om.rect.w} height={om.rect.h} fill="#141821" stroke="#3b4454" strokeDasharray={`${8 / scale} ${6 / scale}`} strokeWidth={1.5 / scale} />
            <text x={om.rect.x + 8 / scale} y={om.rect.y + 18 / scale} fill="#8e99ab" fontSize={14 / scale}>
              {o?.label ?? om.outputId} {o?.format ? `${o.format.width}×${o.format.height}` : ''}
            </text>
          </g>
        );
      })}
      {ordered.map((l, i) => {
        if (!l.rect) return null;
        const def = layerDefs.find((d) => d.id === l.layerId);
        const color = LAYER_COLORS[i % LAYER_COLORS.length];
        const sel = selected === l.layerId;
        return (
          <g key={l.layerId} opacity={l.visible ? 1 : 0.35}>
            <rect
              x={l.rect.x}
              y={l.rect.y}
              width={l.rect.w}
              height={l.rect.h}
              fill={color}
              fillOpacity={0.18 * (l.opacity ?? 1) + 0.08}
              stroke={color}
              strokeWidth={(sel ? 3 : 1.5) / scale}
              style={{ cursor: onChange ? 'move' : 'pointer' }}
              onPointerDown={(e) => down(e, l, 'move')}
            />
            {/* A hidden layer keeps its faint outline but not its labels: in a
                captured state every off layer sits at the same rect, and their
                labels piled up over the output's. */}
            {l.visible ? (
              <>
                <text x={l.rect.x + 8 / scale} y={l.rect.y + 40 / scale} fill={color} fontSize={16 / scale} pointerEvents="none">
                  {def?.label ?? l.layerId}
                  {l.sourceId ? ` — ${sourceLabel(show, l.sourceId)}` : ''}
                </text>
                <text x={l.rect.x + 8 / scale} y={l.rect.y + 58 / scale} fill="#8e99ab" fontSize={12 / scale} pointerEvents="none">
                  {Math.round(l.rect.x)},{Math.round(l.rect.y)} {Math.round(l.rect.w)}×{Math.round(l.rect.h)}
                  {l.opacity !== undefined && l.opacity < 1 ? ` · ${Math.round(l.opacity * 100)}%` : ''}
                </text>
              </>
            ) : null}
            {onChange ? (
              <rect x={l.rect.x + l.rect.w - 14 / scale} y={l.rect.y + l.rect.h - 14 / scale} width={14 / scale} height={14 / scale} fill={color} style={{ cursor: 'nwse-resize' }} onPointerDown={(e) => down(e, l, 'resize')} />
            ) : null}
          </g>
        );
      })}
    </svg>
  );
}
