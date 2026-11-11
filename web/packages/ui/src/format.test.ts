import { describe, expect, it } from 'vitest';
import { ago, bytes, count, duration, hostOf, monogram } from './format';

describe('format', () => {
  it('hosts and monograms are stable', () => {
    expect(hostOf('https://www.index.hr/vijesti')).toBe('index.hr');
    expect(monogram('https://index.hr/a')).toEqual(monogram('https://www.index.hr/b'));
    expect(monogram('https://9gag.com').letter).toBe('9');
  });
  it('numbers', () => {
    expect(bytes(48_300_000)).toBe('48.3 MB');
    expect(bytes(3_120_000_000)).toBe('3.1 GB');
    expect(bytes(512)).toBe('512 B');
    expect(duration(312_000)).toBe('5 min');
    expect(duration(21_600_000)).toBe('6 h');
    expect(count(98311)).toBe('98,311');
  });
  it('relative time', () => {
    const now = 1_800_000_000_000;
    expect(ago(1_800_000_000 - 240, now)).toBe('4 min');
    expect(ago(1_800_000_000 - 3 * 3600, now)).toBe('3 h');
    expect(ago(1_800_000_000 - 26 * 3600, now)).toBe('yesterday');
  });
});
