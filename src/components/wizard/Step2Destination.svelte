<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { Button, Input, Toggle } from 'anasthasia';
    import {
        IconFolderOpen,
        IconTag,
        IconFolderPlus,
        IconAlertCircle,
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

    let dirError = $state('');
    let nameError = $state('');

    async function pickDir() {
        const selected = await open({ directory: true, title: 'Select output folder' });
        if (typeof selected === 'string') {
            wizard.outputDir = selected;
            dirError = '';
        }
    }

    function validate() {
        const issues: string[] = [];
        dirError = '';
        nameError = '';
        if (!wizard.outputDir) {
            dirError = 'Pick a folder';
            issues.push('Select an output folder.');
        }
        if (!wizard.outputName.trim()) {
            nameError = 'Required';
            issues.push('Enter an output name.');
        }
        return issues.length ? issues : null;
    }

    // Clear field errors as the user fixes them
    $effect(() => {
        if (wizard.outputDir && dirError) dirError = '';
    });
    $effect(() => {
        if (wizard.outputName.trim() && nameError) nameError = '';
    });
</script>

<WizardStep
    title="Destination"
    description="Where to save the converted output."
    {onNext}
    {onBack}
    {backDisabled}
    {validate}
>
    <div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
        <!-- Output folder -->
        <div class="flex flex-col gap-2.5 px-4 py-4">
            <div class="flex items-center gap-2">
                <IconFolderOpen size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <span class="text-sm font-medium">Output folder</span>
            </div>
            <div class="flex gap-2">
                <div
                    class="flex min-h-9 flex-1 items-center rounded-lg border bg-anasthasia-bg px-3 py-2 font-mono text-xs break-all transition-colors duration-150
                        {dirError
                        ? 'border-red-500/40 bg-red-500/5'
                        : wizard.outputDir
                          ? 'border-anasthasia-border text-anasthasia-text'
                          : 'border-anasthasia-border text-anasthasia-muted'}"
                >
                    {wizard.outputDir || 'No folder selected'}
                </div>
                <Button onclick={pickDir} size="sm">Browse…</Button>
            </div>
            {#if dirError}
                <div class="flex items-center gap-1 text-xs text-red-400">
                    <IconAlertCircle size={12} />
                    {dirError}
                </div>
            {/if}
        </div>

        <div class="mx-4 border-t border-anasthasia-border"></div>

        <!-- Output name -->
        <div class="flex flex-col gap-2.5 px-4 py-4">
            <div class="flex items-center gap-2">
                <IconTag size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <span class="text-sm font-medium">Output name</span>
            </div>
            <Input
                bind:value={wizard.outputName}
                error={nameError}
                hint={nameError
                    ? undefined
                    : 'Base filename — volume numbers are appended automatically'}
            />
        </div>

        <div class="mx-4 border-t border-anasthasia-border"></div>

        <!-- Subdirectory toggle -->
        <div class="flex items-center justify-between gap-4 px-4 py-4">
            <div class="flex items-center gap-2">
                <IconFolderPlus size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <div>
                    <div class="text-sm font-medium">Create subdirectory</div>
                    <div class="text-xs text-anasthasia-muted">Wrap output in a named folder</div>
                </div>
            </div>
            <Toggle bind:checked={wizard.createDirectory} />
        </div>
    </div>
</WizardStep>
