<script lang="ts">
  import { wizard } from '$lib/wizard/state.svelte';
  import type { VolumeEdit } from '$lib/wizard/state.svelte';

  let { onNext, onBack }: { onNext: () => void; onBack: () => void } = $props();

  // When multiple scan volumes were detected, the unit is "chapters" (one scan vol = one chapter).
  // For a flat/ZIP source with a single scan volume, the unit is individual pages.
  let scanVols = $derived(wizard.scanResult ?? []);
  let unit = $derived(scanVols.length > 1 ? 'chapter' : 'page');
  let total = $derived(
    unit === 'chapter'
      ? scanVols.length
      : (scanVols[0]?.pages.length ?? 0)
  );

  // Volume sizes: just an array of item-counts (chapters or pages per output volume).
  let volumeSizes = $state<number[]>([]);
  let newVolumeInput = $state('');
  let editingIndex = $state<number | null>(null);

  // Initialize from current pageEdits on first load.
  $effect.pre(() => {
    if (volumeSizes.length === 0 && wizard.pageEdits.length > 0) {
      if (unit === 'chapter') {
        volumeSizes = wizard.pageEdits.map((ve) => {
          // Count how many distinct source volumes this VolumeEdit covers.
          const srcVols = new Set(ve.pages.map((p) => p.sourceVolumeNum));
          return Math.max(1, srcVols.size);
        });
      } else {
        volumeSizes = wizard.pageEdits.map((ve) => ve.pages.length);
      }
    }
  });

  let used = $derived(volumeSizes.reduce((s, n) => s + n, 0));
  let remaining = $derived(total - used);
  let isOver = $derived(used > total);
  let isValid = $derived(remaining === 0 && volumeSizes.length > 0 && volumeSizes.every((n) => n > 0));

  // Visual item map: for each item index, which output volume owns it (-1 = unassigned).
  let itemVolumeMap = $derived((() => {
    const map = new Array(total).fill(-1);
    let offset = 0;
    for (let v = 0; v < volumeSizes.length; v++) {
      for (let i = 0; i < volumeSizes[v] && offset < total; i++) map[offset++] = v;
    }
    return map;
  })());

  // Alternating accent colours for volume slots in the map.
  const slotColors = ['#6366f1', '#8b5cf6', '#a78bfa', '#c4b5fd'];
  function slotColor(volumeIndex: number): string {
    return slotColors[volumeIndex % slotColors.length];
  }

  function addVolume() {
    const n = Number(newVolumeInput);
    if (!n || n <= 0 || isNaN(n)) return;
    volumeSizes = [...volumeSizes, n];
    newVolumeInput = '';
  }

  function deleteVolume(i: number) {
    volumeSizes = volumeSizes.filter((_, idx) => idx !== i);
    if (editingIndex === i) editingIndex = null;
  }

  function handleAddKey(e: KeyboardEvent) {
    if (e.key === 'Enter') addVolume();
  }

  function handleNext() {
    if (!isValid) return;

    let newEdits: VolumeEdit[];

    if (unit === 'chapter') {
      // Assign whole scan volumes to output volumes.
      let chapOffset = 0;
      newEdits = volumeSizes.map((count, outIdx) => {
        const assignedScanVols = scanVols.slice(chapOffset, (chapOffset += count));
        return {
          volumeNum: outIdx + 1,
          pages: assignedScanVols.flatMap((sv) =>
            sv.pages.map((_, pi) => ({
              originalPageIndex: pi,
              sourceVolumeNum: sv.volume_num,
              customPath: null as string | null,
              excluded: false,
            }))
          ),
        };
      });
    } else {
      // Assign individual pages from the single scan volume.
      const srcVol = scanVols[0];
      let off = 0;
      newEdits = volumeSizes.map((count, outIdx) => {
        const start = off;
        off += count;
        return {
          volumeNum: outIdx + 1,
          pages: Array.from({ length: count }, (_, pi) => ({
            originalPageIndex: start + pi,
            sourceVolumeNum: srcVol?.volume_num ?? 1,
            customPath: null as string | null,
            excluded: false,
          })),
        };
      });
    }

    wizard.pageEdits = newEdits;
    onNext();
  }
