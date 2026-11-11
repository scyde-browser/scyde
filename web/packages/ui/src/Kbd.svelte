<script lang="ts">
  import type { Snippet } from 'svelte';
  let { children }: { children: Snippet } = $props();
</script>

<kbd>{@render children()}</kbd>

<style>
  kbd { font: inherit; font-family: var(--font-family-mono); color: var(--text-secondary); padding: 0 var(--space-6); border-radius: var(--radius-xs); box-shadow: inset 0 0 0 1px var(--border-hairline); }
</style>
