<script lang="ts">
    import { Button, SegmentedControl } from 'anasthasia';
    import { IconBook2, IconDownload, IconSquare, IconSquareCheck } from '@tabler/icons-svelte';
    import type { ChapterMeta, SearchResult } from '$types/bindings';

    let {
        series,
        chapters,
        selectedIds,
        downloading = false,
        convertAfter = false,
        onToggle,
        onSelectAll,
        onClearSelection,
        onConvertAfter,
        onDownload,
    }: {
        series: SearchResult;
        chapters: ChapterMeta[];
        selectedIds: Set<number>;
        downloading?: boolean;
        convertAfter?: boolean;
        onToggle: (id: number) => void;
        onSelectAll: () => void;
        onClearSelection: () => void;
        onConvertAfter: (value: boolean) => void;
        onDownload: () => void;
    } = $props();

    let downloadedCount = $derived(chapters.filter((chapter) => chapter.downloaded).length);
    let scanlatorCount = $derived(
        new Set(chapters.map((chapter) => chapter.scanlator).filter(Boolean)).size
    );
    let downloadMode = $derived(convertAfter ? 'pipeline' : 'raw');
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
    <div
        class="flex flex-col gap-2 rounded-lg border border-anasthasia-border bg-anasthasia-surface p-3"
    >
        <div class="min-w-0">
            <h2 class="line-clamp-2 text-lg font-bold">{series.title}</h2>
            <p class="mt-1 text-xs text-anasthasia-muted">
                {scanlatorCount > 0
                    ? `${scanlatorCount} scanlator${scanlatorCount === 1 ? '' : 's'} · `
                    : ''}Download raw chapter images or send the selection through Thasia's
                pipeline.
            </p>
        </div>

        <div class="grid gap-3 md:grid-cols-[7.5rem_minmax(0,1fr)]">
            <div class="w-24 flex-none md:w-full">
                {#if series.thumbnail_url}
                    <img
                        src={series.thumbnail_url}
                        alt=""
                        class="aspect-[3/4] w-full rounded-lg border border-anasthasia-border object-cover"
                    />
                {:else}
                    <div
                        class="flex aspect-[3/4] w-full items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted"
                    >
                        <IconBook2 size={24} />
                    </div>
                {/if}
            </div>

            <div class="flex min-w-0 flex-1 flex-col gap-2 self-stretch">
                <div class="grid grid-cols-2 gap-1.5 sm:grid-cols-4">
                    <div
                        class="rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-1"
                    >
                        <div class="text-[9px] font-bold text-anasthasia-muted uppercase">ID</div>
                        <div class="truncate text-xs">{series.id}</div>
                    </div>
                    <div
                        class="rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-1"
                    >
                        <div class="text-[9px] font-bold text-anasthasia-muted uppercase">
                            Chapters
                        </div>
                        <div class="text-xs">{chapters.length}</div>
                    </div>
                    <div
                        class="rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-1"
                    >
                        <div class="text-[9px] font-bold text-anasthasia-muted uppercase">
                            Selected
                        </div>
                        <div class="text-xs">{selectedIds.size}</div>
                    </div>
                    <div
                        class="rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-1"
                    >
                        <div class="text-[9px] font-bold text-anasthasia-muted uppercase">
                            Downloaded
                        </div>
                        <div class="text-xs">{downloadedCount}</div>
                    </div>
                </div>

                <div
                    class="grid gap-2 rounded-md border border-anasthasia-border bg-anasthasia-bg px-2 py-2"
                >
                    <div class="flex min-w-0 flex-wrap items-center gap-2">
                        <Button
                            size="sm"
                            variant="secondary"
                            disabled={chapters.length === 0 || selectedIds.size === chapters.length}
                            onclick={onSelectAll}
                        >
                            <IconSquareCheck size={14} /> Select all
                        </Button>
                        <Button
                            size="sm"
                            variant="ghost"
                            disabled={selectedIds.size === 0}
                            onclick={onClearSelection}
                        >
                            <IconSquare size={14} /> Clear
                        </Button>
                    </div>
                    <div class="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center 2xl:grid-cols-1">
                        <SegmentedControl
                            class="min-w-0 [&_button]:flex-1 [&_button]:whitespace-nowrap"
                            options={[
                                { value: 'raw', label: 'Raw images' },
                                { value: 'pipeline', label: 'Thasia CBZ' },
                            ]}
                            value={downloadMode}
                            onchange={(value) => onConvertAfter(value === 'pipeline')}
                        />
                    <Button
                        size="sm"
                        variant="primary"
                        loading={downloading}
                        loadingLabel="Downloading…"
                        disabled={selectedIds.size === 0}
                        class="justify-center"
                        onclick={onDownload}
                    >
                        <IconDownload size={15} />
                        {convertAfter ? 'Continue to convert' : 'Download raw'}
                    </Button>
                    </div>
                </div>
            </div>
        </div>
    </div>

    <div
        class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-anasthasia-border bg-anasthasia-surface"
    >
        {#if chapters.length === 0}
            <div class="px-4 py-8 text-center text-sm text-anasthasia-muted">
                No chapters are available for this title from the selected source.
            </div>
        {:else}
            <div class="grid grid-cols-1 gap-px bg-anasthasia-border sm:grid-cols-2 2xl:grid-cols-1">
                {#each chapters as chapter (chapter.id)}
                    <label
                        class="flex min-w-0 cursor-pointer items-center justify-between gap-3 bg-anasthasia-surface px-3 py-2 transition-colors duration-150 hover:bg-anasthasia-panel"
                    >
                        <div class="min-w-0">
                            <div class="truncate text-sm font-medium">
                                {chapter.name || `Chapter ${chapter.chapter_number}`}
                            </div>
                            <div class="text-xs text-anasthasia-muted">
                                {chapter.scanlator || 'No scanlator'}{chapter.downloaded
                                    ? ' · downloaded'
                                    : ''}
                            </div>
                        </div>
                        <input
                            type="checkbox"
                            class="h-4 w-4 flex-shrink-0 accent-anasthasia-accent"
                            checked={selectedIds.has(chapter.id)}
                            onchange={() => onToggle(chapter.id)}
                        />
                    </label>
                {/each}
            </div>
        {/if}
    </div>
</div>
