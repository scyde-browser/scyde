# qa

Deterministički fixturei (PLAN §14.2):

- `adblock/`: mreža (6 blokiranih uklj. subframe + 1 iznimka), kozmetika (generička, atributna, host-specifična), rezultat u `window.__scydeQa`
- `popup/`: window.open bez geste, s gestom, `_blank`, popunder, redirect lanac, istekla gesta
- `lists/fixture.txt`: točna pravila za fixture (svaki engine mora dati isti rezultat)
- `serve.mjs`: `node qa/serve.mjs 8787`
- `sites.md`: ručni popis stvarnih stranica
