<script lang="ts">
  import { BOOKMARKS_BAR, OTHER_BOOKMARKS, bridge, type Bookmark, type HistoryEntry } from '@scyde/bridge';
  import { t } from '@scyde/i18n';
  import { BrandMark, Button, Favicon, Icon, IconButton, Tabs, Toast, ago, applyAppearance, hostOf } from '@scyde/ui';

  const PAGE = 50;
  let tab = $state(location.hash === '#bookmarks' ? 'bookmarks' : 'history');
  let query = $state('');
  let entries = $state<HistoryEntry[]>([]);
  let more = $state(false);
  let folder = $state<{ id: number; title: string }[]>([{ id: BOOKMARKS_BAR, title: t('library.bookmarks.bar') }]);
  let items = $state<Bookmark[]>([]);
  let toast = $state('');

  const b = bridge();
  b.then((x) => x.call('settings.get', {}))
    .then((s) => applyAppearance((s.settings.appearance ?? {}) as never))
    .catch(() => (toast = t('error.bridge')));

  async function search(reset = true) {
    const r = await (await b).call('history.query', { query: query.trim() || undefined, limit: PAGE, offset: reset ? 0 : entries.length });
    entries = reset ? r.entries : [...entries, ...r.entries];
    more = r.entries.length === PAGE;
  }
  async function remove(url: string) {
    await (await b).call('history.delete', { url });
    entries = entries.filter((e) => e.url !== url);
  }
  async function clearAll() {
    await (await b).call('history.clear', {});
    entries = [];
    toast = t('library.history.cleared');
  }
  async function open(id: number) {
    items = (await (await b).call('bookmarks.children', { parent: id })).items;
  }
  async function removeBookmark(id: number) {
    await (await b).call('bookmarks.remove', { id });
    items = items.filter((i) => i.id !== id);
  }
  function enter(f: Bookmark) {
    folder = [...folder, { id: f.id, title: f.title }];
  }

  let timer: ReturnType<typeof setTimeout>;
  $effect(() => {
    void query;
    clearTimeout(timer);
    timer = setTimeout(() => search(), 120);
  });
  $effect(() => {
    open(folder.at(-1)!.id);
  });
  $effect(() => {
    history.replaceState(null, '', `#${tab}`);
  });
</script>

