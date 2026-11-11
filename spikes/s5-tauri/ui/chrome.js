// Spike chrome: tab strip + custom caption buttons. Talks to the host only through invoke (chrome webview only).
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const tabsEl = document.getElementById('tabs');

function render(state) {
  tabsEl.innerHTML = '';
  state.titles.forEach((t, i) => {
    const el = document.createElement('div');
    el.className = 'tab';
    el.setAttribute('role', 'tab');
    el.setAttribute('aria-selected', String(i === state.active));
    el.textContent = t;
    el.addEventListener('mousedown', () => invoke('switch_tab', { index: i }));
    tabsEl.appendChild(el);
  });
  document.getElementById('omni').textContent = state.url || '';
}

listen('tabs', (e) => render(e.payload));
document.getElementById('min').onclick = () => invoke('win_minimize');
document.getElementById('max').onclick = () => invoke('win_toggle_maximize');
document.getElementById('close').onclick = () => invoke('win_close');
document.getElementById('pop').onclick = () => invoke('toggle_popover');
invoke('chrome_ready');
