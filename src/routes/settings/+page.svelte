<script lang="ts">
    import { wizard } from '$lib/wizard/state.svelte';
    import { uiPrefs } from '$lib/ui-prefs.svelte';
    import { onMount, onDestroy } from 'svelte';
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import {
        IconCheck,
        IconPhoto,
        IconFileZip,
        IconDirection,
        IconStack,
        IconFolderPlus,
        IconRuler,
        IconKeyboard,
    } from '@tabler/icons-svelte';
    import { Button, Toggle, Input, SegmentedControl } from '$components/ui/index';
    import { duration } from '$lib/transitions';

    const KEY = 'thasia:settings';

    type Defaults = {
        imageFormat: 'avif' | 'webp' | 'original';
        container: 'cbz' | 'epub' | 'raw';
        direction: 'ltr' | 'rtl';
        bundle: 'auto' | 'flatten';
        volumeSeparator: string;
        hideSingleVolume: boolean;
        createDirectory: boolean;
        maxWidth: number | null;
        showKeyHints: boolean;
    };

    let defaults = $state<Defaults>({
        imageFormat: 'avif',
        container: 'cbz',
        direction: 'ltr',
        bundle: 'auto',
        volumeSeparator: ' - ',
        hideSingleVolume: false,
        createDirectory: false,
        maxWidth: null,
        showKeyHints: true,
    });

    let saved = $state(false);
    let maxWidthEnabled = $state(false);
    let savedTimer: ReturnType<typeof setTimeout> | undefined;

    onDestroy(() => clearTimeout(savedTimer));

    onMount(() => {
        const raw = localStorage.getItem(KEY);
        if (raw) {
            try {
                const parsed = JSON.parse(raw);
                Object.assign(defaults, parsed);
                maxWidthEnabled = defaults.maxWidth !== null;
            } catch {}
        }
    });

    function save() {
        if (!maxWidthEnabled) defaults.maxWidth = null;
        localStorage.setItem(KEY, JSON.stringify(defaults));
        wizard.imageFormat = defaults.imageFormat;
        wizard.container = defaults.container;
        wizard.direction = defaults.direction;
        wizard.bundle = defaults.bundle;
        wizard.volumeSeparator = defaults.volumeSeparator;
        wizard.hideSingleVolume = defaults.hideSingleVolume;
        wizard.createDirectory = defaults.createDirectory;
        wizard.maxWidth = defaults.maxWidth;
        uiPrefs.showKeyHints = defaults.showKeyHints;
        saved = true;
        clearTimeout(savedTimer);
        savedTimer = setTimeout(() => (saved = false), 2000);
    }

    const collapse = { duration: duration.base, easing: cubicInOut };

    const formatHint: Record<string, string> = {
        avif: 'Best compression, slower — ideal for archiving',
        webp: 'Good compression, widely supported',
        original: 'No re-encoding — fastest, preserves originals',
    };
</script>

<div class="flex h-full flex-col overflow-hidden">
    <div
        class="mx-auto flex min-h-0 w-full max-w-5xl flex-1 flex-col gap-6 overflow-y-auto px-8 py-8"
    >
        <!-- Header -->
        <div class="flex flex-shrink-0 items-center justify-between">
            <div>
                <h1 class="text-xl font-bold">Settings</h1>
                <p class="mt-0.5 text-sm text-thasia-muted">
                    Default values pre-filled in each new conversion
                </p>
            </div>
            <Button variant="primary" size="lg" onclick={save}>
                {#if saved}<IconCheck size={16} />{/if}
                {saved ? 'Saved' : 'Save defaults'}
            </Button>
        </div>

        <!-- Two section panels -->
        <div class="grid grid-cols-2 gap-4">
            <!-- LEFT: Encoding -->
            <div
                class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface"
            >
                <div
                    class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5"
                >
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Encoding</span
                    >
                </div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconPhoto size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Image Format</span>
                    </div>
                    <SegmentedControl
                        options={[
                            { value: 'avif', label: 'AVIF' },
                            { value: 'webp', label: 'WebP' },
                            { value: 'original', label: 'Original' },
                        ]}
                        bind:value={defaults.imageFormat}
                    />
                    <p class="text-xs text-thasia-muted">{formatHint[defaults.imageFormat]}</p>
                </div>

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center gap-2">
                            <IconRuler size={14} class="flex-shrink-0 text-thasia-muted" />
                            <div>
                                <div class="text-sm font-medium">Max Width</div>
                                <div class="text-xs text-thasia-muted">
                                    Downscale wider images (px)
                                </div>
                            </div>
                        </div>
                        <Toggle
                            bind:checked={maxWidthEnabled}
                            onchange={(v) => {
                                if (!v) defaults.maxWidth = null;
                                else defaults.maxWidth = 1920;
                            }}
                        />
                    </div>
                    {#if maxWidthEnabled}
                        <div transition:slide={collapse}>
                            <Input
                                type="number"
                                min="100"
                                max="9999"
                                bind:value={defaults.maxWidth as number}
                                hint="Common values: 1200, 1440, 1920"
                            />
                        </div>
                    {/if}
                </div>
            </div>

            <!-- RIGHT: Output -->
            <div
                class="flex flex-col overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface"
            >
                <div
                    class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5"
                >
                    <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                        >Output</span
                    >
                </div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconFileZip size={14} class="flex-shrink-0 text-thasia-muted" />
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
                        <div class="flex items-center justify-between" transition:slide={collapse}>
                            <div class="flex items-center gap-1.5">
                                <IconDirection size={13} class="flex-shrink-0 text-thasia-muted" />
                                <span class="text-xs text-thasia-muted">Reading direction</span>
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

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex flex-col gap-2.5 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconStack size={14} class="flex-shrink-0 text-thasia-muted" />
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

                <div class="mx-4 border-t border-thasia-border"></div>

                <div class="flex items-center justify-between gap-4 px-4 py-4">
                    <div class="flex items-center gap-2">
                        <IconFolderPlus size={14} class="flex-shrink-0 text-thasia-muted" />
                        <span class="text-sm font-medium">Create subdirectory</span>
                    </div>
                    <Toggle bind:checked={defaults.createDirectory} />
                </div>
            </div>
        </div>

        <!-- Interface panel -->
        <div
            class="flex-shrink-0 overflow-hidden rounded-xl border border-thasia-border bg-thasia-surface"
        >
            <div class="flex-shrink-0 border-b border-thasia-border bg-thasia-panel px-4 py-2.5">
                <span class="text-[10px] font-bold tracking-widest text-thasia-muted uppercase"
                    >Interface</span
                >
            </div>
            <div class="flex items-center justify-between px-4 py-4">
                <div class="flex items-center gap-2">
                    <IconKeyboard size={14} class="flex-shrink-0 text-thasia-muted" />
                    <div>
                        <div class="text-sm font-medium">Keyboard hint bar</div>
                        <div class="text-xs text-thasia-muted">
                            Show shortcut hints at the bottom of the window
                        </div>
                    </div>
                </div>
                <Toggle bind:checked={defaults.showKeyHints} />
            </div>
        </div>
    </div>
</div>
