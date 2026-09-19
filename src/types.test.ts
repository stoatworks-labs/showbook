import { describe, expect, it } from 'vitest';

import { describeFormat, tail } from './types';

describe('types helpers', () => {
  it('describes formats the way operators write them', () => {
    expect(describeFormat({ width: 1920, height: 1080, rate: 59.94, interlaced: false })).toBe('1920×1080p59.94');
    expect(describeFormat({ width: 3840, height: 2160, rate: 60, interlaced: false })).toBe('3840×2160p60');
    expect(describeFormat({ width: 1920, height: 1080, rate: 0, interlaced: false })).toBe('1920×1080p');
    expect(describeFormat(undefined)).toBe('—');
  });
  it('strips id prefixes', () => {
    expect(tail('scr:S1')).toBe('S1');
    expect(tail('pre:12')).toBe('12');
    expect(tail('plain')).toBe('plain');
  });
});
