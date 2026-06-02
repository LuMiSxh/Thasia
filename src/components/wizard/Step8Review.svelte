<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { formatAppError } from '$lib/errors';
    import { buildConvertOptions, buildVolumeEdits } from '$lib/wizard/payload';
    import { commands, type PipelinePlan } from '$types/bindings';
    import {
        IconFolderOpen,
        IconFolderPlus,
        IconPhoto,
        IconPackage,
        IconDirection,
        IconStack,
        IconBook,
        IconFile,
        IconRoute,
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
    let excludedPages = $derived(
        wizard.pageEdits.reduce((acc, vol) => acc + vol.pages.filter((p) => p.excluded).length, 0)
    );
    let addedPages = $derived(
        wizard.pageEdits.reduce(
            (acc, vol) =>
                acc + vol.pages.filter((p) => p.customPath !== null && !p.excluded).length,
            0
        )
    );
    let pipelinePlan = $state<PipelinePlan | null>(null);
    let pipelineError = $state('');

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
            value: `${wizard.imageFormat.toUpperCase()}${wizard.maxWidth ? ` (max ${wizard.maxWidth}px)` : ''}${wizard.forceReencode ? ' · force re-encode' : ''}${wizard.cleanTones ? ' · clean tones' : ''}${wizard.colorEnhance !== 'off' ? ` · color ${wizard.colorEnhance}` : ''}${wizard.sharpen !== 'off' ? ' · sharpen' : ''}`,
        },
        { icon: IconPackage, label: 'Container', value: wizard.container.toUpperCase() },
        ...(wizard.container === 'epub'
            ? [{ icon: IconDirection, label: 'Direction', value: wizard.direction.toUpperCase() }]
            : []),
        { icon: IconStack, label: 'Bundling', value: wizard.bundle },
        { icon: IconBook, label: 'Volumes', value: String(wizard.pageEdits.length) },
        {
            icon: IconFile,
            label: 'Pages',
            value: `${totalPages} included${excludedPages ? ` · ${excludedPages} excluded` : ''}${addedPages ? ` · ${addedPages} added` : ''}`,
        },
    ]);

    $effect(() => {
        commands
            .buildPipelinePlan(buildConvertOptions(wizard), buildVolumeEdits(wizard))
            .then((result) => {
                if (result.status === 'ok') {
                    pipelinePlan = result.data;
                    pipelineError = '';
                } else {
                    pipelinePlan = null;
                    pipelineError = formatAppError(result.error);
                }
            });
    });

    function costLabel(cost: PipelinePlan['stages'][number]['steps'][number]['cost']) {
        return cost.charAt(0).toUpperCase() + cost.slice(1);
    }
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

    <div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
        <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
            <div class="flex items-center gap-2">
                <IconRoute size={14} class="text-anasthasia-muted" />
                <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                    Pipeline
                </span>
            </div>
        </div>
        {#if pipelinePlan}
            <div class="divide-y divide-anasthasia-border">
                {#each pipelinePlan.stages as stage (stage.id)}
                    <div class="px-4 py-3">
                        <div class="mb-2 flex items-center justify-between gap-3">
                            <span class="text-sm font-medium">{stage.label}</span>
                            <span
                                class="text-xs {stage.enabled
                                    ? 'text-anasthasia-muted'
                                    : 'text-anasthasia-muted/60'}"
                            >
                                {stage.enabled ? 'Active' : 'Skipped'}
                            </span>
                        </div>
                        <div class="flex flex-wrap gap-1.5">
                            {#each stage.steps as step (step.id)}
                                <span
                                    class="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-xs {step.enabled
                                        ? 'text-anasthasia-foreground border-anasthasia-border bg-anasthasia-panel'
                                        : 'border-anasthasia-border/60 bg-transparent text-anasthasia-muted'}"
                                    title={`${step.category} · ${costLabel(step.cost)}`}
                                >
                                    {step.label}
                                </span>
                            {/each}
                        </div>
                    </div>
                {/each}
            </div>
        {:else}
            <div class="px-4 py-3 text-sm text-anasthasia-muted">
                {pipelineError || 'Preparing pipeline…'}
            </div>
        {/if}
    </div>
</WizardStep>
