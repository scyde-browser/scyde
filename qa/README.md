# qa

Deterministic fixtures (PLAN §14.2):

- `adblock/`: network (6 blocked incl. subframe + 1 exception), cosmetic (generic, attribute, host-specific), result in `window.__scydeQa`
- `popup/`: window.open without a gesture, with a gesture, `_blank`, popunder, redirect chain, expired gesture
- `lists/fixture.txt`: exact rules for the fixture (every engine must produce the same result)
- `serve.mjs`: `node qa/serve.mjs 8787`
- `sites.md`: manual list of real sites
