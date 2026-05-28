<script lang="ts">
    import { Alert, Button } from 'anasthasia';
    import DownloadProgress from '$components/download/DownloadProgress.svelte';
    import SeriesDetail from '$components/download/SeriesDetail.svelte';
    import type { ChapterMeta, SearchResult } from '$types/bindings';

    let {
        series,
        chapters,
        selectedIds,
        loadingChapters = false,
        downloading = false,
        convertAfter = false,
        detailError = '',
        onToggle,
        onSelectAll,
        onClearSelection,
        onConvertAfter,
        onDownload,
        onClose,
    }: {
        series: SearchResult;
        chapters: ChapterMeta[];
        selectedIds: Set<number>;
        loadingChapters?: boolean;
        downloading?: boolean;
        convertAfter?: boolean;
        detailError?: string;
        onToggle: (id: number) => void;
        onSelectAll: () => void;
        onClearSelection: () => void;
        onConvertAfter: (value: boolean) => void;
        onDownload: () => void;
        onClose: () => void;
    } = $props();
</script>

<div class="flex h-full min-h-0 flex-col gap-3">
    <div
        class="flex flex-shrink-0 items-center justify-between gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface px-4 py-3"
    >
        <div class="min-w-0">
            <div class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                Selection
            </div>
            <div class="truncate text-sm font-bold">{series.title}</div>
        </div>
        <Button size="sm" variant="ghost" onclick={onClose}>Close</Button>
    </div>

    {#if loadingChapters}
        <div
            class="flex min-h-80 flex-1 items-center justify-center rounded-xl border border-anasthasia-border bg-anasthasia-surface text-sm text-anasthasia-muted"
        >
            Loading chapters…
        </div>
    {:else}
        <SeriesDetail
            {series}
            chapters={detailError ? [] : chapters}
            {selectedIds}
            {downloading}
            {convertAfter}
            {onToggle}
            {onSelectAll}
            {onClearSelection}
            {onConvertAfter}
            {onDownload}
        />
        {#if detailError}
            <Alert variant="danger" title="Could not load chapters">{detailError}</Alert>
        {/if}
        <DownloadProgress />
    {/if}
</div>
