<script lang="ts">
  let { lines = 3 }: { lines?: number } = $props();
</script>

<div class="sk" aria-hidden="true">
  {#each { length: lines } as _, i}<i style:width="{90 - i * 15}%"></i>{/each}
</div>

<style>
  .sk { display: grid; gap: var(--space-12); }
  i { display: block; height: var(--space-12); border-radius: var(--radius-xs); background: var(--surface-sunken); }
</style>
