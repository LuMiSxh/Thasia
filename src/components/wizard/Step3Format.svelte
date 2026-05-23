<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { duration, keyboard, SegmentedControl } from 'anasthasia';
    import { IconPackage, IconDirection } from '@tabler/icons-svelte';
    import WizardStep from './WizardStep.svelte';
    import EncodingControls from './EncodingControls.svelte';

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

    let enableMaxWidth = $state(wizard.maxWidth !== null);

    const collapse = { duration: duration.base, easing: cubicInOut };

    const containerHint: Record<typeof wizard.container, string> = {
        cbz: 'Comic Book ZIP — widely supported by all readers',
        epub: 'EPUB 3 fixed-layout — best for e-readers',
        raw: 'Flat image folder — no packaging',
    };

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['a', () => ((wizard.imageFormat = 'avif'), true)],
            ['w', () => ((wizard.imageFormat = 'webp'), true)],
            ['o', () => ((wizard.imageFormat = 'original'), true)],
            ['c', () => ((wizard.container = 'cbz'), true)],
            ['e', () => ((wizard.container = 'epub'), true)],
            ['r', () => ((wizard.container = 'raw'), true)],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<WizardStep
    title="Format"
    description="How each page is encoded and how the output is packaged."
    {onNext}
    {onBack}
    {backDisabled}
    extraHints={[
        ['keya', 'AVIF'],
        ['keyw', 'WebP'],
        ['keyo', 'Original'],
        ['keyc', 'CBZ'],
        ['keye', 'EPUB'],
        ['keyr', 'Raw'],
    ]}
>
    <div class="grid grid-cols-2 gap-4">
        <!-- Image encoding -->
        <EncodingControls
            bind:format={wizard.imageFormat}
            bind:maxWidth={wizard.maxWidth}
            bind:enableMaxWidth
        />

        <!-- Container -->
        <div
            class="flex flex-col overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconPackage size={14} class="flex-shrink-0 text-anasthasia-muted" />
                    <span class="text-sm font-medium">Container</span>
                </div>
                <SegmentedControl
                    options={[
                        { value: 'cbz', label: 'CBZ' },
                        { value: 'epub', label: 'EPUB' },
                        { value: 'raw', label: 'Raw' },
                    ]}
                    bind:value={wizard.container}
                />
                <p class="text-xs text-anasthasia-muted">{containerHint[wizard.container]}</p>
            </div>

            {#if wizard.container === 'epub'}
                <div transition:slide={collapse}>
                    <div class="mx-4 border-t border-anasthasia-border"></div>
                    <div class="flex flex-col gap-2.5 px-4 py-4">
                        <div class="flex items-center gap-2">
                            <IconDirection size={14} class="flex-shrink-0 text-anasthasia-muted" />
                            <span class="text-sm font-medium">Reading direction</span>
                        </div>
                        <SegmentedControl
                            options={[
                                { value: 'ltr', label: 'Left to Right' },
                                { value: 'rtl', label: 'Right to Left' },
                            ]}
                            bind:value={wizard.direction}
                        />
                        <p class="text-xs text-anasthasia-muted">
                            {wizard.direction === 'rtl'
                                ? 'Right-to-left — standard for manga and manhua'
                                : 'Left-to-right — standard for Western comics and manhwa'}
                        </p>
                    </div>
                </div>
            {/if}
        </div>
    </div>
</WizardStep>
