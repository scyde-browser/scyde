// MV3 service worker for WebView2 (ADR-0003 option A). Network blocking is DNR (static
// rulesets in the manifest); this worker only answers cosmetic lookups.
import { createIndex } from './cosmetics.js';

let index;
async function load() {
  if (index) return index;
  const [generic, hosts] = await Promise.all(
    ['cosmetic/generic.json', 'cosmetic/hosts.json'].map((p) => fetch(chrome.runtime.getURL(p)).then((r) => r.json())),
  );
  return (index = createIndex(generic, hosts));
}

chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
  if (msg?.type === 'scyde.cosmetics') load().then((i) => reply({ hide: i.forHost(msg.host) }));
  else if (msg?.type === 'scyde.generic') load().then((i) => reply({ hide: i.forClassesAndIds(msg.host, msg.classes, msg.ids) }));
  else return false;
  return true; // async reply
});
