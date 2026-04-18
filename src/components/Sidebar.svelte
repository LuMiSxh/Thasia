<script lang="ts">
  import { sidebar } from '$lib/sidebar/state.svelte';
  import { wizard } from '$lib/wizard/state.svelte';
  import { STEPS } from '$lib/wizard/steps';

  let sidebarSteps = $derived(
    STEPS.map((s) => ({
      id: s.id,
      label: s.label,
      status: (
        wizard.completedStepIds.has(s.id) ? 'done' :
        s.id === wizard.currentStepId ? 'active' :
        (s.condition && !s.condition(wizard)) ? 'conditional' :
        'locked'
      ) as 'done' | 'active' | 'locked' | 'conditional',
    }))
  );

  function handleStepClick(id: string, status: string) {
    if (status === 'done') {
      document.dispatchEvent(new CustomEvent('wizard:goto', { detail: id }));
    }
  }
</script>

<!-- Toggle tab — always visible on the left edge -->
<button
  onclick={() => sidebar.toggle()}
  style="position:fixed;left:0;top:50%;transform:translateY(-50%);z-index:100;
         width:20px;height:60px;background:#1f2937;border:none;cursor:pointer;
         border-radius:0 6px 6px 0;"
  aria-label="Toggle sidebar"
>
  {sidebar.isOpen ? '‹' : '›'}
</button>

{#if sidebar.isOpen}
  <nav style="position:fixed;left:20px;top:0;bottom:0;width:180px;background:#111827;
              border-right:1px solid #374151;z-index:99;display:flex;flex-direction:column;
              padding:16px 12px;">

    {#if sidebar.mode === 'nav'}
      <div style="font-weight:bold;margin-bottom:16px;">Thasia</div>
      <a href="/" style="display:block;padding:6px 8px;margin-bottom:4px;">Home</a>
      <a href="/convert" style="display:block;padding:6px 8px;margin-bottom:4px;">Convert</a>
      <a href="/settings" style="display:block;padding:6px 8px;">Settings</a>

    {:else}
      <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;
                  margin-bottom:12px;color:#6b7280;">Wizard</div>
      {#each sidebarSteps as step}
        <button
          onclick={() => handleStepClick(step.id, step.status)}
          disabled={step.status === 'locked' || step.status === 'conditional'}
          style="display:flex;align-items:center;gap:8px;padding:5px 6px;
                 border:none;background:none;width:100%;text-align:left;
                 cursor:{step.status === 'done' ? 'pointer' : 'default'};
                 opacity:{step.status === 'conditional' ? 0.4 : 1};"
        >
          <span style="
            width:16px;height:16px;border-radius:50%;flex-shrink:0;display:flex;
            align-items:center;justify-content:center;font-size:9px;
            background:{step.status === 'done' ? '#10b981' : step.status === 'active' ? '#6366f1' : 'transparent'};
            border:{step.status === 'locked' ? '1px solid #374151' : step.status === 'conditional' ? '1px dashed #374151' : 'none'};
            color:{step.status === 'done' || step.status === 'active' ? 'white' : '#6b7280'};
          ">
            {step.status === 'done' ? '✓' : ''}
          </span>
          <span style="font-size:11px;font-style:{step.status === 'conditional' ? 'italic' : 'normal'};">
            {step.label}
          </span>
        </button>
      {/each}
    {/if}
  </nav>

  <div style="width:200px;flex-shrink:0;"></div>
{/if}
