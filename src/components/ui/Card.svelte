<script lang="ts">
    import type { Snippet } from 'svelte';

    interface Props {
        class?: string;
        children: Snippet;
        /** Render as a clickable surface */
        onclick?: () => void;
    }

    let { class: className = '', children, onclick }: Props = $props();

    const base = 'bg-anasthasia-surface border border-anasthasia-border rounded-xl p-4';
    const interactive =
        'cursor-pointer hover:border-anasthasia-accent/40 transition-colors duration-150 active:translate-y-px';
</script>

{#if onclick}
    <button type="button" {onclick} class="{base} {interactive} {className} w-full text-left">
        {@render children()}
    </button>
{:else}
    <div class="{base} {className}">
        {@render children()}
    </div>
{/if}
