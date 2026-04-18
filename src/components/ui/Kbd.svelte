<script lang="ts">
  import type { Snippet } from 'svelte';
  interface Props { children: Snippet; }
  let { children }: Props = $props();
</script>

<kbd class="font-mono text-[10px] uppercase border border-thasia-border bg-thasia-panel rounded px-1.5 py-0.5 text-thasia-muted tracking-wider">
  {@render children()}
</kbd>
