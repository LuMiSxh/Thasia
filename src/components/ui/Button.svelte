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
            'border-transparent shadow-md active:shadow-none ' +
            'bg-black hover:brightness-110 ' +
            'dark:bg-gold-metallic dark:hover:brightness-110 dark:bevel-dark',
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
    {#if variant === 'primary'}
        <span class="btn-primary-label inline-flex items-center gap-2">
            {@render children()}
        </span>
    {:else}
        {@render children()}
    {/if}
</button>

<style>
    /* Light mode: gradient text. -webkit-text-fill-color leaves SVG stroke/fill
     reading `color` (the flat accent), so icons stay visible. */
    .btn-primary-label {
        background-image: linear-gradient(135deg, #fde047 0%, #d4af37 40%, #a17f1a 100%);
        background-clip: text;
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        color: #b49354; /* flat accent — icons inherit this */
    }

    /* Dark mode: full reset. Black text, no gradient clip. */
    :global(.dark) .btn-primary-label {
        background-image: none;
        -webkit-text-fill-color: black;
        color: black;
    }
</style>
