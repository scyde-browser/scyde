<script lang="ts">
  import { bridge, type RecentEntry, type ShieldToday } from '@scyde/bridge';
  import { t } from '@scyde/i18n';
  import { Favicon, Kbd, ago, applyAppearance, bytes, count, duration, hostOf } from '@scyde/ui';

  let shield = $state<ShieldToday | null>(null);
  let recent = $state<RecentEntry[]>([]);
  let showStats = $state(true);
  let error = $state('');
  let query = $state('');
  let input: HTMLInputElement | undefined = $state();

  async function load() {
    try {
      const b = await bridge();
      const [s, st] = await Promise.all([b.call('settings.get', {}), b.call('newtab.state', {})]);
      applyAppearance((s.settings.appearance ?? {}) as never);
      showStats = (s.settings.newtab as { shield_stats?: boolean } | undefined)?.shield_stats !== false;
      shield = st.shield;
      recent = st.recent;
    } catch {
      error = t('error.bridge');
    }
  }
  load();

  async function go(e: SubmitEvent) {
    e.preventDefault();
    const text = query.trim();
    if (!text) return;
    await (await bridge()).call('navigate', { input: text });
  }

  function onKey(e: KeyboardEvent) {
    // "/" focuses the question from anywhere; digits 1–6 open recent pages.
    if (e.target === input) return;
    if (e.key === '/') {
      e.preventDefault();
      input?.focus();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<main class="nt">
  <section class="main">
    {#if showStats}
      <p class="shield-line rise" style:--i="0" aria-label={t('newtab.shield_today')}>
        <span class="lamp" aria-hidden="true"></span>
        {#if shield}
          <span><b>{count(shield.blocked)}</b> {t('newtab.shield.blocked')}</span>
          <span><b>{bytes(shield.bytesSaved)}</b> {t('newtab.shield.saved')}</span>
          <span><b>{duration(shield.msSaved)}</b> {t('newtab.shield.time')}</span>
        {:else}
          <span>{t('newtab.shield_today')}</span>
        {/if}
      </p>
    {/if}

    <form class="ask rise" style:--i="1" onsubmit={go} role="search">
      <label for="q" class="sr-only">{t('newtab.ask.label')}</label>
      <input
        id="q"
        bind:this={input}
        bind:value={query}
        autocomplete="off"
        spellcheck="false"
        autocapitalize="off"
        enterkeyhint="go"
        placeholder="{t('newtab.ask.1')} {t('newtab.ask.2')}"
      />
    </form>

    <p class="hints rise" style:--i="2">
      <span><Kbd>↵</Kbd> {t('newtab.hint.enter')}</span>
      <span><Kbd>/</Kbd> {t('newtab.ask.label')}</span>
    </p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </section>

  <aside class="recent" aria-labelledby="recent-h">
    <h2 id="recent-h">{t('newtab.recent')}</h2>
    {#if recent.length}
      <ol>
        {#each recent as r, i (r.url)}
          <li class="rise" style:--i={i + 3}>
            <a href={r.url}>
              <Favicon url={r.url} />
              <span class="host">{hostOf(r.url)}</span>
              <time datetime={new Date(r.lastVisit * 1000).toISOString()}>{ago(r.lastVisit)}</time>
            </a>
          </li>
        {/each}
      </ol>
    {:else}
      <p class="empty">{t('newtab.recent.empty')}</p>
    {/if}
    <p class="off">{t('newtab.stories.off_short')}</p>
  </aside>
</main>

<style>
  .nt {
    min-height: 100dvh; display: grid; grid-template-columns: 1fr var(--size-newtab-recent-width); gap: var(--space-80);
    padding: var(--space-64) var(--space-80) calc(var(--size-island-height) + var(--space-80));
  }
  .main { display: flex; flex-direction: column; justify-content: center; min-width: 0; }
  .shield-line {
    margin: 0; display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-8) var(--space-20);
    font-family: var(--font-family-mono); font-size: var(--type-data-size); line-height: var(--type-data-line-height); color: var(--text-secondary);
  }
  .shield-line b { color: var(--text-primary); font-weight: var(--font-weight-medium); font-variant-numeric: tabular-nums; }
  .lamp { width: var(--space-8); height: var(--space-8); border-radius: var(--radius-full); background: var(--status-shield); box-shadow: 0 0 0 var(--space-4) color-mix(in oklab, var(--status-shield) 18%, transparent); }
  .ask { margin: var(--space-24) 0 0; }
  .ask input {
    width: 100%; padding: 0; border: 0; outline: 0; background: transparent; color: var(--text-primary);
    caret-color: var(--accent-ink);
    font-family: var(--font-family-brand); font-size: var(--type-hero-size); line-height: var(--type-hero-line-height);
    letter-spacing: var(--type-hero-tracking); font-weight: var(--font-weight-display); font-variation-settings: 'opsz' 96;
  }
  .ask input::placeholder { color: var(--text-primary); opacity: 1; }
  .ask input:focus::placeholder { color: var(--text-tertiary); }
  .ask:focus-within { border-radius: var(--radius-md); box-shadow: 0 var(--space-4) 0 calc(var(--space-4) * -1) var(--accent-ring); }
  .hints { margin: var(--space-24) 0 0; display: flex; flex-wrap: wrap; gap: var(--space-24); font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); }
  .error { color: var(--status-danger); }
  .recent { align-self: center; }
  .recent h2 { margin: 0 0 var(--space-12); font-family: var(--font-family-mono); font-size: var(--type-caption-size); font-weight: var(--font-weight-regular); color: var(--text-tertiary); }
  ol { list-style: none; margin: 0; padding: 0; }
  a {
    display: grid; grid-template-columns: var(--space-32) 1fr auto; align-items: center; gap: var(--space-8);
    height: var(--size-newtab-recent-row); border-top: 1px solid var(--border-hairline); color: var(--text-primary); text-decoration: none;
    font-size: var(--type-title-size); letter-spacing: var(--type-title-tracking);
    transition: transform var(--motion-duration-fast) var(--motion-ease-out);
  }
  li:last-child a { border-bottom: 1px solid var(--border-hairline); }
  a:hover { transform: translateX(calc(var(--space-4) * var(--motion-distance-scale))); }
  .host { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  time, .empty, .off { font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); font-variant-numeric: tabular-nums; }
  .off { margin-top: var(--space-16); }

  /* Phone (DESIGN §3: hero 54/56), island at the bottom */
  @media (max-width: 760px) {
    .nt { grid-template-columns: 1fr; gap: var(--space-40); padding: var(--space-48) var(--space-20) calc(var(--size-island-mobile-height) + var(--space-48)); }
    /* The question must fit one line on the narrowest phones. */
    .ask input { font-size: min(var(--type-display-size), 10.5vw); line-height: var(--type-display-line-height); }
    .recent { align-self: start; }
    a { font-size: var(--type-body-lg-size); }
  }
  /* Keyboard hints make no sense without a keyboard. */
  @media (hover: none) { .hints { display: none; } }
</style>
