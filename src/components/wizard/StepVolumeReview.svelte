<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import type { VolumeEdit } from '$lib/wizard/state.svelte';
    import ProgressBar from '$components/ui/ProgressBar.svelte';
    import { Button } from '$components/ui/index';
    import {
        IconArrowLeft,
        IconArrowRight,
        IconX,
        IconBook,
        IconLayoutGrid,
    } from '@tabler/icons-svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

    let scanVols = $derived(wizard.scanResult ?? []);
    let unit = $derived(scanVols.length > 1 ? 'chapter' : 'page');
    let total = $derived(unit === 'chapter' ? scanVols.length : (scanVols[0]?.pages.length ?? 0));

    let volumeSizes = $state<number[]>([]);
    let newVolumeInput = $state('');
    let editingIndex = $state<number | null>(null);

    $effect.pre(() => {
        if (volumeSizes.length === 0 && wizard.pageEdits.length > 0) {
            if (unit === 'chapter') {
                const groups: number[] = [];
                let lastVol: number | null = null;
                let count = 0;
                for (const scan of wizard.scanResult ?? []) {
                    if (scan.source_volume_num !== lastVol) {
                        if (count > 0) groups.push(count);
                        lastVol = scan.source_volume_num;
                        count = 1;
                    } else {
                        count++;
                    }
                }
                if (count > 0) groups.push(count);
                volumeSizes = groups.length > 0 ? groups : [total];
            } else {
                volumeSizes = [wizard.pageEdits[0]?.pages.length ?? total];
            }
        }
    });

    let used = $derived(volumeSizes.reduce((s, n) => s + n, 0));
    let remaining = $derived(total - used);
    let isOver = $derived(used > total);
    let isValid = $derived(
        remaining === 0 && volumeSizes.length > 0 && volumeSizes.every((n) => n > 0)
    );

    let usageVariant = $derived(isValid ? 'success' : isOver ? 'danger' : 'warning') as
        | 'success'
        | 'danger'
        | 'warning';

    let itemVolumeMap = $derived(
        (() => {
            const map = new Array(total).fill(-1);
            let offset = 0;
            for (let v = 0; v < volumeSizes.length; v++) {
                for (let i = 0; i < volumeSizes[v] && offset < total; i++) map[offset++] = v;
            }
            return map;
        })()
    );

    const slotColors = ['var(--accent)', '#10b981', '#3b82f6', '#f59e0b', '#ef4444'];
    function slotColor(i: number): string {
        return slotColors[i % slotColors.length];
    }
    function slotBg(i: number): string {
        return `color-mix(in srgb, ${slotColor(i)} 15%, transparent)`;
    }

    function addVolume() {
        const n = Number(newVolumeInput);
        if (!n || n <= 0 || isNaN(n)) return;
        volumeSizes = [...volumeSizes, n];
        newVolumeInput = '';
    }

    function deleteVolume(i: number) {
        volumeSizes = volumeSizes.filter((_, idx) => idx !== i);
        if (editingIndex === i) editingIndex = null;
    }

    function handleAddKey(e: KeyboardEvent) {
        if (e.key === 'Enter') addVolume();
    }

    function handleNext() {
        if (!isValid) return;

        let newEdits: VolumeEdit[];

        if (unit === 'chapter') {
            let chapOffset = 0;
            newEdits = volumeSizes.map((count, outIdx) => {
                const assignedScanVols = scanVols.slice(chapOffset, (chapOffset += count));
                return {
                    volumeNum: outIdx + 1,
                    pages: assignedScanVols.flatMap((sv) =>
                        sv.pages.map((_, pi) => ({
                            originalPageIndex: pi,
                            sourceVolumeNum: sv.volume_num,
                            customPath: null as string | null,
                            excluded: false,
                        }))
                    ),
                };
            });
        } else {
            const srcVol = scanVols[0];
            let off = 0;
            newEdits = volumeSizes.map((count, outIdx) => {
                const start = off;
                off += count;
                return {
                    volumeNum: outIdx + 1,
                    pages: Array.from({ length: count }, (_, pi) => ({
                        originalPageIndex: start + pi,
                        sourceVolumeNum: srcVol?.volume_num ?? 1,
                        customPath: null as string | null,
                        excluded: false,
                    })),
                };
            });
        }

        wizard.pageEdits = newEdits;
        onNext();
    }