</script>

<div style="display:flex;flex-direction:column;gap:16px;padding:16px;height:calc(100vh - 120px);overflow:hidden;">

  <!-- Stat row -->
  <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:8px;flex-shrink:0;">
    <div style="background:#1f2937;border:1px solid #374151;border-radius:8px;padding:12px;">
      <div style="font-size:10px;text-transform:uppercase;color:#6b7280;margin-bottom:4px;">Detected</div>
      <div style="font-size:24px;font-weight:bold;">{total}</div>
      <div style="font-size:11px;color:#6b7280;">{unit}s</div>
    </div>
    <div style="background:#1f2937;border:1px solid #374151;border-radius:8px;padding:12px;">
      <div style="font-size:10px;text-transform:uppercase;color:#6b7280;margin-bottom:4px;">Used</div>
      <div style="font-size:24px;font-weight:bold;color:{isValid ? '#10b981' : isOver ? '#ef4444' : '#f59e0b'};">{used}</div>
      <div style="font-size:11px;color:#6b7280;">{remaining > 0 ? `${remaining} remaining` : isOver ? `${-remaining} over` : 'all assigned'}</div>
    </div>
    <div style="background:#1f2937;border:1px solid #374151;border-radius:8px;padding:12px;">
      <div style="font-size:10px;text-transform:uppercase;color:#6b7280;margin-bottom:4px;">Volumes</div>
      <div style="font-size:24px;font-weight:bold;">{volumeSizes.length}</div>
    </div>
  </div>

  <!-- Main area: volume list + distribution -->
  <div style="display:grid;grid-template-columns:1fr 220px;gap:8px;flex:1;min-height:0;">

    <!-- Volume list -->
    <div style="background:#1f2937;border:1px solid #374151;border-radius:8px;display:flex;flex-direction:column;overflow:hidden;">
      <div style="padding:10px 12px;border-bottom:1px solid #374151;font-size:11px;text-transform:uppercase;color:#6b7280;flex-shrink:0;">
        Volumes
      </div>
      <div style="flex:1;overflow-y:auto;padding:8px;">
        {#if volumeSizes.length === 0}
          <div style="height:100%;display:flex;align-items:center;justify-content:center;color:#6b7280;font-size:13px;">
            No volumes yet — add one below
          </div>
        {:else}
          {#each volumeSizes as count, i}
            {@const pct = Math.min((count / total) * 100, 100)}
            <div style="background:#111827;border:1px solid {editingIndex === i ? '#6366f1' : '#374151'};border-radius:6px;padding:10px 12px;margin-bottom:6px;">
              <div style="display:flex;align-items:center;gap:8px;">
                <div style="width:32px;height:32px;border-radius:50%;background:{slotColor(i)}22;display:flex;align-items:center;justify-content:center;flex-shrink:0;">
                  <span style="font-size:12px;font-weight:bold;color:{slotColor(i)};">{i+1}</span>
                </div>
                <div style="flex:1;min-width:0;">
                  <div style="font-size:13px;font-weight:500;margin-bottom:4px;">Volume {i + 1}</div>
                  <div style="background:#1f2937;height:4px;border-radius:2px;overflow:hidden;">
                    <div style="background:{slotColor(i)};height:4px;width:{pct}%;transition:width .2s;border-radius:2px;"></div>
                  </div>
                </div>
                {#if editingIndex === i}
                  <input
                    type="number"
                    min="1"
                    max={total}
                    value={count}
                    oninput={(e) => { volumeSizes[i] = parseInt((e.target as HTMLInputElement).value) || 0; }}
                    onblur={() => editingIndex = null}
                    onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Escape') editingIndex = null; }}
                    style="width:56px;height:32px;text-align:center;border:1px solid #6366f1;border-radius:6px;background:#111827;color:white;"
                  />
                {:else}
                  <button
                    onclick={() => editingIndex = i}
                    style="width:56px;height:32px;text-align:center;border:1px solid #374151;border-radius:6px;background:#111827;color:white;cursor:pointer;font-size:13px;font-weight:500;"
                    title="Click to edit"
                  >
                    {count}
                  </button>
                {/if}
                <span style="font-size:11px;color:#6b7280;white-space:nowrap;">
                  {unit}{count !== 1 ? 's' : ''}
                </span>
                <button
                  onclick={() => deleteVolume(i)}
                  style="width:28px;height:28px;border-radius:6px;border:1px solid #374151;background:transparent;color:#ef4444;cursor:pointer;display:flex;align-items:center;justify-content:center;font-size:14px;"
                >×</button>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Add volume row -->
      <div style="padding:8px;border-top:1px solid #374151;display:flex;gap:6px;flex-shrink:0;">
        <input
          type="number"
          min="1"
          placeholder="# of {unit}s"
          bind:value={newVolumeInput}
          onkeydown={handleAddKey}
          style="flex:1;padding:6px 8px;border:1px solid #374151;border-radius:6px;background:#111827;color:white;"
        />
        <button
          onclick={addVolume}
          disabled={!newVolumeInput || Number(newVolumeInput) <= 0}
          style="padding:6px 14px;border-radius:6px;background:#6366f1;color:white;border:none;cursor:pointer;"
        >
          Add
        </button>
      </div>
    </div>

    <!-- Distribution sidebar -->
    <div style="background:#1f2937;border:1px solid #374151;border-radius:8px;display:flex;flex-direction:column;overflow:hidden;">
      <div style="padding:10px 12px;border-bottom:1px solid #374151;font-size:11px;text-transform:uppercase;color:#6b7280;flex-shrink:0;">
        Distribution
      </div>
      <div style="flex:1;overflow-y:auto;padding:12px;">
        <!-- Progress bar -->
        <div style="margin-bottom:12px;">
          <div style="display:flex;justify-content:space-between;margin-bottom:4px;font-size:12px;">
            <span>Usage</span>
            <span style="font-family:monospace;">{used}/{total}</span>
          </div>
          <div style="background:#111827;height:6px;border-radius:3px;overflow:hidden;">
            <div style="height:6px;border-radius:3px;transition:width .3s;width:{Math.min((used/total)*100, 100)}%;background:{isValid ? '#10b981' : isOver ? '#ef4444' : '#f59e0b'};"></div>
          </div>
        </div>
        <!-- Item map -->
        <div style="font-size:11px;color:#6b7280;margin-bottom:6px;">{unit === 'chapter' ? 'Chapter' : 'Page'} map</div>
        <div style="display:flex;flex-wrap:wrap;gap:3px;">
          {#each itemVolumeMap as vIdx, itemI}
            <div
              style="width:10px;height:10px;border-radius:2px;cursor:help;transition:background .15s;
                     background:{vIdx === -1 ? '#374151' : slotColor(vIdx)};"
              title="{unit === 'chapter' ? 'Chapter' : 'Page'} {itemI + 1}{vIdx >= 0 ? ` → Volume ${vIdx + 1}` : ' (unassigned)'}"
            ></div>
          {/each}
        </div>
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div style="display:flex;gap:8px;flex-shrink:0;">
    <button onclick={onBack}>← Back</button>
    <button
      onclick={handleNext}
      disabled={!isValid}
      style="margin-left:auto;opacity:{isValid ? 1 : 0.5};"
      title={!isValid ? `${remaining > 0 ? remaining + ' unassigned' : 'over by ' + (-remaining)} — assign all ${unit}s first` : ''}
    >
      Next →
    </button>
  </div>
</div>
