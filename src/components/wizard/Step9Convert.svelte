<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { commands, events } from '$types/bindings';
    import { goto } from '$app/navigation';
    import { Alert, Button, keyboard, ProgressBar } from 'anasthasia';
    import { SvelteMap } from 'svelte/reactivity';
    import { IconArrowLeft, IconCheck, IconX, IconRefresh } from '@tabler/icons-svelte';
    import WizardStep from './WizardStep.svelte';

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

    const IMAGE_FORMAT = { avif: 'Avif', webp: 'Webp', original: 'Original' } as const;
    const OUTPUT_FORMAT = { cbz: 'Cbz', epub: 'Epub', raw: 'Raw' } as const;
    const DIRECTION = { ltr: 'Ltr', rtl: 'Rtl' } as const;

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
            [
                'enter',
                () => {
                    if (status === 'done') startOver();
                    return true;
                },
            ],
        ]);

        status = 'converting';
        wizard.converting = true;
        try {
            const result = await commands.convert(
                {
                    output_dir: wizard.outputDir,
                    output_name: wizard.outputName,
                    create_directory: wizard.createDirectory,
                    image_format: IMAGE_FORMAT[wizard.imageFormat],
                    max_width: wizard.maxWidth,
                    output_format: OUTPUT_FORMAT[wizard.container],
                    direction: DIRECTION[wizard.direction],
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
        } finally {
            wizard.converting = false;
        }
    });

    onDestroy(() => {
        unlisteners.forEach((u) => u());
        cleanupKb?.();
        wizard.converting = false;
    });

    function startOver() {
        wizard.reset();
        goto('/');
    }

    let doneCount = $derived([...volumeMap.values()].filter((v) => v.done && v.success).length);
    let failedCount = $derived([...volumeMap.values()].filter((v) => v.done && !v.success).length);

    let title = $derived(
        status === 'done'
            ? 'Conversion complete'
            : status === 'error'
              ? 'Conversion failed'
              : 'Converting…'
    );
    let description = $derived(
        status === 'done'
            ? `${doneCount} volume${doneCount !== 1 ? 's' : ''} written in ${elapsed.toFixed(1)}s${failedCount > 0 ? ` · ${failedCount} failed` : ''}`
            : status === 'error'
              ? 'An error occurred during conversion'
              : 'Processing pages and packaging output…'
    );
    let extraHints = $derived(
        status === 'done' ? ([['enter', 'Start over']] as [string, string][]) : []
    );
</script>

<WizardStep {title} {description} {extraHints} showFooter={status === 'done' || status === 'error'}>
    {#if volumeMap.size > 0}
        <div
            class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            {#each [...volumeMap.entries()] as [_num, vol], i (_num)}
                <div
                    class="flex flex-col gap-2 px-4 py-3 {i < volumeMap.size - 1
                        ? 'border-b border-anasthasia-border'
                        : ''}"
                >
                    <div class="flex items-center justify-between">
                        <span class="text-sm font-medium">{vol.name}</span>
                        <span
                            class="inline-flex items-center gap-1 text-xs {vol.done && vol.success
                                ? 'text-emerald-500'
                                : vol.done
                                  ? 'text-red-400'
                                  : 'text-anasthasia-muted'}"
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
        <Alert variant="danger" title="Failed">{errorMessage}</Alert>
    {/if}

    {#snippet footer()}
        <div class="flex flex-shrink-0 gap-2 border-t border-anasthasia-border px-5 py-4">
            {#if status === 'error'}
                <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
            {:else if status === 'done'}
                <Button variant="primary" onclick={startOver}>
                    <IconRefresh size={15} /> Start over
                </Button>
            {/if}
        </div>
    {/snippet}
</WizardStep>
