<script lang="ts">
  // A switch (WAI-ARIA switch pattern). The label is the visible text; hint is the description.
  let {
    checked = $bindable(false),
    label,
    hint = '',
    disabled = false,
    onchange,
  }: { checked?: boolean; label: string; hint?: string; disabled?: boolean; onchange?: (v: boolean) => void } = $props();
  const id = `t${Math.random().toString(36).slice(2, 9)}`;
</script>

<div class="row">
  <span class="text">
    <span id="{id}-l">{label}</span>
    {#if hint}<span id="{id}-h" class="hint">{hint}</span>{/if}
  </span>
  <button
    type="button"
    role="switch"
    class="sw"
    aria-checked={checked}
    aria-labelledby="{id}-l"
    aria-describedby={hint ? `${id}-h` : undefined}
    {disabled}
    onclick={() => {
      checked = !checked;
      onchange?.(checked);
    }}
  ><span class="knob"></span></button>
</div>

<style>
  .row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-16); min-height: var(--size-hit-desktop); }
  .text { display: grid; gap: var(--space-2); }
  .hint { color: var(--text-secondary); font-size: var(--type-caption-size); line-height: var(--type-caption-line-height); }
  .sw {
    flex: none; position: relative; width: var(--space-40); height: var(--space-24);
    border: 0; border-radius: var(--radius-full); background: var(--border-strong); padding: 0;
    transition: background-color var(--motion-duration-fast) var(--motion-ease-out);
  }
  .sw[aria-checked='true'] { background: var(--accent-ink); }
  .sw:disabled { opacity: 0.5; cursor: not-allowed; }
  .knob {
    position: absolute; top: var(--space-2); left: var(--space-2);
    width: var(--space-20); height: var(--space-20); border-radius: var(--radius-full);
    background: var(--surface-raised); box-shadow: var(--elev-1);
    transition: transform var(--motion-spring-snappy-duration) var(--motion-spring-snappy);
  }
  .sw[aria-checked='true'] .knob { transform: translateX(var(--space-16)); }
</style>
