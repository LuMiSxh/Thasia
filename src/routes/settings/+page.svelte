<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { loadSettings, saveSettings, applyToWizard, type Settings } from '$lib/settings';
    import { duration, Input, SegmentedControl, Toggle, uiPrefs } from 'anasthasia';
    import { onDestroy, onMount } from 'svelte';
    import { fade, slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { open } from '@tauri-apps/plugin-dialog';
    import {
        IconCheck,
        IconFileZip,
        IconDirection,
        IconStack,
        IconFolderPlus,
        IconFolderOpen,
        IconKeyboard,
        IconX,
    } from '@tabler/icons-svelte';
    import EncodingControls from '$components/wizard/EncodingControls.svelte';

    const initial = loadSettings();
    let defaults = $state<Settings>(initial);
    let maxWidthEnabled = $state(initial.maxWidth !== null);

    let savedPulse = $state(false);
    let pulseTimer: ReturnType<typeof setTimeout> | undefined;
    let saveTimer: ReturnType<typeof setTimeout> | undefined;

    /** Don't run autosave on the initial $effect that fires from $state setup. */
    let primed = $state(false);

    onMount(() => {
        // Defer "primed" until after the initial effect run, so loadSettings()
        // doesn't immediately re-save itself.
        queueMicrotask(() => (primed = true));
    });

    onDestroy(() => {
        clearTimeout(pulseTimer);
        clearTimeout(saveTimer);
    });

    function commit() {
        if (!maxWidthEnabled) defaults.maxWidth = null;
        saveSettings(defaults);
        applyToWizard(defaults, { force: true });
        uiPrefs.showKeyHints = defaults.showKeyHints;
        wizard.createDirectory = defaults.createDirectory;
        savedPulse = true;
        clearTimeout(pulseTimer);
        pulseTimer = setTimeout(() => (savedPulse = false), 1500);
    }

    function scheduleAutosave() {
        clearTimeout(saveTimer);
        saveTimer = setTimeout(commit, 200);
    }

    // Watch the full defaults object — any nested mutation triggers a debounced save.
    $effect(() => {
        // Touch every field so the effect tracks them all.
        void [
            defaults.imageFormat,
            defaults.container,
            defaults.direction,
            defaults.bundle,
            defaults.volumeSeparator,
            defaults.hideSingleVolume,
            defaults.createDirectory,
            defaults.maxWidth,
            defaults.forceReencode,
            defaults.cleanTones,
            defaults.showKeyHints,
            defaults.defaultOutputDir,
            maxWidthEnabled,
        ];
        if (!primed) return;
        scheduleAutosave();
    });

    const collapse = { duration: duration.base, easing: cubicInOut };

    async function pickDefaultOutputDir() {
        const selected = await open({ directory: true, title: 'Select default output folder' });
        if (typeof selected === 'string') defaults.defaultOutputDir = selected;
    }
</script>

<svelte:window
    onkeydown={(e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 's') {
            e.preventDefault();
            clearTimeout(saveTimer);
            commit();
        }
    }}
/>

