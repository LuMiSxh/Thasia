<script lang="ts">
    import { slide } from 'svelte/transition';
    import { cubicInOut } from 'svelte/easing';
    import { duration, Input, SegmentedControl, Toggle } from 'anasthasia';
    import { IconEraser, IconPhoto, IconRefresh, IconRuler } from '@tabler/icons-svelte';

    interface Props {
        format: 'avif' | 'webp' | 'original';
        maxWidth: number | null;
        enableMaxWidth: boolean;
        forceReencode: boolean;
        cleanTones: boolean;
    }

    let {
        format = $bindable(),
        maxWidth = $bindable(),
        enableMaxWidth = $bindable(),
        forceReencode = $bindable(),
        cleanTones = $bindable(),
    }: Props = $props();

    const collapse = { duration: duration.base, easing: cubicInOut };

    const formatHint: Record<Props['format'], string> = {
        avif: 'Best compression, slower — ideal for archiving',
        webp: 'Good compression, widely supported',
        original: 'No re-encoding — fastest, preserves originals',
    };

    $effect(() => {
        if (format === 'original') {
            forceReencode = false;
            cleanTones = false;
        }
    });
</script>

<div class="overflow-hidden rounded-xl border border-anasthasia-border bg-anasthasia-surface">
    <div class="border-b border-anasthasia-border bg-anasthasia-panel px-4 py-2.5">
        <span class="text-[10px] font-bold tracking-widest text-anasthasia-muted uppercase">
            Encoding
        </span>
    </div>

    <!-- Format -->
    <div class="flex flex-col gap-2.5 px-4 py-3">
        <div class="flex items-center gap-2">
            <IconPhoto size={14} class="flex-shrink-0 text-anasthasia-muted" />
            <span class="text-sm font-medium">Format</span>
        </div>
        <SegmentedControl
            options={[
                { value: 'avif', label: 'AVIF' },
                { value: 'webp', label: 'WebP' },
                { value: 'original', label: 'Original' },
            ]}
            bind:value={format}
        />
        <p class="text-xs text-anasthasia-muted">{formatHint[format]}</p>
    </div>

    <div class="mx-4 border-t border-anasthasia-border"></div>

    <!-- Re-encode -->
    <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div class="flex items-center gap-2">
            <IconRefresh size={14} class="flex-shrink-0 text-anasthasia-muted" />
            <div>
                <div class="text-sm font-medium">Force re-encode</div>
                <div class="text-xs text-anasthasia-muted">
                    Do not pass through matching formats
                </div>
            </div>
        </div>
        <Toggle bind:checked={forceReencode} disabled={format === 'original'} />
    </div>

    <div class="mx-4 border-t border-anasthasia-border"></div>

    <!-- Cleanup -->
    <div class="flex items-center justify-between gap-3 px-4 py-3">
        <div class="flex items-center gap-2">
            <IconEraser size={14} class="flex-shrink-0 text-anasthasia-muted" />
            <div>
                <div class="text-sm font-medium">Clean scan tones</div>
                <div class="text-xs text-anasthasia-muted">Normalize paper white and ink black</div>
            </div>
        </div>
        <Toggle bind:checked={cleanTones} disabled={format === 'original'} />
    </div>

    <div class="mx-4 border-t border-anasthasia-border"></div>

    <!-- Max width -->
    <div class="flex flex-col gap-2.5 px-4 py-3">
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
                <IconRuler size={14} class="flex-shrink-0 text-anasthasia-muted" />
                <div>
                    <div class="text-sm font-medium">Max Width</div>
                    <div class="text-xs text-anasthasia-muted">Downscale wider images (px)</div>
                </div>
            </div>
            <Toggle
                bind:checked={enableMaxWidth}
                onchange={(v) => {
                    maxWidth = v ? 1920 : null;
                }}
            />
        </div>
        {#if enableMaxWidth}
            <div transition:slide={collapse}>
                <Input
                    type="number"
                    min="400"
                    max="4000"
                    step="100"
                    bind:value={maxWidth as number}
                    hint="Common values: 1200, 1440, 1920"
                />
            </div>
        {/if}
    </div>
</div>
