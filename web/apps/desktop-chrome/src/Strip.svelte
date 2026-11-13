<script lang="ts">
  // The tab strip at the top of the Windows window (ADR-0015, owner 2026-11-13): every tab with its
  // icon and title, +, a drag area and the caption buttons. Navigation and the address stay in the island.
  import { t } from '@scyde/i18n';
  import { Favicon, Icon, applyAppearance } from '@scyde/ui';
  import { action, connect, type ChromeState, type TabView } from './host';

  let chrome = $state<ChromeState>({ tabs: [], active: null, appearance: null, filters: '' });

  connect(
    (s) => {
      chrome = s;
      if (s.appearance) applyAppearance(s.appearance as never);
    },
    () => {},
  );

  function label(tab: TabView) {
    return tab.title || (tab.display.internal ? t('tab.new') : tab.display.domain) || t('tab.new');
  }

  function onTabKey(e: KeyboardEvent, i: number) {
    // Arrow keys move between tabs (WAI-ARIA tabs pattern); Delete closes.
    const next = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : -1;
    const target = chrome.tabs[next];
    if (target) {
      e.preventDefault();
      action('switch', { id: target.id });
      queueMicrotask(() => document.querySelectorAll<HTMLElement>('[role=tab]')[next]?.focus());
    } else if (e.key === 'Delete') {
      const tab = chrome.tabs[i];
      if (tab) action('close', { id: tab.id });
    }
  }

  function drag(e: MouseEvent) {
    if (e.button !== 0) return;
    if (e.detail === 2) action('win-max');
    else action('win-drag');
  }
</script>

<header class="strip">
  <div class="tabs" role="tablist" aria-label={t('tabs.shelf')}>
    {#each chrome.tabs as tab, i (tab.id)}
      {@const on = tab.id === chrome.active}
      <div class="tab" class:on>
        <button
          role="tab"
          class="pick"
          aria-selected={on}
          tabindex={on ? 0 : -1}
          title={tab.url}
          onclick={() => action('switch', { id: tab.id })}
          onauxclick={(e) => e.button === 1 && action('close', { id: tab.id })}
          onkeydown={(e) => onTabKey(e, i)}
        >
          {#if tab.loading}
            <span class="spin" aria-hidden="true"></span>
          {:else}
            <Favicon url={tab.display.internal ? 'https://scyde' : tab.url} size="var(--size-chrome-favicon)" />
          {/if}
          <span class="title">{label(tab)}</span>
        </button>
        <button class="x" aria-label={t('tab.close')} title={t('tab.close')} tabindex="-1" onclick={() => action('close', { id: tab.id })}>
          <Icon name="close" size="var(--space-12)" />
        </button>
      </div>
    {/each}
    <button class="add" aria-label={t('tab.new')} title={t('tab.new')} onclick={() => action('new')}>
      <Icon name="plus" size="var(--space-16)" />
    </button>
  </div>
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

<style>
  :global(html), :global(body) { height: 100%; overflow: hidden; background: var(--surface-chrome); user-select: none; }
  .strip { height: var(--size-chrome-tabstrip); display: flex; align-items: center; background: var(--surface-chrome); color: var(--text-primary); }
  .tabs { min-width: 0; flex: 0 1 auto; display: flex; align-items: center; gap: var(--space-2); padding: 0 var(--space-8); overflow: hidden; }
  .tab {
    position: relative; flex: 0 1 auto; width: var(--size-chrome-tab-max); min-width: var(--size-chrome-tab-min); height: var(--size-chrome-tab-height);
    display: flex; align-items: center; border-radius: var(--radius-sm); color: var(--text-secondary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  .tab:hover { background: var(--surface-sunken); }
  .tab.on { background: var(--surface-raised); color: var(--text-primary); box-shadow: inset 0 0 0 1px var(--border-hairline); }
  .pick {
    flex: 1; min-width: 0; height: 100%; display: flex; align-items: center; gap: var(--space-8); padding: 0 var(--space-4) 0 var(--space-12);
    border: 0; background: transparent; color: inherit; font-size: var(--type-label-size); text-align: left;
  }
  .pick :global(.fav) { flex: none; }
  .title { flex: 1; min-width: 0; overflow: hidden; white-space: nowrap; mask-image: linear-gradient(90deg, black calc(100% - var(--space-24)), transparent); }
  .x {
    flex: none; width: var(--size-hit-desktop); height: var(--size-hit-desktop); display: grid; place-items: center;
    border: 0; border-radius: var(--radius-xs); background: transparent; color: var(--text-tertiary); opacity: 0;
    transition: opacity var(--motion-duration-instant) var(--motion-ease-out);
  }
  .tab:hover .x, .tab.on .x, .x:focus-visible { opacity: 1; }
  .x:hover { background: var(--surface-sunken); color: var(--text-primary); }
  .spin {
    flex: none; width: var(--size-chrome-favicon); height: var(--size-chrome-favicon); border-radius: var(--radius-full);
    border: var(--space-2) solid var(--border-hairline); border-top-color: var(--accent-fill); animation: spin calc(var(--motion-duration-slow) * 3) linear infinite;
  }
  @keyframes spin { to { transform: rotate(1turn); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; } }
  .add {
    flex: none; width: var(--size-hit-desktop); height: var(--size-hit-desktop); display: grid; place-items: center;
    border: 0; border-radius: var(--radius-sm); background: transparent; color: var(--text-secondary);
  }
  .add:hover { background: var(--surface-sunken); color: var(--text-primary); }
  .drag { flex: 1; align-self: stretch; min-width: var(--space-48); }
  .caption { flex: none; display: flex; align-self: stretch; }
  .caption button {
    width: var(--size-chrome-toolbar); height: 100%; display: grid; place-items: center; border: 0; background: transparent; color: var(--text-primary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  .caption svg { width: var(--space-12); height: var(--space-12); fill: none; stroke: currentColor; stroke-width: 1; }
  .caption button:hover { background: var(--surface-sunken); }
  .caption .close:hover { background: var(--status-danger); color: var(--text-on-ember); }
  button:focus-visible { outline: var(--size-focus-ring) solid var(--accent-fill); outline-offset: -2px; }
</style>
