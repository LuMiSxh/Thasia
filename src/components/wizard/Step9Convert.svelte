<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { formatAppError } from '$lib/errors';
    import { buildConvertOptions, buildVolumeEdits } from '$lib/wizard/payload';
    import { commands, events, type ConversionOutput } from '$types/bindings';
    import { goto } from '$app/navigation';
    import { Alert, Button, keyboard, ProgressBar } from 'anasthasia';
    import { SvelteMap } from 'svelte/reactivity';
    import {
        IconArrowLeft,
        IconCheck,
        IconX,
        IconRefresh,
        IconPlayerStop,
    } from '@tabler/icons-svelte';
    import WizardStep from './WizardStep.svelte';

    let { onBack }: { onNext: () => void; onBack: () => void } = $props();

    type VolumeProgress = {
        name: string;
        current: number;
        total: number;
        done: boolean;
        success?: boolean;
        elapsedSecs: number;
        pagesPerSec: number;
        estimatedRemainingSecs: number | null;
        inputBytes: number;
        outputBytes: number;
        passthroughPages: number;
        encodedPages: number;
    };

    let status = $state<'idle' | 'converting' | 'done' | 'error'>('idle');
    let volumeMap = new SvelteMap<number, VolumeProgress>();
    let errorMessage = $state('');
    let elapsed = $state(0);
    let liveElapsed = $state(0);
    let startedAt = 0;
    let timer: ReturnType<typeof setInterval> | undefined;
    let resultStats = $state<{
        totalPages: number;
        inputBytes: number;
        outputBytes: number;
        passthroughPages: number;
        encodedPages: number;
        fetchMs: number;
        decodeMs: number;
        transformMs: number;
        encodeMs: number;
        outputs: ConversionOutput[];
    } | null>(null);

    let unlisteners: Array<() => void> = [];
    let cleanupKb: (() => void) | undefined;

    onMount(async () => {
        unlisteners.push(
            await events.volumeStartEvent.listen((e) => {
                volumeMap.set(e.payload.volume_num, {
                    name: e.payload.volume_name,
                    current: 0,
                    total: 0,
                    done: false,
                    elapsedSecs: 0,
                    pagesPerSec: 0,
                    estimatedRemainingSecs: null,
                    inputBytes: 0,
                    outputBytes: 0,
                    passthroughPages: 0,
                    encodedPages: 0,
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
                        elapsedSecs: e.payload.elapsed_secs,
                        pagesPerSec: e.payload.pages_per_sec,
                        estimatedRemainingSecs: e.payload.estimated_remaining_secs,
                        inputBytes: e.payload.input_bytes,
                        outputBytes: e.payload.output_bytes,
                        passthroughPages: e.payload.passthrough_pages,
                        encodedPages: e.payload.encoded_pages,
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
                liveElapsed = e.payload.duration_secs;
                resultStats = {
                    totalPages: e.payload.total_pages,
                    inputBytes: e.payload.input_bytes,
                    outputBytes: e.payload.output_bytes,
                    passthroughPages: e.payload.passthrough_pages,
                    encodedPages: e.payload.encoded_pages,
                    fetchMs: e.payload.fetch_ms,
                    decodeMs: e.payload.decode_ms,
                    transformMs: e.payload.transform_ms,
                    encodeMs: e.payload.encode_ms,
                    outputs: e.payload.outputs,
                };
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
        startedAt = performance.now();
        timer = setInterval(() => {
            if (status === 'converting') liveElapsed = (performance.now() - startedAt) / 1000;
        }, 500);
        try {
            const result = await commands.convert(
                buildConvertOptions(wizard),
                buildVolumeEdits(wizard)
            );
            if (result.status === 'error') {
                status = 'error';
                errorMessage = formatAppError(result.error);
            }
        } catch (e) {
            status = 'error';
            errorMessage = formatAppError(e);
        } finally {
            if (timer) {
                clearInterval(timer);
                timer = undefined;
            }
            wizard.converting = false;
        }
    });

    onDestroy(() => {
        unlisteners.forEach((u) => u());
        cleanupKb?.();
        if (timer) clearInterval(timer);
        wizard.converting = false;
    });

    function startOver() {
        wizard.reset();
        goto('/');
    }

    let cancelling = $state(false);
    async function cancel() {
        if (cancelling) return;
        cancelling = true;
        await commands.cancelConversion().catch(() => {});
    }

    let doneCount = $derived([...volumeMap.values()].filter((v) => v.done && v.success).length);
    let failedCount = $derived([...volumeMap.values()].filter((v) => v.done && !v.success).length);
    let currentPages = $derived([...volumeMap.values()].reduce((acc, v) => acc + v.current, 0));
    let totalPages = $derived([...volumeMap.values()].reduce((acc, v) => acc + v.total, 0));
    let pagesPerSec = $derived(currentPages / Math.max(liveElapsed, 0.001));
    let remainingSecs = $derived(
        totalPages > currentPages && pagesPerSec > 0
            ? (totalPages - currentPages) / pagesPerSec
            : null
    );

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

    function formatDuration(seconds: number | null | undefined) {
        if (seconds == null || !Number.isFinite(seconds)) return '—';
        if (seconds < 60) return `${Math.max(0, seconds).toFixed(0)}s`;
        const minutes = Math.floor(seconds / 60);
        const rest = Math.round(seconds % 60);
        return `${minutes}m ${rest.toString().padStart(2, '0')}s`;
    }

    function formatBytes(bytes: number) {
        if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB'];
        let value = bytes;
        let unit = 0;
        while (value >= 1024 && unit < units.length - 1) {
            value /= 1024;
            unit += 1;
        }
        return `${value.toFixed(value >= 10 || unit === 0 ? 0 : 1)} ${units[unit]}`;
    }

    function formatRate(value: number) {
        if (!Number.isFinite(value) || value <= 0) return '—';
        return `${value.toFixed(value >= 10 ? 1 : 2)} pages/s`;
    }
</script>

<WizardStep {title} {description} {extraHints} showFooter>
    {#if status === 'converting' || status === 'done'}
        <div class="grid gap-2 sm:grid-cols-4">
            <div class="rounded-lg border border-anasthasia-border bg-anasthasia-surface px-3 py-2">
                <p class="text-xs text-anasthasia-muted">Elapsed</p>
                <p class="text-sm font-semibold">{formatDuration(liveElapsed || elapsed)}</p>
            </div>
            <div class="rounded-lg border border-anasthasia-border bg-anasthasia-surface px-3 py-2">
                <p class="text-xs text-anasthasia-muted">Remaining</p>
                <p class="text-sm font-semibold">
                    {status === 'converting' ? formatDuration(remainingSecs) : '—'}
                </p>
            </div>
            <div class="rounded-lg border border-anasthasia-border bg-anasthasia-surface px-3 py-2">
                <p class="text-xs text-anasthasia-muted">Progress</p>
                <p class="text-sm font-semibold">{currentPages}/{totalPages || currentPages}</p>
            </div>
            <div class="rounded-lg border border-anasthasia-border bg-anasthasia-surface px-3 py-2">
                <p class="text-xs text-anasthasia-muted">Speed</p>
                <p class="text-sm font-semibold">{formatRate(pagesPerSec)}</p>
            </div>
        </div>
    {/if}

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
                    {#if vol.current > 0 && !vol.done}
                        <div class="flex justify-between text-xs text-anasthasia-muted">
                            <span>{formatRate(vol.pagesPerSec)}</span>
                            <span>ETA {formatDuration(vol.estimatedRemainingSecs)}</span>
                        </div>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}

    {#if status === 'done' && resultStats}
        <div
            class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            <div class="grid gap-2 border-b border-anasthasia-border p-3 sm:grid-cols-3">
                <div class="rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 py-2">
                    <p class="text-xs text-anasthasia-muted">Input</p>
                    <p class="text-sm font-semibold">{formatBytes(resultStats.inputBytes)}</p>
                </div>
                <div class="rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 py-2">
                    <p class="text-xs text-anasthasia-muted">Encoded</p>
                    <p class="text-sm font-semibold">{formatBytes(resultStats.outputBytes)}</p>
                </div>
                <div class="rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 py-2">
                    <p class="text-xs text-anasthasia-muted">Pages</p>
                    <p class="text-sm font-semibold">
                        {resultStats.totalPages} total · {resultStats.encodedPages} encoded · {resultStats.passthroughPages}
                        passthrough
                    </p>
                </div>
            </div>
            <div class="divide-y divide-anasthasia-border">
                {#each resultStats.outputs as output (output.volume_num)}
                    <div class="flex min-w-0 items-center gap-3 px-4 py-3">
                        <span class="w-24 flex-shrink-0 truncate text-sm font-medium">
                            {output.volume_name}
                        </span>
                        <span
                            class="min-w-0 flex-1 truncate font-mono text-xs text-anasthasia-muted"
                            title={output.path}
                        >
                            {output.path}
                        </span>
                    </div>
                {/each}
            </div>
        </div>
    {/if}

    {#if status === 'error'}
        <Alert variant="danger" title="Failed">{errorMessage}</Alert>
    {/if}

    {#snippet footer()}
        <div class="flex flex-shrink-0 gap-2 border-t border-anasthasia-border px-5 py-4">
            {#if status === 'converting'}
                <Button
                    variant="danger"
                    onclick={cancel}
                    loading={cancelling}
                    loadingLabel="Stopping…"
                    class="ml-auto"
                >
                    <IconPlayerStop size={15} /> Cancel
                </Button>
            {:else if status === 'error'}
                <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
            {:else if status === 'done'}
                <Button variant="primary" onclick={startOver}>
                    <IconRefresh size={15} /> Start over
                </Button>
            {/if}
        </div>
    {/snippet}
</WizardStep>
