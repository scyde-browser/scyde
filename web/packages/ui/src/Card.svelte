<script lang="ts">
  import type { Snippet } from 'svelte';
  let { title = '', children, id = undefined }: { title?: string; children: Snippet; id?: string } = $props();
</script>

<section class="card" aria-labelledby={title && id ? `${id}-t` : undefined}>
  {#if title}<h2 id={id ? `${id}-t` : undefined}>{title}</h2>{/if}
  {@render children()}
</section>

<style>
  .card { background: var(--surface-raised); border-radius: var(--radius-lg); box-shadow: var(--elev-1); padding: var(--space-20) var(--space-24); }
  h2 { margin: 0 0 var(--space-12); font-size: var(--type-title-size); line-height: var(--type-title-line-height); letter-spacing: var(--type-title-tracking); font-weight: var(--font-weight-medium); }
</style>