<main class="page">
  <header>
    <BrandMark size="var(--space-32)" />
    <h1>{t('library.title')}</h1>
  </header>

  <Tabs label={t('library.title')} bind:value={tab} tabs={[{ value: 'history', label: t('library.tab.history') }, { value: 'bookmarks', label: t('library.tab.bookmarks') }]} />

  {#if tab === 'history'}
    <div class="panel" id="panel-history" role="tabpanel" tabindex="0" aria-labelledby="tab-history">
      <div class="bar">
        <label class="search">
          <Icon name="search" />
          <span class="sr-only">{t('library.search')}</span>
          <input type="search" placeholder={t('library.search')} bind:value={query} autocomplete="off" />
        </label>
        {#if entries.length}<Button variant="danger" onclick={clearAll}>{t('library.history.clear')}</Button>{/if}
      </div>
      {#if entries.length}
        <ul class="list">
          {#each entries as e (e.url)}
            <li>
              <a href={e.url}>
                <Favicon url={e.url} />
                <span class="title">{e.title || hostOf(e.url)}</span>
                <span class="host">{hostOf(e.url)}</span>
                <time datetime={new Date(e.lastVisit * 1000).toISOString()}>{ago(e.lastVisit)}</time>
              </a>
              <IconButton icon="close" label={t('library.history.delete', { title: e.title || hostOf(e.url) })} onclick={() => remove(e.url)} />
            </li>
          {/each}
        </ul>
        {#if more}<Button variant="ghost" onclick={() => search(false)}>{t('library.history.more')}</Button>{/if}
      {:else}
        <p class="empty">{query ? t('library.history.no_results', { q: query }) : t('library.history.empty')}</p>
      {/if}
    </div>
  {:else}
    <div class="panel" id="panel-bookmarks" role="tabpanel" tabindex="0" aria-labelledby="tab-bookmarks">
      <nav class="crumbs" aria-label={t('library.tab.bookmarks')}>
        {#if folder.length > 1}
          <IconButton icon="back" label={t('library.bookmarks.up', { title: folder.at(-2)!.title })} onclick={() => (folder = folder.slice(0, -1))} />
        {/if}
        <span>{folder.map((f) => f.title).join(' / ')}</span>
        {#if folder.length === 1}
          <Button variant="ghost" onclick={() => (folder = [{ id: folder[0]!.id === BOOKMARKS_BAR ? OTHER_BOOKMARKS : BOOKMARKS_BAR, title: folder[0]!.id === BOOKMARKS_BAR ? t('library.bookmarks.other') : t('library.bookmarks.bar') }])}>
            {folder[0]!.id === BOOKMARKS_BAR ? t('library.bookmarks.other') : t('library.bookmarks.bar')}
          </Button>
        {/if}
      </nav>
      {#if items.length}
        <ul class="list">
          {#each items as i (i.id)}
            <li>
              {#if i.isFolder}
                <button class="row" onclick={() => enter(i)}><Icon name="folder" /><span class="title">{i.title}</span><Icon name="chevron" /></button>
              {:else}
                <a href={i.url}><Favicon url={i.url ?? ''} /><span class="title">{i.title}</span><span class="host">{hostOf(i.url ?? '')}</span></a>
              {/if}
              <IconButton icon="trash" label={t('library.bookmarks.remove', { title: i.title })} onclick={() => removeBookmark(i.id)} />
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">{t('library.bookmarks.empty')}</p>
      {/if}
    </div>
  {/if}
</main>
<Toast bind:message={toast} />

<style>
  .page { max-width: calc(var(--size-newtab-recent-width) * 2.4); margin: 0 auto; padding: var(--space-48) var(--space-20) var(--space-80); display: grid; gap: var(--space-20); }
  header { display: flex; align-items: center; gap: var(--space-12); }
  h1 { margin: 0; font-family: var(--font-family-brand); font-size: var(--type-headline-size); line-height: var(--type-headline-line-height); letter-spacing: var(--type-headline-tracking); font-weight: var(--font-weight-medium); }
  .panel { display: grid; gap: var(--space-16); }
  .bar { display: flex; gap: var(--space-12); align-items: center; }
  .search { flex: 1; display: flex; align-items: center; gap: var(--space-8); min-height: var(--size-hit-desktop); padding: 0 var(--space-16); border-radius: var(--radius-full); background: var(--surface-sunken); color: var(--text-tertiary); }
  .search:focus-within { outline: var(--size-focus-ring) solid var(--accent-ring); outline-offset: var(--size-focus-offset); }
  .search input { flex: 1; min-width: 0; border: 0; outline: 0; background: transparent; color: var(--text-primary); font-size: var(--type-body-lg-size); }
  .list { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; gap: var(--space-8); border-top: 1px solid var(--border-hairline); }
  li:last-child { border-bottom: 1px solid var(--border-hairline); }
  a, .row {
    flex: 1; min-width: 0; display: flex; align-items: center; gap: var(--space-12); min-height: var(--size-newtab-recent-row);
    color: var(--text-primary); text-decoration: none; border: 0; background: transparent; padding: 0; text-align: left; font-size: var(--type-body-lg-size);
  }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .host { font-family: var(--font-family-mono); font-size: var(--type-data-size); color: var(--accent-ink); white-space: nowrap; }
  time, .empty, .crumbs span { font-family: var(--font-family-mono); font-size: var(--type-caption-size); color: var(--text-tertiary); }
  .crumbs { display: flex; align-items: center; gap: var(--space-8); }
  .crumbs span { flex: 1; }
  @media (max-width: 560px) { .host { display: none; } }
</style>