</script>

<div class="flex h-full flex-col">
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Volume Assignment</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">
            Detected {total}
            {unit}{total !== 1 ? 's' : ''} — assign them to output volumes.
        </p>
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col gap-3 overflow-hidden px-5 py-5">
        <!-- Stat row -->
        <div class="grid flex-shrink-0 grid-cols-3 gap-2">
            <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                <div class="border-b border-thasia-border bg-thasia-panel px-4 py-2">
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Detected</span
                    >
                </div>
                <div class="px-4 py-3">
                    <div class="text-2xl font-bold">{total}</div>
                    <div class="text-xs text-thasia-muted">{unit}s</div>
                </div>
            </div>
            <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                <div class="border-b border-thasia-border bg-thasia-panel px-4 py-2">
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Assigned</span
                    >
                </div>
                <div class="px-4 py-3">
                    <div
                        class="text-2xl font-bold {isValid
                            ? 'text-emerald-500'
                            : isOver
                              ? 'text-red-400'
                              : 'text-amber-400'}"
                    >
                        {used}
                    </div>
                    <div class="text-xs text-thasia-muted">
                        {remaining > 0
                            ? `${remaining} remaining`
                            : isOver
                              ? `${-remaining} over`
                              : 'all assigned'}
                    </div>
                </div>
            </div>
            <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                <div class="border-b border-thasia-border bg-thasia-panel px-4 py-2">
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Volumes</span
                    >
                </div>
                <div class="px-4 py-3">
                    <div class="text-2xl font-bold">{volumeSizes.length}</div>
                    <div class="text-xs text-thasia-muted">output files</div>
                </div>
            </div>
        </div>

        <!-- Main area -->
        <div class="grid min-h-0 flex-1 gap-2" style="grid-template-columns: 1fr 220px;">
            <!-- Volume list -->
            <div
                class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface"
            >
                <div
                    class="flex flex-shrink-0 items-center gap-2 border-b border-thasia-border bg-thasia-panel px-4 py-2.5"
                >
                    <IconBook size={13} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Volumes</span
                    >
                </div>
                <div class="flex-1 overflow-y-auto p-2">
                    {#if volumeSizes.length === 0}
                        <div
                            class="flex h-full items-center justify-center text-sm text-thasia-muted"
                        >
                            No volumes yet — add one below
                        </div>
                    {:else}
                        {#each volumeSizes as count, i (i)}
                            <div
                                class="mb-1.5 rounded-lg border bg-thasia-bg px-3 py-2.5 transition-colors duration-150
                                {editingIndex === i
                                    ? 'border-thasia-accent'
                                    : 'border-thasia-border'}"
                            >
                                <div class="flex items-center gap-2">
                                    <div
                                        class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full"
                                        style="background: {slotBg(i)};"
                                    >
                                        <span
                                            class="text-xs font-bold"
                                            style="color: {slotColor(i)};">{i + 1}</span
                                        >
                                    </div>

                                    <div class="min-w-0 flex-1">
                                        <div class="mb-1.5 text-sm font-medium">Volume {i + 1}</div>
                                        <ProgressBar
                                            value={count / total}
                                            color={slotColor(i)}
                                            class="h-1"
                                        />
                                    </div>

                                    {#if editingIndex === i}
                                        <input
                                            type="number"
                                            min="1"
                                            max={total}
                                            value={count}
                                            oninput={(e) => {
                                                volumeSizes[i] =
                                                    parseInt(
                                                        (e.target as HTMLInputElement).value
                                                    ) || 0;
                                            }}
                                            onblur={() => (editingIndex = null)}
                                            onkeydown={(e) => {
                                                if (e.key === 'Enter' || e.key === 'Escape')
                                                    editingIndex = null;
                                            }}
                                            class="h-8 w-14 rounded-md border border-thasia-accent bg-thasia-bg text-center text-sm text-thasia-text
                                            focus:ring-1 focus:ring-thasia-accent focus:outline-none"
                                        />
                                    {:else}
                                        <button
                                            onclick={() => (editingIndex = i)}
                                            class="h-8 w-14 cursor-pointer rounded-md border border-thasia-border bg-thasia-bg text-center
                                            text-sm font-medium text-thasia-text transition-colors duration-150 hover:border-thasia-accent"
                                            title="Click to edit"
                                        >
                                            {count}
                                        </button>
                                    {/if}

                                    <span class="text-xs whitespace-nowrap text-thasia-muted">
                                        {unit}{count !== 1 ? 's' : ''}
                                    </span>

                                    <Button
                                        variant="danger"
                                        size="sm"
                                        onclick={() => deleteVolume(i)}
                                        class="h-7 w-7 p-0"><IconX size={13} /></Button
                                    >
                                </div>
                            </div>
                        {/each}
                    {/if}
                </div>

                <!-- Add volume row -->
                <div class="flex flex-shrink-0 gap-1.5 border-t border-thasia-border p-2">
                    <input
                        type="number"
                        min="1"
                        placeholder="# of {unit}s"
                        bind:value={newVolumeInput}
                        onkeydown={handleAddKey}
                        class="flex-1 rounded-md border border-thasia-border bg-thasia-bg px-2 py-1.5 text-sm text-thasia-text
                        transition-colors duration-150 placeholder:text-thasia-muted focus:border-thasia-accent focus:ring-1
                        focus:ring-thasia-accent focus:outline-none"
                    />
                    <Button
                        variant="primary"
                        onclick={addVolume}
                        disabled={!newVolumeInput || Number(newVolumeInput) <= 0}
                    >
                        Add
                    </Button>
                </div>
            </div>

            <!-- Distribution sidebar -->
            <div
                class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface"
            >
                <div
                    class="flex flex-shrink-0 items-center gap-2 border-b border-thasia-border bg-thasia-panel px-4 py-2.5"
                >
                    <IconLayoutGrid size={13} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Distribution</span
                    >
                </div>
                <div class="flex-1 overflow-y-auto p-3">
                    <!-- Usage bar -->
                    <div class="mb-3">
                        <div class="mb-1 flex justify-between text-xs">
                            <span>Usage</span>
                            <span class="font-mono">{used}/{total}</span>
                        </div>
                        <ProgressBar
                            value={used / Math.max(total, 1)}
                            variant={usageVariant}
                            class="h-1.5"
                        />
                    </div>

                    <!-- Item map -->
                    <div class="mb-1.5 text-[11px] text-thasia-muted">
                        {unit === 'chapter' ? 'Chapter' : 'Page'} map
                    </div>
                    <div class="flex flex-wrap gap-0.5">
                        {#each itemVolumeMap as vIdx, itemI (itemI)}
                            <div
                                class="h-2.5 w-2.5 cursor-help rounded-sm transition-colors duration-150"
                                style="background: {vIdx === -1
                                    ? 'var(--border)'
                                    : slotColor(vIdx)};"
                                title="{unit === 'chapter' ? 'Chapter' : 'Page'} {itemI + 1}{vIdx >=
                                0
                                    ? ` → Volume ${vIdx + 1}`
                                    : ' (unassigned)'}"
                            ></div>
                        {/each}
                    </div>
                </div>
            </div>
        </div>
    </div>

    <!-- Footer -->
    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
        <Button
            onclick={handleNext}
            disabled={!isValid}
            class="ml-auto"
            title={!isValid
                ? `${remaining > 0 ? remaining + ' unassigned' : 'over by ' + -remaining} — assign all ${unit}s first`
                : ''}>Next <IconArrowRight size={15} /></Button
        >
    </div>
</div>
