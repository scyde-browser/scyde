<script lang="ts">
  import { monogram } from './format';
  // Monogram in an Aura pastel, stable per host (DESIGN.md §7). Real favicons come from the host cache.
  let { url, src = '', size = 'var(--space-24)' }: { url: string; src?: string; size?: string } = $props();
  const m = $derived(monogram(url));
</script>

{#if src}
  <img class="fav" {src} alt="" style:width={size} style:height={size} />
{:else}
  <span class="fav" aria-hidden="true" style:width={size} style:height={size} style:--fill="var(--aura-{m.tone}-fill)">{m.letter}</span>
{/if}

<style>
  .fav {
    flex: none; display: inline-grid; place-items: center; border-radius: var(--radius-xs);
    background: var(--fill, var(--accent-fill)); color: var(--accent-on-fill);
    font-size: var(--type-micro-size); font-weight: var(--font-weight-medium); object-fit: cover;
  }
</style>
