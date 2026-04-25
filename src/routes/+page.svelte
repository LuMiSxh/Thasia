<script lang="ts">
    import { getVersion } from '@tauri-apps/api/app';
    import { onMount } from 'svelte';
    import { IconZip, IconSettings, IconInfoCircle, IconChevronRight } from '@tabler/icons-svelte';

    let version = $state('...');
    onMount(async () => {
        try { version = await getVersion(); } catch { version = '?' }
    });

    const cardClass =
        'group flex flex-1 flex-col gap-3 rounded-xl border border-thasia-border bg-thasia-surface ' +
        'p-5 transition-all duration-150 hover:border-thasia-accent/40 hover:bg-thasia-panel active:translate-y-px';
</script>

<div class="flex h-full flex-col items-center justify-center gap-12 p-12">
    <!-- Wordmark -->
    <div class="flex flex-col items-center gap-3 text-center">
        <div class="text-5xl font-bold tracking-tight text-thasia-text">Thasia</div>
        <div class="text-xs font-bold tracking-[0.3em] text-thasia-muted uppercase">
            Manga Processing Engine
        </div>
        <div
            class="rounded-md border border-thasia-border bg-thasia-panel px-2 py-0.5 font-mono text-xs text-thasia-muted"
        >
            v{version}
        </div>
    </div>

    <!-- Action cards -->
    <div class="flex w-full max-w-2xl gap-4">
        <a href="/convert" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-thasia-accent/20 bg-thasia-accent/10"
                >
                    <IconZip size={18} class="text-thasia-accent" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-thasia-border transition-colors duration-150 group-hover:text-thasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-thasia-text">Start Converting</div>
                <div class="mt-0.5 text-xs text-thasia-muted">Launch the conversion wizard</div>
            </div>
        </a>

        <a href="/settings" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-thasia-border bg-thasia-panel"
                >
                    <IconSettings size={18} class="text-thasia-muted" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-thasia-border transition-colors duration-150 group-hover:text-thasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-thasia-text">Settings</div>
                <div class="mt-0.5 text-xs text-thasia-muted">Set defaults for the wizard</div>
            </div>
        </a>

        <a href="/about" class={cardClass}>
            <div class="flex items-center justify-between">
                <div
                    class="flex h-9 w-9 items-center justify-center rounded-lg border border-thasia-border bg-thasia-panel"
                >
                    <IconInfoCircle size={18} class="text-thasia-muted" />
                </div>
                <IconChevronRight
                    size={16}
                    class="text-thasia-border transition-colors duration-150 group-hover:text-thasia-accent"
                />
            </div>
            <div>
                <div class="text-sm font-bold text-thasia-text">About</div>
                <div class="mt-0.5 text-xs text-thasia-muted">The story behind Thasia</div>
            </div>
        </a>
    </div>

    <!-- Accent line decoration -->
    <div
        class="h-px w-full bg-gradient-to-r from-transparent via-thasia-accent/40 to-transparent"
    ></div>
</div>
