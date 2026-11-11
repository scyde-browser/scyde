// Host → shard of the cosmetic host index (FNV-1a 32-bit, 256 shards). Used by the build and by
// the content script, so both sides must stay identical.
export const SHARDS = 256;
export function shardOf(host) {
  let h = 0x811c9dc5;
  for (let i = 0; i < host.length; i++) {
    h ^= host.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return (h % SHARDS).toString(16).padStart(2, '0');
}
