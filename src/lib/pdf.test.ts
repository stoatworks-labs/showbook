import { PDFDocument } from 'pdf-lib';
import { describe, expect, it } from 'vitest';

import type { Show } from '../types';
import { buildPdf } from './pdf';

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
    cues: [{ id: 'cue:0', number: 1, label: 'Open', steps: [{ kind: 'recall-preset', presetId: 'pre:0', screenIds: [], delayMs: 0 }] }],
    multiviewers: [{ id: 'mv:1', label: 'MV', outputIds: [], layouts: [{ id: 'mvl:1.1', label: 'Layout 1', size: { w: 1920, h: 1080 }, widgets: [{ id: 'w:0', rect: { x: 0, y: 0, w: 640, h: 360 }, sourceId: 'src:0', showLabel: true, tally: false }] }] }],
    stills: [],
    vendor: [],
    notes: [{ level: 'info', path: 'inputs', message: 'sample' }],
  };
}

describe('pdf', () => {
  it('builds a document with every section', async () => {
    const bytes = await buildPdf(sample(), [{ id: 'abc', at: '2026-09-19T10:00:00Z', message: 'first', hash: 'h', changes: 0, summary: { id: 'x', name: 'Gala', platform: 'barco-em', model: 'Encore3', firmware: '', modified: '', inputs: 1, outputs: 1, screens: 1, auxes: 0, presets: 1, masterPresets: 0, cues: 1, multiviewers: 1, notesDropped: 0 } }], {
      includePresets: true,
      includeMultiviewers: true,
      includeGlossary: true,
      includeHistory: true,
      preparedBy: 'the crew',
    });
    expect(bytes.length).toBeGreaterThan(5000);
    expect(String.fromCharCode(...bytes.subarray(0, 5))).toBe('%PDF-');
    // Cover, system/patch, screen, presets list, preset page, cues, MV, glossary, notes+history.
    const doc = await PDFDocument.load(bytes);
    expect(doc.getPageCount()).toBeGreaterThanOrEqual(8);
  });
});
