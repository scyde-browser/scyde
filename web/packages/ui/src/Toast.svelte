<script lang="ts">
  // One message, at most one action, 4 s (DESIGN.md §8). Announced politely.
  let { message = $bindable(''), action = undefined }: { message?: string; action?: { label: string; run: () => void } } = $props();
  $effect(() => {
    if (!message) return;
    const t = setTimeout(() => (message = ''), 4000);
    return () => clearTimeout(t);
  });
</script>

<div class="wrap" role="status" aria-live="polite">
  {#if message}
    <div class="toast rise">
      <span>{message}</span>
      {#if action}<button onclick={() => { action?.run(); message = ''; }}>{action.label}</button>{/if}
    </div>
  {/if}
</div>

<style>
  .wrap { position: fixed; left: 50%; bottom: var(--space-32); transform: translateX(-50%); z-index: 50; }
  .toast { display: flex; align-items: center; gap: var(--space-16); padding: var(--space-12) var(--space-20); border-radius: var(--radius-full); background: var(--island-bg); color: var(--island-text); box-shadow: var(--elev-island); }
  button { border: 0; background: transparent; color: var(--accent-island); font-weight: var(--font-weight-medium); min-height: var(--size-hit-desktop); }
</style>
