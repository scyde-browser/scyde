# ADR-0005: UI biblioteka za desktop chrome i interne stranice

- **Status:** Prihvaćeno
- **Datum:** 2026-11-11
- **Odlučuje:** Claude Code (Faza 0.2)
- **Veza:** PLAN §8.1 (JS budgeti), §11 Faza 0.2, CLAUDE.md (arhitektonske napomene)

## Kontekst
Desktop chrome (tab strip, toolbar, omnibox, popoveri) i interne stranice (`scyde://newtab`, `settings`, `shields`, `library`) pišu se u web tehnologiji i rade u tri enginea: WebView2 (Chromium), GeckoView (Gecko) i WKWebView (WebKit).

Ograničenja:
- desktop chrome ≤ **80 KB gz** JS, New Tab ≤ **120 KB gz** (PLAN §8.1)
- New Tab mora biti prikazan ≤ 120 ms nakon Ctrl+T na Windowsu
- izgled je vlastiti (DESIGN.md "Aura"); generički izgled UI kitova nije prihvatljiv (R1, R4)
- animacije samo `transform`/`opacity`, kroz CSS, WAAPI i `linear()` springove (bez GSAP-a)
- skill `pick-ui-library` nije instaliran u ovom okruženju, pa je odluka donesena ručno prema istim kriterijima (zabilježeno u `docs/STATUS.md`)

## Opcije
| Opcija | Runtime (gz, približno) | Prednosti | Mane |
|---|---|---|---|
| **Svelte 5 (runes) + vlastite komponente** | **14,2 KB** izmjereno za cijelu mini aplikaciju (vidi dolje) | kompajlirano, fina reaktivnost bez virtualnog DOM-a, mali runtime, scoped CSS, dobar TS | manji ekosustav; runes su noviji API |
| Solid + vlastite komponente | ~7–9 KB | najmanji runtime, fina reaktivnost | JSX, manji ekosustav od Reacta; tim (Claude Code + vlasnik) već planira Svelte |
| Preact + vlastite komponente | ~4–5 KB + hooks | sitan, poznat API | VDOM diff na svakom tipkanju u omniboxu; signals su dodatak |
| React 19 + UI kit (npr. Radix/shadcn) | ~45–50 KB samo React + kit | ogroman ekosustav, gotove a11y primitive | troši ~60 % budgeta chromea prije prvog retka našeg koda; kit vuče prema generičkom izgledu |
| Lit (web components) | ~6 KB | standardni custom elementi | shadow DOM komplicira tokene, fokus i testove; više boilerplatea |

## Odluka
**Svelte 5 (runes) + TypeScript `strict` + vlastite komponente u `web/packages/ui`, bez UI kita.** Stilovi isključivo kroz CSS varijable iz `@scyde/tokens`. A11y obrasci (menu, listbox, dialog, tabs) pišu se prema WAI-ARIA APG i testiraju axe-om i Playwrightom u sva tri enginea.

## Posljedice
- Svaka komponenta ima sva stanja iz DESIGN §8 (default, hover, press, focus-visible, disabled, loading); to radimo sami, bez kita.
- `pnpm size` (size-limit ili ekvivalent) postaje obavezan job u `web.yml` od Faze 2; budget: desktop-chrome 80 KB gz, newtab 120 KB gz.
- Animacije: CSS transitions + WAAPI, springovi kao unaprijed generirani `linear()` (`tools/springs`). Bez animacijskih biblioteka.
- Ako Faza 2 pokaže da Svelte runtime + naš kod prelazi 60 KB gz za desktop chrome, ponovno otvaramo ovaj ADR (kandidat: Solid).

## Mjerenja / dokazi
Mini aplikacija (lista tabova s `$state`, `$derived`, `{#each}` s ključem, `bind:value`, `{#if}`, event handleri), Svelte 5.57.3 + Vite 8.3.8, produkcijski build:

```
dist/assets/index-*.js  36.57 kB │ gzip: 14.35 kB
```

To je ~18 % budgeta desktop chromea (80 KB gz) i ~12 % budgeta New Taba (120 KB gz), uz ostatak za naše komponente i logiku.
