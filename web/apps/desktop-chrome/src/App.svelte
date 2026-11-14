<script lang="ts">
  // The Windows sidebar (ADR-0016): caption buttons · navigation · address and command bar ·
  // pinned tabs · the current Space's tabs · Spaces. One webview on the left of the main window.
  import { t } from '@scyde/i18n';
  import { BrandMark, Favicon, Icon, applyAppearance, type IconName } from '@scyde/ui';
  import { action, connect, suggestions, type ChromeState, type Hit, type Space, type TabView } from './host';

  let chrome = $state<ChromeState>({
    tabs: [], active: null, split: null, spaces: [], space: '', appearance: null, filters: '', maximized: false, collapsed: false,
  });
  let editing = $state(false);
  let value = $state('');
  let hits = $state<Hit[]>([]);
  let selected = $state(0);
  let renaming = $state(false);
  let spaceName = $state('');
  let input: HTMLInputElement | undefined = $state();

  const active = $derived(chrome.tabs.find((tab) => tab.id === chrome.active) ?? null);
  const pinned = $derived(chrome.tabs.filter((tab) => tab.pinned));
  const tabs = $derived(chrome.tabs.filter((tab) => !tab.pinned && tab.space === chrome.space));
  const space = $derived(chrome.spaces.find((s) => s.id === chrome.space) ?? null);

  connect(
    (s) => {
      chrome = s;
      if (s.appearance) applyAppearance(s.appearance as never);
    },
    () => startEdit(),
  );

  const spaceLabel = (s: Space | null) => s?.name || t('space.default');
  const spaceColor = (s: Space | null) => (s && s.color !== 'onyx' ? `var(--aura-${s.color}-fill)` : 'var(--text-primary)');
  const tabLabel = (tab: TabView) => tab.title || (tab.display.internal ? t('tab.new') : tab.display.domain) || t('tab.new');
  const inSplit = (id: number) => chrome.split?.includes(id) ?? false;

  // ---- command bar: commands, open tabs, history, then "go" ----
  interface Row { key: string; icon: IconName; label: string; detail?: string; run: () => void }

  const commands = $derived<Row[]>([
    { key: 'c-new', icon: 'plus', label: t('island.new_tab'), run: () => action('new') },
    { key: 'c-reopen', icon: 'reload', label: t('tab.reopen'), run: () => action('reopen') },
    ...(active ? [
      { key: 'c-pin', icon: 'pin' as const, label: t(active.pinned ? 'tab.unpin' : 'tab.pin'), run: () => action('pin', { id: active.id }) },
      { key: 'c-close', icon: 'close' as const, label: t('tab.close'), run: () => action('close', { id: active.id }) },
    ] : []),
    ...(chrome.split ? [{ key: 'c-unsplit', icon: 'split' as const, label: t('tab.unsplit'), run: () => action('split', { id: chrome.split![1] }) }] : []),
    { key: 'c-history', icon: 'clock', label: t('library.tab.history'), run: () => action('open', { input: 'library' }) },
    { key: 'c-bookmarks', icon: 'bookmark', label: t('library.tab.bookmarks'), run: () => action('open', { input: 'library#bookmarks' }) },
    { key: 'c-shield', icon: 'shield', label: t('shield.label'), run: () => action('open', { input: 'shields' }) },
    { key: 'c-settings', icon: 'settings', label: t('rail.settings'), run: () => action('open', { input: 'settings' }) },
    { key: 'c-space', icon: 'folder', label: t('space.new'), run: () => action('space-new', { input: t('space.new') }) },
    { key: 'c-sidebar', icon: 'sidebar', label: t('sidebar.hide'), run: () => action('sidebar') },
  ]);

  const rows = $derived.by<Row[]>(() => {
    const q = value.trim().toLowerCase();
    if (!q) return commands;
    const out: Row[] = [{ key: 'go', icon: 'search', label: t('omnibox.suggest.search', { q: value.trim() }), run: () => action('go', { input: value.trim() }) }];
    for (const tab of chrome.tabs) {
      if (tab.id !== chrome.active && (tab.title + tab.url).toLowerCase().includes(q)) {
        out.push({ key: `t-${tab.id}`, icon: 'external', label: tabLabel(tab), detail: t('omnibox.switch_to_tab'), run: () => action('switch', { id: tab.id }) });
      }
    }
    for (const h of hits) out.push({ key: `h-${h.url}`, icon: 'clock', label: h.title || h.url, detail: h.url, run: () => action('go', { input: h.url }) });
    for (const c of commands) if (c.label.toLowerCase().includes(q)) out.push(c);
    return out.slice(0, 12);
  });

  function startEdit() {
    editing = true;
    selected = 0;
    hits = [];
    value = !active || active.url === 'scyde://newtab' ? '' : active.url;
    action('hold');
    queueMicrotask(() => {
      input?.focus();
      input?.select();
    });
  }

  function stopEdit(toPage = false) {
    if (!editing) return;
    editing = false;
    action('release');
    if (toPage) action('focus-page');
  }

  async function onInput() {
    selected = 0;
    const typed = value;
    const found = typed.trim() ? await suggestions(typed.trim()) : [];
    if (value === typed) hits = found;
  }

  function run(row: Row | undefined) {
    stopEdit();
    row?.run();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      const text = value.trim();
      // Alt+Enter opens what was typed in a new tab; otherwise the highlighted row runs.
      if (e.altKey && text) {
        stopEdit();
        action('go-new', { input: text });
      } else run(rows[selected]);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const n = rows.length || 1;
      selected = (selected + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
    } else if (e.key === 'Escape') {
      e.preventDefault();
      stopEdit(true);
    }
  }

  function drag(e: MouseEvent) {
    if (e.button !== 0) return;
    action(e.detail === 2 ? 'win-max' : 'win-drag');
  }

  function startRename() {
    if (!space) return;
    spaceName = spaceLabel(space);
    renaming = true;
  }

  function endRename(save: boolean) {
    if (save && space && spaceName.trim()) action('space-rename', { input: `${space.id}\n${spaceName.trim()}` });
    renaming = false;
  }

  function onTabKey(e: KeyboardEvent, list: TabView[], i: number) {
    const next = list[e.key === 'ArrowDown' ? i + 1 : e.key === 'ArrowUp' ? i - 1 : -1];
    if (next) {
      e.preventDefault();
      action('switch', { id: next.id });
    } else if (e.key === 'Delete' && list[i]) action('close', { id: list[i].id });
  }
