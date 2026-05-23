<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { duration, Input, keyboard, SegmentedControl, Toggle } from 'anasthasia';
    import { IconStack, IconSeparator } from '@tabler/icons-svelte';
    import WizardStep from './WizardStep.svelte';

    let {
        onNext,
        onBack,
        backDisabled = false,
    }: {
        onNext: () => void;
        onBack: () => void;
        nextDisabled?: boolean;
        backDisabled?: boolean;
    } = $props();

    const collapse = { duration: duration.base, easing: cubicInOut };

    function handleNext() {
        wizard.reconcileBundling();
        onNext();
    }

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['a', () => ((wizard.bundle = 'auto'), true)],
            ['f', () => ((wizard.bundle = 'flatten'), true)],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<WizardStep
    title="Bundling"
    description="How detected chapters are grouped into output volumes."
    onNext={handleNext}
    {onBack}
    {backDisabled}
    extraHints={[
        ['keya', 'Auto'],
        ['keyf', 'Flatten'],
    ]}
>
    <div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
        <!-- Mode -->
        <div class="flex flex-col gap-2.5 px-4 py-4">
            <div class="flex items-center gap-2">
                <IconStack size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <span class="text-sm font-medium">Mode</span>
            </div>
            <SegmentedControl
                options={[
                    { value: 'auto', label: 'Auto' },
                    { value: 'flatten', label: 'Flatten' },
                ]}
                bind:value={wizard.bundle}
            />
            <p class="text-xs text-anasthasia-muted">
                {wizard.bundle === 'auto'
                    ? 'Group chapters by detected volume number'
                    : 'Merge everything into a single output file'}
            </p>
        </div>

        {#if wizard.bundle === 'auto'}
            <div transition:slide={collapse}>
                <div class="mx-4 border-t border-anasthasia-border"></div>
                <div class="flex flex-col gap-3 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconSeparator size={14} class="flex-shrink-0 text-anasthasia-muted" />
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
</WizardStep>
