import { describe, expect, it } from 'vitest';
import { createBridge, detectTransport, isTrustedLocation, METHODS } from './index';
import { mockState, mockTransport } from './mock';

describe('bridge', () => {
  it('only trusts internal origins', () => {
    expect(isTrustedLocation({ protocol: 'scyde:', hostname: 'newtab' })).toBe(true);
    expect(isTrustedLocation({ protocol: 'moz-extension:', hostname: 'x' })).toBe(true);
    expect(isTrustedLocation({ protocol: 'http:', hostname: 'tauri.localhost' })).toBe(true);
    expect(isTrustedLocation({ protocol: 'https:', hostname: 'evil.example' })).toBe(false);
    expect(isTrustedLocation({ protocol: 'file:', hostname: '' })).toBe(false);
  });

  it('refuses methods outside the allowlist', async () => {
    const b = createBridge(mockTransport());
    await expect(b.call('fs.read' as never, {} as never)).rejects.toThrow('not allowed');
  });

  it('round-trips through the mock and surfaces host errors', async () => {
    const state = mockState();
    const b = createBridge(mockTransport(state));
    const s = await b.call('newtab.state', {});
    expect(s.recent.length).toBeGreaterThan(0);
    await b.call('navigate', { input: 'example.com' });
    expect(state.navigations).toEqual(['example.com']);
    await b.call('settings.set', { path: 'appearance.theme', value: 'dark' });
    expect((state.settings.appearance as any).theme).toBe('dark');
    const failing = createBridge({ name: 'x', call: async () => ({ ok: false, error: 'nope' }) });
    await expect(failing.call('history.clear', {})).rejects.toThrow('nope');
  });

  it('times out', async () => {
    const hung = createBridge({ name: 'x', call: () => new Promise(() => {}) }, 20);
    await expect(hung.call('history.clear', {})).rejects.toThrow('timed out');
  });

  it('detects host transports', () => {
    expect(detectTransport({} as any)).toBeNull();
    expect(detectTransport({ __TAURI_INTERNALS__: { invoke: async () => 1 } } as any)?.name).toBe('tauri');
    expect(detectTransport({ webkit: { messageHandlers: { scyde: { postMessage: async () => 1 } } } } as any)?.name).toBe('webkit');
    expect(detectTransport({ browser: { runtime: { sendNativeMessage: async () => 1 } } } as any)?.name).toBe('gecko');
  });

  it('gecko replies arrive as JSON text and are parsed', async () => {
    const t = detectTransport({ browser: { runtime: { sendNativeMessage: async () => '{"id":1,"ok":true,"result":{"ok":true}}' } } } as any);
    expect(await t?.call({ id: 1, method: 'navigate', params: {} })).toEqual({ id: 1, ok: true, result: { ok: true } });
    expect(METHODS).toContain('navigate');
  });
});
