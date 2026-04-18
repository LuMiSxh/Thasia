<script lang="ts">
  import '../app.css';
  import Sidebar from '$components/Sidebar.svelte';

  let { children } = $props();
</script>

<div style="display:flex;height:100vh;overflow:hidden;">
  <Sidebar />
  <main style="flex:1;overflow:auto;">
    {@render children()}
  </main>
</div>
