<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { commands } from '$types/bindings';
    import { Button } from '$components/ui/index';
    import {
        IconFolderOpen,
        IconFileZip,
        IconAlertCircle,
        IconArrowLeft,
        IconArrowRight,
    } from '@tabler/icons-svelte';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';

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
    let error = $state('');

    let nextHints = $derived(
        wizard.sourcePath ? [['shift+arrowright', 'Next step'] as [string, string]] : []
    );

    let cleanupKb: (() => void) | undefined;
    onMount(() => {
        cleanupKb = keyboard.smartRegister([
            [
                'o',
                () => {
                    pickSource();
                    return true;
                },
            ],
            [
                'z',
                () => {
                    pickArchive();
                    return true;
                },
            ],
            [
                'shift+arrowright',
                (e) => {
                    e.preventDefault();
                    handleNext();
                    return true;
                },
            ],
        ]);
    });
    onDestroy(() => cleanupKb?.());

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

    async function handleNext() {
        if (!wizard.sourcePath) {
            error = 'Please select a source.';
            return;
        }
        loading = true;
        error = '';
        try {
            const result = await commands.scanSource(wizard.sourcePath);
            if (result.status === 'error') {
                error = result.error;
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
            error = String(e);
        } finally {
            loading = false;
        }
    }
</script>

<div
    class="flex h-full flex-col"
    use:mountedHint={[['keyo', 'Open folder'], ['keyz', 'Open archive'], ...nextHints]}
>
    <div class="shrink-0 border-b border-thasia-border px-5 py-4">
        <h2 class="text-base font-bold">Source</h2>
        <p class="mt-0.5 text-xs text-thasia-muted">
            Select a folder, ZIP, or CBZ containing your manga images.
        </p>
    </div>

    <div class="flex flex-1 flex-col gap-3 overflow-y-auto px-5 py-5">
        <div class="overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface">
            <!-- Path display -->
            <div class="flex flex-col gap-2.5 px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconFolderOpen size={14} class="flex-shrink-0 text-thasia-muted" />
                    <span class="text-sm font-medium">Selected source</span>
                </div>
                <div
                    class="rounded-lg border border-thasia-border bg-thasia-bg px-3 py-2 font-mono text-xs
                            {wizard.sourcePath ? 'text-thasia-text' : 'text-thasia-muted'}"
                >
                    {wizard.sourcePath || 'No source selected'}
                </div>
            </div>

            <div class="mx-4 border-t border-thasia-border"></div>

            <!-- Browse buttons -->
            <div class="flex gap-2 px-4 py-4">
                <Button onclick={pickSource} class="flex-1">
                    <IconFolderOpen size={14} /> Browse folder…
                </Button>
                <Button onclick={pickArchive} class="flex-1">
                    <IconFileZip size={14} /> Browse archive…
                </Button>
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

        {#if loading}
            <p class="px-1 text-xs text-thasia-muted">Scanning source…</p>
        {/if}
    </div>

    <div class="flex flex-shrink-0 gap-2 border-t border-thasia-border px-5 py-4">
        <Button onclick={onBack} disabled={backDisabled}><IconArrowLeft size={15} /> Back</Button>
        <Button onclick={handleNext} disabled={loading || !wizard.sourcePath} class="ml-auto">
            {loading ? 'Scanning…' : 'Next'}
            <IconArrowRight size={15} />
        </Button>
    </div>
</div>
