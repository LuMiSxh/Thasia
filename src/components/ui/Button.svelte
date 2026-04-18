<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: Size;
    children: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    class: className = '',
    children,
    ...rest
  }: Props = $props();

  const base =
    'inline-flex items-center justify-center gap-2 font-bold rounded-lg border transition-all duration-150 select-none cursor-pointer ' +
    'active:translate-y-px focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-thasia-accent focus-visible:ring-offset-2 focus-visible:ring-offset-thasia-surface ' +
    'disabled:opacity-40 disabled:pointer-events-none';

  const sizes: Record<Size, string> = {
    sm: 'px-3 py-1 text-xs',
    md: 'px-4 py-1.5 text-sm',
    lg: 'px-6 py-2 text-sm',
  };

  const variants: Record<Variant, string> = {
    primary:
      'border-transparent shadow-md ' +
      'bg-zinc-900 text-thasia-accent hover:bg-zinc-800 active:shadow-none ' +
      'dark:bg-gold-metallic dark:text-black dark:hover:brightness-110 dark:bevel-dark dark:border-transparent',
    secondary:
      'bg-thasia-bg border-thasia-border text-thasia-text ' +
      'hover:border-thasia-accent/50 hover:text-thasia-text active:shadow-none',
    ghost:
      'border-transparent bg-transparent text-thasia-muted ' +
      'hover:text-thasia-text hover:bg-thasia-panel active:shadow-none',
    danger:
      'border-transparent bg-red-600/10 text-red-400 border border-red-500/30 ' +
      'hover:bg-red-600/20 hover:border-red-500/50 active:shadow-none',
  };
</script>

<button class="{base} {sizes[size]} {variants[variant]} {className}" {...rest}>
  {@render children()}
</button>
