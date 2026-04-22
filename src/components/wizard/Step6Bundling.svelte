<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { Button, SegmentedControl, Input, Toggle } from '$components/ui/index';
    import { IconArrowLeft, IconArrowRight, IconStack, IconSeparator } from '@tabler/icons-svelte';
    import { duration } from '$lib/transitions';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

    const collapse = { duration: duration.base, easing: cubicInOut };

    function handleNext() {
        if (wizard.bundle === 'flatten' && wizard.pageEdits.length > 1) {
            const firstNum = wizard.pageEdits[0]?.volumeNum ?? 1;
            wizard.pageEdits = [{
                volumeNum: firstNum,
                pages: wizard.pageEdits.flatMap((ve) => ve.pages),
            }];
        } else if (wizard.bundle === 'auto' && wizard.scanResult && wizard.pageEdits.length === 1) {
            wizard.pageEdits = wizard.scanResult.map((vol) => ({
                volumeNum: vol.volume_num,
                pages: vol.pages.map((_, i) => ({
                    originalPageIndex: i,
                    sourceVolumeNum: vol.volume_num,
                    customPath: null,
                    excluded: false,
                })),
            }));
        }
        onNext();
    }

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['keya', () => { wizard.bundle = 'auto'; return true; }],
            ['keyf', () => { wizard.bundle = 'flatten'; return true; }],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<div class="flex h-full flex-col" use:mountedHint={[['keya', 'Auto'], ['keyf', 'Flatten']]}>
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Bundling</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">How detected chapters are grouped into output volumes.</p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <!-- Mode -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconStack size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Mode</span>
                </div>
                <SegmentedControl
                    options={[
                        { value: 'auto', label: 'Auto' },
                        { value: 'flatten', label: 'Flatten' },
                    ]}
                    bind:value={wizard.bundle}
                />
                <p class="text-xs text-thasia-muted">
                    {wizard.bundle === 'auto'
                        ? 'Group chapters by detected volume number'
                        : 'Merge everything into a single output file'}
                </p>
            </div>

            {#if wizard.bundle === 'auto'}
                <div transition:slide={collapse}>
                    <div class="mx-4 border-t border-thasia-border"></div>
                    <div class="flex flex-col gap-3 px-4 py-4">
                        <div class="flex items-center gap-2">
                            <IconSeparator size={14} class="flex-shrink-0 text-thasia-muted" />
                            <span class="text-sm font-medium">Volume separator</span>
                        </div>
                        <Input
                            bind:value={wizard.volumeSeparator}
                            hint={`e.g. "${wizard.outputName}${wizard.volumeSeparator}1"`}
                        />
                        <Toggle
                            bind:checked={wizard.hideSingleVolume}
                            label="Omit volume number when only one volume is produced"
                        />
                    </div>
                </div>
            {/if}
        </div>
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={handleNext} class="ml-auto">Next <IconArrowRight size={15} /></Button>
    </div>
</div>
