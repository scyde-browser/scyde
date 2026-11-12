<script lang="ts">
  // First run (PLAN Phase 6 onboarding): theme, search engine, Shield. Three screens, no dark
  // patterns: every step can be skipped, nothing is pre-selected that sends data anywhere.
  import schema from '../../../../core/settings/schema/settings.schema.json';
  import { bridge } from '@scyde/bridge';
  import { t, type MessageKey } from '@scyde/i18n';
  import { BrandMark, Button, Segmented, Toast, applyAppearance } from '@scyde/ui';

  type Enum = { enum?: string[] };
  const props = (schema as unknown as { properties: Record<string, { properties: Record<string, Enum> } | undefined> }).properties;
  const options = (section: string, key: string) => props[section]?.properties[key]?.enum ?? [];
  const themes = options('appearance', 'theme');
  const accents = options('appearance', 'accent').filter((a) => a !== 'custom');
  const engines = options('search', 'engine').filter((e) => e !== 'custom');
  const label = (key: string) => t(key as MessageKey);

  let step = $state(0);
  let theme = $state('system');
  let accent = $state('ember');
  let engine = $state('duckduckgo');
  let toast = $state('');

  async function load() {
    try {
      const r = await (await bridge()).call('settings.get', {});
      const s = r.settings as { appearance?: { theme?: string; accent?: string }; search?: { engine?: string } };
      theme = s.appearance?.theme ?? theme;
      accent = s.appearance?.accent ?? accent;
      engine = s.search?.engine ?? engine;
      applyAppearance((s.appearance ?? {}) as never);
    } catch {
      toast = t('error.bridge');
    }
  }
  load();

  async function set(path: string, value: unknown) {
    try {
      await (await bridge()).call('settings.set', { path, value });
    } catch {
      toast = t('error.bridge');
    }
  }

  function pickTheme(v: string) {
    theme = v;
    applyAppearance({ theme: v, accent } as never);
    set('appearance.theme', v);
  }
  function pickAccent(v: string) {
    accent = v;
    applyAppearance({ theme, accent: v } as never);
    set('appearance.accent', v);
  }
  function pickEngine(v: string) {
    engine = v;
    set('search.engine', v);
  }
  // The host closes the welcome sheet when this setting turns true.
  const finish = () => set('onboarding.completed', true);
</script>

