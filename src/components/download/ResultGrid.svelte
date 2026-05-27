<script lang="ts">
    import { IconBook2, IconChevronRight, IconPhotoOff } from '@tabler/icons-svelte';
    import type { SearchResult } from '$types/bindings';

    let {
        results,
        onPick,
    }: {
        results: SearchResult[];
        onPick: (series: SearchResult) => void;
    } = $props();

    let failedImages = $state<Set<number>>(new Set());

    function markImageFailed(id: number) {
        const next = new Set(failedImages);
        next.add(id);
        failedImages = next;
    }
</script>

<div class="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
    {#each results as result (result.id)}
        <button
            class="group overflow-hidden rounded-lg border border-anasthasia-border bg-anasthasia-surface text-left transition-colors duration-150 hover:border-anasthasia-accent/40 hover:bg-anasthasia-panel"
            onclick={() => onPick(result)}
        >
            <div class="relative aspect-[3/4] bg-anasthasia-bg">
                {#if result.thumbnail_url && !failedImages.has(result.id)}
                    <img
                        src={result.thumbnail_url}
                        alt=""
                        class="h-full w-full object-cover"
                        loading="lazy"
                        onerror={() => markImageFailed(result.id)}
                    />
                {:else}
                    <div class="flex h-full items-center justify-center px-4 text-center text-xs text-anasthasia-muted">
                        <IconPhotoOff size={22} />
                    </div>
                {/if}
                <div class="absolute right-2 top-2 rounded bg-black/65 px-1.5 py-0.5 text-[10px] font-bold uppercase text-white">
                    {result.initialized ? 'Loaded' : 'New'}
                </div>
            </div>
            <div class="flex min-h-24 flex-col justify-between gap-2 px-2.5 py-2.5">
                <div class="line-clamp-3 text-sm font-semibold leading-5">{result.title}</div>
                <div class="flex items-center justify-between gap-2 text-xs text-anasthasia-muted">
                    <span class="flex min-w-0 items-center gap-1">
                        <IconBook2 size={13} />
                        <span class="truncate">Manga #{result.id}</span>
                    </span>
                    <IconChevronRight size={14} class="shrink-0 transition-transform group-hover:translate-x-0.5" />
                </div>
            </div>
        </button>
    {/each}
</div>
