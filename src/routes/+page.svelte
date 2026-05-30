<script lang="ts">
    import { getVersion } from '@tauri-apps/api/app';
    import { dev } from '$app/environment';
    import { onMount } from 'svelte';
    import {
        IconZip,
        IconSettings,
        IconSearch,
        IconArrowRight,
        IconChevronRight,
        IconServer,
    } from '@tabler/icons-svelte';
    import { Badge, Kbd } from 'anasthasia';
    import { commands, type RuntimeState } from '$types/bindings';
    import pfpUrl from '$assets/pfp.avif';

    let version = $state('...');
    let runtime = $state<RuntimeState>({ state: 'not_running' });
    let primaryShortcutLabel = $state('⌘');

    onMount(async () => {
        primaryShortcutLabel = navigator.platform.startsWith('Mac') ? '⌘' : 'Ctrl';
        try {
            version = await getVersion();
        } catch {
            version = '?';
        }
        const status = await commands.suwayomiStatus();
        if (status.status === 'ok') runtime = status.data;
    });

    const secondaryCard =
        'group flex min-w-0 flex-1 items-center gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface/80 backdrop-blur ' +
        'p-3.5 transition-all duration-150 hover:border-anasthasia-accent/40 hover:bg-anasthasia-panel active:translate-y-px';

    function runtimeBadge(state: RuntimeState): 'default' | 'success' | 'warning' | 'danger' {
        if (state.state === 'ready') return 'success';
        if (state.state === 'starting') return 'warning';
        if (state.state === 'error') return 'danger';
        return 'default';
    }

    function runtimeLabel(state: RuntimeState) {
        if (state.state === 'ready') return dev ? `Ready:${state.port}` : 'Ready';
        if (state.state === 'starting') return 'Starting';
        if (state.state === 'not_installed') return 'Setup needed';
        if (state.state === 'error') return 'Error';
        return 'Stopped';
    }
</script>

<div class="relative flex h-full overflow-hidden">
    <!-- Character — large background, anchored to the right -->
    <div
        class="pointer-events-none absolute inset-y-0 right-0 z-0 hidden md:block"
        aria-hidden="true"
    >
        <!-- Soft accent halo behind the head -->
        <div
            class="absolute top-[18%] right-[12%] h-[40%] w-[40%] rounded-full bg-anasthasia-accent/40 blur-3xl"
        ></div>
        <img
            src={pfpUrl}
            alt=""
            class="relative h-full w-auto max-w-none [mask-image:linear-gradient(to_left,black_60%,transparent_100%)] object-cover
                   object-left opacity-90
                   [-webkit-mask-image:linear-gradient(to_left,black_60%,transparent_100%)]
                   dark:opacity-95"
        />
    </div>

    <!-- Subtle bottom edge gradient to anchor content -->
    <div
        class="pointer-events-none absolute inset-x-0 bottom-0 z-0 h-32 bg-gradient-to-t from-anasthasia-bg to-transparent"
        aria-hidden="true"
    ></div>

    <!-- Content -->
    <div
        class="relative z-10 flex h-full w-full flex-col justify-center gap-8 px-14 py-10 md:max-w-xl"
    >
        <!-- Wordmark -->
        <div class="flex flex-col items-start gap-2.5">
            <div class="flex flex-wrap items-center gap-2">
                <Badge variant="mono">v{version}</Badge>
                <Badge variant={runtimeBadge(runtime)}>
                    <IconServer size={12} />
                    {runtimeLabel(runtime)}
                </Badge>
            </div>
            <div class="text-accent-gradient text-7xl leading-none font-bold tracking-tight">
                Thasia
            </div>
            <div class="text-[11px] font-bold tracking-[0.35em] text-anasthasia-muted uppercase">
                Manga Processing Engine
            </div>
        </div>

        <div class="grid gap-3">
            <a
                href="/convert"
                class="group relative flex items-center justify-between gap-4 overflow-hidden rounded-xl
                       border border-anasthasia-accent/30 bg-anasthasia-surface/85 p-5
                       backdrop-blur transition-all duration-150
                       hover:border-anasthasia-accent/60 hover:bg-anasthasia-panel active:translate-y-px"
            >
                <span
                    class="pointer-events-none absolute inset-0 bg-accent-gradient opacity-[0.06] transition-opacity duration-200 group-hover:opacity-[0.14]"
                    aria-hidden="true"
                ></span>

                <div class="relative flex items-center gap-4">
                    <div
                        class="flex h-11 w-11 items-center justify-center rounded-lg border border-anasthasia-accent/30 bg-anasthasia-accent/10"
                    >
                        <IconZip size={22} class="text-anasthasia-accent" />
                    </div>
                    <div class="text-left">
                        <div class="text-base font-bold text-anasthasia-text">
                            Convert local manga
                        </div>
                        <div class="mt-0.5 text-xs text-anasthasia-muted">
                            Folder, ZIP, or CBZ into reader-ready output
                        </div>
                    </div>
                </div>

                <IconArrowRight
                    size={18}
                    class="relative text-anasthasia-muted transition-all duration-150 group-hover:translate-x-0.5 group-hover:text-anasthasia-accent"
                />
            </a>

            <div class="flex gap-3">
                <a href="/discover" class={secondaryCard}>
                    <div
                        class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel"
                    >
                        <IconSearch
                            size={16}
                            class="text-anasthasia-muted transition-colors duration-150 group-hover:text-anasthasia-accent"
                        />
                    </div>
                    <div class="min-w-0 flex-1">
                        <div class="text-sm font-bold text-anasthasia-text">Discover</div>
                        <div class="truncate text-xs text-anasthasia-muted">Catalogs/Downloads</div>
                    </div>
                    <IconChevronRight
                        size={14}
                        class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                    />
                </a>
                <a href="/settings" class={secondaryCard}>
                    <div
                        class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel"
                    >
                        <IconSettings
                            size={16}
                            class="text-anasthasia-muted transition-colors duration-150 group-hover:text-anasthasia-accent"
                        />
                    </div>
                    <div class="min-w-0 flex-1">
                        <div class="text-sm font-bold text-anasthasia-text">Settings</div>
                        <div class="truncate text-xs text-anasthasia-muted">Defaults</div>
                    </div>
                    <IconChevronRight
                        size={14}
                        class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                    />
                </a>
            </div>
        </div>
        <div class="flex items-center gap-2 text-xs text-anasthasia-muted">
            <span>Or press</span>
            <Kbd>{primaryShortcutLabel}</Kbd>
            <Kbd>2</Kbd>
            <span>to jump straight in</span>
        </div>
    </div>
</div>
