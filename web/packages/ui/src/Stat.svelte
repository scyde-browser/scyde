<script lang="ts">
  let { value, label }: { value: string; label: string } = $props();
</script>

<div class="stat">
  <b>{value}</b>
  <span>{label}</span>
</div>

<style>
  .stat { padding: var(--space-12) 0; }
  b { display: block; font-size: var(--type-headline-size); line-height: var(--type-headline-line-height); letter-spacing: var(--type-headline-tracking); font-weight: var(--font-weight-medium); font-variant-numeric: tabular-nums; }
  span { display: block; font-family: var(--font-family-mono); font-size: var(--type-caption-size); line-height: var(--type-caption-line-height); color: var(--text-secondary); }
</style>
