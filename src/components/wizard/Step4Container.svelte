<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';

    let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
</script>

<h2>Output Container</h2>

<fieldset>
    <legend>Format</legend>
    <label
        ><input type="radio" bind:group={wizard.container} value="cbz" /> CBZ — Comic Book ZIP (widely
        supported)</label
    ><br />
    <label
        ><input type="radio" bind:group={wizard.container} value="epub" /> EPUB 3 — Fixed layout (e-readers)</label
    ><br />
    <label
        ><input type="radio" bind:group={wizard.container} value="raw" /> Raw — flat image folder</label
    >
</fieldset>

<div style="margin-top:24px;">
    <button onclick={onBack}>← Back</button>
    <button onclick={onNext}>Next →</button>
</div>
