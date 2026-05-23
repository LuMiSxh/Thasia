<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import {
        IconFolderOpen,
        IconFolderPlus,
        IconPhoto,
        IconPackage,
        IconDirection,
        IconStack,
        IconBook,
        IconFile,
    } from '@tabler/icons-svelte';
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

    let totalPages = $derived(
        wizard.pageEdits.reduce((acc, vol) => acc + vol.pages.filter((p) => !p.excluded).length, 0)
    );

    const rows = $derived([
        { icon: IconFolderOpen, label: 'Source', value: wizard.sourcePath },
        {
            icon: IconFolderPlus,
            label: 'Output',
            value: `${wizard.outputDir} / ${wizard.outputName}`,
        },
        {
            icon: IconPhoto,
            label: 'Image format',
            value: `${wizard.imageFormat.toUpperCase()}${wizard.maxWidth ? ` (max ${wizard.maxWidth}px)` : ''}`,
        },
        { icon: IconPackage, label: 'Container', value: wizard.container.toUpperCase() },
        ...(wizard.container === 'epub'
            ? [{ icon: IconDirection, label: 'Direction', value: wizard.direction.toUpperCase() }]
            : []),
        { icon: IconStack, label: 'Bundling', value: wizard.bundle },
        { icon: IconBook, label: 'Volumes', value: String(wizard.pageEdits.length) },
        { icon: IconFile, label: 'Total pages', value: String(totalPages) },
    ]);
</script>

<WizardStep
    title="Review"
    description="Confirm your settings before converting."
    {onNext}
    {onBack}
    {backDisabled}
    nextLabel="Start Converting"
    nextVariant="primary"
>
    <div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
        {#each rows as row, i (row.label)}
            {@const Icon = row.icon}
            <div
                class="flex items-center gap-3 px-4 py-3 {i !== rows.length - 1
                    ? 'border-b border-anasthasia-border'
                    : ''}"
            >
                <Icon size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <span class="w-28 flex-shrink-0 text-xs text-anasthasia-muted">{row.label}</span>
                <span class="min-w-0 flex-1 truncate text-sm font-medium" title={row.value}>
                    {row.value}
                </span>
            </div>
        {/each}
    </div>
</WizardStep>
