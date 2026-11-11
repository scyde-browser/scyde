<script lang="ts" module>
  // Line icons, 1.5 px stroke on a 24 grid (DESIGN.md §7). Our own paths.
  export const ICONS = {
    search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14ZM20 20l-4-4',
    shield: 'M12 3l7 3v6c0 4.4-3 7.6-7 9-4-1.4-7-4.6-7-9V6l7-3Z',
    back: 'M15 5l-7 7 7 7',
    forward: 'M9 5l7 7-7 7',
    reload: 'M19 12a7 7 0 1 1-2.05-4.95M19 4v4h-4',
    plus: 'M12 5v14M5 12h14',
    close: 'M6 6l12 12M18 6L6 18',
    lock: 'M7 11V8a5 5 0 0 1 10 0v3M6 11h12v9H6z',
    clock: 'M12 7v5l3 2M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18Z',
    bookmark: 'M7 4h10v16l-5-3.5L7 20V4Z',
    folder: 'M3 7h7l2 2h9v10H3V7Z',
    trash: 'M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13',
    settings: 'M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6ZM12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M5.6 18.4l2.1-2.1M16.3 7.7l2.1-2.1',
    check: 'M5 12.5l4.5 4.5L19 7.5',
    chevron: 'M9 6l6 6-6 6',
    download: 'M12 4v11M7 10l5 5 5-5M5 20h14',
    moon: 'M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5Z',
    external: 'M14 4h6v6M20 4l-9 9M18 14v6H4V6h6',
  } as const;
  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name, size = 'var(--size-icon-desktop)' }: { name: IconName; size?: string } = $props();
</script>

<svg class="icon" viewBox="0 0 24 24" style:width={size} style:height={size} aria-hidden="true">
  <path d={ICONS[name]} />
</svg>

<style>
  .icon { flex: none; fill: none; stroke: currentColor; stroke-width: var(--size-icon-stroke); stroke-linecap: round; stroke-linejoin: round; }
</style>
