<script lang="ts">
  // Single choice as a radiogroup with arrow-key navigation (WAI-ARIA radio group).
  let {
    value = $bindable(''),
    options,
    label,
    onchange,
  }: { value?: string; options: { value: string; label: string }[]; label: string; onchange?: (v: string) => void } = $props();

  function pick(v: string) {
    value = v;
    onchange?.(v);
  }
  function key(e: KeyboardEvent, i: number) {
    const d = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const next = options[(i + d + options.length) % options.length]!;
    pick(next.value);
    ((e.currentTarget as HTMLElement).parentElement?.children[options.indexOf(next)] as HTMLElement | undefined)?.focus();
  }
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o, i (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={value === o.value}
      tabindex={value === o.value ? 0 : -1}
      onclick={() => pick(o.value)}
      onkeydown={(e) => key(e, i)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; flex-wrap: wrap; gap: var(--space-2); padding: var(--space-2); border-radius: var(--radius-full); background: var(--surface-sunken); }
  button {
    min-height: calc(var(--size-hit-desktop) - var(--space-4)); padding: 0 var(--space-12);
    border: 0; border-radius: var(--radius-full); background: transparent; color: var(--text-secondary);
    font-size: var(--type-label-size);
    transition: background-color var(--motion-duration-fast) var(--motion-ease-out), color var(--motion-duration-fast) var(--motion-ease-out);
  }
  button[aria-checked='true'] { background: var(--surface-raised); color: var(--text-primary); box-shadow: var(--elev-1), inset 0 0 0 1px var(--border-strong); }
</style>
