<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import Sidebar from '$components/Sidebar.svelte';
    import { theme } from '$lib/theme.svelte';

    let { children } = $props();

    onMount(() => {
        theme.init();
    });
</script>

<div class="flex h-screen flex-col overflow-hidden bg-thasia-bg text-thasia-text">
    <!-- macOS title bar: sits behind traffic lights, draggable -->
    <div
        class="titlebar h-8 flex-shrink-0 border-b border-thasia-border bg-thasia-surface"
        data-tauri-drag-region
    ></div>

    <div class="flex flex-1 overflow-hidden">
        <Sidebar />
        <main class="flex-1 overflow-auto">
            {@render children()}
        </main>
    </div>
</div>
