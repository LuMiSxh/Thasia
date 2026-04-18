<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';

  let version = $state('...');
  onMount(async () => {
    version = await getVersion();
  });
</script>

<div style="padding:48px;max-width:600px;margin:0 auto;">
  <h1>Thasia <span style="font-size:14px;color:#6b7280;">v{version}</span></h1>
  <p>Headless manga processing engine.</p>

  <div style="display:flex;flex-direction:column;gap:12px;margin-top:32px;">
    <a href="/convert" style="display:block;padding:16px;border:1px solid #374151;border-radius:8px;text-decoration:none;">
      <strong>▶ Start Converting</strong>
      <p style="margin:4px 0 0;color:#6b7280;font-size:13px;">Launch the conversion wizard</p>
    </a>
    <a href="/settings" style="display:block;padding:16px;border:1px solid #374151;border-radius:8px;text-decoration:none;">
      <strong>⚙ Settings</strong>
      <p style="margin:4px 0 0;color:#6b7280;font-size:13px;">Set default values for the wizard</p>
    </a>
  </div>
</div>
