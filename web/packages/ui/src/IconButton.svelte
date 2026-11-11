<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon, { type IconName } from './Icon.svelte';

  let { icon, label, ...rest }: { icon: IconName; label: string } & HTMLButtonAttributes = $props();
</script>

<button class="ib" aria-label={label} title={label} {...rest}><Icon name={icon} /></button>

<style>
  .ib {
    flex: none; display: grid; place-items: center;
    width: var(--size-hit-desktop); height: var(--size-hit-desktop);
    border: 0; border-radius: var(--radius-full); background: transparent; color: var(--text-secondary);
    transition: background-color var(--motion-duration-instant) var(--motion-ease-out), transform var(--motion-duration-instant) var(--motion-ease-out);
  }
  .ib:hover:not(:disabled) { background: var(--surface-sunken); color: var(--text-primary); }
  .ib:active:not(:disabled) { transform: scale(var(--motion-press-scale)); }
  .ib:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
