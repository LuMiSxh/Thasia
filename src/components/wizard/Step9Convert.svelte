<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { wizard } from '$lib/wizard/state.svelte';
  import { commands, events } from '$types/bindings';
  import { goto } from '$app/navigation';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

  type VolumeProgress = { name: string; current: number; total: number; done: boolean; success?: boolean };

  let status = $state<'idle' | 'converting' | 'done' | 'error'>('idle');
  let volumeMap = $state(new Map<number, VolumeProgress>());
  let errorMessage = $state('');
  let duration = $state(0);

  let unlisteners: Array<() => void> = [];

  function capitalize(s: string): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  onMount(async () => {
    unlisteners.push(
      await events.volumeStartEvent.listen((e) => {
        volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
          name: e.payload.volume_name,
          current: 0,
          total: 0,
          done: false,
        });
      })
    );
    unlisteners.push(
      await events.imageProgressEvent.listen((e) => {
        const existing = volumeMap.get(e.payload.volume_num);
        if (existing) {
          volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
            ...existing,
            current: e.payload.current,
            total: e.payload.total,
          });
        }
      })
    );
    unlisteners.push(
      await events.volumeCompleteEvent.listen((e) => {
        const existing = volumeMap.get(e.payload.volume_num);
        if (existing) {
          volumeMap = new Map(volumeMap).set(e.payload.volume_num, {
            ...existing,
            done: true,
            success: e.payload.success,
          });
        }
      })
    );
    unlisteners.push(
      await events.conversionCompleteEvent.listen((e) => {
        duration = e.payload.duration_secs;
        status = 'done';
      })
    );

    status = 'converting';
    try {
      const result = await commands.convert(
        {
          output_dir: wizard.outputDir,
          output_name: wizard.outputName,
          create_directory: wizard.createDirectory,
          image_format: capitalize(wizard.imageFormat) as 'Avif' | 'Webp' | 'Original',
          max_width: wizard.maxWidth,
          output_format: capitalize(wizard.container) as 'Cbz' | 'Epub' | 'Raw',
          direction: capitalize(wizard.direction) as 'Ltr' | 'Rtl',
          bundle: wizard.bundle,
          volume_separator: wizard.volumeSeparator,
          hide_single_volume: wizard.hideSingleVolume,
        },
        wizard.pageEdits.map((vol) => ({
          volume_num: vol.volumeNum,
          pages: vol.pages.map((p) => ({
            original_page_index: p.originalPageIndex,
            source_volume_num: p.sourceVolumeNum,
            custom_path: p.customPath,
            excluded: p.excluded,
          })),
        }))
      );
      if (result.status === 'error') {
        status = 'error';
        errorMessage = result.error;
      }
    } catch (e) {
      status = 'error';
      errorMessage = String(e);
    }
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
  });
</script>

<h2>{status === 'done' ? 'Done!' : status === 'error' ? 'Conversion failed' : 'Converting…'}</h2>

{#if status === 'converting' || status === 'done'}
  {#each [...volumeMap.entries()] as [num, vol]}
    <div style="margin-bottom:12px;">
      <div style="display:flex;justify-content:space-between;margin-bottom:4px;">
        <span>{vol.name}</span>
        <span style="color:{vol.done && vol.success ? '#10b981' : vol.done ? '#ef4444' : '#6b7280'};">
          {vol.done ? (vol.success ? '✓ Done' : '✗ Failed') : `${vol.current}/${vol.total}`}
        </span>
      </div>
      <div style="background:#374151;border-radius:4px;height:8px;">
        <div style="background:#6366f1;border-radius:4px;height:8px;width:{vol.total ? (vol.current/vol.total*100) : 0}%;transition:width .3s;"></div>
      </div>
    </div>
  {/each}
{/if}

{#if status === 'done'}
  <p style="color:#10b981;">All done in {duration.toFixed(1)}s!</p>
  <button onclick={() => { wizard.reset(); goto('/'); }}>Start over</button>
{/if}

{#if status === 'error'}
  <p style="color:#ef4444;">Error: {errorMessage}</p>
  <button onclick={onBack}>← Back</button>
{/if}
