<script lang="ts">
  // Tabs with automatic activation and roving focus (WAI-ARIA tabs pattern).
  let {
    value = $bindable(''),
    tabs,
    label,
  }: { value?: string; tabs: { value: string; label: string }[]; label: string } = $props();
  function key(e: KeyboardEvent, i: number) {
    const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = (i + d + tabs.length) % tabs.length;
    value = tabs[n]!.value;
    document.getElementById(`tab-${value}`)?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label}>
  {#each tabs as t, i (t.value)}
    <button
      id="tab-{t.value}"
      role="tab"
      aria-selected={value === t.value}
      aria-controls="panel-{t.value}"
      tabindex={value === t.value ? 0 : -1}
      onclick={() => (value = t.value)}
      onkeydown={(e) => key(e, i)}>{t.label}</button>
  {/each}
</div>

<style>
  .tabs { display: flex; gap: var(--space-24); border-bottom: 1px solid var(--border-hairline); }
  button { position: relative; min-height: var(--size-hit-desktop); padding: 0; border: 0; background: transparent; color: var(--text-secondary); font-size: var(--type-body-lg-size); }
  button[aria-selected='true'] { color: var(--text-primary); }
  button[aria-selected='true']::after { content: ''; position: absolute; left: 0; right: 0; bottom: -1px; height: var(--space-2); background: var(--accent-ink); border-radius: var(--space-2); }
</style>
