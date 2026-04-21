<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

    function handleNext() {
        if (wizard.bundle === 'flatten' && wizard.pageEdits.length > 1) {
            // Collapse all volumes into one so the downstream steps see a single volume.
            const firstNum = wizard.pageEdits[0]?.volumeNum ?? 1;
            wizard.pageEdits = [
                {
                    volumeNum: firstNum,
                    pages: wizard.pageEdits.flatMap((ve) => ve.pages),
                },
            ];
        } else if (wizard.bundle === 'auto' && wizard.scanResult && wizard.pageEdits.length === 1) {
            // Restore auto grouping from scan result if user toggled back to auto after flattening.
            wizard.pageEdits = wizard.scanResult.map((vol) => ({
                volumeNum: vol.volume_num,
                pages: vol.pages.map((_, i) => ({
                    originalPageIndex: i,
                    sourceVolumeNum: vol.volume_num,
                    customPath: null,
                    excluded: false,
                })),
            }));
        }
        onNext();
    }
</script>

<h2>Bundling</h2>

<fieldset>
    <legend>Strategy</legend>
    <label>
        <input type="radio" bind:group={wizard.bundle} value="auto" />
        Auto — group chapters by detected volume number
    </label><br />
    <label>
        <input type="radio" bind:group={wizard.bundle} value="flatten" />
        Flatten — merge everything into a single output file
    </label>
</fieldset>

{#if wizard.bundle === 'auto'}
    <div style="margin-top:16px;padding:12px;border:1px solid #374151;">
        <label style="display:block;margin-bottom:8px;">
            Volume separator
            <input
                type="text"
                bind:value={wizard.volumeSeparator}
                style="width:120px;margin-left:8px;"
            />
            <span style="margin-left:8px;color:#6b7280;"
                >e.g. "{wizard.outputName}{wizard.volumeSeparator}1"</span
            >
        </label>
        <label>
            <input type="checkbox" bind:checked={wizard.hideSingleVolume} />
            Omit volume number when only one volume is produced
        </label>
    </div>
{/if}

<div style="margin-top:24px;">
    <button onclick={onBack}>← Back</button>
    <button onclick={handleNext}>Next →</button>
</div>
