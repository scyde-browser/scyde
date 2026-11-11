<script lang="ts">
  import { bridge, type SiteShield, type ShieldToday } from '@scyde/bridge';
  import { t } from '@scyde/i18n';
  import { BrandMark, Card, Stat, Toast, Toggle, applyAppearance, bytes, count, duration } from '@scyde/ui';

  let today = $state<ShieldToday | null>(null);
  let total = $state<ShieldToday | null>(null);
  let version = $state('');
  let sites = $state<SiteShield[]>([]);
  let toast = $state('');

  async function load() {
    try {
      const b = await bridge();
      const [s, sum] = await Promise.all([b.call('settings.get', {}), b.call('shield.summary', {})]);
      applyAppearance((s.settings.appearance ?? {}) as never);
      ({ today, total, sites } = sum);
      version = sum.filtersVersion;
    } catch {
      toast = t('error.bridge');
    }
  }
  load();

  async function toggle(host: string, enabled: boolean) {
    await (await bridge()).call('shield.setSite', { host, enabled });
  }
</script>

<main class="page">
  <header>
    <BrandMark size="var(--space-32)" />
    <h1>{t('shields.title')}</h1>
    {#if version}<span class="ver">{t('shields.filters', { version })}</span>{/if}
  </header>

  {#each [[t('shields.today'), today, 'today'], [t('shields.total'), total, 'total']] as const as [title, v, id] (id)}
    <Card {title} {id}>
      {#if v}
        <div class="stats" aria-live="polite">
          <Stat value={count(v.blocked)} label={t('shields.blocked')} />
          <Stat value={bytes(v.bytesSaved)} label={t('shields.data_saved')} />
          <Stat value={duration(v.msSaved)} label={t('shields.time_saved')} />
        </div>
      {/if}
    </Card>
  {/each}

  <Card title={t('shields.sites')} id="sites">
    {#if sites.length}
      <ul>
        {#each sites as s (s.host)}
          <li>
            <Toggle label={t('shields.site_toggle', { host: s.host })} hint={t('shield.blocked_count', { count: s.blocked })} checked={s.enabled} onchange={(v) => toggle(s.host, v)} />
          </li>
        {/each}
      </ul>
    {:else}
      <p class="empty">{t('shields.sites.empty')}</p>
    {/if}
  </Card>
</main>
<Toast bind:message={toast} />

<style>
  .page { max-width: calc(var(--size-newtab-recent-width) * 2); margin: 0 auto; padding: var(--space-48) var(--space-20) var(--space-80); display: grid; gap: var(--space-20); }
  header { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-12); margin-bottom: var(--space-12); }
  h1 { margin: 0; flex: 1; font-family: var(--font-family-brand); font-size: var(--type-headline-size); line-height: var(--type-headline-line-height); letter-spacing: var(--type-headline-tracking); font-weight: var(--font-weight-medium); }
  .ver { font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); }
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--space-16); border-top: 1px solid var(--border-hairline); }
  ul { list-style: none; margin: 0; padding: 0; }
  li + li { border-top: 1px solid var(--border-hairline); }
  .empty { margin: 0; color: var(--text-secondary); }
  @media (max-width: 520px) { .stats { grid-template-columns: 1fr 1fr; } }
</style>
