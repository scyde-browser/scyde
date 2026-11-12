<script lang="ts">
  // Rendered from core/settings/schema/settings.schema.json, the same file the Rust core validates
  // against. Every change goes through the host (settings.set), which validates again.
  import schema from '../../../../core/settings/schema/settings.schema.json';
  import { bridge } from '@scyde/bridge';
  import { t, type MessageKey } from '@scyde/i18n';
  import { BrandMark, Card, Segmented, Toast, Toggle, applyAppearance } from '@scyde/ui';

  type Node = {
    type: string;
    enum?: string[];
    minimum?: number;
    maximum?: number;
    properties?: Record<string, Node>;
    'x-scyde-hidden'?: boolean;
    'x-scyde-options'?: number[];
  };
  // Hidden nodes (per-site lists, onboarding state) are managed elsewhere, not on this page.
  const shown = (n: Node) => !n['x-scyde-hidden'];
  const sections = Object.entries((schema as unknown as { properties: Record<string, Node> }).properties)
    .filter(([k, n]) => k !== 'version' && shown(n))
    .map(([k, n]) => [k, { ...n, properties: Object.fromEntries(Object.entries(n.properties ?? {}).filter(([, f]) => shown(f))) }] as const);

  let values = $state<Record<string, Record<string, unknown>>>({});
  let ready = $state(false);
  let toast = $state('');

  const label = (key: string) => t(key as MessageKey);
  const has = (key: string) => label(key) !== key;

  async function load() {
    try {
      const r = await (await bridge()).call('settings.get', {});
      values = r.settings as never;
      applyAppearance((values.appearance ?? {}) as never);
      if (r.issues.length) toast = t('settings.repaired');
      ready = true;
    } catch {
      toast = t('error.bridge');
    }
  }
  load();

  async function set(section: string, key: string, value: unknown) {
    const prev = values[section]?.[key];
    values[section] = { ...values[section], [key]: value };
    const r = await (await bridge()).call('settings.set', { path: `${section}.${key}`, value });
    if (!r.ok) {
      values[section] = { ...values[section], [key]: prev };
      toast = r.error ?? t('error.bridge');
      return;
    }
    if (section === 'appearance') applyAppearance(values.appearance as never);
    toast = t('settings.saved');
  }

  function visible(section: string, key: string): boolean {
    if (section === 'search' && key === 'custom_template') return values.search?.engine === 'custom';
    if (section === 'appearance' && key === 'custom_accent') return values.appearance?.accent === 'custom';
    return true;
  }
</script>

<main class="page">
  <header>
    <BrandMark size="var(--space-32)" />
    <h1>{t('settings.title')}</h1>
  </header>

  {#if ready}
    {#each sections as [section, node] (section)}
      <Card title={label(`settings.section.${section}`)} id={section}>
        <div class="fields">
          {#each Object.entries(node.properties ?? {}) as [key, field] (key)}
            {#if visible(section, key)}
              {@const path = `settings.${section}.${key}`}
              {@const hint = has(`${path}.hint`) ? label(`${path}.hint`) : ''}
              {@const value = values[section]?.[key]}
              <div class="field">
                {#if field.type === 'boolean'}
                  <Toggle label={label(path)} {hint} checked={Boolean(value)} onchange={(v) => set(section, key, v)} />
                {:else if field.enum && section === 'appearance' && key === 'accent'}
                  <span class="name" id="{section}-{key}">{label(path)}</span>
                  <div class="swatches" role="radiogroup" aria-labelledby="{section}-{key}">
                    {#each field.enum as opt (opt)}
                      <button
                        type="button"
                        role="radio"
                        class="swatch"
                        aria-checked={value === opt}
                        aria-label={label(`${path}.${opt}`)}
                        title={label(`${path}.${opt}`)}
                        data-accent={opt}
                        onclick={() => set(section, key, opt)}
                      ></button>
                    {/each}
                  </div>
                {:else if field.enum}
                  <span class="name" id="{section}-{key}">{label(path)}</span>
                  <Segmented
                    label={label(path)}
                    value={String(value)}
                    options={field.enum.map((o) => ({ value: o, label: label(`${path}.${o}`) }))}
                    onchange={(v) => set(section, key, v)}
                  />
                {:else if field.type === 'integer'}
                  <span class="name">{label(path)}</span>
                  {#if hint}<span class="hint">{hint}</span>{/if}
                  <Segmented
                    label={label(path)}
                    value={String(value)}
                    options={(
                      field['x-scyde-options'] ??
                      Array.from({ length: Math.min((field.maximum ?? 1) - (field.minimum ?? 0) + 1, 12) }, (_, i) => (field.minimum ?? 0) + i)
                    ).map((n) => ({ value: String(n), label: n === 0 && field['x-scyde-options'] ? t('settings.off') : String(n) }))}
                    onchange={(v) => set(section, key, Number(v))}
                  />
                {:else if key === 'custom_accent'}
                  <label class="name" for="{section}-{key}">{label(path)}</label>
                  <input id="{section}-{key}" type="color" value={String(value)} onchange={(e) => set(section, key, e.currentTarget.value.toUpperCase())} />
                {:else}
                  <label class="name" for="{section}-{key}">{label(path)}</label>
                  {#if hint}<span class="hint">{hint}</span>{/if}
                  <input
                    id="{section}-{key}"
                    class="text"
                    type="url"
                    inputmode="url"
                    spellcheck="false"
                    value={String(value ?? '')}
                    onchange={(e) => set(section, key, e.currentTarget.value.trim())}
                  />
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      </Card>
    {/each}
  {/if}
</main>
<Toast bind:message={toast} />

<style>
  .page { max-width: calc(var(--size-newtab-recent-width) * 2); margin: 0 auto; padding: var(--space-48) var(--space-20) var(--space-80); display: grid; gap: var(--space-20); }
  header { display: flex; align-items: center; gap: var(--space-12); margin-bottom: var(--space-12); }
  h1 { margin: 0; font-family: var(--font-family-brand); font-size: var(--type-headline-size); line-height: var(--type-headline-line-height); letter-spacing: var(--type-headline-tracking); font-weight: var(--font-weight-medium); }
  .fields { display: grid; }
  .field { display: grid; gap: var(--space-8); padding: var(--space-12) 0; border-top: 1px solid var(--border-hairline); }
  .field:first-child { border-top: 0; }
  .name { font-size: var(--type-body-size); }
  .hint { color: var(--text-secondary); font-size: var(--type-caption-size); }
  .text { min-height: var(--size-hit-desktop); padding: 0 var(--space-12); border: 0; border-radius: var(--radius-md); background: var(--surface-sunken); font-family: var(--font-family-mono); font-size: var(--type-data-size); }
  input[type='color'] { width: var(--size-hit-desktop); height: var(--size-hit-desktop); padding: 0; border: 0; border-radius: var(--radius-full); background: transparent; }
  .swatches { display: flex; flex-wrap: wrap; gap: var(--space-8); }
  .swatch {
    width: var(--size-hit-desktop); height: var(--size-hit-desktop); border: 0; border-radius: var(--radius-full);
    background: var(--accent-fill); box-shadow: inset 0 0 0 1px var(--border-hairline);
  }
  .swatch[data-accent='custom'] { background: conic-gradient(from 0deg, var(--aura-ember-fill), var(--aura-mint-fill), var(--aura-sky-fill), var(--aura-lavender-fill), var(--aura-ember-fill)); }
  .swatch[aria-checked='true'] { box-shadow: 0 0 0 var(--space-2) var(--surface-raised), 0 0 0 var(--space-4) var(--accent-ring); }
</style>
