// The contract between internal pages and every host (Tauri/WebView2, WKWebView, GeckoView).
// Hosts implement exactly these methods; anything else is refused on both sides.

export interface ShieldToday {
  blocked: number;
  bytesSaved: number;
  msSaved: number;
}

export interface RecentEntry {
  url: string;
  title: string;
  /** Unix seconds */
  lastVisit: number;
}

export interface TopSite {
  url: string;
  title: string;
  pinned: boolean;
}

export interface HistoryEntry extends RecentEntry {
  visitCount: number;
}

export interface Bookmark {
  id: number;
  parent: number;
  isFolder: boolean;
  url: string | null;
  title: string;
}

export interface SiteShield {
  host: string;
  enabled: boolean;
  blocked: number;
}

export interface SettingsIssue {
  path: string;
  message: string;
}

/** Method name → [params, result]. */
export interface Api {
  'newtab.state': [Record<string, never>, { shield: ShieldToday; recent: RecentEntry[]; topSites: TopSite[]; storiesEnabled: boolean }];
  navigate: [{ input: string; newTab?: boolean }, { ok: boolean }];
  'settings.schema': [Record<string, never>, { schema: unknown }];
  'settings.get': [Record<string, never>, { settings: Record<string, unknown>; issues: SettingsIssue[] }];
  'settings.set': [{ path: string; value: unknown }, { ok: boolean; error?: string }];
  'settings.reset': [{ path: string }, { ok: boolean }];
  'history.query': [{ query?: string; limit: number; offset: number }, { entries: HistoryEntry[] }];
  'history.delete': [{ url: string }, { ok: boolean }];
  'history.clear': [Record<string, never>, { ok: boolean }];
  'bookmarks.children': [{ parent: number }, { items: Bookmark[] }];
  'bookmarks.remove': [{ id: number }, { ok: boolean }];
  'shield.summary': [Record<string, never>, { today: ShieldToday; total: ShieldToday; filtersVersion: string; sites: SiteShield[] }];
  'shield.setSite': [{ host: string; enabled: boolean }, { ok: boolean }];
}

export type Method = keyof Api;
export type Params<M extends Method> = Api[M][0];
export type Result<M extends Method> = Api[M][1];

/** The allowlist hosts and pages share (rule 6). */
export const METHODS: readonly Method[] = [
  'newtab.state',
  'navigate',
  'settings.schema',
  'settings.get',
  'settings.set',
  'settings.reset',
  'history.query',
  'history.delete',
  'history.clear',
  'bookmarks.children',
  'bookmarks.remove',
  'shield.summary',
  'shield.setSite',
];

/** Built-in bookmark folders (core/places). */
export const BOOKMARKS_BAR = 2;
export const OTHER_BOOKMARKS = 3;