<main class="page">
  <header>
    <BrandMark size="var(--space-32)" />
    <span class="step" aria-live="polite">{t('welcome.step', { n: step + 1 })}</span>
    <button type="button" class="skip" onclick={finish}>{t('welcome.skip')}</button>
  </header>

  {#key step}
    <section class="body rise" aria-labelledby="h{step}">
      {#if step === 0}
        <h1 id="h0">{t('welcome.theme.title')}</h1>
        <p>{t('welcome.theme.body')}</p>
        <Segmented
          label={label('settings.appearance.theme')}
          value={theme}
          options={themes.map((o) => ({ value: o, label: label(`settings.appearance.theme.${o}`) }))}
          onchange={pickTheme}
        />
        <div class="swatches" role="radiogroup" aria-label={label('settings.appearance.accent')}>
          {#each accents as a (a)}
            <button
              type="button"
              role="radio"
              class="swatch"
              data-accent={a}
              aria-checked={accent === a}
              aria-label={label(`settings.appearance.accent.${a}`)}
              onclick={() => pickAccent(a)}
            ><span>{label(`settings.appearance.accent.${a}`)}</span></button>
          {/each}
        </div>
      {:else if step === 1}
        <h1 id="h1">{t('welcome.search.title')}</h1>
        <p>{t('welcome.search.body')}</p>
        <ul class="engines" role="radiogroup" aria-labelledby="h1">
          {#each engines as e (e)}
            <li>
              <button type="button" role="radio" aria-checked={engine === e} onclick={() => pickEngine(e)}>
                <span>{label(`settings.search.engine.${e}`)}</span>
                <span class="dot" aria-hidden="true"></span>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <h1 id="h2">{t('welcome.shield.title')}</h1>
        <p>{t('welcome.shield.body')}</p>
        <ul class="facts">
          <li><span class="lamp" aria-hidden="true"></span>{t('welcome.shield.fact.ads')}</li>
          <li><span class="lamp" aria-hidden="true"></span>{t('welcome.shield.fact.private')}</li>
          <li><span class="lamp" aria-hidden="true"></span>{t('welcome.shield.fact.site')}</li>
        </ul>
      {/if}
    </section>
  {/key}

  <footer>
    {#if step > 0}
      <Button variant="ghost" onclick={() => step--}>{t('welcome.back')}</Button>
    {:else}
      <span></span>
    {/if}
    {#if step < 2}
      <Button variant="primary" onclick={() => step++}>{t('welcome.next')}</Button>
    {:else}
      <Button variant="primary" onclick={finish}>{t('welcome.done')}</Button>
    {/if}
  </footer>
</main>
<Toast bind:message={toast} />

<style>
  .page {
    min-height: 100dvh; display: grid; grid-template-rows: auto 1fr auto; gap: var(--space-24);
    max-width: calc(var(--size-newtab-recent-width) * 1.6); margin: 0 auto;
    padding: max(env(safe-area-inset-top), var(--space-24)) var(--space-24) max(env(safe-area-inset-bottom), var(--space-24));
  }
  header { display: flex; align-items: center; gap: var(--space-12); }
  .step { flex: 1; font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); }
  .skip { min-height: var(--size-hit-android); padding: 0 var(--space-12); border: 0; background: transparent; color: var(--text-secondary); font-size: var(--type-label-size); }
  .body { display: grid; align-content: center; gap: var(--space-20); }
  h1 {
    margin: 0; font-family: var(--font-family-brand); font-weight: var(--font-weight-display);
    font-size: calc(var(--type-hero-size) * 0.6); line-height: calc(var(--type-hero-line-height) * 0.62); letter-spacing: var(--type-hero-tracking);
  }
  p { margin: 0; color: var(--text-secondary); font-size: var(--type-body-lg-size); line-height: var(--type-body-lg-line-height); }
  .swatches { display: grid; grid-template-columns: repeat(auto-fill, minmax(calc(var(--space-80) + var(--space-16)), 1fr)); gap: var(--space-8); }
  .swatch {
    display: flex; align-items: flex-end; min-height: var(--space-64); padding: var(--space-8) var(--space-12); border: 0; border-radius: var(--radius-lg);
    background: var(--accent-fill); color: var(--accent-on-fill); font-size: var(--type-label-size); text-align: left;
  }
  .swatch[aria-checked='true'] { box-shadow: 0 0 0 var(--space-2) var(--surface-canvas), 0 0 0 var(--space-4) var(--accent-ring); }
  .engines { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border-hairline); }
  .engines button {
    display: flex; width: 100%; align-items: center; justify-content: space-between; min-height: var(--size-hit-android);
    border: 0; border-bottom: 1px solid var(--border-hairline); background: transparent; color: var(--text-primary); font-size: var(--type-body-lg-size);
  }
  .dot { width: var(--space-20); height: var(--space-20); border-radius: var(--radius-full); box-shadow: inset 0 0 0 1.5px var(--border-strong); }
  [aria-checked='true'] .dot { box-shadow: inset 0 0 0 var(--space-6) var(--accent-ring); }
  .facts { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--space-12); font-size: var(--type-body-size); }
  .facts li { display: flex; gap: var(--space-12); align-items: baseline; }
  .lamp { flex: none; width: var(--space-8); height: var(--space-8); border-radius: var(--radius-full); background: var(--status-shield); transform: translateY(calc(var(--space-2) * -1)); }
  footer { display: flex; justify-content: space-between; align-items: center; }
</style>
