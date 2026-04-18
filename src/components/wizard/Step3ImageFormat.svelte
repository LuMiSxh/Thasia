<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
  let enableMaxWidth = $state(wizard.maxWidth !== null);

  function toggleMaxWidth() {
    enableMaxWidth = !enableMaxWidth;
    wizard.maxWidth = enableMaxWidth ? 1800 : null;
  }
</script>

<h2>Image Format</h2>

<fieldset>
  <legend>Encoding</legend>
  <label><input type="radio" bind:group={wizard.imageFormat} value="avif" /> AVIF (best compression, slowest)</label><br />
  <label><input type="radio" bind:group={wizard.imageFormat} value="webp" /> WebP (good compression, faster)</label><br />
  <label><input type="radio" bind:group={wizard.imageFormat} value="original" /> Original (no re-encoding)</label>
</fieldset>

<div style="margin-top:16px;">
  <label>
    <input type="checkbox" checked={enableMaxWidth} onchange={toggleMaxWidth} />
    Downscale images wider than
  </label>
  {#if enableMaxWidth}
    <input type="number" bind:value={wizard.maxWidth} min="400" max="4000" step="100" style="width:80px;margin-left:8px;" />
    px
  {/if}
</div>

<div style="margin-top:24px;">
  <button onclick={onBack}>← Back</button>
  <button onclick={onNext}>Next →</button>
</div>
