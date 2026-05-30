<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { SvelteSet } from 'svelte/reactivity';
    import { Button, keyboard } from 'anasthasia';
    import { IconArrowLeft, IconArrowRight, IconPlus, IconRefresh } from '@tabler/icons-svelte';
    import { mountedHint } from '$lib/keyhint.svelte';

    let {
        onNext,
        onBack,
        nextDisabled = false,
        backDisabled = false,
    }: {
        onNext: () => void;
        onBack: () => void;
        nextDisabled?: boolean;
        backDisabled?: boolean;
    } = $props();

    let activeVolumeIndex = $state(0);

    let volumes = $derived(wizard.pageEdits);
    let activeEdits = $derived(wizard.pageEdits[activeVolumeIndex]?.pages ?? []);
    let firstNonExcluded = $derived(activeEdits.findIndex((e) => !e.excluded));

    let dragOverIndex = $state<number | null>(null);
    let gridEl: HTMLDivElement | undefined = $state();
    let scrollTop = $state(0);
    let viewportHeight = $state(0);
    let gridContentWidth = $state(0);
    let isScrolling = $state(false);
    let preview: {
        key: string;
        src: string;
        fileName: string;
        pageNumber: number;
        excluded: boolean;
        custom: boolean;
    } | null = $state(null);

    const minTileWidth = 90;
    const gridGap = 8;
    const fileNameHeight = 16;
    const overscanRows = 2;
    const hoverPreviewDelayMs = 1500;
    const scrollPreviewCooldownMs = 180;

    const loadedImages = new SvelteSet<string>();
    const failedImages = new SvelteSet<string>();
    let previewTimer: ReturnType<typeof setTimeout> | undefined;
    let scrollTimer: ReturnType<typeof setTimeout> | undefined;

    type PageEdit = (typeof activeEdits)[number];

    let virtualGridWidth = $derived(Math.max(minTileWidth, gridContentWidth));

    let columnCount = $derived(
        Math.max(1, Math.floor((virtualGridWidth + gridGap) / (minTileWidth + gridGap)))
    );
    let tileWidth = $derived(
        Math.max(minTileWidth, (virtualGridWidth - gridGap * (columnCount - 1)) / columnCount)
    );
    let tileHeight = $derived(tileWidth * 1.5 + fileNameHeight);
    let rowStride = $derived(tileHeight + gridGap);
    let totalRows = $derived(Math.ceil(activeEdits.length / columnCount));
    let virtualHeight = $derived(Math.max(0, totalRows * rowStride - gridGap));
    let startRow = $derived(Math.max(0, Math.floor(scrollTop / rowStride) - overscanRows));
    let endRow = $derived(
        Math.min(totalRows, Math.ceil((scrollTop + viewportHeight) / rowStride) + overscanRows)
    );
    let startIndex = $derived(startRow * columnCount);
    let endIndex = $derived(Math.min(activeEdits.length, endRow * columnCount));
    let visibleItems = $derived(
        activeEdits.slice(startIndex, endIndex).map((edit, offset) => ({
            edit,
            index: startIndex + offset,
        }))
    );

    function getSourcePage(edit: PageEdit): { url: string; file_name: string } {
        if (edit.customPath) {
            return {
                url: `thasia://image?path=${encodeURIComponent(edit.customPath)}`,
                file_name: edit.customPath.split('/').at(-1) ?? 'custom',
            };
        }
        if (edit.originalPageIndex !== null) {
            const volNum = edit.sourceVolumeNum ?? wizard.pageEdits[activeVolumeIndex]?.volumeNum;
            const srcVol = wizard.scanResult?.find((v) => v.volume_num === volNum);
            const page = srcVol?.pages[edit.originalPageIndex];
            return { url: page?.url ?? '', file_name: page?.file_name ?? '' };
        }
        return { url: '', file_name: '' };
    }

    let cleanupKb: (() => void) | undefined;
    let cleanupResize: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            [
                'arrowleft',
                (e) => {
                    e.preventDefault();
                    if (activeVolumeIndex > 0) activeVolumeIndex--;
                    return true;
                },
            ],
            [
                'arrowright',
                (e) => {
                    e.preventDefault();
                    if (activeVolumeIndex < volumes.length - 1) activeVolumeIndex++;
                    return true;
                },
            ],
            [
                'shift+arrowright',
                (e) => {
                    e.preventDefault();
                    onNext();
                    return true;
                },
            ],
            [
                'shift+arrowleft',
                (e) => {
                    if (backDisabled) return false;
                    e.preventDefault();
                    onBack();
                    return true;
                },
            ],
        ]);

        const updateGridMetrics = () => {
            if (!gridEl) return;
            const style = getComputedStyle(gridEl);
            const paddingLeft = parseFloat(style.paddingLeft) || 0;
            const paddingRight = parseFloat(style.paddingRight) || 0;
            scrollTop = gridEl.scrollTop;
            viewportHeight = gridEl.clientHeight;
            gridContentWidth = Math.max(0, gridEl.clientWidth - paddingLeft - paddingRight);
        };

        requestAnimationFrame(updateGridMetrics);
        if (typeof ResizeObserver !== 'undefined') {
            const resizeObserver = new ResizeObserver(updateGridMetrics);
            if (gridEl) resizeObserver.observe(gridEl);
            cleanupResize = () => resizeObserver.disconnect();
        } else {
            window.addEventListener('resize', updateGridMetrics);
            cleanupResize = () => window.removeEventListener('resize', updateGridMetrics);
        }
    });
    onDestroy(() => {
        cleanupKb?.();
        cleanupResize?.();
        clearPreviewTimer();
        clearScrollTimer();
    });

    let previousActiveVolumeIndex = -1;
    $effect(() => {
        if (activeVolumeIndex === previousActiveVolumeIndex) return;
        previousActiveVolumeIndex = activeVolumeIndex;
        scrollTop = 0;
        if (gridEl) gridEl.scrollTop = 0;
    });

    function pageKey(edit: PageEdit): string {
        if (edit.customPath) return 'custom:' + edit.customPath;
        return `${edit.sourceVolumeNum ?? 0}-${edit.originalPageIndex ?? 0}`;
    }

    function thumbnailKey(edit: PageEdit): string {
        return pageKey(edit);
    }

    function setActiveVolume(i: number) {
        activeVolumeIndex = i;
    }

    function toggleExclude(pageIndex: number) {
        const ve = wizard.pageEdits[activeVolumeIndex];
        if (!ve) return;
        const pages = ve.pages.map((p, i) =>
            i === pageIndex ? { ...p, excluded: !p.excluded } : p
        );
        const updated = [...wizard.pageEdits];
        updated[activeVolumeIndex] = { ...ve, pages };
        wizard.pageEdits = updated;
    }

    function onDragStart(e: DragEvent, i: number) {
        e.dataTransfer?.setData('application/page-drag', String(i));
        if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    }

    function onDragEnter(e: DragEvent, i: number) {
        e.preventDefault();
        e.stopPropagation();
        if (e.dataTransfer?.types.includes('application/page-drag')) dragOverIndex = i;
    }

    function onDragOver(e: DragEvent) {
        e.preventDefault();
        e.stopPropagation();
        if (e.dataTransfer?.types.includes('application/page-drag'))
            e.dataTransfer.dropEffect = 'move';
    }

    function onDragLeave(e: DragEvent) {
        e.preventDefault();
        e.stopPropagation();
        dragOverIndex = null;
    }

    function onDrop(e: DragEvent, targetIndex: number) {
        e.preventDefault();
        e.stopPropagation();
        dragOverIndex = null;
        if (!e.dataTransfer?.types.includes('application/page-drag')) return;
        const fromIndex = parseInt(e.dataTransfer.getData('application/page-drag'));
        if (isNaN(fromIndex) || fromIndex === targetIndex) return;

        const ve = wizard.pageEdits[activeVolumeIndex];
        if (!ve) return;
        const pages = [...ve.pages];
        const [moved] = pages.splice(fromIndex, 1);
        pages.splice(targetIndex, 0, moved);
        const updated = [...wizard.pageEdits];
        updated[activeVolumeIndex] = { ...ve, pages };
        wizard.pageEdits = updated;
    }

    async function addCustomImage() {
        const selected = await open({
            filters: [{ name: 'Image', extensions: ['jpg', 'jpeg', 'png', 'webp', 'avif'] }],
            multiple: false,
            title: 'Add custom image',
        });
        if (typeof selected !== 'string') return;

        const ve = wizard.pageEdits[activeVolumeIndex];
        if (!ve) return;
        const updated = [...wizard.pageEdits];
        updated[activeVolumeIndex] = {
            ...ve,
            pages: [
                {
                    originalPageIndex: null,
                    sourceVolumeNum: null,
                    customPath: selected,
                    excluded: false,
                },
                ...ve.pages,
            ],
        };
        wizard.pageEdits = updated;
    }

    function borderTone(i: number, edit: PageEdit): string {
        if (i === firstNonExcluded) return 'border-anasthasia-accent';
        if (dragOverIndex === i) return 'border-anasthasia-accent/50';
        if (edit.excluded) return 'border-red-500/60';
        if (edit.customPath) return 'border-anasthasia-accent-strong/60';
        return 'border-anasthasia-border';
    }

    function onGridScroll() {
        if (!gridEl) return;
        scrollTop = gridEl.scrollTop;
        isScrolling = true;
        hidePreview();
        clearScrollTimer();
        scrollTimer = setTimeout(() => {
            isScrolling = false;
            scrollTimer = undefined;
        }, scrollPreviewCooldownMs);
    }

    function markImageLoaded(key: string) {
        failedImages.delete(key);
        loadedImages.add(key);
    }

    function markImageFailed(key: string) {
        failedImages.add(key);
    }

    function clearPreviewTimer() {
        if (previewTimer) {
            clearTimeout(previewTimer);
            previewTimer = undefined;
        }
    }

    function clearScrollTimer() {
        if (scrollTimer) {
            clearTimeout(scrollTimer);
            scrollTimer = undefined;
        }
    }

    function schedulePreview(edit: PageEdit, pageIndex: number) {
        if (isScrolling) return;
        clearPreviewTimer();
        const src = getSourcePage(edit);
        const key = thumbnailKey(edit);
        previewTimer = setTimeout(() => {
            if (failedImages.has(key) || !src.url) return;
            if (isScrolling) return;
            preview = {
                key,
                src: src.url,
                fileName: src.file_name,
                pageNumber: pageIndex + 1,
                excluded: edit.excluded,
                custom: edit.customPath !== null,
            };
        }, hoverPreviewDelayMs);
    }

    function hidePreview() {
        clearPreviewTimer();
        preview = null;
    }
