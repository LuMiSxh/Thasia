<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

  let totalPages = $derived(
    wizard.pageEdits.reduce((acc, vol) => acc + vol.pages.filter(p => !p.excluded).length, 0)
  );
</script>

<h2>Review</h2>
<p>Check your settings before converting.</p>

<table style="width:100%;border-collapse:collapse;">
  <tbody>
    <tr><td style="padding:6px;color:#6b7280;">Source</td><td style="padding:6px;">{wizard.sourcePath}</td></tr>
    <tr><td style="padding:6px;color:#6b7280;">Output</td><td style="padding:6px;">{wizard.outputDir} / {wizard.outputName}</td></tr>
    <tr><td style="padding:6px;color:#6b7280;">Image format</td><td style="padding:6px;">{wizard.imageFormat.toUpperCase()}{wizard.maxWidth ? ` (max ${wizard.maxWidth}px wide)` : ''}</td></tr>
    <tr><td style="padding:6px;color:#6b7280;">Container</td><td style="padding:6px;">{wizard.container.toUpperCase()}</td></tr>
    {#if wizard.container === 'epub'}
      <tr><td style="padding:6px;color:#6b7280;">Direction</td><td style="padding:6px;">{wizard.direction.toUpperCase()}</td></tr>
    {/if}
    <tr><td style="padding:6px;color:#6b7280;">Bundling</td><td style="padding:6px;">{wizard.bundle}</td></tr>
    <tr><td style="padding:6px;color:#6b7280;">Volumes</td><td style="padding:6px;">{wizard.pageEdits.length}</td></tr>
    <tr><td style="padding:6px;color:#6b7280;">Total pages</td><td style="padding:6px;">{totalPages}</td></tr>
  </tbody>
</table>

<div style="margin-top:24px;">
  <button onclick={onBack}>← Back</button>
  <button onclick={onNext} style="background:#6366f1;color:white;padding:8px 24px;">Start Converting →</button>
</div>
