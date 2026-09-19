import type { Platform } from '../types';

export interface GlossaryEntry {
  term: string;
  text: string;
  platforms?: Platform[];
}

/** Plain-English notes for the documentation's "what the settings mean" page. */
export const GLOSSARY: GlossaryEntry[] = [
  { term: 'Input', text: 'A physical signal arriving at a connector on the chassis. Its format is what the switcher detected (or was told to expect); its capacity class says how much processing the input claims — a 4K input takes what two DL/2K or four SL/HD inputs would.' },
  { term: 'Source', text: 'What a layer is fed with. Usually an input, but also a still, a screen\'s own program output (for a confidence monitor or a fold-back), a multiviewer, or a colour/background. On Event Master a source may carry an area-of-interest crop of its input.' },
  { term: 'Output', text: 'A physical connector driving a display, projector, LED processor or recorder, at a fixed raster (format). Outputs are assigned to a screen (as part of its canvas), to an aux (a single scaled feed), or to a multiviewer.' },
  { term: 'Screen (destination)', text: 'A canvas made of one or more outputs placed edge to edge or blended, on which layers are composited. Its size in pixels is the union of its outputs. Presets recall a state per screen.' },
  { term: 'Aux (auxiliary screen)', text: 'A single-output destination that shows one source at a time, scaled to the output, without layers. Used for records, confidence feeds and simple secondary displays.' },
  { term: 'Layer', text: 'A window composited on a screen: a source, a position and size on the canvas, optional crop, opacity, border and keying. Mixing layers cross-fade between two states (preview and program); a background/native layer sits underneath everything and does not scale.' },
  { term: 'Capacity (4K / DL / 2K / SL)', text: 'How much of the mixing pool a layer or input reserves. The families count in different units but the ratio is the same: one 4K = two dual-link/2K = four single-link/HD. Conversion between platforms uses this arithmetic to check that a show fits.' },
  { term: 'Preset / memory', text: 'A stored state. On Event Master a preset can hold several destinations at once; on Analog Way a memory holds one screen and a master memory recalls a memory per screen together. Recalling to preview then TAKE puts it on air; recalling to program cuts straight to it.' },
  { term: 'TAKE / transition', text: 'Preview becomes program on the chosen screens, with the screen\'s transition time. On a LivePremier the two live states are lettered (A/B/C) and the T-bar state says which letter is on air.', platforms: ['aw-live-premier', 'aw-midra4k', 'aw-alta4k'] },
  { term: 'Cue', text: 'A stored sequence on the frame: recalls, takes and waits played in order from one trigger.', platforms: ['barco-em', 'barco-pds4k'] },
  { term: 'DSK', text: 'A downstream key layer on top of every mixing layer of a screen, typically for a logo or lower-third with an alpha or luma key. Event Master destinations have one; Analog Way screens put keying on ordinary layers instead.', platforms: ['barco-em', 'barco-pds4k'] },
  { term: 'Multiviewer', text: 'An output that tiles many sources into windows with labels (UMD) and tally colours so the operator can see inputs, previews and programs on one monitor. A layout is one arrangement of windows; some processors store several and switch between them.' },
  { term: 'HDCP', text: 'Content protection negotiated on an HDMI/DP link. An HDCP-encrypted input can only reach outputs that also negotiate HDCP; a screen with a non-HDCP output will show black for that source.' },
  { term: 'Genlock', text: 'The frame\'s timing reference: internal, an input, or an external black-burst/tri-level sync. Outputs run at the native rate locked to it; a show that mixes 50 and 60 Hz sources relies on frame-rate conversion at the inputs.' },
  { term: 'Native rate', text: 'The frame rate every output runs at. Sources at other rates are converted at the input, which costs a frame or two of delay.' },
  { term: 'Test pattern', text: 'A pattern the output generates itself in place of the canvas, used to check the display chain without content. Showbook can generate labelled patterns per output for the same purpose (Documents & export).' },
  { term: 'Vendor file', text: 'The switcher\'s own show file: an Event Master backup archive (the frame\'s XML store) or a LivePremier .awc (an encrypted configuration package). Showbook keeps it next to its own model so a show can go back to the same hardware exactly.' },
];
