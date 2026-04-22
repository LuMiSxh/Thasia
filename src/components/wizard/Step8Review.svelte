<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { Button } from '$components/ui/index';
    import {
        IconArrowLeft,
        IconArrowRight,
        IconFolderOpen,
        IconFolderPlus,
        IconPhoto,
        IconPackage,
        IconDirection,
        IconStack,
        IconBook,
        IconFile,
    } from '@tabler/icons-svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

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

<div class="flex h-full flex-col">
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Review</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">Confirm your settings before converting.</p>
    </div>

    <div class="flex flex-1 flex-col overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            {#each rows as row, i (row.label)}
                {@const Icon = row.icon}
                <div
                    class="flex items-center gap-3 px-4 py-3 {i !== rows.length - 1
                        ? 'border-b border-thasia-border'
                        : ''}"
                >
                    <Icon size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="w-28 flex-shrink-0 text-xs text-thasia-muted">{row.label}</span>
                    <span class="min-w-0 flex-1 truncate text-sm font-medium" title={row.value}
                        >{row.value}</span
                    >
                </div>
            {/each}
        </div>
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
        <Button variant="primary" onclick={onNext} class="ml-auto"
            >Start Converting <IconArrowRight size={15} /></Button
        >
    </div>
</div>