</script>

<aside class="side" aria-label={t('sidebar.label')}>
  <header class="top">
    <span class="mark"><BrandMark /></span>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="drag" onmousedown={drag}></div>
    <div class="caption">
      <button aria-label={t('window.minimize')} title={t('window.minimize')} onclick={() => action('win-min')}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5.5h10" /></svg>
      </button>
      <button aria-label={t(chrome.maximized ? 'window.restore' : 'window.maximize')} title={t(chrome.maximized ? 'window.restore' : 'window.maximize')} onclick={() => action('win-max')}>
        {#if chrome.maximized}
          <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M2.5 2.5V.5h7v7h-2M.5 2.5h7v7h-7z" /></svg>
        {:else}
          <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5 .5h9v9h-9z" /></svg>
        {/if}
      </button>
      <button class="close" aria-label={t('window.close')} title={t('window.close')} onclick={() => action('win-close')}>
        <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M.5 .5l9 9M9.5 .5l-9 9" /></svg>
      </button>
    </div>
  </header>

  <div class="nav">
    <button class="ib" aria-label={t('nav.back')} title={t('nav.back')} onclick={() => action('back')}><Icon name="back" /></button>
    <button class="ib" aria-label={t('nav.forward')} title={t('nav.forward')} onclick={() => action('forward')}><Icon name="forward" /></button>
    <button class="ib" aria-label={t('nav.reload')} title={t('nav.reload')} onclick={() => action(active?.loading ? 'stop' : 'reload')}>
      <Icon name={active?.loading ? 'close' : 'reload'} />
    </button>
    <span class="grow"></span>
    <button class="ib" aria-label={t('shield.label')} title={t('island.shield', { n: active?.popups_blocked ?? 0 })} onclick={() => action('open', { input: 'shields' })}>
      <span class="lamp" aria-hidden="true"></span>
    </button>
    <button class="ib" aria-label={t('sidebar.hide')} title={t('sidebar.hide')} onclick={() => action('sidebar')}><Icon name="sidebar" /></button>
  </div>

  {#if editing}
    <label class="addr focus">
      <span class="sr-only">{t('omnibox.placeholder')}</span>
      <Icon name="search" size="var(--space-16)" />
      <input
        bind:this={input}
        bind:value
        oninput={onInput}
        onkeydown={onKey}
        onblur={() => stopEdit()}
        placeholder={t('command.hint')}
        spellcheck="false"
        autocomplete="off"
        role="combobox"
        aria-expanded="true"
        aria-controls="results"
        aria-activedescendant={rows[selected] ? `r-${selected}` : undefined}
      />
    </label>
    <!-- Mouse down must not blur the input before the click runs the row. -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <ul class="results" id="results" role="listbox" onmousedown={(e) => e.preventDefault()}>
      {#each rows as row, i (row.key)}
        <li id="r-{i}" role="option" aria-selected={i === selected} class:on={i === selected}>
          <button tabindex="-1" onclick={() => run(row)} onmouseenter={() => (selected = i)}>
            <Icon name={row.icon} size="var(--space-16)" />
            <span class="text">
              <span class="title">{row.label}</span>
              {#if row.detail}<span class="detail">{row.detail}</span>{/if}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <button class="addr" onclick={startEdit} title={active?.url ?? ''}>
      {#if active && !(active.display.internal && active.display.domain === 'newtab')}
        {#if !active.display.internal && active.display.scheme === 'https'}<span class="lock" title={t('omnibox.secure')}><Icon name="lock" size="var(--space-16)" /></span>{/if}
        {#if active.display.internal}<span class="ghost">scyde://</span>{/if}
        {#if active.display.subdomain}<span class="ghost">{active.display.subdomain}</span>{/if}
        <span class="domain">{active.display.domain}</span>
        <span class="path">{active.display.rest}</span>
      {:else}
        <Icon name="search" size="var(--space-16)" />
        <span class="ghost">{t('omnibox.placeholder')}</span>
      {/if}
    </button>

    {#if pinned.length}
      <div class="pinned" role="tablist" aria-label={t('sidebar.pinned')}>
        {#each pinned as tab (tab.id)}
          <button
            role="tab"
            class="tile"
            class:on={tab.id === chrome.active}
            aria-selected={tab.id === chrome.active}
            aria-label={tabLabel(tab)}
            title={tabLabel(tab)}
            onclick={() => action('switch', { id: tab.id })}
            onauxclick={(e) => e.button === 1 && action('pin', { id: tab.id })}
            oncontextmenu={(e) => (e.preventDefault(), action('pin', { id: tab.id }))}
          >
            <Favicon url={tab.display.internal ? 'https://scyde' : tab.url} size="var(--space-20)" />
          </button>
        {/each}
      </div>
    {/if}

    <div class="space">
      <span class="dot" style:background={spaceColor(space)} aria-hidden="true"></span>
      {#if renaming}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="rename"
          bind:value={spaceName}
          aria-label={t('space.name')}
          maxlength="40"
          autofocus
          onblur={() => endRename(true)}
          onkeydown={(e) => (e.key === 'Enter' ? endRename(true) : e.key === 'Escape' ? endRename(false) : null)}
        />
      {:else}
        <button class="name" title={t('space.rename')} ondblclick={startRename}>{spaceLabel(space)}</button>
        {#if chrome.spaces.length > 1 && space}
          <button class="mini" aria-label={t('space.delete')} title={t('space.delete')} onclick={() => action('space-delete', { input: space.id })}>
            <Icon name="trash" size="var(--space-16)" />
          </button>
        {/if}
      {/if}
    </div>

    <button class="row new" onclick={() => action('new')}>
      <Icon name="plus" size="var(--space-16)" />
      <span class="title">{t('island.new_tab')}</span>
      <kbd>Ctrl T</kbd>
    </button>

    <div class="tabs" role="tablist" aria-orientation="vertical" aria-label={t('tabs.shelf')}>
      {#each tabs as tab, i (tab.id)}
        {@const on = tab.id === chrome.active}
        <div class="row tab" class:on class:split={inSplit(tab.id)} class:asleep={tab.asleep}>
          <button
            role="tab"
            class="pick"
            aria-selected={on}
            tabindex={on ? 0 : -1}
            title={tab.url}
            onclick={() => action('switch', { id: tab.id })}
            onauxclick={(e) => e.button === 1 && action('close', { id: tab.id })}
            onkeydown={(e) => onTabKey(e, tabs, i)}
          >
            {#if tab.loading}
              <span class="spin" aria-hidden="true"></span>
            {:else}
              <Favicon url={tab.display.internal ? 'https://scyde' : tab.url} size="var(--size-chrome-favicon)" />
            {/if}
            <span class="title">{tabLabel(tab)}</span>
          </button>
          <span class="acts">
            {#if !on || inSplit(tab.id)}
              <button class="mini" tabindex="-1" aria-label={t(inSplit(tab.id) ? 'tab.unsplit' : 'tab.split')} title={t(inSplit(tab.id) ? 'tab.unsplit' : 'tab.split')} onclick={() => action('split', { id: tab.id })}>
                <Icon name="split" size="var(--space-16)" />
              </button>
            {/if}
            <button class="mini" tabindex="-1" aria-label={t('tab.pin')} title={t('tab.pin')} onclick={() => action('pin', { id: tab.id })}>
              <Icon name="pin" size="var(--space-16)" />
            </button>
            <button class="mini" tabindex="-1" aria-label={t('tab.close')} title={t('tab.close')} onclick={() => action('close', { id: tab.id })}>
              <Icon name="close" size="var(--space-16)" />
            </button>
          </span>
        </div>
      {/each}
    </div>
  {/if}

  <footer class="foot">
    <div class="spaces" role="tablist" aria-label={t('spaces.label')}>
      {#each chrome.spaces as s (s.id)}
        <button
          role="tab"
          class="sp"
          class:on={s.id === chrome.space}
          aria-selected={s.id === chrome.space}
          aria-label={spaceLabel(s)}
          title={spaceLabel(s)}
          onclick={() => action('space', { input: s.id })}
        >
          <span class="dot" style:background={spaceColor(s)}></span>
        </button>
      {/each}
      <button class="ib" aria-label={t('space.new')} title={t('space.new')} onclick={() => action('space-new', { input: t('space.new') })}><Icon name="plus" size="var(--space-16)" /></button>
    </div>
    <span class="grow"></span>
    <button class="ib" aria-label={t('library.tab.history')} title={t('library.tab.history')} onclick={() => action('open', { input: 'library' })}><Icon name="clock" /></button>
    <button class="ib" aria-label={t('rail.settings')} title={t('rail.settings')} onclick={() => action('open', { input: 'settings' })}><Icon name="settings" /></button>
  </footer>
</aside>

<style>
  :global(html), :global(body) { height: 100%; overflow: hidden; background: var(--surface-chrome); user-select: none; }
  .side {
    height: 100dvh; display: flex; flex-direction: column; gap: var(--space-8); padding: 0 var(--space-8) var(--space-8);
    background: var(--surface-chrome); color: var(--text-primary); border-right: 1px solid var(--border-hairline);
  }
  button { font: inherit; }
  button:focus-visible, input:focus-visible { outline: var(--size-focus-ring) solid var(--accent-fill); outline-offset: -2px; }
  .grow { flex: 1; }

  .top { flex: none; display: flex; align-items: stretch; height: var(--size-chrome-tabstrip); margin: 0 calc(var(--space-8) * -1); }
  .mark { display: grid; place-items: center; padding: 0 var(--space-12); }
  .drag { flex: 1; }
  .caption { display: flex; }
  .caption button {
    width: var(--space-40); display: grid; place-items: center; border: 0; background: transparent; color: var(--text-primary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  .caption svg { width: var(--space-12); height: var(--space-12); fill: none; stroke: currentColor; stroke-width: 1; }
  .caption button:hover { background: var(--surface-sunken); }
  .caption .close:hover { background: var(--status-danger); color: var(--text-on-ember); }

  .nav, .foot { flex: none; display: flex; align-items: center; gap: var(--space-2); }
  .ib, .mini {
    flex: none; display: grid; place-items: center; border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--text-secondary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out), transform var(--motion-duration-instant) var(--motion-ease-out);
  }
  .ib { width: var(--size-hit-desktop); height: var(--size-hit-desktop); }
  .mini { width: var(--space-24); height: var(--space-24); border-radius: var(--radius-xs); }
  .ib:hover, .mini:hover { background: var(--surface-sunken); color: var(--text-primary); }
  .ib:active { transform: scale(var(--motion-press-scale)); }
  .lamp { width: var(--space-8); height: var(--space-8); border-radius: var(--radius-full); background: var(--status-shield); box-shadow: 0 0 0 var(--space-4) color-mix(in oklab, var(--status-shield) 22%, transparent); }

  .addr {
    flex: none; min-width: 0; display: flex; align-items: center; gap: var(--space-4); height: var(--space-40); padding: 0 var(--space-12);
    border: 0; border-radius: var(--radius-md); background: var(--surface-sunken); color: var(--text-primary);
    font-size: var(--type-label-size); white-space: nowrap; overflow: hidden; text-align: left;
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  button.addr:hover { background: var(--surface-raised); box-shadow: inset 0 0 0 1px var(--border-hairline); }
  .addr.focus { gap: var(--space-8); background: var(--surface-raised); box-shadow: inset 0 0 0 var(--size-focus-ring) var(--accent-fill); }
  .addr input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: inherit; font: inherit; user-select: text; }
  .addr input::placeholder { color: var(--text-tertiary); }
  .domain { flex: none; }
  .ghost { flex: none; color: var(--text-tertiary); }
  .path { min-width: 0; overflow: hidden; font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); mask-image: linear-gradient(90deg, black 70%, transparent); }
  .lock { display: inline-flex; color: var(--text-secondary); margin-right: var(--space-4); }

  .results { flex: 1; min-height: 0; margin: 0; padding: 0; list-style: none; overflow-y: auto; scrollbar-width: none; display: flex; flex-direction: column; gap: var(--space-2); }
  .results button {
    width: 100%; min-height: var(--space-40); display: flex; align-items: center; gap: var(--space-12); padding: var(--space-4) var(--space-12);
    border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--text-secondary); text-align: left;
  }
  .results .on button { background: var(--surface-raised); color: var(--text-primary); box-shadow: inset 0 0 0 1px var(--border-hairline); }
  .text { min-width: 0; display: flex; flex-direction: column; }
  .detail { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; font-size: var(--type-caption-size); color: var(--text-tertiary); }

  .pinned { flex: none; display: grid; grid-template-columns: repeat(auto-fill, minmax(var(--size-chrome-pinned-tile), 1fr)); gap: var(--space-6); }
  .tile {
    height: var(--size-chrome-pinned-tile); display: grid; place-items: center; border: 0; border-radius: var(--radius-md); background: var(--surface-sunken);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out), transform var(--motion-duration-instant) var(--motion-ease-out);
  }
  .tile:hover { background: var(--surface-raised); }
  .tile:active { transform: scale(var(--motion-press-scale)); }
  .tile.on { background: var(--surface-raised); box-shadow: inset 0 0 0 var(--size-focus-ring) var(--accent-fill); }

  .space { flex: none; display: flex; align-items: center; gap: var(--space-8); height: var(--space-24); padding: 0 var(--space-8); margin-top: var(--space-4); }
  .dot { flex: none; width: var(--space-8); height: var(--space-8); border-radius: var(--radius-full); }
  .name, .rename {
    flex: 1; min-width: 0; padding: 0; border: 0; background: transparent; color: var(--text-secondary); text-align: left;
    font-size: var(--type-caption-size); font-weight: var(--font-weight-medium); letter-spacing: var(--type-caption-tracking);
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  }
  .rename { user-select: text; color: var(--text-primary); }

  .row {
    flex: none; display: flex; align-items: center; height: var(--space-40); border-radius: var(--radius-sm); color: var(--text-secondary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  .row:hover { background: var(--surface-sunken); }
  .new { gap: var(--space-12); padding: 0 var(--space-12); border: 0; background: transparent; font-size: var(--type-label-size); text-align: left; }
  .new kbd { font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); }
  .tabs { flex: 1; min-height: 0; overflow-y: auto; scrollbar-width: none; display: flex; flex-direction: column; gap: var(--space-2); }
  .tab.on { background: var(--surface-raised); color: var(--text-primary); box-shadow: inset 0 0 0 1px var(--border-hairline); }
  .tab.split:not(.on) { box-shadow: inset 0 0 0 1px var(--border-strong); color: var(--text-primary); }
  .tab.asleep .pick { opacity: var(--motion-sleep-opacity); }
  .pick {
    flex: 1; min-width: 0; height: 100%; display: flex; align-items: center; gap: var(--space-12); padding: 0 var(--space-4) 0 var(--space-12);
    border: 0; background: transparent; color: inherit; font-size: var(--type-label-size); text-align: left;
  }
  .pick :global(.fav) { flex: none; }
  .title { flex: 1; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .acts { flex: none; display: none; align-items: center; gap: var(--space-2); padding-right: var(--space-8); }
  .tab:hover .acts, .tab:focus-within .acts { display: flex; }
  .spin {
    flex: none; width: var(--size-chrome-favicon); height: var(--size-chrome-favicon); border-radius: var(--radius-full);
    border: var(--space-2) solid var(--border-hairline); border-top-color: var(--accent-fill); animation: spin calc(var(--motion-duration-slow) * 3) linear infinite;
  }
  @keyframes spin { to { transform: rotate(1turn); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; } }

  .spaces { display: flex; align-items: center; gap: var(--space-2); min-width: 0; overflow: hidden; }
  .sp { flex: none; width: var(--size-hit-desktop); height: var(--size-hit-desktop); display: grid; place-items: center; border: 0; border-radius: var(--radius-sm); background: transparent; }
  .sp .dot { width: var(--space-12); height: var(--space-12); opacity: var(--motion-sleep-opacity); }
  .sp:hover { background: var(--surface-sunken); }
  .sp.on { background: var(--surface-raised); box-shadow: inset 0 0 0 1px var(--border-hairline); }
  .sp.on .dot { opacity: 1; }
</style>
