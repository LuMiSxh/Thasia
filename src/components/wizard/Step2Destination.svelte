<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { Button, Input, Toggle } from '$components/ui/index';
    import {
        IconAlertCircle,
        IconArrowLeft,
        IconArrowRight,
        IconFolderOpen,
        IconTag,
        IconFolderPlus,
    } from '@tabler/icons-svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
    let error = $state('');

    async function pickDir() {
        const selected = await open({ directory: true, title: 'Select output folder' });
        if (typeof selected === 'string') wizard.outputDir = selected;
    }

    function handleNext() {
        if (!wizard.outputDir) {
            error = 'Please select an output folder.';
            return;
        }
        if (!wizard.outputName) {
            error = 'Please enter a name.';
            return;
        }
        onNext();
    }
</script>

<div class="flex h-full flex-col">
    <div class="flex-shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Destination</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">Where to save the converted output.</p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <!-- Output folder -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconFolderOpen size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Output folder</span>
                </div>
                <div class="flex gap-2">
                    <div
                        class="flex-1 rounded-lg border border-thasia-border bg-thasia-bg px-3 py-2 font-mono text-xs
                                {wizard.outputDir ? 'text-thasia-text' : 'text-thasia-muted'}"
                    >
                        {wizard.outputDir || 'No folder selected'}
                    </div>
                    <Button onclick={pickDir} size="sm">Browse…</Button>
                </div>
            </div>

            <div class="mx-4 border-t border-thasia-border"></div>

            <!-- Output name -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconTag size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Output name</span>
                </div>
                <Input
                    bind:value={wizard.outputName}
                    hint="Base filename — volume numbers are appended automatically"
                />
            </div>

            <div class="mx-4 border-t border-thasia-border"></div>

            <!-- Subdirectory toggle -->
            <div class="flex items-center justify-between gap-4 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconFolderPlus size={14} class="flex-shrink-0 text-thasia-muted" />
                    <div>
                        <div class="text-sm font-medium">Create subdirectory</div>
                        <div class="text-xs text-thasia-muted">Wrap output in a named folder</div>
                    </div>
                </div>
                <Toggle bind:checked={wizard.createDirectory} />
            </div>
        </div>

        {#if error}
            <div
                class="flex items-center gap-2 rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-xs text-red-400"
            >
                <IconAlertCircle size={14} class="flex-shrink-0" />
                {error}
            </div>
        {/if}
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={handleNext} class="ml-auto">Next <IconArrowRight size={15} /></Button>
    </div>
</div>
