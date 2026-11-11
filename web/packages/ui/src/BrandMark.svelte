<script lang="ts">
  // The owner's SCYDE S in Ember: just the letter, no plate (DESIGN.md §2.4, §7).
  let { size = 'var(--space-24)', label = '' }: { size?: string; label?: string } = $props();
</script>

<svg
  class="mark"
  viewBox="0 0 1024 1024"
  style:width={size}
  style:height={size}
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
>
  <g transform="translate(512 512) scale(2.3267) translate(-263.5 -284)">
    <path d="M400 82 L251 82 C110 82 68 101 68 241 C68 328 160 382 160 382 L243 308.5 L214 242 Z" />
    <path d="M400 82 L251 82 C110 82 68 101 68 241 C68 328 160 382 160 382 L243 308.5 L214 242 Z" transform="rotate(180 263.5 284)" />
  </g>
</svg>

<style>
  .mark { flex: none; fill: var(--brand-mark); }
</style>
