<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { duration, Input, keyboard, SegmentedControl, Toggle } from 'anasthasia';
    import { IconPackage, IconDirection, IconStack, IconSeparator } from '@tabler/icons-svelte';
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

    function handleNext() {
        wizard.reconcileBundling();
        onNext();
    }

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['a', () => ((wizard.imageFormat = 'avif'), true)],
            ['w', () => ((wizard.imageFormat = 'webp'), true)],
            ['o', () => ((wizard.imageFormat = 'original'), true)],
            ['c', () => ((wizard.container = 'cbz'), true)],
            ['e', () => ((wizard.container = 'epub'), true)],
            ['r', () => ((wizard.container = 'raw'), true)],
            ['b', () => ((wizard.bundle = 'auto'), true)],
            ['f', () => ((wizard.bundle = 'flatten'), true)],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<WizardStep
    title="Output"
    description="Encoding, container, and how chapters are grouped."
    onNext={handleNext}
    {onBack}
    {backDisabled}
    extraHints={[
        ['keya', 'AVIF'],
        ['keyw', 'WebP'],
        ['keyo', 'Original'],
        ['keyc', 'CBZ'],
        ['keye', 'EPUB'],
        ['keyr', 'Raw'],
        ['keyb', 'Auto bundle'],
        ['keyf', 'Flatten'],
    ]}
>
    <div
        class="grid gap-3 lg:grid-cols-2 2xl:grid-cols-[minmax(0,0.85fr)_minmax(0,1fr)_minmax(0,1fr)]"
    >
        <!-- Image encoding -->
        <div class="2xl:col-auto">
            <EncodingControls
                bind:format={wizard.imageFormat}
                bind:maxWidth={wizard.maxWidth}
                bind:enableMaxWidth
                bind:forceReencode={wizard.forceReencode}
                bind:cleanTones={wizard.cleanTones}
                bind:colorEnhance={wizard.colorEnhance}
                bind:sharpen={wizard.sharpen}
            />
        </div>

        <div
            class="flex flex-col overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                    Packaging
                </span>
            </div>

            <div class="flex flex-col gap-2.5 px-4 py-3">
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
                    <div class="flex flex-col gap-2.5 px-4 py-3">
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

        <div
            class="flex flex-col overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface lg:col-span-2 2xl:col-span-1"
        >
            <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                    Bundling
                </span>
            </div>
            <div class="flex flex-col gap-2.5 px-4 py-3">
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
                    <div class="flex flex-col gap-2.5 px-4 py-3">
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
    </div>
</WizardStep>
