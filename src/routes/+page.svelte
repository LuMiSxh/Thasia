<script lang="ts">
    import { getVersion } from '@tauri-apps/api/app';
    import { onMount } from 'svelte';
    import { IconZip, IconSettings, IconInfoCircle, IconChevronRight } from '@tabler/icons-svelte';

    let version = $state('...');
    onMount(async () => {
        try {
            version = await getVersion();
        } catch {
            version = '?';
        }
    });

    const cardClass =
        'group flex flex-1 flex-col gap-3 rounded-xl border border-anasthasia-border bg-anasthasia-surface ' +
        'p-5 transition-all duration-150 hover:border-anasthasia-accent/40 hover:bg-anasthasia-panel active:translate-y-px';

    const pfpUrl = '/brand/pfp.avif';
</script>

<div class="flex h-full flex-col items-center justify-center gap-12 p-12">
    <!-- Wordmark -->
    <div class="flex flex-col items-center gap-3 text-center">
        <img
            src={pfpUrl}
            alt="App avatar"
            class="h-20 w-20 rounded-2xl border border-anasthasia-border bg-anasthasia-panel object-cover"
            onerror={(e) => {
                // If the one-off branding script hasn't been run yet, keep the UI clean.
                (e.currentTarget as HTMLImageElement).style.display = 'none';
            }}
        />
        <div class="text-accent-gradient text-5xl font-bold tracking-tight">Thasia</div>
        <div class="text-xs font-bold tracking-[0.3em] text-anasthasia-muted uppercase">
            Manga Processing Engine
        </div>
        <div
            class="rounded-md border border-anasthasia-border bg-anasthasia-panel px-2 py-0.5 font-mono text-xs text-anasthasia-muted"
        >
            v{version}
        </div>
    </div>

    <!-- Action cards -->
    <div class="flex w-full max-w-2xl gap-4">
        <a href="/convert" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-anasthasia-accent/20 bg-anasthasia-accent/10"
                >
                    <IconZip size={18} class="text-anasthasia-accent" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-anasthasia-text">Start Converting</div>
                <div class="mt-0.5 text-xs text-anasthasia-muted">Launch the conversion wizard</div>
            </div>
        </a>

        <a href="/settings" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel"
                >
                    <IconSettings size={18} class="text-anasthasia-muted" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-anasthasia-text">Settings</div>
                <div class="mt-0.5 text-xs text-anasthasia-muted">Set defaults for the wizard</div>
            </div>
        </a>

        <a href="/about" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-panel"
                >
                    <IconInfoCircle size={18} class="text-anasthasia-muted" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-anasthasia-border transition-colors duration-150 group-hover:text-anasthasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-anasthasia-text">About</div>
                <div class="mt-0.5 text-xs text-anasthasia-muted">The story behind Thasia</div>
            </div>
        </a>
    </div>

    <!-- Accent line decoration -->
    <div
        class="h-px w-full bg-gradient-to-r from-transparent via-anasthasia-accent/40 to-transparent"
    ></div>
</div>
