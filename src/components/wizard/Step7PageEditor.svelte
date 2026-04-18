<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { flip } from 'svelte/animate';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

  let activeVolumeIndex = $state(0);

  let volumes = $derived(wizard.pageEdits);
  let activeEdits = $derived(wizard.pageEdits[activeVolumeIndex]?.pages ?? []);
  let firstNonExcluded = $derived(activeEdits.findIndex((e) => !e.excluded));

  let dragOverIndex = $state<number | null>(null);

  function pageKey(edit: typeof activeEdits[number]): string {
    if (edit.customPath) return 'custom:' + edit.customPath;
    return `${edit.sourceVolumeNum ?? 0}-${edit.originalPageIndex ?? 0}`;
  }

  function setActiveVolume(i: number) {
    activeVolumeIndex = i;
  }

  function toggleExclude(pageIndex: number) {
    const ve = wizard.pageEdits[activeVolumeIndex];
    if (!ve) return;
    const pages = ve.pages.map((p, i) =>
      i === pageIndex ? { ...p, excluded: !p.excluded } : p
    );
    const updated = [...wizard.pageEdits];
    updated[activeVolumeIndex] = { ...ve, pages };
    wizard.pageEdits = updated;
  }

  function onDragStart(e: DragEvent, i: number) {
    e.dataTransfer?.setData('application/page-drag', String(i));
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function onDragEnter(e: DragEvent, i: number) {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer?.types.includes('application/page-drag')) dragOverIndex = i;
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer?.types.includes('application/page-drag')) e.dataTransfer.dropEffect = 'move';
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragOverIndex = null;
  }

  function onDrop(e: DragEvent, targetIndex: number) {
    e.preventDefault();
    e.stopPropagation();
    dragOverIndex = null;
    if (!e.dataTransfer?.types.includes('application/page-drag')) return;
    const fromIndex = parseInt(e.dataTransfer.getData('application/page-drag'));
    if (isNaN(fromIndex) || fromIndex === targetIndex) return;

    const ve = wizard.pageEdits[activeVolumeIndex];
    if (!ve) return;
    const pages = [...ve.pages];
    const [moved] = pages.splice(fromIndex, 1);
    pages.splice(targetIndex, 0, moved);
    const updated = [...wizard.pageEdits];
    updated[activeVolumeIndex] = { ...ve, pages };
    wizard.pageEdits = updated;
  }

  async function addCustomImage() {
    const selected = await open({
      filters: [{ name: 'Image', extensions: ['jpg', 'jpeg', 'png', 'webp', 'avif'] }],
      multiple: false,
      title: 'Add custom image',
    });
    if (typeof selected !== 'string') return;

    const ve = wizard.pageEdits[activeVolumeIndex];
    if (!ve) return;
    const updated = [...wizard.pageEdits];
    updated[activeVolumeIndex] = {
      ...ve,
      pages: [{ originalPageIndex: null, sourceVolumeNum: null, customPath: selected, excluded: false }, ...ve.pages],
    };
    wizard.pageEdits = updated;
  }

  function imageUrl(edit: typeof activeEdits[number]): string {
    if (edit.customPath) return `thasia://image?path=${encodeURIComponent(edit.customPath)}`;
    if (edit.originalPageIndex !== null) {
      const srcVol = wizard.scanResult?.find((v) => v.volume_num === (edit.sourceVolumeNum ?? wizard.pageEdits[activeVolumeIndex]?.volumeNum));
      return srcVol?.pages[edit.originalPageIndex]?.url ?? '';
    }
    return '';
  }

  function fileName(edit: typeof activeEdits[number]): string {
    if (edit.customPath) return edit.customPath.split('/').at(-1) ?? 'custom';
    if (edit.originalPageIndex !== null) {
      const srcVol = wizard.scanResult?.find((v) => v.volume_num === (edit.sourceVolumeNum ?? wizard.pageEdits[activeVolumeIndex]?.volumeNum));
      return srcVol?.pages[edit.originalPageIndex]?.file_name ?? '';
    }
    return '';
  }
</script>

