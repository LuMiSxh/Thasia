<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { open } from '@tauri-apps/plugin-dialog';
    import { commands } from '$types/bindings';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
    let loading = $state(false);
    let error = $state('');

    function inferOutputName(path: string) {
        const base = path.split(/[\\/]/).at(-1) ?? '';
        // Strip known archive extensions
        return base.replace(/\.(zip|cbz)$/i, '') || base;
    }

    function applySource(path: string) {
        wizard.sourcePath = path;
        // Only infer if user hasn't customised the name yet
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

<h2>Source</h2>
<p>Select a folder, ZIP, or CBZ file containing your manga images.</p>

<div>
    <input
        type="text"
        readonly
        value={wizard.sourcePath}
        placeholder="No source selected"
        style="width:100%;padding:8px;margin-bottom:8px;"
    />
    <button onclick={pickSource}>Browse folder…</button>
    <button onclick={pickArchive}>Browse archive…</button>
</div>

{#if error}<p style="color:red;">{error}</p>{/if}
{#if loading}<p>Scanning…</p>{/if}

<div style="margin-top:24px;">
    <button onclick={onBack} disabled={true}>Back</button>
    <button onclick={handleNext} disabled={loading || !wizard.sourcePath}>Next →</button>
</div>
