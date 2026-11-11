// Applies appearance settings as root attributes (DESIGN.md §9): data-theme, data-accent,
// data-density, data-motion. `system` follows the OS and keeps following it.
export interface Appearance {
  theme?: 'system' | 'light' | 'dark';
  accent?: string;
  density?: 'comfortable' | 'compact';
  reduce_motion?: 'system' | 'on' | 'off';
}

let mq: MediaQueryList | undefined;
let listener: (() => void) | undefined;

export function applyAppearance(a: Appearance, root: HTMLElement = document.documentElement): void {
  mq ??= matchMedia('(prefers-color-scheme: dark)');
  const resolve = () => (a.theme === 'dark' || (a.theme !== 'light' && mq!.matches) ? 'dark' : 'light');
  root.dataset.theme = resolve();
  if (listener) mq.removeEventListener('change', listener);
  listener = a.theme === 'system' || !a.theme ? () => (root.dataset.theme = resolve()) : undefined;
  if (listener) mq.addEventListener('change', listener);
  root.dataset.accent = a.accent && a.accent !== 'custom' ? a.accent : 'ember';
  root.dataset.density = a.density ?? 'comfortable';
  if (a.reduce_motion === 'on') root.dataset.motion = 'reduced';
  else if (a.reduce_motion === 'off') root.dataset.motion = 'full';
  else delete root.dataset.motion;
}