<div style="display:flex;height:calc(100vh - 120px);gap:0;">

  <!-- Volume list -->
  <div style="width:160px;border-right:1px solid #374151;padding:12px;overflow-y:auto;flex-shrink:0;">
    <div style="font-size:11px;text-transform:uppercase;margin-bottom:8px;">Volumes</div>
    {#each volumes as ve, i}
      <button
        onclick={() => setActiveVolume(i)}
        style="width:100%;text-align:left;padding:6px 8px;margin-bottom:4px;
               background:{i === activeVolumeIndex ? '#1e1b4b' : 'transparent'};
               border:1px solid {i === activeVolumeIndex ? '#6366f1' : '#374151'};border-radius:4px;cursor:pointer;"
      >
        <div style="font-weight:bold;">Vol {ve.volumeNum}</div>
        <div style="font-size:10px;color:#6b7280;">{ve.pages.filter(p => !p.excluded).length} pages</div>
      </button>
    {/each}
  </div>

  <!-- Page grid -->
  <div style="flex:1;display:flex;flex-direction:column;overflow:hidden;">

    <!-- Toolbar -->
    <div style="padding:8px 16px;border-bottom:1px solid #374151;display:flex;align-items:center;gap:12px;">
      <span style="font-weight:bold;">Volume {volumes[activeVolumeIndex]?.volumeNum ?? '—'}</span>
      <span style="color:#6b7280;font-size:12px;">Drag to reorder · × to exclude · first image = cover</span>
      <button onclick={addCustomImage} style="margin-left:auto;">+ Add image</button>
    </div>

    <!-- Grid -->
    <div role="list" style="flex:1;padding:12px;overflow-y:auto;display:grid;grid-template-columns:repeat(auto-fill,minmax(90px,1fr));gap:8px;align-content:start;">
      {#each activeEdits as edit, i (pageKey(edit))}
        <div
          role="listitem"
          draggable="true"
          ondragstart={(e) => onDragStart(e, i)}
          ondragenter={(e) => onDragEnter(e, i)}
          ondragover={onDragOver}
          ondragleave={onDragLeave}
          ondrop={(e) => onDrop(e, i)}
          animate:flip={{ duration: 250 }}
          style="position:relative;opacity:{edit.excluded ? 0.4 : 1};"
        >
          <div style="
            aspect-ratio:2/3;border-radius:4px;overflow:hidden;
            border:2px solid {i === firstNonExcluded ? '#6366f1' : dragOverIndex === i ? '#a5b4fc' : edit.excluded ? '#ef4444' : edit.customPath ? '#10b981' : '#374151'};
            border-style:{edit.excluded ? 'dashed' : 'solid'};
            cursor:grab;position:relative;background:#1f2937;
          ">
            <img
              src={imageUrl(edit)}
              alt={fileName(edit)}
              draggable="false"
              style="width:100%;height:100%;object-fit:cover;"
              loading="lazy"
            />
          </div>

          {#if i === firstNonExcluded}
            <div style="position:absolute;top:-8px;left:50%;transform:translateX(-50%);
                        background:#6366f1;color:white;font-size:8px;font-weight:bold;
                        padding:1px 6px;border-radius:3px;white-space:nowrap;">COVER</div>
          {/if}

          {#if edit.customPath}
            <div style="position:absolute;top:{i === firstNonExcluded ? 4 : -8}px;left:50%;transform:translateX(-50%);
                        background:#10b981;color:white;font-size:8px;font-weight:bold;
                        padding:1px 6px;border-radius:3px;white-space:nowrap;">ADDED</div>
          {/if}

          <button
            onclick={() => toggleExclude(i)}
            style="position:absolute;top:2px;right:2px;width:16px;height:16px;border-radius:50%;
                   background:{edit.excluded ? '#10b981' : '#ef4444'};color:white;border:none;
                   cursor:pointer;font-size:9px;display:flex;align-items:center;justify-content:center;"
            title={edit.excluded ? 'Restore' : 'Exclude'}
          >
            {edit.excluded ? '↺' : '×'}
          </button>

          <div style="font-size:8px;color:#6b7280;text-align:center;margin-top:2px;
                      overflow:hidden;white-space:nowrap;text-overflow:ellipsis;">
            {fileName(edit)}
          </div>
        </div>
      {/each}
    </div>

    <!-- Footer -->
    <div style="padding:8px 16px;border-top:1px solid #374151;display:flex;gap:8px;">
      <button onclick={onBack}>← Back</button>
      <button onclick={onNext} style="margin-left:auto;">Next →</button>
    </div>
  </div>
</div>
