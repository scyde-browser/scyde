// Bridge client for internal pages. Picks the host transport at runtime:
// Tauri (Windows), WKWebView message handler with reply (iOS), runtime.sendNativeMessage from
// the built-in extension (Android/GeckoView), or the in-memory mock (dev server, tests).
import { METHODS, type Method, type Params, type Result } from './api';

export * from './api';

export interface Transport {
  readonly name: string;
  call(message: { id: number; method: string; params: unknown }): Promise<unknown>;
}

export class BridgeError extends Error {}

/** Internal pages are served only from these schemes; elsewhere the bridge refuses to talk. */
export const ALLOWED_PROTOCOLS = ['scyde:', 'moz-extension:', 'http:', 'https:'] as const;
/** http(s) is accepted only for the Tauri custom-protocol hosts and the dev server. */
export const ALLOWED_HTTP_HOSTS = ['scyde.localhost', 'tauri.localhost', 'localhost', '127.0.0.1'];

export function isTrustedLocation(loc: Pick<Location, 'protocol' | 'hostname'>): boolean {
  if (loc.protocol === 'scyde:' || loc.protocol === 'moz-extension:') return true;
  if (loc.protocol === 'http:' || loc.protocol === 'https:') return ALLOWED_HTTP_HOSTS.includes(loc.hostname);
  return false;
}

type AnyWindow = Window & {
  __TAURI_INTERNALS__?: { invoke(cmd: string, args: unknown): Promise<unknown> };
  webkit?: { messageHandlers?: { scyde?: { postMessage(m: unknown): Promise<unknown> } } };
  browser?: { runtime?: { sendNativeMessage(app: string, m: unknown): Promise<unknown> } };
};

export function detectTransport(w: AnyWindow = window as AnyWindow): Transport | null {
  if (w.__TAURI_INTERNALS__) {
    const t = w.__TAURI_INTERNALS__;
    return { name: 'tauri', call: (m) => t.invoke('bridge', { message: m }) };
  }
  const wk = w.webkit?.messageHandlers?.scyde;
  if (wk) return { name: 'webkit', call: (m) => wk.postMessage(m) };
  const rt = w.browser?.runtime;
  if (rt?.sendNativeMessage) return { name: 'gecko', call: (m) => rt.sendNativeMessage('scyde', m) };
  return null;
}

export interface Bridge {
  readonly transport: string;
  call<M extends Method>(method: M, params: Params<M>): Promise<Result<M>>;
}

export function createBridge(transport: Transport, timeoutMs = 10_000): Bridge {
  let next = 1;
  return {
    transport: transport.name,
    async call(method, params) {
      if (!METHODS.includes(method)) throw new BridgeError(`method not allowed: ${String(method)}`);
      const id = next++;
      let timer: ReturnType<typeof setTimeout> | undefined;
      const timeout = new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(new BridgeError(`${method} timed out`)), timeoutMs);
      });
      try {
        const reply = (await Promise.race([transport.call({ id, method, params }), timeout])) as
          | { id?: number; ok: boolean; result?: unknown; error?: string }
          | undefined;
        if (!reply || reply.ok !== true) throw new BridgeError(reply?.error ?? `${method} failed`);
        return reply.result as never;
      } finally {
        clearTimeout(timer);
      }
    },
  };
}

let shared: Bridge | undefined;

/** The page's bridge. Uses the mock when no host is present (dev server, Playwright). */
export async function bridge(): Promise<Bridge> {
  if (shared) return shared;
  if (!isTrustedLocation(location)) throw new BridgeError('untrusted origin');
  const t = detectTransport();
  if (t) {
    shared = createBridge(t);
  } else {
    const { mockTransport } = await import('./mock');
    shared = createBridge(mockTransport());
  }
  return shared;
}
