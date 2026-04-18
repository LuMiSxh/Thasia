<script lang="ts">
  import type { Snippet } from 'svelte';

  type Variant = 'default' | 'accent' | 'success' | 'warning' | 'danger' | 'mono';

  interface Props {
    variant?: Variant;
    class?: string;
    children: Snippet;
  }

  let { variant = 'default', class: className = '', children }: Props = $props();

  const variants: Record<Variant, string> = {
    default: 'bg-thasia-panel border-thasia-border text-thasia-muted',
    accent:  'bg-thasia-accent/10 border-thasia-accent/30 text-thasia-accent',
    success: 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400',
    warning: 'bg-amber-500/10 border-amber-500/30 text-amber-400',
    danger:  'bg-red-500/10 border-red-500/30 text-red-400',
    mono:    'bg-thasia-panel border-thasia-border text-thasia-text font-mono',
  };
</script>

<span class="inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-xs font-bold {variants[variant]} {className}">
  {@render children()}
</span>
