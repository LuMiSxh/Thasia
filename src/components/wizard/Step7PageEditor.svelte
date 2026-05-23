<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { flip } from 'svelte/animate';
    import { Button, duration, keyboard } from 'anasthasia';
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

    type PageEdit = (typeof activeEdits)[number];

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

    function pageKey(edit: PageEdit): string {
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

    function borderTone(i: number, edit: PageEdit): string {
        if (i === firstNonExcluded) return 'border-anasthasia-accent';
        if (dragOverIndex === i) return 'border-anasthasia-accent/50';
        if (edit.excluded) return 'border-red-500/60';
        if (edit.customPath) return 'border-anasthasia-accent-strong/60';
        return 'border-anasthasia-border';
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
    <div class="flex flex-1 flex-col overflow-hidden">
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
            role="list"
            class="grid flex-1 grid-cols-[repeat(auto-fill,minmax(90px,1fr))] content-start gap-2 overflow-y-auto p-3"
        >
            {#each activeEdits as edit, i (pageKey(edit))}
                {@const src = getSourcePage(edit)}
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
                    <button
                        onclick={() => toggleExclude(i)}
                        title={edit.excluded ? 'Click to include' : 'Click to exclude'}
                        class={`relative aspect-[2/3] w-full cursor-pointer overflow-hidden rounded-md border-2 bg-anasthasia-panel transition-opacity duration-150 ${borderTone(i, edit)} ${edit.excluded ? 'border-dashed' : 'border-solid'}`}
                    >
                        <img
                            src={src.url}
                            alt={src.file_name}
                            draggable="false"
                            class="h-full w-full object-cover"
                            loading="lazy"
                        />

                        {#if i === firstNonExcluded}
                            <div
                                class="absolute top-0 left-1/2 -translate-x-1/2 -translate-y-1/2
                                       rounded-sm bg-anasthasia-accent px-1.5 py-px text-[8px] font-bold text-anasthasia-text"
                            >
                                COVER
                            </div>
                        {/if}

                        {#if edit.customPath}
                            <div
                                class="absolute left-1/2 -translate-x-1/2 -translate-y-1/2 rounded-sm
                                       bg-anasthasia-accent-strong px-1.5 py-px text-[8px] font-bold text-anasthasia-text"
                                style:top={i === firstNonExcluded ? '16px' : '0'}
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
