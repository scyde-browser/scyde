// Host calls for the Windows sidebar. Outside Tauri (dev server, Playwright) a small demo state is used.
export interface Display {
  internal: boolean;
  scheme: string;
  subdomain: string;
  domain: string;
  rest: string;
  punycode: boolean;
}
export interface TabView {
  id: number;
  title: string;
  url: string;
  display: Display;
  loading: boolean;
  popups_blocked: number;
  pinned: boolean;
  space: string;
  /** Restored but not loaded yet. */
  asleep: boolean;
}
export interface Space {
  id: string;
  /** Empty for the default Space (shown with its localized name). */
  name: string;
  color: string;
}
export interface ChromeState {
  tabs: TabView[];
  active: number | null;
  /** The two tabs of the split view, left and right. */
  split: [number, number] | null;
  spaces: Space[];
  space: string;
  appearance: Record<string, string> | null;
  filters: string;
  maximized: boolean;
  collapsed: boolean;
}
export interface Hit {
  url: string;
  title: string;
}

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function action(action: string, args: { id?: number; input?: string } = {}): Promise<void> {
  if (!inTauri) return demoAction(action, args);
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('chrome_action', { action, id: args.id ?? null, input: args.input ?? null });
}

/** History matches for the command bar (local only). */
export async function suggestions(input: string): Promise<Hit[]> {
  if (!inTauri) {
    const all = [
      { url: 'https://www.index.hr/', title: 'Index.hr' },
      { url: 'https://github.com/scyde-browser/scyde', title: 'scyde-browser/scyde' },
    ];
    return all.filter((h) => (h.url + h.title).toLowerCase().includes(input.toLowerCase()));
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<Hit[]>('chrome_suggestions', { input });
}

export async function connect(onState: (s: ChromeState) => void, onFocus: () => void): Promise<void> {
  if (!inTauri) {
    demoListener = onState;
    onState(demo);
    if (new URLSearchParams(location.search).has('edit')) onFocus();
    return;
  }
  const { listen } = await import('@tauri-apps/api/event');
  const { invoke } = await import('@tauri-apps/api/core');
  await listen<ChromeState>('scyde://state', (e) => onState(e.payload));
  await listen('scyde://focus-address', () => onFocus());
  await invoke('chrome_ready');
}

// ---- demo (dev server and screenshots) ----
const d = (domain: string, sub = '', rest = '/'): Display => ({ internal: false, scheme: 'https', subdomain: sub, domain, rest, punycode: false });
const tab = (id: number, title: string, url: string, display: Display, extra: Partial<TabView> = {}): TabView => ({
  id, title, url, display, loading: false, popups_blocked: 0, pinned: false, space: 'personal', asleep: false, ...extra,
});
let demo: ChromeState = {
  tabs: [
    tab(1, 'Gmail', 'https://mail.google.com/', d('google.com', 'mail.'), { pinned: true }),
    tab(2, 'YouTube', 'https://www.youtube.com/', d('youtube.com', 'www.'), { pinned: true }),
    tab(3, 'Index.hr', 'https://www.index.hr/vijesti', d('index.hr', 'www.', '/vijesti'), { popups_blocked: 2 }),
    tab(4, 'scyde-browser/scyde: SCYDE Browser', 'https://github.com/scyde-browser/scyde', d('github.com', '', '/scyde-browser/scyde')),
    tab(5, 'Wikipedia', 'https://en.wikipedia.org/wiki/Zagreb', d('wikipedia.org', 'en.', '/wiki/Zagreb'), { loading: true }),
    tab(6, 'New Tab', 'scyde://newtab', { ...d('newtab'), internal: true, scheme: 'scyde' }),
    tab(7, 'Figma', 'https://www.figma.com/', d('figma.com', 'www.'), { space: 'space-1' }),
  ],
  active: 3,
  split: [3, 4],
  spaces: [
    { id: 'personal', name: '', color: 'ember' },
    { id: 'space-1', name: 'Work', color: 'sky' },
  ],
  space: 'personal',
  appearance: null,
  filters: '20261111',
  maximized: false,
  collapsed: false,
};
let demoListener: ((s: ChromeState) => void) | undefined;
function demoAction(a: string, args: { id?: number; input?: string }) {
  const { id, input } = args;
  if (a === 'switch' && id) demo = { ...demo, active: id };
  if (a === 'close' && id) demo = { ...demo, tabs: demo.tabs.filter((t) => t.id !== id) };
  if (a === 'pin' && id) demo = { ...demo, tabs: demo.tabs.map((t) => (t.id === id ? { ...t, pinned: !t.pinned } : t)) };
  if (a === 'split' && id) demo = { ...demo, split: demo.split?.includes(id) ? null : demo.active && demo.active !== id ? [demo.active, id] : demo.split };
  if (a === 'space' && input) demo = { ...demo, space: input };
  demoListener?.(demo);
}
