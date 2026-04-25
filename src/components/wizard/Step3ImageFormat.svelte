<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { Button, SegmentedControl, Toggle, Input } from '$components/ui/index';
    import { IconArrowLeft, IconArrowRight, IconPhoto, IconRuler } from '@tabler/icons-svelte';
    import { duration } from '$lib/transitions';
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
    let enableMaxWidth = $state(wizard.maxWidth !== null);

    const collapse = { duration: duration.base, easing: cubicInOut };

    const formatHint: Record<string, string> = {
        avif: 'Best compression, slower — ideal for archiving',
        webp: 'Good compression, widely supported',
        original: 'No re-encoding — fastest, preserves originals',
    };

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            [
                'a',
                () => {
                    wizard.imageFormat = 'avif';
                    return true;
                },
            ],
            [
                'w',
                () => {
                    wizard.imageFormat = 'webp';
                    return true;
                },
            ],
            [
                'o',
                () => {
                    wizard.imageFormat = 'original';
                    return true;
                },
            ],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<div
    class="flex h-full flex-col"
    use:mountedHint={[
        ['keya', 'AVIF'],
        ['keyw', 'WebP'],
        ['keyo', 'Original'],
    ]}
>
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Image Format</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">
            How each page image will be encoded in the output.
        </p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <!-- Format -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconPhoto size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Format</span>
                </div>
                <SegmentedControl
                    options={[
                        { value: 'avif', label: 'AVIF' },
                        { value: 'webp', label: 'WebP' },
                        { value: 'original', label: 'Original' },
                    ]}
                    bind:value={wizard.imageFormat}
                />
                <p class="text-xs text-thasia-muted">{formatHint[wizard.imageFormat]}</p>
            </div>

            <div class="mx-4 border-t border-thasia-border"></div>

            <!-- Max width -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        <IconRuler size={14} class="flex-shrink-0 text-thasia-muted" />
                        <div>
                            <div class="text-sm font-medium">Max Width</div>
                            <div class="text-xs text-thasia-muted">Downscale wider images (px)</div>
                        </div>
                    </div>
                    <Toggle
                        bind:checked={enableMaxWidth}
                        onchange={(v) => {
                            wizard.maxWidth = v ? 1920 : null;
                        }}
                    />
                </div>
                {#if enableMaxWidth}
                    <div transition:slide={collapse}>
                        <Input
                            type="number"
                            min="400"
                            max="4000"
                            step="100"
                            bind:value={wizard.maxWidth as number}
                            hint="Common values: 1200, 1440, 1920"
                        />
                    </div>
                {/if}
            </div>
        </div>
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack} disabled={backDisabled}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={onNext} disabled={nextDisabled} class="ml-auto"
            >Next <IconArrowRight size={15} /></Button
        >
    </div>
</div>
