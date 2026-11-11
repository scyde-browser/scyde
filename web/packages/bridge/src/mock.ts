// In-memory host for the dev server and Playwright. Deterministic data, no network (rule 2).
import type { Bookmark, HistoryEntry, SiteShield } from './api';
import type { Transport } from './index';

const NOW = Math.floor(Date.now() / 1000);

function demoHistory(): HistoryEntry[] {
  const rows: [string, string, number, number][] = [
    ['https://www.index.hr/vijesti', 'Index.hr: vijesti', 4 * 60, 12],
    ['https://github.com/scyde-browser/scyde', 'scyde-browser/scyde', 22 * 60, 30],
    ['https://www.dnevnik.hr/', 'Dnevnik.hr', 58 * 60, 5],
    ['https://en.wikipedia.org/wiki/Island', 'Island · Wikipedia', 3 * 3600, 2],
    ['https://www.youtube.com/', 'YouTube', 5 * 3600, 40],
    ['https://news.ycombinator.com/', 'Hacker News', 26 * 3600, 9],
  ];
  return rows.map(([url, title, ago, visitCount]) => ({ url, title, lastVisit: NOW - ago, visitCount }));
}

export interface MockState {
  history: HistoryEntry[];
  bookmarks: Bookmark[];
  settings: Record<string, unknown>;
  sites: SiteShield[];
  navigations: string[];
}

export function mockState(): MockState {
  return {
    history: demoHistory(),
    bookmarks: [
      { id: 10, parent: 2, isFolder: false, url: 'https://github.com/', title: 'GitHub' },
      { id: 11, parent: 2, isFolder: true, url: null, title: 'News' },
      { id: 12, parent: 11, isFolder: false, url: 'https://www.index.hr/', title: 'Index' },
    ],
    settings: {
      version: 1,
      appearance: { theme: 'system', accent: 'ember', custom_accent: '#F2603F', density: 'comfortable', reduce_motion: 'system' },
      search: { engine: 'duckduckgo', custom_template: '', suggestions: true },
      shield: { enabled: true, level: 'standard', cookie_banners: true, filter_updates: true },
      privacy: { https_first: true, block_popups: true, clear_on_exit: false },
      newtab: { speed_dial: true, speed_dial_rows: 2, shield_stats: true, top_stories: false },
      startup: { open: 'restore' },
    },
    sites: [
      { host: 'index.hr', enabled: true, blocked: 214 },
      { host: 'youtube.com', enabled: true, blocked: 61 },
      { host: 'github.com', enabled: false, blocked: 0 },
    ],
    navigations: [],
  };
}

export function mockTransport(state: MockState = mockState(), schema: unknown = null): Transport {
  const handlers: Record<string, (p: any) => unknown> = {
    'newtab.state': () => ({
      shield: { blocked: 1284, bytesSaved: 48_300_000, msSaved: 312_000 },
      recent: state.history.slice(0, 6),
      topSites: state.history.slice(0, 8).map((h) => ({ url: h.url, title: h.title, pinned: false })),
      storiesEnabled: Boolean((state.settings.newtab as any)?.top_stories),
    }),
    navigate: ({ input }) => (state.navigations.push(input), { ok: true }),
    'settings.schema': () => ({ schema }),
    'settings.get': () => ({ settings: structuredClone(state.settings), issues: [] }),
    'settings.set': ({ path, value }) => {
      const keys = String(path).split('.');
      let o: any = state.settings;
      for (const k of keys.slice(0, -1)) o = o[k];
      o[keys.at(-1)!] = value;
      return { ok: true };
    },
    'settings.reset': () => ({ ok: true }),
    'history.query': ({ query, limit, offset }) => {
      const q = (query ?? '').toLowerCase();
      const hits = state.history.filter((h) => !q || h.url.toLowerCase().includes(q) || h.title.toLowerCase().includes(q));
      return { entries: hits.slice(offset, offset + limit) };
    },
    'history.delete': ({ url }) => ((state.history = state.history.filter((h) => h.url !== url)), { ok: true }),
    'history.clear': () => ((state.history = []), { ok: true }),
    'bookmarks.children': ({ parent }) => ({ items: state.bookmarks.filter((b) => b.parent === parent) }),
    'bookmarks.remove': ({ id }) => ((state.bookmarks = state.bookmarks.filter((b) => b.id !== id && b.parent !== id)), { ok: true }),
    'shield.summary': () => ({
      today: { blocked: 1284, bytesSaved: 48_300_000, msSaved: 312_000 },
      total: { blocked: 98_311, bytesSaved: 3_120_000_000, msSaved: 21_600_000 },
      filtersVersion: '20261111',
      sites: state.sites,
    }),
    'shield.setSite': ({ host, enabled }) => {
      const s = state.sites.find((x) => x.host === host);
      if (s) s.enabled = enabled;
      return { ok: true };
    },
  };
  return {
    name: 'mock',
    async call({ id, method, params }) {
      const h = handlers[method];
      if (!h) return { id, ok: false, error: `unknown method ${method}` };
      return { id, ok: true, result: h(params) };
    },
  };
}
