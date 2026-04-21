<script lang="ts">
    import type { Snippet } from 'svelte';

    interface Props {
        class?: string;
        children: Snippet;
    }

    let { class: className = '', children }: Props = $props();
</script>

<span class="text-xs font-bold tracking-wider text-thasia-muted uppercase {className}">
    {@render children()}
</span>