</script>

<div
    class="flex h-full gap-0"
    use:mountedHint={[
        ['arrowleft', 'Prev volume'],
        ['arrowright', 'Next volume'],
        ['shift+arrowright', 'Next step'],
        ...(!backDisabled ? [['shift+arrowleft', 'Back'] as [string, string]] : []),
    ]}
>
    <!-- Volume list -->
    <div class="flex w-44 flex-shrink-0 flex-col overflow-hidden border-r border-anasthasia-border">
        <div
            class="flex-shrink-0 border-b border-anasthasia-border px-3 py-2.5 text-[10px] font-bold tracking-wider text-anasthasia-muted uppercase"
        >
            Volumes
        </div>
        <div class="flex-1 overflow-y-auto p-2">
            {#each volumes as ve, i (ve)}
                <button
                    onclick={() => setActiveVolume(i)}
                    class="mb-1 w-full rounded-lg border px-3 py-2 text-left transition-colors duration-150
                           {i === activeVolumeIndex
                        ? 'border-anasthasia-accent/40 bg-anasthasia-accent/8 text-anasthasia-text'
                        : 'border-anasthasia-border bg-transparent text-anasthasia-muted hover:border-anasthasia-accent/25 hover:bg-anasthasia-panel hover:text-anasthasia-text'}"
                >
                    <div class="text-sm font-bold">Vol {ve.volumeNum}</div>
                    <div class="text-xs text-anasthasia-muted">
                        {ve.pages.filter((p) => !p.excluded).length} pages
                    </div>
                </button>
            {/each}
        </div>
    </div>

    <!-- Page grid -->
    <div class="relative flex flex-1 flex-col overflow-hidden">
        <!-- Toolbar -->
        <div
            class="flex flex-shrink-0 items-center gap-3 border-b border-anasthasia-border px-4 py-2.5"
        >
            <span class="text-sm font-bold">
                Volume {volumes[activeVolumeIndex]?.volumeNum ?? '—'}
            </span>
            <span class="text-xs text-anasthasia-muted">
                Drag to reorder · click to exclude · first image = cover
            </span>
            <Button onclick={addCustomImage} size="sm" class="ml-auto">
                <IconPlus size={13} /> Add image
            </Button>
        </div>

        <!-- Grid -->
        <div
            bind:this={gridEl}
            role="list"
            class="flex-1 overflow-y-auto px-3 py-3"
            onscroll={onGridScroll}
        >
            <div
                class="relative"
                style:height={`${virtualHeight}px`}
                style:width={`${virtualGridWidth}px`}
            >
                {#each visibleItems as item (pageKey(item.edit))}
                    {@const edit = item.edit}
                    {@const i = item.index}
                    {@const src = getSourcePage(edit)}
                    {@const key = thumbnailKey(edit)}
                    {@const row = Math.floor(i / columnCount)}
                    {@const col = i % columnCount}
                    <div
                        role="listitem"
                        draggable="true"
                        ondragstart={(e) => onDragStart(e, i)}
                        onpointerenter={() => schedulePreview(edit, i)}
                        onpointerleave={hidePreview}
                        onfocusin={() => schedulePreview(edit, i)}
                        onfocusout={hidePreview}
                        ondragenter={(e) => onDragEnter(e, i)}
                        ondragover={onDragOver}
                        ondragleave={onDragLeave}
                        ondrop={(e) => onDrop(e, i)}
                        class="absolute {edit.excluded ? 'opacity-40' : ''}"
                        style:width={`${tileWidth}px`}
                        style:transform={`translate(${col * (tileWidth + gridGap)}px, ${row * rowStride}px)`}
                    >
                        <button
                            onclick={() => toggleExclude(i)}
                            title={edit.excluded ? 'Click to include' : 'Click to exclude'}
                            class={`relative aspect-[2/3] w-full cursor-pointer overflow-hidden rounded-md border-2 bg-anasthasia-panel transition-opacity duration-150 ${borderTone(i, edit)} ${edit.excluded ? 'border-dashed' : 'border-solid'}`}
                        >
                            {#if !loadedImages.has(key) && !failedImages.has(key)}
                                <div
                                    class="absolute inset-0 animate-pulse bg-anasthasia-bg"
                                    aria-hidden="true"
                                ></div>
                            {/if}

                            {#if failedImages.has(key)}
                                <div class="absolute inset-0 flex items-center justify-center">
                                    <IconRefresh size={16} class="text-anasthasia-muted" />
                                </div>
                            {:else}
                                <img
                                    src={src.url}
                                    alt={src.file_name}
                                    draggable="false"
                                    class="h-full w-full object-cover transition-opacity duration-150 {loadedImages.has(
                                        key
                                    )
                                        ? 'opacity-100'
                                        : 'opacity-0'}"
                                    loading="lazy"
                                    decoding="async"
                                    onload={() => markImageLoaded(key)}
                                    onerror={() => markImageFailed(key)}
                                />
                            {/if}

                            {#if i === firstNonExcluded}
                                <div
                                    class="absolute top-2 left-1/2 -translate-x-1/2 -translate-y-1/2
                                       rounded-sm bg-anasthasia-accent px-1.5 py-px text-[8px] font-bold text-anasthasia-text"
                                >
                                    COVER
                                </div>
                            {/if}

                            {#if edit.customPath}
                                <div
                                    class="absolute left-1/2 -translate-x-1/2 -translate-y-1/2 rounded-sm
                                       bg-anasthasia-accent-strong px-1.5 py-px text-[8px] font-bold text-anasthasia-text"
                                    style:top={i === firstNonExcluded ? '24px' : '8px'}
                                >
                                    ADDED
                                </div>
                            {/if}

                            {#if edit.excluded}
                                <div class="absolute inset-0 flex items-center justify-center">
                                    <IconRefresh size={16} class="text-anasthasia-muted" />
                                </div>
                            {/if}
                        </button>

                        <div
                            class="mt-1 overflow-hidden text-center text-[8px] text-ellipsis whitespace-nowrap text-anasthasia-muted"
                        >
                            {src.file_name}
                        </div>
                    </div>
                {/each}
            </div>
        </div>

        {#if preview}
            <div
                class="pointer-events-none absolute inset-x-48 top-16 bottom-16 z-[120] flex items-center justify-center"
            >
                <div
                    class="flex max-h-full max-w-[min(42rem,70vw)] flex-col overflow-hidden rounded-lg border border-anasthasia-border bg-anasthasia-bg/95 shadow-2xl backdrop-blur"
                >
                    <div
                        class="flex flex-shrink-0 items-center gap-2 border-b border-anasthasia-border bg-anasthasia-surface px-3 py-2"
                    >
                        <span class="text-xs font-bold text-anasthasia-text">
                            Page {preview.pageNumber}
                        </span>
                        {#if preview.excluded}
                            <span
                                class="rounded-sm bg-red-500/15 px-1.5 py-0.5 text-[9px] font-bold text-red-400 uppercase"
                            >
                                excluded
                            </span>
                        {/if}
                        {#if preview.custom}
                            <span
                                class="rounded-sm bg-anasthasia-accent-strong/20 px-1.5 py-0.5 text-[9px] font-bold text-anasthasia-accent uppercase"
                            >
                                added
                            </span>
                        {/if}
                        <span class="ml-auto truncate text-[10px] text-anasthasia-muted">
                            {preview.fileName}
                        </span>
                    </div>
                    <div class="min-h-0 overflow-auto bg-black/20 p-3">
                        <img
                            src={preview.src}
                            alt={preview.fileName}
                            class="mx-auto max-h-[70vh] max-w-full rounded-sm object-contain"
                            decoding="async"
                        />
                    </div>
                </div>
            </div>
        {/if}

        <!-- Footer -->
        <div class="flex flex-shrink-0 gap-2 border-t border-anasthasia-border px-4 py-3">
            <Button onclick={onBack} disabled={backDisabled}>
                <IconArrowLeft size={15} /> Back
            </Button>
            <Button onclick={onNext} disabled={nextDisabled} class="ml-auto">
                Next <IconArrowRight size={15} />
            </Button>
        </div>
    </div>
</div>
