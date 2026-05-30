<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { commands } from '$types/bindings';
    import { Alert, Button, Input, keyboard, Toggle } from 'anasthasia';
    import {
        IconAlertCircle,
        IconFileZip,
        IconFolderOpen,
        IconFolderPlus,
        IconTag,
    } from '@tabler/icons-svelte';
    import WizardStep from './WizardStep.svelte';

    let {
        onNext,
        onBack,
        backDisabled = true,
    }: {
        onNext: () => void;
        onBack: () => void;
        nextDisabled?: boolean;
        backDisabled?: boolean;
    } = $props();

    let loading = $state(false);
    let scanError = $state('');
    let dirError = $state('');
    let nameError = $state('');

    function inferOutputName(path: string) {
        const base = path.split(/[\\/]/).at(-1) ?? '';
        return base.replace(/\.(zip|cbz)$/i, '') || base;
    }

    function applySource(path: string) {
        wizard.sourcePath = path;
        wizard.scanResult = null;
        wizard.pageEdits = [];
        if (wizard.outputName === 'output' || wizard.outputName === '') {
            wizard.outputName = inferOutputName(path);
        }
    }

    async function pickSource() {
        const selected = await open({
            directory: true,
            multiple: false,
            title: 'Select manga source folder',
        });
        if (typeof selected === 'string') applySource(selected);
    }

    async function pickArchive() {
        const selected = await open({
            filters: [{ name: 'Archive', extensions: ['zip', 'cbz'] }],
            multiple: false,
            title: 'Select ZIP or CBZ file',
        });
        if (typeof selected === 'string') applySource(selected);
    }

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
        if (!wizard.sourcePath) issues.push('Select a source folder or archive.');
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

    async function handleNext() {
        if (wizard.scanResult && wizard.pageEdits.length > 0) {
            onNext();
            return;
        }
        loading = true;
        scanError = '';
        try {
            const result = await commands.scanSource(wizard.sourcePath);
            if (result.status === 'error') {
                scanError = result.error;
                return;
            }
            wizard.scanResult = result.data;
            wizard.pageEdits = result.data.map((vol) => ({
                volumeNum: vol.volume_num,
                pages: vol.pages.map((_, i) => ({
                    originalPageIndex: i,
                    sourceVolumeNum: vol.volume_num,
                    customPath: null,
                    excluded: false,
                })),
            }));
            onNext();
        } catch (e) {
            scanError = String(e);
        } finally {
            loading = false;
        }
    }

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            ['o', () => (pickSource(), true)],
            ['z', () => (pickArchive(), true)],
            ['d', () => (pickDir(), true)],
            [
                'shift+arrowright',
                (e) => {
                    if (validate()) return false;
                    e.preventDefault();
                    handleNext();
                    return true;
                },
            ],
        ]);
    });
    onDestroy(() => cleanupKb?.());

    $effect(() => {
        if (wizard.outputDir && dirError) dirError = '';
    });
    $effect(() => {
        if (wizard.outputName.trim() && nameError) nameError = '';
    });
</script>

<WizardStep
    title="Setup"
    description="Choose input and where the converted output should be written."
    onNext={handleNext}
    {onBack}
    {backDisabled}
    {validate}
    {loading}
    loadingLabel="Scanning…"
    selfManagedNext
    extraHints={[
        ['keyo', 'Folder'],
        ['keyz', 'Archive'],
        ['keyd', 'Destination'],
    ]}
>
    <div
        class="grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] 2xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]"
    >
        <div
            class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                    Source
                </span>
            </div>
            <div class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center">
                <div class="flex w-24 flex-shrink-0 items-center gap-2">
                    <IconFolderOpen size={14} class="flex-shrink-0 text-anasthasia-muted" />
                    <span class="text-sm font-medium">Source</span>
                </div>
                <div
                    class="flex h-9 min-w-0 flex-1 items-center rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 font-mono text-xs {wizard.sourcePath
                        ? 'text-anasthasia-text'
                        : 'text-anasthasia-muted'}"
                    title={wizard.sourcePath || undefined}
                >
                    <span class="truncate">{wizard.sourcePath || 'No source selected'}</span>
                </div>
                <div class="flex flex-shrink-0 gap-1.5">
                    <Button onclick={pickSource} size="sm">
                        <IconFolderOpen size={13} /> Folder
                    </Button>
                    <Button onclick={pickArchive} size="sm">
                        <IconFileZip size={13} /> Archive
                    </Button>
                </div>
            </div>
        </div>

        <div
            class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
        >
            <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
                    Destination
                </span>
            </div>
            <div class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-start">
                <div class="flex w-24 flex-shrink-0 items-center gap-2 sm:pt-2">
                    <IconFolderOpen size={14} class="flex-shrink-0 text-anasthasia-muted" />
                    <span class="text-sm font-medium">Folder</span>
                </div>
                <div class="min-w-0 flex-1">
                    <div class="flex flex-col gap-2 sm:flex-row">
                        <div
                            class="flex h-9 min-w-0 flex-1 items-center rounded-lg border bg-anasthasia-bg px-3 font-mono text-xs transition-colors duration-150
                        {dirError
                                ? 'border-red-500/40 bg-red-500/5'
                                : wizard.outputDir
                                  ? 'border-anasthasia-border text-anasthasia-text'
                                  : 'border-anasthasia-border text-anasthasia-muted'}"
                        >
                            <span class="truncate" title={wizard.outputDir || undefined}>
                                {wizard.outputDir || 'No folder selected'}
                            </span>
                        </div>
                        <Button onclick={pickDir} size="sm" class="sm:flex-shrink-0">Browse…</Button
                        >
                    </div>
                    {#if dirError}
                        <div class="flex items-center gap-1 text-xs text-red-400">
                            <IconAlertCircle size={12} />
                            {dirError}
                        </div>
                    {/if}
                </div>
            </div>

            <div class="mx-4 border-t border-anasthasia-border"></div>

            <div class="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-start">
                <div class="flex w-24 flex-shrink-0 items-center gap-2 sm:pt-2">
                    <IconTag size={14} class="flex-shrink-0 text-anasthasia-muted" />
                    <span class="text-sm font-medium">Name</span>
                </div>
                <div class="min-w-0 flex-1">
                    <Input
                        bind:value={wizard.outputName}
                        error={nameError}
                        hint={nameError ? undefined : 'Volume numbers are appended automatically'}
                    />
                </div>
            </div>

            <div class="mx-4 border-t border-anasthasia-border"></div>

            <div class="flex items-center justify-between gap-4 px-4 py-2.5">
                <div class="flex min-w-0 items-center gap-2">
                    <IconFolderPlus size={14} class="flex-shrink-0 text-anasthasia-muted" />
                    <div>
                        <div class="text-sm font-medium">Create subdirectory</div>
                    </div>
                </div>
                <Toggle bind:checked={wizard.createDirectory} />
            </div>
        </div>
    </div>

    {#if scanError}
        <Alert variant="danger" title="Scan failed">{scanError}</Alert>
    {/if}
</WizardStep>
