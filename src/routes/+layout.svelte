<script lang="ts">
    import '../app.css';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import Sidebar from '$components/Sidebar.svelte';
    import { KeyHintBar } from '$components/ui/index';
    import { theme } from '$lib/theme.svelte';
    import { keyboard } from '$lib/keyboard';
    import { mountedHint } from '$lib/keyhint.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';

    let { children } = $props();

    const navRoutes: [string, string, string][] = [
        ['meta+1', 'Home', '/'],
        ['meta+2', 'Convert', '/convert'],
        ['meta+3', 'Settings', '/settings'],
        ['meta+4', 'About', '/about'],
    ];

    let navHints = $derived(
        (
            navRoutes
                .filter(([, , route]) =>
                    route === '/' ? page.url.pathname !== '/' : !page.url.pathname.startsWith(route)
                )
                .map(([key, label]) => [key, label]) as [string, string][]
        ).concat([['meta+keyb', 'Sidebar']])
    );

    onMount(() => {
        theme.init();
        uiPrefs.init();
        const unmount = keyboard.mount();
        const cleanup = keyboard.smartRegister([
            [
                'meta+1',
                () => {
                    goto('/');
                    return true;
                },
            ],
            [
                'meta+2',
                () => {
                    goto('/convert');
                    return true;
                },
            ],
            [
                'meta+3',
                () => {
                    goto('/settings');
                    return true;
                },
            ],
            [
                'meta+4',
                () => {
                    goto('/about');
                    return true;
                },
            ],
            [
                'meta+keyb',
                () => {
                    sidebar.toggle();
                    return true;
                },
            ],
        ]);
        return () => {
            unmount();
            cleanup();
        };
    });
</script>

<div
    class="flex h-screen flex-col overflow-hidden bg-thasia-bg text-thasia-text"
    use:mountedHint={navHints}
>
    <!-- macOS title bar: sits behind traffic lights, draggable -->
    <div
        class="titlebar h-8 flex-shrink-0 border-b border-thasia-border bg-thasia-surface"
        data-tauri-drag-region
    ></div>

    <div class="flex flex-1 overflow-hidden">
        <Sidebar />
        <main class="flex flex-1 flex-col overflow-hidden">
            <div class="flex-1 overflow-auto">
                {@render children()}
            </div>
            <KeyHintBar />
        </main>
    </div>
</div>
