<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';
  import { onMount } from 'svelte';

  const KEY = 'thasia:settings';

  type Defaults = {
    imageFormat: 'avif' | 'webp' | 'original';
    container: 'cbz' | 'epub' | 'raw';
    direction: 'ltr' | 'rtl';
    bundle: 'auto' | 'flatten';
    volumeSeparator: string;
    hideSingleVolume: boolean;
    createDirectory: boolean;
    maxWidth: number | null;
  };

  let defaults = $state<Defaults>({
    imageFormat: 'avif',
    container: 'cbz',
    direction: 'ltr',
    bundle: 'auto',
    volumeSeparator: ' - ',
    hideSingleVolume: false,
    createDirectory: false,
    maxWidth: null,
  });

  let saved = $state(false);

  onMount(() => {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      try { Object.assign(defaults, JSON.parse(raw)); } catch {}
    }
  });

  function save() {
    localStorage.setItem(KEY, JSON.stringify(defaults));
    wizard.imageFormat = defaults.imageFormat;
    wizard.container = defaults.container;
    wizard.direction = defaults.direction;
    wizard.bundle = defaults.bundle;
    wizard.volumeSeparator = defaults.volumeSeparator;
    wizard.hideSingleVolume = defaults.hideSingleVolume;
    wizard.createDirectory = defaults.createDirectory;
    wizard.maxWidth = defaults.maxWidth;
    saved = true;
    setTimeout(() => (saved = false), 2000);
  }
</script>

<div style="padding:48px;max-width:600px;margin:0 auto;">
  <a href="/">← Home</a>
  <h1>Settings</h1>
  <p style="color:#6b7280;">These defaults pre-fill the wizard on each new conversion.</p>

  <fieldset style="margin-bottom:16px;">
    <legend>Image Format</legend>
    <label><input type="radio" bind:group={defaults.imageFormat} value="avif" /> AVIF</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.imageFormat} value="webp" /> WebP</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.imageFormat} value="original" /> Original</label>
  </fieldset>

  <fieldset style="margin-bottom:16px;">
    <legend>Output Container</legend>
    <label><input type="radio" bind:group={defaults.container} value="cbz" /> CBZ</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.container} value="epub" /> EPUB</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.container} value="raw" /> Raw</label>
  </fieldset>

  <fieldset style="margin-bottom:16px;">
    <legend>EPUB Direction</legend>
    <label><input type="radio" bind:group={defaults.direction} value="ltr" /> LTR</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.direction} value="rtl" /> RTL</label>
  </fieldset>

  <fieldset style="margin-bottom:16px;">
    <legend>Bundling</legend>
    <label><input type="radio" bind:group={defaults.bundle} value="auto" /> Auto</label>
    <label style="margin-left:12px;"><input type="radio" bind:group={defaults.bundle} value="flatten" /> Flatten</label>
    {#if defaults.bundle === 'auto'}
      <div style="margin-top:8px;">
        <label>Volume separator: <input type="text" bind:value={defaults.volumeSeparator} style="width:80px;" /></label>
        <label style="margin-left:12px;"><input type="checkbox" bind:checked={defaults.hideSingleVolume} /> Hide single volume number</label>
      </div>
    {/if}
  </fieldset>

  <label style="display:block;margin-bottom:16px;">
    <input type="checkbox" bind:checked={defaults.createDirectory} />
    Create named subdirectory by default
  </label>

  <button onclick={save} style="padding:8px 24px;">Save defaults</button>
  {#if saved}<span style="margin-left:12px;color:#10b981;">Saved!</span>{/if}
</div>
