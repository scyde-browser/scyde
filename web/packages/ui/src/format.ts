import { t } from '@scyde/i18n';

const TONES = ['sky', 'periwinkle', 'lavender', 'mint', 'ember'] as const;

/** Host without `www.` for display lists (the omnibox uses core/omnibox for full safety). */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

/** Stable monogram letter + Aura tone per host. */
export function monogram(url: string): { letter: string; tone: (typeof TONES)[number] } {
  const host = hostOf(url);
  let h = 0;
  for (const c of host) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return { letter: (host.match(/[a-z0-9]/i)?.[0] ?? '·').toUpperCase(), tone: TONES[h % TONES.length]! };
}

/** 48300000 → "48.3 MB" (decimal units, like file managers). */
export function bytes(n: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0;
  let v = n;
  while (v >= 1000 && i < units.length - 1) {
    v /= 1000;
    i++;
  }
  return `${new Intl.NumberFormat('en', { maximumFractionDigits: v < 100 && i > 0 ? 1 : 0 }).format(v)} ${units[i]}`;
}

/** 312000 ms → "5 min", 21600000 → "6 h". */
export function duration(ms: number): string {
  const min = Math.round(ms / 60_000);
  if (min < 60) return t('time.min', { n: Math.max(min, ms > 0 ? 1 : 0) });
  return t('time.h', { n: Math.round(min / 60) });
}

/** Relative time for "recent" rows: "4 min", "3 h", "yesterday", "Mon", or a date. */
export function ago(unixSeconds: number, nowMs = Date.now()): string {
  const s = Math.max(0, nowMs / 1000 - unixSeconds);
  if (s < 3600) return t('time.min', { n: Math.max(1, Math.round(s / 60)) });
  if (s < 86_400) return t('time.h', { n: Math.round(s / 3600) });
  if (s < 2 * 86_400) return t('time.yesterday');
  const d = new Date(unixSeconds * 1000);
  if (s < 7 * 86_400) return new Intl.DateTimeFormat('en', { weekday: 'short' }).format(d);
  return new Intl.DateTimeFormat('en', { day: 'numeric', month: 'short' }).format(d);
}

export function count(n: number): string {
  return new Intl.NumberFormat('en').format(n);
}
