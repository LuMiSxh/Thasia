<script lang="ts">
    import 'anasthasia/bootstrap';
    import '../app.css';
    import { onMount } from 'svelte';
    import { goto } from '$app/navigation';
    import { page } from '$app/state';
    import Sidebar from '$components/Sidebar.svelte';
    import { KeyHintBar, keyboard, theme } from 'anasthasia';
    import { mountedHint } from '$lib/keyhint.svelte';
    import { sidebar } from '$lib/sidebar/state.svelte';
    import { wizard } from '$lib/wizard/state.svelte';
    import { applyUiPrefs, loadSettings } from '$lib/settings';

    let { children } = $props();

    type NavRoute = { combo: string; label: string; route: string };

    const navRoutes: NavRoute[] = [
        { combo: 'meta+1', label: 'Home', route: '/' },
        { combo: 'meta+2', label: 'Convert', route: '/convert' },
        { combo: 'meta+3', label: 'Settings', route: '/settings' },
        { combo: 'meta+4', label: 'About', route: '/about' },
    ];

    let navHints = $derived(
        wizard.converting
            ? ([['meta+keyb', 'Sidebar']] as [string, string][])
            : (
                  navRoutes
                      .filter(({ route }) =>
                          route === '/'
                              ? page.url.pathname !== '/'
                              : !page.url.pathname.startsWith(route)
                      )
                      .map(({ combo, label }) => [combo, label]) as [string, string][]
              ).concat([['meta+keyb', 'Sidebar']])
    );

    onMount(() => {
        theme.init();
        applyUiPrefs(loadSettings());

        const unmount = keyboard.mount();
        const cleanup = keyboard.smartRegister([
            ...navRoutes.map(
                ({ combo, route }) =>
                    [
                        combo,
                        () => {
                            if (wizard.converting) return false;
                            goto(route);
                            return true;
                        },
                    ] as [string, () => boolean]
            ),
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
    class="flex h-screen flex-col overflow-hidden bg-anasthasia-bg text-anasthasia-text"
    use:mountedHint={navHints}
>
    <!-- macOS title bar: sits behind traffic lights, draggable -->
    <div
        class="titlebar relative z-[110] h-8 shrink-0 border-b border-anasthasia-border bg-anasthasia-surface"
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
