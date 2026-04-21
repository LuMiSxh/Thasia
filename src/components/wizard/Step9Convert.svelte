<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { commands, events } from '$types/bindings';
    import { goto } from '$app/navigation';
    import ProgressBar from '$components/ui/ProgressBar.svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

    type VolumeProgress = {
        name: string;
        current: number;
        total: number;
        done: boolean;
        success?: boolean;
    };

    let status = $state<'idle' | 'converting' | 'done' | 'error'>('idle');
    let volumeMap = $state(new Map<number, VolumeProgress>());
    let errorMessage = $state('');
    let duration = $state(0);

    let unlisteners: Array<() => void> = [];

    function capitalize(s: string): string {
        return s.charAt(0).toUpperCase() + s.slice(1);
    }

    onMount(async () => {
        unlisteners.push(
            await events.volumeStartEvent.listen((e) => {
                volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
                    name: e.payload.volume_name,
                    current: 0,
                    total: 0,
                    done: false,
                });
            })
        );
        unlisteners.push(
            await events.imageProgressEvent.listen((e) => {
                const existing = volumeMap.get(e.payload.volume_num);
                if (existing) {
                    volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
                        ...existing,
                        current: e.payload.current,
                        total: e.payload.total,
                    });
                }
            })
        );
        unlisteners.push(
            await events.volumeCompleteEvent.listen((e) => {
                const existing = volumeMap.get(e.payload.volume_num);
                if (existing) {
                    volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
                        ...existing,
                        done: true,
                        success: e.payload.success,
                    });
                }
            })
        );
        unlisteners.push(
            await events.conversionCompleteEvent.listen((e) => {
                duration = e.payload.duration_secs;
                status = 'done';
            })
        );

        status = 'converting';
        try {
            const result = await commands.convert(
                {
                    output_dir: wizard.outputDir,
                    output_name: wizard.outputName,
                    create_directory: wizard.createDirectory,
                    image_format: capitalize(wizard.imageFormat) as 'Avif' | 'Webp' | 'Original',
                    max_width: wizard.maxWidth,
                    output_format: capitalize(wizard.container) as 'Cbz' | 'Epub' | 'Raw',
                    direction: capitalize(wizard.direction) as 'Ltr' | 'Rtl',
                    bundle: wizard.bundle,
                    volume_separator: wizard.volumeSeparator,
                    hide_single_volume: wizard.hideSingleVolume,
                },
                wizard.pageEdits.map((vol) => ({
                    volume_num: vol.volumeNum,
                    pages: vol.pages.map((p) => ({
                        original_page_index: p.originalPageIndex,
                        source_volume_num: p.sourceVolumeNum,
                        custom_path: p.customPath,
                        excluded: p.excluded,
                    })),
                }))
            );
            if (result.status === 'error') {
                status = 'error';
                errorMessage = result.error;
            }
        } catch (e) {
            status = 'error';
            errorMessage = String(e);
        }
    });

    onDestroy(() => {
        unlisteners.forEach((u) => u());
    });
</script>

<div class="flex h-[calc(100vh-120px)] flex-col gap-6 overflow-y-auto p-6">
    <h2 class="text-xl font-bold">
        {status === 'done' ? 'Done!' : status === 'error' ? 'Conversion failed' : 'Converting…'}
    </h2>

    {#if status === 'converting' || status === 'done'}
        <div class="flex flex-col gap-4">
            {#each [...volumeMap.entries()] as [num, vol]}
                <div>
                    <div class="mb-1.5 flex justify-between text-sm">
                        <span>{vol.name}</span>
                        <span
                            class={vol.done && vol.success
                                ? 'text-emerald-500'
                                : vol.done
                                  ? 'text-red-400'
                                  : 'text-thasia-muted'}
                        >
                            {vol.done
                                ? vol.success
                                    ? '✓ Done'
                                    : '✗ Failed'
                                : `${vol.current}/${vol.total}`}
                        </span>
                    </div>
                    <ProgressBar
                        value={vol.total ? vol.current / vol.total : 0}
                        variant={vol.done ? (vol.success ? 'success' : 'danger') : 'accent'}
                        class="h-2"
                    />
                </div>
            {/each}
        </div>
    {/if}

    {#if status === 'done'}
        <p class="text-sm text-emerald-500">All done in {duration.toFixed(1)}s!</p>
        <button
            onclick={() => {
                wizard.reset();
                goto('/');
            }}
            class="self-start rounded-lg border border-thasia-border bg-thasia-bg px-4 py-1.5 text-sm font-bold text-thasia-text
             transition-colors duration-150 hover:border-thasia-accent/50">Start over</button
        >
    {/if}

    {#if status === 'error'}
        <p class="text-sm text-red-400">Error: {errorMessage}</p>
        <button
            onclick={onBack}
            class="self-start rounded-lg border border-thasia-border bg-thasia-bg px-4 py-1.5 text-sm font-bold text-thasia-text
             transition-colors duration-150 hover:border-thasia-accent/50">← Back</button
        >
    {/if}
</div>
