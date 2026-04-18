<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';
  import { open } from '@tauri-apps/plugin-dialog';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();
  let error = $state('');

  async function pickDir() {
    const selected = await open({ directory: true, title: 'Select output folder' });
    if (typeof selected === 'string') wizard.outputDir = selected;
  }

  function handleNext() {
    if (!wizard.outputDir) { error = 'Please select an output folder.'; return; }
    if (!wizard.outputName) { error = 'Please enter a name.'; return; }
    onNext();
  }
</script>

<h2>Destination</h2>

<label>
  Output folder
  <div>
    <input type="text" readonly value={wizard.outputDir} placeholder="No folder selected" style="width:100%;padding:8px;" />
    <button onclick={pickDir}>Browse…</button>
  </div>
</label>

<label style="display:block;margin-top:16px;">
  Output name (base filename)
  <input type="text" bind:value={wizard.outputName} style="width:100%;padding:8px;margin-top:4px;" />
</label>

<label style="display:block;margin-top:16px;">
  <input type="checkbox" bind:checked={wizard.createDirectory} />
  Create a named subdirectory inside the output folder
</label>

{#if error}<p style="color:red;">{error}</p>{/if}

<div style="margin-top:24px;">
  <button onclick={onBack}>← Back</button>
  <button onclick={handleNext}>Next →</button>
</div>
