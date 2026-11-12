// Global styles for internal pages. Tokens are imported from TypeScript (not via CSS @import):
// Vite resolves package exports the same way on every OS, CSS bare imports do not on Windows.
import '@scyde/tokens/tokens.css';
import './base.css';

// Inside a native sheet (phones) the sheet already shows the page's title: `?embedded` hides the
// page's own mark and heading so the title is not shown twice.
if (new URLSearchParams(location.search).has('embedded')) document.documentElement.dataset.embedded = '';
