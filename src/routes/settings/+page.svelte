<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { loadSettings, saveSettings, applyToWizard, type Settings } from '$lib/settings';
    import { Button, duration, Input, SegmentedControl, Toggle, uiPrefs } from 'anasthasia';
    import { onDestroy } from 'svelte';
    import { slide } from 'svelte/transition';
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
    let saved = $state(false);
    let savedTimer: ReturnType<typeof setTimeout> | undefined;

    onDestroy(() => clearTimeout(savedTimer));

    function save() {
        if (!maxWidthEnabled) defaults.maxWidth = null;
        saveSettings(defaults);
        applyToWizard(defaults, { force: true });
        uiPrefs.showKeyHints = defaults.showKeyHints;
        wizard.createDirectory = defaults.createDirectory;
        saved = true;
        clearTimeout(savedTimer);
        savedTimer = setTimeout(() => (saved = false), 2000);
    }

    const collapse = { duration: duration.base, easing: cubicInOut };

    async function pickDefaultOutputDir() {
        const selected = await open({ directory: true, title: 'Select default output folder' });
        if (typeof selected === 'string') defaults.defaultOutputDir = selected;
    }
</script>

<div class="flex h-full flex-col">
    <!-- Header -->
    <div class="flex-shrink-0 border-b border-anasthasia-border px-8 py-5">
        <h1 class="text-xl font-bold">Settings</h1>
        <p class="mt-0.5 text-sm text-anasthasia-muted">
            Default values pre-filled in each new conversion
        </p>
    </div>

    <!-- Content -->
    <div class="flex flex-1 flex-col overflow-y-auto">
        <div class="mx-auto flex w-full max-w-5xl flex-col gap-4 px-8 py-6">
            <!-- Default output directory -->
            <div
                class="flex-shrink-0 overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
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
                        <IconFolderOpen size={14} class="flex-shrink-0 text-anasthasia-muted" />
                        <div>
                            <div class="text-sm font-medium">Default output folder</div>
                            <div class="text-xs text-anasthasia-muted">
                                Pre-filled in the wizard when starting a new conversion
                            </div>
                        </div>
                    </div>
                    <div class="flex gap-2">
                        <div
                            class="flex min-h-9 flex-1 items-center rounded-lg border border-anasthasia-border bg-anasthasia-bg px-3 py-2 font-mono text-xs break-all
                                {defaults.defaultOutputDir
                                ? 'text-anasthasia-text'
                                : 'text-anasthasia-muted'}"
                        >
                            {defaults.defaultOutputDir || 'No default — wizard starts empty'}
                        </div>
                        {#if defaults.defaultOutputDir}
                            <Button
                                onclick={() => (defaults.defaultOutputDir = '')}
                                size="sm"
                                title="Clear default"
                            >
                                <IconX size={13} />
                            </Button>
                        {/if}
                        <Button onclick={pickDefaultOutputDir} size="sm">Browse…</Button>
                    </div>
                </div>
            </div>

            <!-- Two section panels -->
            <div class="grid grid-cols-2 gap-4">
                <!-- LEFT: Encoding -->
                <EncodingControls
                    bind:format={defaults.imageFormat}
                    bind:maxWidth={defaults.maxWidth}
                    bind:enableMaxWidth={maxWidthEnabled}
                />

                <!-- RIGHT: Output -->
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

                    <div class="flex flex-col gap-2.5 px-4 py-4">
                        <div class="flex items-center gap-2">
                            <IconFileZip size={14} class="flex-shrink-0 text-anasthasia-muted" />
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

                    <div class="flex flex-col gap-2.5 px-4 py-4">
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

                    <div class="flex items-center justify-between gap-4 px-4 py-4">
                        <div class="flex items-center gap-2">
                            <IconFolderPlus size={14} class="flex-shrink-0 text-anasthasia-muted" />
                            <span class="text-sm font-medium">Create subdirectory</span>
                        </div>
                        <Toggle bind:checked={defaults.createDirectory} />
                    </div>
                </div>
            </div>

            <!-- Interface panel -->
            <div
                class="flex-shrink-0 overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface"
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
                        <IconKeyboard size={14} class="flex-shrink-0 text-anasthasia-muted" />
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
    </div>

    <!-- Bottom toolbar -->
    <div class="flex flex-shrink-0 justify-end gap-2 border-t border-anasthasia-border px-8 py-4">
        <Button variant="primary" size="lg" onclick={save}>
            {#if saved}<IconCheck size={16} />{/if}
            {saved ? 'Saved' : 'Save defaults'}
        </Button>
    </div>
</div>
