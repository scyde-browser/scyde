<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  // primary = Ember CTA (the only Ember action on a view), secondary = hairline, ghost = text.
  let {
    variant = 'secondary',
    loading = false,
    children,
    ...rest
  }: { variant?: 'primary' | 'secondary' | 'ghost' | 'danger'; loading?: boolean; children: Snippet } & HTMLButtonAttributes = $props();
</script>

<button class="btn {variant}" aria-busy={loading || undefined} disabled={rest.disabled || loading} {...rest}>
  {#if loading}<span class="spin" aria-hidden="true"></span>{/if}
  {@render children()}
</button>

<style>
  .btn {
    display: inline-flex; align-items: center; justify-content: center; gap: var(--space-8);
    min-height: var(--size-hit-desktop); padding: 0 var(--space-16);
    border: 0; border-radius: var(--radius-full);
    font-size: var(--type-label-size); line-height: var(--type-label-line-height); font-weight: var(--font-weight-medium);
    transition: transform var(--motion-duration-instant) var(--motion-ease-out), background-color var(--motion-duration-instant) var(--motion-ease-out);
  }
  .btn:active:not(:disabled) { transform: scale(var(--motion-press-scale)); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .primary { background: var(--cta-fill); color: var(--cta-text); }
  .primary:hover:not(:disabled) { background: var(--cta-fill-hover); }
  .secondary { background: transparent; color: var(--text-primary); box-shadow: inset 0 0 0 1px var(--border-strong); }
  .secondary:hover:not(:disabled) { background: var(--surface-sunken); }
  .ghost { background: transparent; color: var(--accent-ink); }
  .ghost:hover:not(:disabled) { background: var(--surface-sunken); }
  .danger { background: transparent; color: var(--status-danger); box-shadow: inset 0 0 0 1px var(--border-strong); }
  .danger:hover:not(:disabled) { background: var(--surface-sunken); }
  .spin { width: var(--space-12); height: var(--space-12); border-radius: var(--radius-full); border: var(--space-2) solid currentColor; border-right-color: transparent; animation: spin var(--motion-duration-slow) linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spin { animation: none; } }
</style>
