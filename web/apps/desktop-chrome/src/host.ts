// Host calls for the island. Outside Tauri (dev server, Playwright) a small demo state is used.
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
}
export interface ChromeState {
  tabs: TabView[];
  active: number | null;
  appearance: Record<string, string> | null;
  filters: string;
}

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function action(action: string, args: { id?: number; input?: string } = {}): Promise<void> {
  if (!inTauri) return demoAction(action, args);
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('chrome_action', { action, id: args.id ?? null, input: args.input ?? null });
}

export async function suggest(input: string): Promise<string | null> {
  if (!inTauri) return 'index.hr'.startsWith(input) && input ? 'index.hr'.slice(input.length) : null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<string | null>('chrome_suggest', { input });
}

export async function connect(onState: (s: ChromeState) => void, onFocus: () => void): Promise<void> {
  if (!inTauri) {
    demoListener = onState;
    onState(demo);
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
let demo: ChromeState = {
  tabs: [
    { id: 1, title: 'Index.hr', url: 'https://www.index.hr/vijesti', display: d('index.hr', 'www.', '/vijesti'), loading: false, popups_blocked: 2 },
    { id: 2, title: 'GitHub', url: 'https://github.com/scyde-browser/scyde', display: d('github.com', '', '/scyde-browser/scyde'), loading: false, popups_blocked: 0 },
    { id: 3, title: 'New Tab', url: 'scyde://newtab', display: { ...d('newtab'), internal: true, scheme: 'scyde' }, loading: false, popups_blocked: 0 },
  ],
  active: 1,
  appearance: null,
  filters: '20261111',
};
let demoListener: ((s: ChromeState) => void) | undefined;
function demoAction(a: string, args: { id?: number }) {
  if (a === 'switch' && args.id) demo = { ...demo, active: args.id };
  if (a === 'close' && args.id) demo = { ...demo, tabs: demo.tabs.filter((t) => t.id !== args.id) };
  demoListener?.(demo);
}
