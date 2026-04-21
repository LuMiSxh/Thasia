<script lang="ts">
    import type { Snippet } from 'svelte';
    interface Props {
        children: Snippet;
    }
    let { children }: Props = $props();
</script>

<kbd
    class="rounded border border-thasia-border bg-thasia-panel px-1.5 py-0.5 font-mono text-[10px] tracking-wider text-thasia-muted uppercase"
>
    {@render children()}
</kbd>
