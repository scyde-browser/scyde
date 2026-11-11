<script lang="ts">
  // The island (DESIGN.md §8): mark (menu) · back · forward · reload | address · Shield pebble | tab stack · +
  import { t } from '@scyde/i18n';
  import { BrandMark, Favicon, Icon, applyAppearance } from '@scyde/ui';
  import { action, connect, suggest, type ChromeState, type TabView } from './host';

  let chrome = $state<ChromeState>({ tabs: [], active: null, appearance: null, filters: '' });
  let editing = $state(false);
  let menu = $state(false);
  let value = $state('');
  let input: HTMLInputElement | undefined = $state();

  const active = $derived(chrome.tabs.find((tab) => tab.id === chrome.active) ?? null);
  const shown = $derived(chrome.tabs.slice(-6));
  const hidden = $derived(Math.max(0, chrome.tabs.length - shown.length));

  connect(
    (s) => {
      chrome = s;
      if (s.appearance) applyAppearance(s.appearance as never);
    },
    () => startEdit(),
  );

  function startEdit() {
    menu = false;
    editing = true;
    value = active?.url === 'scyde://newtab' ? '' : (active?.url ?? '');
    queueMicrotask(() => {
      input?.focus();
      input?.select();
    });
  }

  function stopEdit(toPage = false) {
    editing = false;
    if (toPage) action('focus-page');
  }

  let typed = '';
  async function onInput(e: Event) {
    const ev = e as InputEvent;
    typed = value;
    if (ev.inputType?.startsWith('delete') || !value) return;
    const rest = await suggest(value);
    if (rest && value === typed && input) {
      value = typed + rest;
      queueMicrotask(() => input?.setSelectionRange(typed.length, value.length));
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      const text = value.trim();
      if (text) action(e.altKey ? 'go-new' : 'go', { input: text });
      stopEdit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      stopEdit(true);
    }
  }

  function open(page: string) {
    menu = false;
    action('open', { input: page });
  }

  function tabLabel(tab: TabView) {
    return tab.title || tab.display.domain || t('tab.new');
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && menu && (menu = false)} />

