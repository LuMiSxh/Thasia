<script lang="ts">
    import { getVersion } from '@tauri-apps/api/app';
    import { onMount } from 'svelte';
    import {
        IconZip,
        IconSettings,
        IconInfoCircle,
        IconArrowRight,
        IconChevronRight,
    } from '@tabler/icons-svelte';
    import { Badge, Kbd } from 'anasthasia';
    import pfpUrl from '$assets/pfp.avif';

    let version = $state('...');
    onMount(async () => {
        try {
            version = await getVersion();
        } catch {
            version = '?';
        }
    });

    const secondaryCard =
        'group flex flex-1 items-center gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface/80 backdrop-blur ' +
        'p-3.5 transition-all duration-150 hover:border-anasthasia-accent/40 hover:bg-anasthasia-panel active:translate-y-px';
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
        class="relative z-10 flex h-full w-full flex-col justify-center gap-10 px-14 py-12 md:max-w-xl"
    >
        <!-- Wordmark -->
        <div class="flex flex-col items-start gap-3">
            <Badge variant="mono">v{version}</Badge>
            <div class="text-accent-gradient text-7xl leading-none font-bold tracking-tight">
                Thasia
            </div>
            <div class="text-[11px] font-bold tracking-[0.35em] text-anasthasia-muted uppercase">
                Manga Processing Engine
            </div>
        </div>

        <!-- Primary CTA -->
        <a
            href="/convert"
            class="group relative flex items-center justify-between gap-4 overflow-hidden rounded-xl
                   border border-anasthasia-accent/30 bg-anasthasia-surface/80 p-5
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
                    <div class="text-base font-bold text-anasthasia-text">Start Converting</div>
                    <div class="mt-0.5 text-xs text-anasthasia-muted">
                        Launch the wizard and process pages
                    </div>
                </div>
            </div>

            <IconArrowRight
                size={18}
                class="relative text-anasthasia-muted transition-all duration-150 group-hover:translate-x-0.5 group-hover:text-anasthasia-accent"
            />
        </a>

        <!-- Secondary actions -->
        <div class="flex gap-3">
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
                    <div class="truncate text-xs text-anasthasia-muted">Wizard defaults</div>
                </div>
                <IconChevronRight
                    size={14}
                    class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                />
            </a>
        </div>

        <!-- Keyboard hint -->
        <div class="flex items-center gap-2 text-xs text-anasthasia-muted">
            <span>Or press</span>
            <Kbd>⌘</Kbd>
            <Kbd>2</Kbd>
            <span>to jump straight in</span>
        </div>
    </div>
</div>
