<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { flip } from 'svelte/animate';
    import { duration } from '$lib/transitions';
    import { Button } from '$components/ui/index';
    import { IconArrowLeft, IconArrowRight, IconPlus, IconRefresh } from '@tabler/icons-svelte';
    import { keyboard } from '$lib/keyboard';
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

    let cleanupKb: (() => void) | undefined;
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
    });
    onDestroy(() => cleanupKb?.());

    function pageKey(edit: (typeof activeEdits)[number]): string {
        if (edit.customPath) return 'custom:' + edit.customPath;
        return `${edit.sourceVolumeNum ?? 0}-${edit.originalPageIndex ?? 0}`;
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

    function imageUrl(edit: (typeof activeEdits)[number]): string {
        if (edit.customPath) return `thasia://image?path=${encodeURIComponent(edit.customPath)}`;
        if (edit.originalPageIndex !== null) {
            const srcVol = wizard.scanResult?.find(
                (v) =>
                    v.volume_num ===
                    (edit.sourceVolumeNum ?? wizard.pageEdits[activeVolumeIndex]?.volumeNum)
            );
            return srcVol?.pages[edit.originalPageIndex]?.url ?? '';
        }
        return '';
    }

    function fileName(edit: (typeof activeEdits)[number]): string {
        if (edit.customPath) return edit.customPath.split('/').at(-1) ?? 'custom';
        if (edit.originalPageIndex !== null) {
            const srcVol = wizard.scanResult?.find(
                (v) =>
                    v.volume_num ===
                    (edit.sourceVolumeNum ?? wizard.pageEdits[activeVolumeIndex]?.volumeNum)
            );
            return srcVol?.pages[edit.originalPageIndex]?.file_name ?? '';
        }
        return '';
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
    <div class="flex w-44 flex-shrink-0 flex-col overflow-hidden border-r border-thasia-border">
        <div
            class="flex-shrink-0 border-b border-thasia-border px-3 py-2.5 text-[10px] font-bold tracking-wider text-thasia-muted uppercase"
        >
            Volumes
        </div>
        <div class="flex-1 overflow-y-auto p-2">
            {#each volumes as ve, i (ve)}
                <button
                    onclick={() => setActiveVolume(i)}
                    class="mb-1 w-full rounded-lg border px-3 py-2 text-left transition-colors duration-150
                           {i === activeVolumeIndex
                        ? 'border-thasia-accent/40 bg-thasia-accent/8 text-thasia-text'
                        : 'border-thasia-border bg-transparent text-thasia-muted hover:border-thasia-accent/25 hover:bg-thasia-panel hover:text-thasia-text'}"
                >
                    <div class="text-sm font-bold">Vol {ve.volumeNum}</div>
                    <div class="text-xs text-thasia-muted">
                        {ve.pages.filter((p) => !p.excluded).length} pages
                    </div>
                </button>
            {/each}
        </div>
    </div>

    <!-- Page grid -->
    <div class="flex flex-1 flex-col overflow-hidden">
        <!-- Toolbar -->
        <div
            class="flex flex-shrink-0 items-center gap-3 border-b border-thasia-border px-4 py-2.5"
        >
            <span class="text-sm font-bold">
                Volume {volumes[activeVolumeIndex]?.volumeNum ?? '—'}
            </span>
            <span class="text-xs text-thasia-muted">
                Drag to reorder · click to exclude · first image = cover
            </span>
            <Button onclick={addCustomImage} size="sm" class="ml-auto">
                <IconPlus size={13} /> Add image
            </Button>
        </div>

        <!-- Grid -->
        <div
            role="list"
            class="flex-1 overflow-y-auto p-3"
            style="display:grid;grid-template-columns:repeat(auto-fill,minmax(90px,1fr));gap:8px;align-content:start;"
        >
            {#each activeEdits as edit, i (pageKey(edit))}
                <div
                    role="listitem"
                    draggable="true"
                    ondragstart={(e) => onDragStart(e, i)}
                    ondragenter={(e) => onDragEnter(e, i)}
                    ondragover={onDragOver}
                    ondragleave={onDragLeave}
                    ondrop={(e) => onDrop(e, i)}
                    animate:flip={{ duration: duration.slow }}
                    class="relative {edit.excluded ? 'opacity-40' : ''}"
                >
                    <!-- Image card — click anywhere to toggle exclude -->
                    <button
                        onclick={() => toggleExclude(i)}
                        title={edit.excluded ? 'Click to include' : 'Click to exclude'}
                        class="relative w-full overflow-hidden rounded-md transition-opacity duration-150"
                        style="
                            aspect-ratio: 2/3;
                            display: block;
                            border: 2px solid {i === firstNonExcluded
                            ? 'var(--accent)'
                            : dragOverIndex === i
                              ? 'color-mix(in srgb, var(--accent) 50%, transparent)'
                              : edit.excluded
                                ? '#ef4444'
                                : edit.customPath
                                  ? '#10b981'
                                  : 'var(--border)'};
                            border-style: {edit.excluded ? 'dashed' : 'solid'};
                            cursor: pointer;
                            background: var(--panel);
                        "
                    >
                        <img
                            src={imageUrl(edit)}
                            alt={fileName(edit)}
                            draggable="false"
                            class="h-full w-full object-cover"
                            loading="lazy"
                        />

                        {#if i === firstNonExcluded}
                            <div
                                class="absolute top-0 left-1/2 -translate-x-1/2 -translate-y-1/2
                                       rounded-sm px-1.5 py-px text-[8px] font-bold text-black"
                                style="background: var(--accent);"
                            >
                                COVER
                            </div>
                        {/if}

                        {#if edit.customPath}
                            <div
                                class="absolute left-1/2 -translate-x-1/2 -translate-y-1/2 rounded-sm
                                       bg-emerald-500 px-1.5 py-px text-[8px] font-bold text-white"
                                style="top: {i === firstNonExcluded ? '16px' : '0'};"
                            >
                                ADDED
                            </div>
                        {/if}

                        {#if edit.excluded}
                            <div class="absolute inset-0 flex items-center justify-center">
                                <IconRefresh size={16} class="text-thasia-muted" />
                            </div>
                        {/if}
                    </button>

                    <div
                        class="mt-1 overflow-hidden text-center text-[8px] text-ellipsis whitespace-nowrap text-thasia-muted"
                    >
                        {fileName(edit)}
                    </div>
                </div>
            {/each}
        </div>

        <!-- Footer -->
        <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-4 py-3">
            <Button onclick={onBack} disabled={backDisabled}
                ><IconArrowLeft size={15} /> Back</Button
            >
            <Button onclick={onNext} disabled={nextDisabled} class="ml-auto"
                >Next <IconArrowRight size={15} /></Button
            >
        </div>
    </div>
</div>