<div class="band">
  <nav class="island" class:swell={editing || menu} aria-label={t('island.label')}>
    <button class="ib mark" aria-label={t('island.menu')} aria-expanded={menu} onclick={() => ((menu = !menu), (editing = false))}>
      <BrandMark />
    </button>

    {#if menu}
      <div class="menu" role="menu" aria-label={t('menu.label')}>
        <button role="menuitem" onclick={() => ((menu = false), action('new'))}>{t('island.new_tab')}</button>
        <button role="menuitem" onclick={() => open('library')}>{t('library.tab.history')}</button>
        <button role="menuitem" onclick={() => open('library')}>{t('library.tab.bookmarks')}</button>
        <button role="menuitem" onclick={() => open('shields')}>{t('shield.label')}</button>
        <button role="menuitem" onclick={() => open('settings')}>{t('rail.settings')}</button>
      </div>
    {:else}
      <button class="ib" aria-label={t('nav.back')} title={t('nav.back')} onclick={() => action('back')}><Icon name="back" /></button>
      <button class="ib" aria-label={t('nav.forward')} title={t('nav.forward')} onclick={() => action('forward')}><Icon name="forward" /></button>
      <button class="ib" aria-label={t('nav.reload')} title={t('nav.reload')} onclick={() => action(active?.loading ? 'stop' : 'reload')}>
        {#if active?.loading}<Icon name="close" />{:else}<Icon name="reload" />{/if}
      </button>
      <span class="sep" aria-hidden="true"></span>

      {#if editing}
        <label class="addr focus">
          <span class="sr-only">{t('omnibox.placeholder')}</span>
          <input
            bind:this={input}
            bind:value
            oninput={onInput}
            onkeydown={onKey}
            onblur={() => stopEdit()}
            placeholder={t('omnibox.placeholder')}
            spellcheck="false"
            autocomplete="off"
          />
        </label>
      {:else}
        <button class="addr" onclick={startEdit} title={active?.url ?? ''}>
          {#if active && !(active.display.internal && active.display.domain === 'newtab')}
            {#if !active.display.internal && active.display.scheme === 'https'}<span class="lock" title={t('omnibox.secure')}><Icon name="lock" size="var(--space-16)" /></span>{/if}
            {#if active.display.internal}<span class="ghost">scyde://</span>{/if}
            {#if active.display.subdomain}<span class="ghost">{active.display.subdomain}</span>{/if}
            <span class="domain">{active.display.domain}</span>
            <span class="path">{active.display.rest}</span>
          {:else}
            <span class="ghost">{t('omnibox.placeholder')}</span>
          {/if}
        </button>
      {/if}

      <button class="pebble" title={t('island.shield', { n: active?.popups_blocked ?? 0 })} aria-label={t('island.shield', { n: active?.popups_blocked ?? 0 })} onclick={() => open('shields')}>
        <span class="lamp" aria-hidden="true"></span>
        <span class="num">{active?.popups_blocked ?? 0}</span>
      </button>
      <span class="sep" aria-hidden="true"></span>

      <div class="stack" role="tablist" aria-label={t('tabs.shelf')}>
        {#each shown as tab (tab.id)}
          <button
            role="tab"
            class="dot"
            class:on={tab.id === chrome.active}
            aria-selected={tab.id === chrome.active}
            aria-label={tabLabel(tab)}
            title={tabLabel(tab)}
            onclick={() => action('switch', { id: tab.id })}
            onauxclick={(e) => e.button === 1 && action('close', { id: tab.id })}
          >
            <Favicon url={tab.display.internal ? 'https://scyde' : tab.url} size="var(--size-island-tab-dot)" />
          </button>
        {/each}
        {#if hidden}<span class="more">{t('island.more_tabs', { n: hidden })}</span>{/if}
      </div>
      <button class="ib" aria-label={t('island.new_tab')} title={t('island.new_tab')} onclick={() => action('new')}><Icon name="plus" /></button>
    {/if}
  </nav>
</div>

<style>
  :global(html), :global(body) { height: 100%; overflow: hidden; background: var(--surface-canvas); }
  .band { height: 100dvh; display: grid; place-items: center; }
  .island {
    width: min(calc(100% - var(--space-24)), var(--size-island-width)); height: var(--size-island-height);
    display: flex; align-items: center; gap: var(--space-4); padding: 0 var(--space-8);
    border-radius: var(--radius-island); background: var(--island-bg); color: var(--island-text);
    box-shadow: var(--elev-island), inset 0 0 0 1px var(--island-border);
    transition: width var(--motion-spring-smooth-duration) var(--motion-spring-smooth);
  }
  .island.swell { width: min(calc(100% - var(--space-24)), var(--size-island-width-expanded)); }
  .island :global(:focus-visible) { outline-color: var(--accent-island); }
  .ib {
    flex: none; width: var(--size-island-button); height: var(--size-island-button); display: grid; place-items: center;
    border: 0; border-radius: var(--radius-full); background: transparent; color: var(--island-text-secondary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out), transform var(--motion-duration-instant) var(--motion-ease-out);
  }
  .ib:hover { background: var(--island-raised); color: var(--island-text); }
  .ib:active { transform: scale(var(--motion-press-scale)); }
  .sep { flex: none; width: 1px; height: var(--space-24); background: var(--island-hairline); margin: 0 var(--space-4); }
  .addr {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--space-4); height: var(--size-island-button);
    padding: 0 var(--space-12); border: 0; border-radius: var(--radius-full); background: transparent; color: var(--island-text);
    font-size: var(--type-address-size); line-height: var(--type-address-line-height); letter-spacing: var(--type-address-tracking);
    white-space: nowrap; overflow: hidden; text-align: left;
  }
  .addr:hover { background: var(--island-raised); }
  .addr.focus { background: var(--island-raised); box-shadow: inset 0 0 0 var(--size-focus-ring) var(--accent-island); }
  .addr input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: inherit; font: inherit; }
  .addr input::selection { background: var(--accent-island); color: var(--island-bg); }
  .addr input::placeholder { color: var(--island-text-secondary); }
  .domain { flex: none; }
  .ghost { color: var(--island-text-secondary); flex: none; }
  .path { min-width: 0; overflow: hidden; font-family: var(--font-family-mono); font-size: var(--type-data-size); color: var(--island-text-secondary); mask-image: linear-gradient(90deg, black 70%, transparent); }
  .lock { display: inline-flex; color: var(--island-text-secondary); margin-right: var(--space-4); }
  .pebble {
    flex: none; display: flex; align-items: center; gap: var(--space-8); height: calc(var(--size-island-button) - var(--space-4));
    padding: 0 var(--space-12); border: 0; border-radius: var(--radius-full); background: var(--island-raised); color: var(--island-text);
    font-family: var(--font-family-mono); font-size: var(--type-data-size);
  }
  .lamp { width: var(--space-8); height: var(--space-8); border-radius: var(--radius-full); background: var(--island-shield); box-shadow: 0 0 0 var(--space-4) color-mix(in oklab, var(--island-shield) 22%, transparent); }
  .stack { flex: none; display: flex; align-items: center; padding: 0 var(--space-6); }
  .dot { display: grid; padding: 0; margin-left: calc(var(--space-8) * -1); border: 0; border-radius: var(--radius-full); background: transparent; box-shadow: 0 0 0 var(--space-2) var(--island-bg); }
  .dot:first-child { margin-left: 0; }
  .dot :global(.fav) { border-radius: var(--radius-full); }
  .dot.on { box-shadow: 0 0 0 var(--space-2) var(--island-bg), 0 0 0 var(--space-4) var(--accent-island); position: relative; z-index: 1; }
  .more { margin-left: var(--space-8); font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--island-text-secondary); }
  .menu { flex: 1; display: flex; gap: var(--space-4); overflow-x: auto; scrollbar-width: none; }
  .menu button {
    flex: none; height: calc(var(--size-island-button) - var(--space-4)); padding: 0 var(--space-16); border: 0; border-radius: var(--radius-full);
    background: transparent; color: var(--island-text); font-size: var(--type-label-size);
  }
  .menu button:hover, .menu button:focus-visible { background: var(--island-raised); }
</style>
