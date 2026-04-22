<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { commands, events } from '$types/bindings';
    import { goto } from '$app/navigation';
    import ProgressBar from '$components/ui/ProgressBar.svelte';
    import { Button } from '$components/ui/index';
    import { SvelteMap } from 'svelte/reactivity';
    import { IconArrowLeft, IconCheck, IconX, IconRefresh } from '@tabler/icons-svelte';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

    let { onBack }: { onNext: () => void; onBack: () => void } = $props();

    type VolumeProgress = {
        name: string;
        current: number;
        total: number;
        done: boolean;
        success?: boolean;
    };

    let status = $state<'idle' | 'converting' | 'done' | 'error'>('idle');
    let volumeMap = new SvelteMap<number, VolumeProgress>();
    let errorMessage = $state('');
    let elapsed = $state(0);

    let unlisteners: Array<() => void> = [];
    let cleanupKb: (() => void) | undefined;

    function capitalize(s: string): string {
        return s.charAt(0).toUpperCase() + s.slice(1);
    }

    onMount(async () => {
        unlisteners.push(
            await events.volumeStartEvent.listen((e) => {
                volumeMap.set(e.payload.volume_num, {
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
                    volumeMap.set(e.payload.volume_num, {
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
                    volumeMap.set(e.payload.volume_num, {
                        ...existing,
                        done: true,
                        success: e.payload.success,
                    });
                }
            })
        );
        unlisteners.push(
            await events.conversionCompleteEvent.listen((e) => {
                elapsed = e.payload.duration_secs;
                status = 'done';
            })
        );

        cleanupKb = keyboard.smartRegister([
            ['enter', () => {
                if (status === 'done') { wizard.reset(); goto('/'); }
                return true;
            }],
        ]);

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
        cleanupKb?.();
    });

    let doneCount = $derived([...volumeMap.values()].filter((v) => v.done && v.success).length);
    let failedCount = $derived([...volumeMap.values()].filter((v) => v.done && !v.success).length);
</script>

<div class="flex h-full flex-col" use:mountedHint={status === 'done' ? [['enter', 'Start over']] : []}>
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">
            {#if status === 'done'}
                Conversion complete
            {:else if status === 'error'}
                Conversion failed
            {:else}
                Converting…
            {/if}
        </h2>
        <p class="mt-0.5 text-xs text-thasia-muted">
            {#if status === 'done'}
                {doneCount} volume{doneCount !== 1 ? 's' : ''} written in {elapsed.toFixed(1)}s
                {#if failedCount > 0}· {failedCount} failed{/if}
            {:else if status === 'error'}
                An error occurred during conversion
            {:else}
                Processing pages and packaging output…
            {/if}
        </p>
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        {#if volumeMap.size > 0}
            <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
                {#each [...volumeMap.entries()] as [_num, vol], i ([_num, vol])}
                    <div
                        class="flex flex-col gap-2 px-4 py-3 {i < volumeMap.size - 1
                            ? 'border-b border-thasia-border'
                            : ''}"
                    >
                        <div class="flex items-center justify-between">
                            <span class="text-sm font-medium">{vol.name}</span>
                            <span
                                class="inline-flex items-center gap-1 text-xs {vol.done &&
                                vol.success
                                    ? 'text-emerald-500'
                                    : vol.done
                                      ? 'text-red-400'
                                      : 'text-thasia-muted'}"
                            >
                                {#if vol.done && vol.success}
                                    <IconCheck size={12} /> Done
                                {:else if vol.done}
                                    <IconX size={12} /> Failed
                                {:else}
                                    {vol.current}/{vol.total}
                                {/if}
                            </span>
                        </div>
                        <ProgressBar
                            value={vol.total ? vol.current / vol.total : 0}
                            variant={vol.done ? (vol.success ? 'success' : 'danger') : 'accent'}
                            class="h-1.5"
                        />
                    </div>
                {/each}
            </div>
        {/if}

        {#if status === 'error'}
            <div
                class="overflow-hidden rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3"
            >
                <p class="text-xs text-red-400">{errorMessage}</p>
            </div>
        {/if}
    </div>

    <!-- Footer — only shown when there's an action to take -->
    {#if status === 'done' || status === 'error'}
        <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
            {#if status === 'error'}
                <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
            {:else}
                <Button
                    variant="primary"
                    onclick={() => {
                        wizard.reset();
                        goto('/');
                    }}
                >
                    <IconRefresh size={15} /> Start over
                </Button>
            {/if}
        </div>
    {/if}
</div>
