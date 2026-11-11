// English only (ADR-0009). Keys stay so more locales can be added later.
import en from './locales/en.json' with { type: 'json' };

export const locales = { en };
export const defaultLocale = 'en';

const plural = new Intl.PluralRules('en');

/**
 * Formats a message: `{name}` placeholders and ICU-style plurals
 * `{count, plural, one {# blocked} other {# blocked}}`. Unknown keys return the key, so a
 * missing string is visible in tests and screenshots instead of silently empty.
 * @param {string} key
 * @param {Record<string, string | number>} [params]
 */
export function t(key, params = {}) {
  const msg = locales.en[key];
  if (msg === undefined) return key;
  const withPlurals = msg.replace(/\{(\w+), plural, ((?:\w+ \{[^}]*\} ?)+)\}/g, (_, name, forms) => {
    const n = Number(params[name] ?? 0);
    const map = Object.fromEntries([...forms.matchAll(/(\w+) \{([^}]*)\}/g)].map((m) => [m[1], m[2]]));
    const form = map[`=${n}`] ?? map[plural.select(n)] ?? map.other ?? '';
    return form.replaceAll('#', new Intl.NumberFormat('en').format(n));
  });
  return withPlurals.replace(/\{(\w+)\}/g, (m, name) => (name in params ? String(params[name]) : m));
}