<div class="flex h-full flex-col">
    <!-- Header — title + inline saved indicator -->
    <div
        class="flex flex-shrink-0 items-baseline justify-between gap-4 border-b border-anasthasia-border px-8 py-5"
    >
        <div>
            <h1 class="text-xl font-bold">Settings</h1>
            <p class="mt-0.5 text-sm text-anasthasia-muted">Conversion defaults and interface</p>
        </div>
        {#if savedPulse}
            <span
                class="inline-flex items-center gap-1.5 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-2 py-1 text-xs font-bold text-emerald-500"
                in:fade={{ duration: duration.fast }}
                out:fade={{ duration: duration.base }}
            >
                <IconCheck size={12} stroke={3} /> Saved
            </span>
        {/if}
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col overflow-y-auto">
        <div class="flex w-full flex-col gap-4 px-6 py-5">
            <div class="grid gap-4 xl:grid-cols-[minmax(18rem,0.8fr)_minmax(0,1.6fr)]">
                <div class="grid gap-4 md:grid-cols-2 xl:flex xl:flex-col">
                    <div
                        class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                    >
                        <div
                            class="flex-shrink-0 border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5"
                        >
                            <span
                                class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >
                                Destination
                            </span>
                        </div>
                        <div class="flex flex-col gap-2.5 px-4 py-4">
                            <div class="flex items-center gap-2">
                                <IconFolderOpen
                                    size={14}
                                    class="flex-shrink-0 text-anasthasia-muted"
                                />
                                <div>
                                    <div class="text-sm font-medium">Default output folder</div>
                                    <div class="text-xs text-anasthasia-muted">
                                        Pre-filled in the wizard when starting a new conversion
                                    </div>
                                </div>
                            </div>
                            <div class="flex gap-2">
                                <div
                                    class="flex h-9 min-w-0 flex-1 items-center rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 font-mono text-xs
                                {defaults.defaultOutputDir
                                        ? 'text-anasthasia-text'
                                        : 'text-anasthasia-muted'}"
                                    title={defaults.defaultOutputDir || undefined}
                                >
                                    <span class="truncate">
                                        {defaults.defaultOutputDir ||
                                            'No default - wizard starts empty'}
                                    </span>
                                </div>
                                {#if defaults.defaultOutputDir}
                                    <button
                                        onclick={() => (defaults.defaultOutputDir = '')}
                                        title="Clear default"
                                        aria-label="Clear default output folder"
                                        class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg border border-anasthasia-border bg-anasthasia-bg text-anasthasia-muted transition-colors duration-150 hover:border-anasthasia-accent/40 hover:text-anasthasia-text"
                                    >
                                        <IconX size={13} />
                                    </button>
                                {/if}
                                <button
                                    onclick={pickDefaultOutputDir}
                                    class="flex-shrink-0 rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 text-sm text-anasthasia-text transition-colors duration-150 hover:border-anasthasia-accent/40"
                                >
                                    Browse…
                                </button>
                            </div>
                        </div>
                    </div>

                    <div
                        class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                    >
                        <div
                            class="flex-shrink-0 border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5"
                        >
                            <span
                                class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >
                                Interface
                            </span>
                        </div>
                        <div class="flex items-center justify-between px-4 py-4">
                            <div class="flex items-center gap-2">
                                <IconKeyboard
                                    size={14}
                                    class="flex-shrink-0 text-anasthasia-muted"
                                />
                                <div>
                                    <div class="text-sm font-medium">Keyboard hint bar</div>
                                    <div class="text-xs text-anasthasia-muted">
                                        Show shortcut hints at the bottom of the window
                                    </div>
                                </div>
                            </div>
                            <Toggle bind:checked={defaults.showKeyHints} />
                        </div>
                    </div>
                </div>

                <div
                    class="grid gap-4 lg:grid-cols-2 2xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]"
                >
                    <EncodingControls
                        bind:format={defaults.imageFormat}
                        bind:maxWidth={defaults.maxWidth}
                        bind:enableMaxWidth={maxWidthEnabled}
                        bind:forceReencode={defaults.forceReencode}
                        bind:cleanTones={defaults.cleanTones}
                    />

                    <div
                        class="flex flex-col overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
                    >
                        <div
                            class="flex-shrink-0 border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5"
                        >
                            <span
                                class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase"
                            >
                                Output
                            </span>
                        </div>

                        <div class="flex flex-col gap-2.5 px-4 py-3">
                            <div class="flex items-center gap-2">
                                <IconFileZip
                                    size={14}
                                    class="flex-shrink-0 text-anasthasia-muted"
                                />
                                <span class="text-sm font-medium">Container</span>
                            </div>
                            <SegmentedControl
                                options={[
                                    { value: 'cbz', label: 'CBZ' },
                                    { value: 'epub', label: 'EPUB' },
                                    { value: 'raw', label: 'Raw' },
                                ]}
                                bind:value={defaults.container}
                            />
                            {#if defaults.container === 'epub'}
                                <div
                                    class="flex items-center justify-between"
                                    transition:slide={collapse}
                                >
                                    <div class="flex items-center gap-1.5">
                                        <IconDirection
                                            size={13}
                                            class="flex-shrink-0 text-anasthasia-muted"
                                        />
                                        <span class="text-xs text-anasthasia-muted">
                                            Reading direction
                                        </span>
                                    </div>
                                    <SegmentedControl
                                        options={[
                                            { value: 'ltr', label: 'LTR' },
                                            { value: 'rtl', label: 'RTL' },
                                        ]}
                                        bind:value={defaults.direction}
                                    />
                                </div>
                            {/if}
                        </div>

                        <div class="mx-4 border-t border-anasthasia-border"></div>

                        <div class="flex flex-col gap-2.5 px-4 py-3">
                            <div class="flex items-center gap-2">
                                <IconStack size={14} class="flex-shrink-0 text-anasthasia-muted" />
                                <span class="text-sm font-medium">Bundling</span>
                            </div>
                            <SegmentedControl
                                options={[
                                    { value: 'auto', label: 'Auto' },
                                    { value: 'flatten', label: 'Flatten' },
                                ]}
                                bind:value={defaults.bundle}
                            />
                            {#if defaults.bundle === 'auto'}
                                <div class="flex flex-col gap-3" transition:slide={collapse}>
                                    <Input
                                        label="Volume separator"
                                        bind:value={defaults.volumeSeparator}
                                        hint={`e.g. "Manga${defaults.volumeSeparator}1"`}
                                    />
                                    <Toggle
                                        bind:checked={defaults.hideSingleVolume}
                                        label="Omit volume number when only one volume"
                                    />
                                </div>
                            {/if}
                        </div>

                        <div class="mx-4 border-t border-anasthasia-border"></div>

                        <div class="flex items-center justify-between gap-4 px-4 py-3">
                            <div class="flex items-center gap-2">
                                <IconFolderPlus
                                    size={14}
                                    class="flex-shrink-0 text-anasthasia-muted"
                                />
                                <span class="text-sm font-medium">Create subdirectory</span>
                            </div>
                            <Toggle bind:checked={defaults.createDirectory} />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</div>
