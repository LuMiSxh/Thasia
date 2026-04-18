<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
</script>

<h2>Reading Direction</h2>
<p>Applies to EPUB output only.</p>

<fieldset>
  <legend>Direction</legend>
  <label><input type="radio" bind:group={wizard.direction} value="ltr" /> Left-to-right (Western comics, manhwa)</label><br />
  <label><input type="radio" bind:group={wizard.direction} value="rtl" /> Right-to-left (manga, manhua)</label>
</fieldset>

<div style="margin-top:24px;">
  <button onclick={onBack}>← Back</button>
  <button onclick={onNext}>Next →</button>
</div>
