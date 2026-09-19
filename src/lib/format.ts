import type { Connector, Show } from '../types';

export function fmtDate(iso: string): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}

export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export function sourceLabel(show: Show, id?: string): string {
  if (!id) return '';
  return show.sources.find((s) => s.id === id)?.label ?? id;
}

/** Who uses a connector: the input or output plugged into it. */
export function connectorUse(show: Show, c: Connector): { label: string; kind: 'input' | 'output' } | null {
  const i = show.inputs.find((x) => x.connectorIds.includes(c.id));
  if (i) return { label: i.label, kind: 'input' };
  const o = show.outputs.find((x) => x.connectorIds.includes(c.id));
  if (o) return { label: o.label, kind: 'output' };
  return null;
}
