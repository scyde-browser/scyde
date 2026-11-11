// Lookup over the bundle's cosmetic index (core/shield-compiler cosmetic.rs):
// generic.json {classes, ids, other, exceptions}, hosts.json {host: {hide, unhide, scriptlets, procedural}}.

/** example.co.uk → [example.co.uk, co.uk, uk] (every suffix may carry rules). */
export function hostChain(host) {
  const parts = host.split('.');
  return parts.map((_, i) => parts.slice(i).join('.'));
}

export function createIndex(generic, hosts) {
  function hostRules(host) {
    const hide = new Set();
    const unhide = new Set(generic.exceptions ?? []);
    for (const h of hostChain(host)) {
      const r = hosts[h];
      if (!r) continue;
      for (const s of r.hide ?? []) hide.add(s);
      for (const s of r.unhide ?? []) unhide.add(s);
    }
    return { hide, unhide };
  }
  return {
    /** Host-specific selectors plus generic selectors that are not keyed by class/id. */
    forHost(host) {
      const { hide, unhide } = hostRules(host);
      for (const s of generic.other ?? []) hide.add(s);
      return [...hide].filter((s) => !unhide.has(s));
    },
    /** Generic selectors keyed by classes/ids present on the page. */
    forClassesAndIds(host, classes, ids) {
      const { unhide } = hostRules(host);
      const out = [];
      for (const c of classes) for (const s of generic.classes?.[c] ?? []) if (!unhide.has(s)) out.push(s);
      for (const i of ids) for (const s of generic.ids?.[i] ?? []) if (!unhide.has(s)) out.push(s);
      return out;
    },
  };
}
