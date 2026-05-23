<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { commands } from '$types/bindings';
    import { Alert, Button, keyboard, PathDisplay } from 'anasthasia';
    import { IconFolderOpen, IconFileZip } from '@tabler/icons-svelte';
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

    function inferOutputName(path: string) {
        const base = path.split(/[\\/]/).at(-1) ?? '';
        return base.replace(/\.(zip|cbz)$/i, '') || base;
    }

    function applySource(path: string) {
        wizard.sourcePath = path;
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

    function validate() {
        if (!wizard.sourcePath) return 'Select a source folder or archive to continue.';
        return null;
    }

    async function handleNext() {
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
            [
                'shift+arrowright',
                (e) => {
                    if (!wizard.sourcePath) return false;
                    e.preventDefault();
                    handleNext();
                    return true;
                },
            ],
        ]);
    });
    onDestroy(() => cleanupKb?.());
</script>

<WizardStep
    title="Source"
    description="Select a folder, ZIP, or CBZ containing your manga images."
    onNext={handleNext}
    {onBack}
    {backDisabled}
    {validate}
    {loading}
    loadingLabel="Scanning…"
    selfManagedNext
>
    <div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
        <div class="flex flex-col gap-2.5 px-4 py-4">
            <div class="flex items-center gap-2">
                <IconFolderOpen size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <span class="text-sm font-medium">Selected source</span>
            </div>
            <PathDisplay value={wizard.sourcePath} empty="No source selected" />
        </div>

        <div class="mx-4 border-t border-anasthasia-border"></div>

        <div class="flex gap-2 px-4 py-4">
            <Button onclick={pickSource} class="flex-1">
                <IconFolderOpen size={14} /> Browse folder…
            </Button>
            <Button onclick={pickArchive} class="flex-1">
                <IconFileZip size={14} /> Browse archive…
            </Button>
        </div>
    </div>

    {#if scanError}
        <Alert variant="danger" title="Scan failed">{scanError}</Alert>
    {/if}
</WizardStep>
